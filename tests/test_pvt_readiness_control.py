"""Portable lifecycle models; no Apple service/import acceptance is claimed."""
from contextlib import redirect_stdout
import hashlib
import io
import json
from pathlib import Path
import plistlib
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from scripts.apple.pvt_native_import import check_native_import as product
from scripts.apple.pvt_native_import import readiness_control as control


class ControlModel:
    source_sha = 'a' * 40
    uuid = '11111111-2222-3333-4444-555555555555'
    foreign = 'AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE'
    runtime = 'com.apple.CoreSimulator.SimRuntime.iOS-27-0'
    device = 'com.apple.CoreSimulator.SimDeviceType.iPhone-11'

    def __init__(self, root, failure=None, payload=None):
        self.root, self.failure, self.payload = root, failure, payload
        self.devices = root / 'devices'; self.devices.mkdir()
        (self.devices / self.foreign).mkdir()
        self.commands, self.probes = [], 0
        self.report = {'commands': [], 'expected_head': self.source_sha,
                       'control_stdout_path': str(root / 'control-listapps.stdout')}

    def run(self, name, argv, *, timeout=60, required=True):
        argv = list(map(str, argv)); self.commands.append((name, argv, timeout))
        result = subprocess.CompletedProcess(argv, 0, b'', b'')
        if name == 'control-runtimes':
            value = {'runtimes': [{'identifier': self.runtime, 'version': '27.0', 'isAvailable': True,
                                  'supportedDeviceTypes': [{'identifier': self.device}]}]}
        elif name == 'control-device-types':
            value = {'devicetypes': [{'identifier': self.device, 'productFamily': 'iPhone',
                                     'minRuntimeVersion': 13 * 65536}]}
        elif name == 'control-create':
            if self.failure == 'create-timeout': raise control.ReadinessBlocked('create client result unknown')
            (self.devices / self.uuid).mkdir(); result.stdout = (self.uuid + '\n').encode(); return result
        elif name in ('control-boot', 'control-bootstatus', 'control-shutdown', 'control-delete'):
            self.assert_owned(argv)
            if self.failure == name:
                raise control.ReadinessBlocked(name + ' result unknown')
            if name == 'control-delete': (self.devices / self.uuid).rmdir()
            return result
        elif name == 'control-listapps':
            self.assert_owned(argv)
            if self.failure == 'inventory-timeout': raise control.ReadinessBlocked('inventory timeout')
            if self.failure == 'inventory-error': result.returncode = 1; return result
            result.stdout = self.payload if self.payload is not None else b'{"com.apple.mobileslideshow": {}}'
            Path(self.report['control_stdout_path']).write_bytes(result.stdout)
            return result
        elif name == 'control-inventory-plist':
            assert Path(argv[-1]).read_bytes() == self.payload
            result.stdout = b'{"openstep": {}}'; return result
        elif name == 'control-cleanup-verification':
            if self.failure == 'cleanup-query': raise control.ReadinessBlocked('verification timeout')
            rows = [{'udid': self.foreign}]
            if self.failure == 'cleanup-present': rows.append({'udid': self.uuid})
            value = {'devices': {self.runtime: rows}}
        else:
            raise AssertionError('unexpected native action: ' + name)
        result.stdout = json.dumps(value).encode(); return result

    def assert_owned(self, argv):
        assert self.uuid in argv and self.foreign not in argv

    def probe(self):
        self.probes += 1
        self.assert_control_cleaned()
        assert self.report['status'] == 'product-probe-running'
        return 0, {'head': self.source_sha, 'selected_runtime': self.runtime,
                   'selected_device_type': self.device,
                   'status': 'native-direct-pvt-import-and-readback-pass', 'cleanup_status': 'completed',
                   'synthetic_input_unchanged': True, 'package_bytes_unchanged': True}

    def assert_control_cleaned(self):
        assert not (self.devices / self.uuid).exists() and (self.devices / self.foreign).exists()

    def execute(self, probe=None):
        return control.run_control_and_probe(self.run, product.select_device, probe or self.probe,
                                             self.report, 'pvt-readiness-control-' + 'a' * 32, self.devices,
                                             expected_head=self.source_sha)


