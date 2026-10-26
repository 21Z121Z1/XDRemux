"""PVT harness facts; synthetic traces here do not establish Photos support."""
import json
import hashlib
import ast
import copy
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import Mock
from types import SimpleNamespace

from scripts.apple.pvt_native_import import check_native_import as probe
from scripts.apple.pvt_native_import.observations import check_reader_observation
from scripts.apple.pvt_native_import.owned_cleanup import cleanup_owned_simulator, observe_receipt_absence
import subprocess


HEAD = 'a' * 40
SESSION = 'org.xdremux.audit.PVTReadback.u' + 'b' * 32


def event(phase, boundary, elapsed, **facts):
    return 'PVT_STAGE_EVENT: ' + json.dumps({
        'schema_version': 1, 'source_head': HEAD, 'session_id': SESSION,
        'phase': phase, 'boundary': boundary, 'elapsed_seconds': elapsed, **facts})


class PVTObservationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        tree = ast.parse(Path(probe.__file__).read_text())
        main = next(node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name == 'main')
        outer = next(node for node in main.body if isinstance(node, ast.Try))
        handler = next(node for node in outer.handlers if isinstance(node.type, ast.Name) and node.type.id == 'Blocked')
        stop = next(node for node in outer.finalbody if isinstance(node, ast.Expr)
                    and isinstance(node.value, ast.Call) and isinstance(node.value.func, ast.Attribute)
                    and node.value.func.attr == 'stop_active')
        cls.finish_code = compile(ast.fix_missing_locations(ast.Module(
            body=copy.deepcopy([*handler.body, stop]), type_ignores=[])), probe.__file__, 'exec')
        reader = next(node for node in main.body if isinstance(node, ast.FunctionDef) and node.name == 'read_report')
        cls.reader_code = compile(ast.fix_missing_locations(ast.Module(
            body=[copy.deepcopy(reader)], type_ignores=[])), probe.__file__, 'exec')

    def finish_failure(self, stages, report, error):
        exec(self.finish_code, {**vars(probe), 'stages': stages, 'report': report, 'error': error})

    def test_photokit_poll_timeout_is_unknown_for_its_current_stage(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            stages = probe.PVTStages(HEAD, SESSION, ('baseline',))
            report = {'commands': []}
            clock = iter((0, 46))
            def run(name, command):
                report['commands'].append({'name': name, 'exit_code': 0})
                return subprocess.CompletedProcess(command, 0, b'', b'')
            namespace = {**vars(probe), 'stages': stages, 'report': report, 'run': run,
                         'args': SimpleNamespace(expected_head=HEAD), 'session_id': SESSION,
                         'owned_udid': 'owned', 'app_container': lambda *args: root, 'work': root,
                         'time': SimpleNamespace(monotonic=lambda: next(clock), sleep=lambda _: None)}
            exec(self.reader_code, namespace)
            try:
                namespace['read_report']('baseline', 'owned.bundle')
            except probe.Blocked as error:
                self.finish_failure(stages, report, error)
            else:
                self.fail('missing PhotoKit report must time out')
            self.assertEqual(stages.facts['phases']['baseline']['state'], 'unknown_after_timeout')
            self.assertEqual(stages.facts['admission'], 'unobserved')

    def test_recovered_install_timeout_cannot_relabel_later_explicit_failure(self):
        from tests.test_pvt_simulator_install import SimulatorInstallModel
        with tempfile.TemporaryDirectory() as raw:
            model = SimulatorInstallModel(Path(raw), 'timeout-installed')
            self.assertEqual(model.install()['status'], 'installed-verified-after-client-timeout')
            stages = probe.PVTStages(HEAD, SESSION, ('readiness', 'baseline'))
            stages.record('readiness', 'completed', 'reader_installed_and_photos_granted')
            stages.record('baseline', 'started', 'photokit_launch_submitted')
            stages.record('baseline', 'observed', 'photokit_report_received', native_status='denied')
            self.finish_failure(stages, model.report, probe.Blocked('baseline is not authorized'))
            self.assertEqual(stages.facts['phases']['readiness']['state'], 'completed')
            self.assertEqual(stages.facts['phases']['baseline']['state'], 'blocked')
            self.assertEqual(model.report['reader_installation']['install_client_outcome'], 'uncertain-after-timeout')

    def test_test_process_not_started_does_not_claim_setup_or_product_failure(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            result = probe.run_share_ui([str(root / 'missing')], root, root / 'stdout', root / 'stderr',
                                        timeout=1, source_head=HEAD, session_id=SESSION)
            self.assertEqual(result['outcome'], 'not-started')
            self.assertEqual(result['stage_observations']['phases']['test_runner']['state'], 'not_started')
            self.assertEqual(result['stage_observations']['phases']['test_setup']['state'], 'not_started')

    def test_outer_timeout_retains_setup_without_inventing_test_body_or_rejection(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            result = probe.run_share_ui([sys.executable, '-c', 'import time; time.sleep(30)'],
                                        root, root / 'stdout', root / 'stderr', timeout=.2,
                                        source_head=HEAD, session_id=SESSION)
            facts = result['stage_observations']
            self.assertEqual(facts['phases']['test_runner']['state'], 'unknown_after_timeout')
            self.assertEqual(facts['phases']['test_body']['state'], 'not_started')
            self.assertEqual(facts['missing_representation_observations'], ['share-control', 'share-url', 'share'])
            self.assertEqual(facts['admission'], 'unobserved')
            self.assertEqual(facts['source_head'], HEAD)
            self.assertEqual(facts['session_id'], SESSION)

    def test_structured_partial_trace_records_last_boundary_and_missing_provider(self):
        lines = [event('test_setup', 'started', 0), event('test_setup', 'completed', .1),
                 event('test_body', 'started', .2), event('share-control', 'launch_started', .3),
                 event('share-control', 'app_ready', 1),
                 event('share-control', 'sheet_observed', 2, save_candidates=1),
                 event('share-control', 'completed', 3), event('share-url', 'launch_started', 4),
                 event('share-url', 'app_ready', 5),
                 event('share-url', 'sheet_observed', 6, save_candidates=0),
                 event('share-url', 'completed', 7), event('share', 'launch_started', 8)]
        facts = probe.parse_share_observations('\n'.join(lines), source_head=HEAD,
                                               session_id=SESSION, outcome='uncertain-after-timeout')
        self.assertEqual(facts['last_observed_boundary']['boundary'], 'launch_started')
        self.assertEqual(facts['phases']['share']['state'], 'unknown_after_timeout')
        self.assertEqual(facts['missing_representation_observations'], ['share'])
        self.assertEqual(facts['phases']['save_callback']['state'], 'not_started')
        self.assertEqual(facts['admission'], 'unobserved')
        self.assertEqual(facts['events'][-1]['elapsed_seconds'], 8)

    def test_duplicate_out_of_order_wrong_binding_and_decreasing_time_are_rejected(self):
        setup = [event('test_setup', 'started', 0), event('test_setup', 'completed', 1),
                 event('test_body', 'started', 2)]
        invalid = [setup + [setup[-1]],
                   [event('share', 'sheet_observed', 0, save_candidates=0)],
                   [event('test_setup', 'completed', 0)],
                   setup + [event('share-url', 'launch_started', 3)],
                   [event('test_setup', 'started', 2), event('test_setup', 'completed', 1)],
                   [event('test_setup', 'started', 0).replace(HEAD, 'c' * 40)],
                   [event('test_setup', 'started', 0).replace(SESSION, SESSION + 'x')]]
        for lines in invalid:
            with self.subTest(lines=lines), self.assertRaises(ValueError):
                probe.parse_share_observations('\n'.join(lines), source_head=HEAD, session_id=SESSION)

    def test_legacy_partial_log_keeps_missing_monotonic_times_and_no_admission(self):
        facts = probe.parse_share_observations(
            'PVT_SHARE_UI_OBSERVATION: phase=share-control save_candidates=1\n'
            'PVT_SHARE_UI_OBSERVATION: phase=share-url save_candidates=0\n',
            source_head=HEAD, session_id=SESSION, outcome='uncertain-after-timeout')
        self.assertEqual(facts['format'], 'legacy-text')
        self.assertFalse(facts['monotonic_boundaries_available'])
        self.assertIsNone(facts['events'][0]['elapsed_seconds'])
        self.assertEqual(facts['phases']['share']['state'], 'not_started')
        self.assertEqual(facts['missing_representation_observations'], ['share'])
        self.assertEqual(facts['admission'], 'unobserved')

    def test_complete_save_event_still_does_not_establish_independent_readback(self):
        lines = [event('test_setup', 'started', 0), event('test_setup', 'completed', 1),
                 event('test_body', 'started', 2), event('share-control', 'launch_started', 3),
                 event('share-control', 'app_ready', 4),
                 event('share-control', 'sheet_observed', 5, save_candidates=1),
                 event('share-control', 'completed', 6), event('share-url', 'launch_started', 7),
                 event('share-url', 'app_ready', 8), event('share-url', 'sheet_observed', 9, save_candidates=1),
                 event('save_callback', 'selected', 10), event('save_callback', 'completed', 11),
                 event('share-url', 'completed', 12), event('test_body', 'completed', 13)]
        facts = probe.parse_share_observations('\n'.join(lines), source_head=HEAD, session_id=SESSION)
        self.assertEqual(facts['phases']['save_callback']['state'], 'completed')
        self.assertEqual(facts['phases']['test_body']['state'], 'completed')
        self.assertEqual(facts['missing_representation_observations'], ['share'])
        self.assertEqual(facts['admission'], 'unobserved')
        for changed in (lines[:6] + [event('save_callback', 'selected', 5.5)],
                        lines[:-1] + [event('share', 'launch_started', 13)]):
            with self.assertRaises(ValueError):
                probe.parse_share_observations('\n'.join(changed), source_head=HEAD, session_id=SESSION)

    def test_all_observed_sheets_without_save_leave_admission_unobserved(self):
        lines = [event('test_setup', 'started', 0), event('test_setup', 'completed', 1),
                 event('test_body', 'started', 2)]
        for phase in ('share-control', 'share-url', 'share'):
            for boundary in ('launch_started', 'app_ready', 'sheet_observed', 'completed'):
                lines.append(event(phase, boundary, len(lines), **({'save_candidates': 0} if boundary == 'sheet_observed' else {})))
        lines.append(event('test_body', 'blocked', len(lines)))
        facts = probe.parse_share_observations('\n'.join(lines), source_head=HEAD, session_id=SESSION,
                                               outcome='native-save-activity-unavailable')
        self.assertTrue(facts['representation_coverage_complete'])
        self.assertEqual(facts['phases']['test_body']['state'], 'blocked')
        self.assertEqual(facts['phases']['save_callback']['state'], 'not_started')
        self.assertEqual(facts['admission'], 'unobserved')

    def test_native_callback_and_readback_must_bind_to_source_session_and_monotonic_clock(self):
        observed = {'source_head': HEAD, 'session_id': SESSION, 'phase': 'after',
                    'clock': 'reader-process-monotonic', 'elapsed_seconds': .5}
        check_reader_observation({'observation': observed}, source_head=HEAD, session_id=SESSION, phase='after')
        for change in ({'source_head': 'c' * 40}, {'session_id': 'other'}, {'phase': 'share'},
                       {'elapsed_seconds': -1}, {'elapsed_seconds': float('nan')}, {'clock': 'wall-clock'}):
            with self.subTest(change=change), self.assertRaises(ValueError):
                check_reader_observation({'observation': {**observed, **change}},
                                         source_head=HEAD, session_id=SESSION, phase='after')


class PVTOwnedCleanupTests(unittest.TestCase):
    owned = '11111111-2222-3333-4444-555555555555'
    foreign = 'AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE'

    def observe(self, root, *, failure=None):
        commands = []
        records = []
        def run(name, argv, **kwargs):
            commands.append((name, argv, kwargs))
            if name == 'shutdown-owned' and failure == 'shutdown-timeout':
                records.append({'name': name, 'outcome': 'uncertain-after-timeout'})
                raise RuntimeError('shutdown timeout')
            if name == 'cleanup-owned-verification':
                self.assertEqual(kwargs['timeout'], 20)
                if failure == 'absence-timeout':
                    raise RuntimeError('inventory timeout')
                return subprocess.CompletedProcess(argv, 0, json.dumps({'devices': {'runtime': [{'udid': self.foreign}]}}).encode())
            return subprocess.CompletedProcess(argv, 53 if failure == 'shutdown-error' and name == 'shutdown-owned' else 0)
        binding = {'source_head': HEAD, 'session_id': SESSION, 'owned_udid': self.owned}
        result = cleanup_owned_simulator(run, self.owned, binding, root, prefix='product', command_records=records)
        return result, commands

    def test_slow_shutdown_has_a_bounded_window_without_retry_or_absence_promotion(self):
        for prefix in ('control', 'product'):
            for shutdown_seconds, accepted in ((90, True), (181, False)):
                with self.subTest(prefix=prefix, seconds=shutdown_seconds), tempfile.TemporaryDirectory() as raw:
                    root = Path(raw)
                    (root / self.owned).mkdir()
                    (root / self.foreign).mkdir()
                    records, verbs = [], []
                    def run(name, argv, **kwargs):
                        verb = argv[2]
                        verbs.append(verb)
                        if verb == 'shutdown':
                            timeout = kwargs.get('timeout', 60)
                            self.assertLessEqual(timeout, 180)
                            if shutdown_seconds > timeout:
                                records.append({'name': name, 'outcome': 'uncertain-after-timeout'})
                                raise RuntimeError('shutdown timed out')
                        if verb == 'delete':
                            (root / self.owned).rmdir()
                        data = json.dumps({'devices': {'runtime': [{'udid': self.foreign}]}}).encode()
                        return subprocess.CompletedProcess(argv, 0, data)
                    binding = {'source_head': HEAD, 'session_id': SESSION, 'owned_udid': self.owned}
                    result = cleanup_owned_simulator(run, self.owned, binding, root,
                        prefix=prefix, command_records=records)
                    self.assertEqual(result['accepted'], accepted)
                    self.assertEqual(result['all_commands_succeeded'], accepted)
                    self.assertTrue(result['absence']['verified_absent'])
                    self.assertTrue((root / self.foreign).is_dir())
                    self.assertEqual(verbs, ['shutdown', 'delete', 'list'])

    def test_command_timeout_or_nonzero_and_observed_absence_remain_orthogonal(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            (root / self.foreign).mkdir()
            for failure in ('shutdown-timeout', 'shutdown-error'):
                result, commands = self.observe(root, failure=failure)
                self.assertFalse(result['all_commands_succeeded'])
                self.assertTrue(result['absence']['verified_absent'])
                self.assertFalse(result['accepted'])
                self.assertTrue((root / self.foreign).is_dir())
                for _, argv, _ in commands[:2]:
                    self.assertEqual(argv[-1], self.owned)
                    self.assertNotIn(self.foreign, argv)
            self.assertEqual(result['commands']['shutdown']['exit_code'], 53)

    def test_missing_or_contradictory_absence_never_accepts_successful_commands(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            for failure in ('absence-timeout', 'directory-remains', 'dangling-directory-link'):
                with self.subTest(failure=failure):
                    target = root / self.owned
                    if failure == 'directory-remains': target.mkdir()
                    if failure == 'dangling-directory-link': target.symlink_to(root / 'missing')
                    result, _ = self.observe(root, failure=failure)
                    self.assertTrue(result['all_commands_succeeded'])
                    self.assertIsNot(result['absence']['verified_absent'], True)
                    self.assertFalse(result['accepted'])
                    if target.is_symlink(): target.unlink()
                    elif target.exists(): target.rmdir()
            result, _ = self.observe(root)
            self.assertTrue(result['accepted'])

    def test_unowned_or_noncanonical_uuid_is_never_targeted(self):
        run = Mock()
        for target in (self.foreign, self.owned.lower() + 'x', 'all', 'booted'):
            with self.subTest(target=target), self.assertRaises(ValueError):
                cleanup_owned_simulator(run, target,
                    {'source_head': HEAD, 'session_id': SESSION, 'owned_udid': self.owned}, Path('/unused'), prefix='product')
        run.assert_not_called()

    def test_later_absence_links_original_receipt_hash_without_rewriting_failure(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            receipt = root / 'result.json'
            receipt.write_text(json.dumps({'head': HEAD, 'session_id': SESSION, 'owned_udid': self.owned,
                                           'status': 'blocked', 'cleanup_status': 'failed'}))
            original = receipt.read_bytes()
            run = Mock(return_value=subprocess.CompletedProcess([], 0, b'{"devices": {"runtime": []}}'))
            result = observe_receipt_absence(run, receipt, self.owned, root)
            self.assertTrue(result['verified_absent'])
            self.assertEqual(result['original_native_status'], 'blocked')
            self.assertEqual(result['original_cleanup_status'], 'failed')
            self.assertEqual(result['original_receipt_sha256'], hashlib.sha256(original).hexdigest())
            self.assertEqual(result['source_head'], HEAD)
            self.assertEqual(receipt.read_bytes(), original)
            run.assert_called_once_with('supplemental-cleanup-verification',
                                         ['xcrun', 'simctl', 'list', 'devices', '--json'], timeout=20)
            run.reset_mock()
            with self.assertRaises(ValueError):
                observe_receipt_absence(run, receipt, self.foreign, root)
            run.assert_not_called()


if __name__ == '__main__':
    unittest.main()