class ReadinessControlTests(unittest.TestCase):
    def test_checkout_binding_refuses_a_different_head_or_dirty_source(self):
        for replies in (['b' * 40, 'c' * 40, ''], ['a' * 40, 'c' * 40, ' M file']):
            with self.subTest(replies=replies), patch.object(control.subprocess, 'check_output', side_effect=replies):
                with self.assertRaises(control.ReadinessBlocked):
                    control.checkout_facts(Path('/source'), 'a' * 40)

    def test_checkout_binding_requires_full_sha_and_observes_the_actual_tree(self):
        with patch.object(control.subprocess, 'check_output') as git:
            with self.assertRaises(control.ReadinessBlocked): control.checkout_facts(Path('/source'), 'main')
            git.assert_not_called()
        with patch.object(control.subprocess, 'check_output', side_effect=['a' * 40, 'c' * 40, '']):
            self.assertEqual(control.checkout_facts(Path('/source'), 'a' * 40),
                             {'head': 'a' * 40, 'tree': 'c' * 40, 'status': ''})

    def test_current_checkout_command_and_receipt_use_the_requested_complete_head(self):
        repo = Path(control.__file__).resolve().parents[3]
        original_run = subprocess.run
        original_control = control.run_control_and_probe
        for expected in ('a' * 40, 'b' * 40):
            with self.subTest(expected=expected), tempfile.TemporaryDirectory() as raw:
                root = Path(raw)
                model = ControlModel(root)
                model.source_sha = expected
                commands = []
                def subprocess_run(argv, **kwargs):
                    if argv[0] != sys.executable:
                        return original_run(argv, **kwargs)
                    commands.append(argv)
                    model.assert_control_cleaned()
                    receipt = root / 'product' / 'pvt-native-test' / 'result.json'
                    receipt.parent.mkdir()
                    _, result = model.probe()
                    receipt.write_text(json.dumps(result))
                    return subprocess.CompletedProcess(argv, 0)
                def environment(run, choose, invoke, report, name, device_root, *, expected_head):
                    model.report = report
                    original_control(model.run, choose, invoke, report, name, model.devices,
                                     expected_head=expected_head)
                argv = ['readiness_control.py', '--repo', str(repo), '--expected-head', expected,
                        '--output-root', str(root / 'control'), '--product-output-root', str(root / 'product'),
                        '--execute-owned-simulator']
                facts = {'head': expected, 'tree': 'c' * 40, 'status': ''}
                with patch.object(sys, 'argv', argv), patch.object(control, 'checkout_facts', return_value=facts) as checkout, \
                        patch.object(control.platform, 'system', return_value='Darwin'), \
                        patch.object(control.platform, 'machine', return_value='arm64'), \
                        patch.object(control, 'run_control_and_probe', side_effect=environment), \
                        patch.object(control.subprocess, 'run', side_effect=subprocess_run), redirect_stdout(io.StringIO()):
                    self.assertEqual(control.main(), 0)
                checkout.assert_called_once_with(repo, expected)
                self.assertEqual(len(commands), 1)
                command = commands[0]
                self.assertEqual(command[1], str(repo / 'scripts/apple/pvt_native_import/check_native_import.py'))
                self.assertEqual(command[command.index('--repo') + 1], str(repo))
                self.assertEqual(command[command.index('--expected-head') + 1], expected)
                self.assertEqual(command[command.index('--cli') + 1], str(repo / 'target/debug/xdremux'))
                report = json.loads(next((root / 'control').glob('control-*/result.json')).read_text())
                self.assertEqual(report['source'], facts)
                self.assertEqual(report['expected_head'], expected)
                self.assertEqual(report['product_result']['head'], expected)
                self.assertEqual(set(report['source_bindings']['product_trees']),
                                 {'Cargo.toml', 'Cargo.lock', 'Package.swift', 'crates', 'Sources', 'apps', 'fixtures'})
                self.assertEqual(report['product_probe_sha256'], hashlib.sha256(Path(command[1]).read_bytes()).hexdigest())
                self.assertEqual(report['source_bindings']['probe_files_sha256']['scripts/apple/pvt_native_import/readiness_control.py'],
                                 hashlib.sha256(Path(control.__file__).read_bytes()).hexdigest())
                self.assertEqual(report['product_probe_invocations'], 1)

    def test_readiness_independently_rejects_an_unsupported_selected_pair_before_creation(self):
        with tempfile.TemporaryDirectory() as raw:
            model = ControlModel(Path(raw))
            control.run_control_and_probe(model.run, lambda *_: (model.runtime, 'unsupported-phone'),
                                          model.probe, model.report, 'pvt-readiness-control-' + 'a' * 32,
                                          model.devices, expected_head=model.source_sha)
            self.assertEqual(model.report['status'], 'control-environment-blocked')
            self.assertEqual(model.probes, 0)
            self.assertEqual([name for name, _, _ in model.commands], ['control-runtimes', 'control-device-types'])
            self.assertEqual(model.report['control']['cleanup_status'], 'not-created')

    def test_absent_runtime_support_blocks_before_simulator_creation(self):
        with tempfile.TemporaryDirectory() as raw:
            model = ControlModel(Path(raw))
            original_run = model.run
            def run(name, argv, **kwargs):
                result = original_run(name, argv, **kwargs)
                if name == 'control-runtimes':
                    value = json.loads(result.stdout)
                    del value['runtimes'][0]['supportedDeviceTypes']
                    result.stdout = json.dumps(value).encode()
                return result
            model.run = run
            model.execute()
            self.assertEqual(model.report['status'], 'control-environment-blocked')
            self.assertEqual(model.probes, 0)
            self.assertEqual([name for name, _, _ in model.commands], ['control-runtimes', 'control-device-types'])
            self.assertEqual(model.report['control']['cleanup_status'], 'not-created')

    def test_success_is_one_parsed_query_then_verified_cleanup_then_one_current_probe(self):
        for payload in (None, plistlib.dumps({'com.apple.mobileslideshow': {}}), b'{ openstep = {}; }'):
            with self.subTest(payload=payload), tempfile.TemporaryDirectory() as raw:
                m = ControlModel(Path(raw), payload=payload); m.execute()
                self.assertEqual(m.report['status'], 'native-direct-pvt-import-and-readback-pass')
                self.assertEqual(m.probes, 1)
                queries = [c for c in m.commands if c[0] == 'control-listapps']
                self.assertEqual(len(queries), 1); self.assertLessEqual(queries[0][2], 120)
                self.assertEqual(next(c[2] for c in m.commands if c[0] == 'control-bootstatus'), 180)
                self.assertTrue(m.report['control_prewarms_core_simulator'])
                self.assertEqual(m.report['control']['cleanup_status'], 'verified-absent')
                m.assert_control_cleaned()

    def test_bad_or_unresponsive_inventory_never_retries_or_runs_the_product_probe(self):
        for failure, payload in [('inventory-timeout', None), ('inventory-error', None),
                                 (None, b'[]'), (None, b'{"bundle": []}')]:
            with self.subTest(failure=failure, payload=payload), tempfile.TemporaryDirectory() as raw:
                m = ControlModel(Path(raw), failure, payload); m.execute()
                self.assertEqual(m.probes, 0)
                self.assertEqual(sum(c[0] == 'control-listapps' for c in m.commands), 1)
                self.assertEqual(m.report['status'], 'control-environment-blocked')
                m.assert_control_cleaned()

    def test_slow_inventory_can_complete_within_a_bounded_startup_budget(self):
        for reply_after, accepted in ((30, True), (121, False)):
            with self.subTest(reply_after=reply_after), tempfile.TemporaryDirectory() as raw:
                m = ControlModel(Path(raw))
                original = m.run
                def run(name, argv, **kwargs):
                    if name == 'control-listapps' and reply_after > kwargs['timeout']:
                        raise control.ReadinessBlocked('inventory did not reply within its deadline')
                    return original(name, argv, **kwargs)
                m.run = run
                m.execute()
                self.assertEqual(m.probes, int(accepted))
                self.assertEqual(m.report['control']['status'], 'passed-cleaned' if accepted else 'blocked')
                m.assert_control_cleaned()

    def test_inventory_timeout_preserves_owned_host_diagnostics_before_cleanup(self):
        with tempfile.TemporaryDirectory() as raw:
            m = ControlModel(Path(raw), 'inventory-timeout')
            m.report['commands'].append({'name': 'control-listapps', 'outcome': 'uncertain-after-timeout'})
            def diagnose(run, owned, report):
                self.assertEqual(owned, m.uuid)
                self.assertTrue((m.devices / owned).is_dir())
                self.assertEqual(report['owned_udid'], owned)
                report['host_simulator_diagnostics'] = {'owned_udid': owned, 'processes': []}
            with patch.object(control, 'capture_host_simulator_diagnostics', side_effect=diagnose, create=True) as collector:
                m.execute()
            collector.assert_called_once()
            facts = m.report['control']
            self.assertEqual(facts['host_simulator_diagnostics']['owned_udid'], m.uuid)
            self.assertEqual(facts['host_simulator_diagnostics']['source_head'], m.source_sha)
            self.assertEqual(facts['host_simulator_diagnostics']['session_id'], facts['name'])
            self.assertEqual(facts['reason'], 'inventory timeout')
            self.assertEqual(m.probes, 0)
            self.assertEqual(facts['cleanup_status'], 'verified-absent')
            m.assert_control_cleaned()

    def test_diagnostic_failure_cannot_replace_inventory_failure_or_skip_cleanup(self):
        with tempfile.TemporaryDirectory() as raw:
            m = ControlModel(Path(raw), 'inventory-timeout')
            m.report['commands'].append({'name': 'control-listapps', 'outcome': 'uncertain-after-timeout'})
            with patch.object(control, 'capture_host_simulator_diagnostics', side_effect=OSError('diagnostic unavailable'), create=True):
                m.execute()
            facts = m.report['control']
            self.assertEqual(facts['reason'], 'inventory timeout')
            self.assertIn('diagnostic unavailable', facts['host_simulator_diagnostics_error'])
            self.assertEqual(m.probes, 0)
            self.assertEqual(facts['cleanup_status'], 'verified-absent')
            m.assert_control_cleaned()

    def test_boot_or_uncertain_cleanup_stops_the_experiment(self):
        for failure in ('control-bootstatus', 'control-shutdown', 'control-delete', 'cleanup-query', 'cleanup-present'):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as raw:
                m = ControlModel(Path(raw), failure); m.execute()
                self.assertEqual(m.probes, 0)
                self.assertEqual(m.report['status'], 'control-environment-blocked')
                self.assertTrue((m.devices / m.foreign).exists())
                for name in ('control-shutdown', 'control-delete'):
                    self.assertEqual(sum(c[0] == name for c in m.commands), 1)

    def test_unknown_create_does_not_guess_a_uuid_or_start_the_product(self):
        with tempfile.TemporaryDirectory() as raw:
            m = ControlModel(Path(raw), 'create-timeout'); m.execute()
            self.assertEqual(m.probes, 0)
            self.assertEqual(m.report['control']['cleanup_status'], 'creation-outcome-uncertain')
            self.assertFalse(any(c[0] in ('control-shutdown', 'control-delete') for c in m.commands))

    def test_product_failure_or_changed_identity_cannot_be_promoted_by_control_success(self):
        for changes, exit_code in [({}, 2), ({'status': 'blocked'}, 0), ({'head': 'f' * 40}, 0),
                                   ({'selected_runtime': 'different'}, 0),
                                   ({'selected_device_type': 'different'}, 0),
                                   ({'cleanup_status': 'failed'}, 0), ({'synthetic_input_unchanged': False}, 0),
                                   ({'package_bytes_unchanged': False}, 0)]:
            with self.subTest(changes=changes), tempfile.TemporaryDirectory() as raw:
                m = ControlModel(Path(raw))
                def probe():
                    code, result = m.probe(); return exit_code, {**result, **changes}
                m.execute(probe)
                self.assertEqual(m.probes, 1)
                self.assertEqual(m.report['status'], 'product-probe-not-accepted')

    def test_product_cleanup_failure_preserves_verified_admission_observation(self):
        with tempfile.TemporaryDirectory() as raw:
            m = ControlModel(Path(raw))
            observed = {'source_head': m.source_sha, 'session_id': 'owned-product-session',
                        'admission': 'verified'}
            def probe():
                _, result = m.probe()
                return 1, {**result, 'status': 'native-validation-passed-cleanup-failed',
                           'cleanup_status': 'failed', 'stage_observations': observed}
            m.execute(probe)
            self.assertEqual(m.probes, 1)
            self.assertEqual(m.report['product_probe_exit_code'], 1)
            self.assertEqual(m.report['status'], 'product-probe-not-accepted')
            self.assertEqual(m.report['product_result']['cleanup_status'], 'failed')
            self.assertEqual(m.report['product_observations']['stage_observations'], observed)
            self.assertNotIn('product_admission', m.report)


if __name__ == '__main__':
    unittest.main()
