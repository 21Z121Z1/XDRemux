"""Portable installation state tests; these do not run an Apple simulator."""
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from scripts.apple.pvt_native_import import check_native_import as probe


class SimulatorInstallModel:
    udid = '12345678-ABCD-1234-ABCD-123456789ABC'
    bundle = 'org.xdremux.audit.PVTReadback.u' + 'a' * 32

    def __init__(self, root, behavior='success', ready=True, inventory_delay=0,
                 post_install_inventory_delay=0, container_query_delay=0):
        self.root, self.behavior, self.ready = root, behavior, ready
        self.inventory_delay = inventory_delay
        self.post_install_inventory_delay = post_install_inventory_delay
        self.container_query_delay = container_query_delay
        self.work = root / 'proof'; self.work.mkdir()
        self.staged = root / 'PVTReadback.app'; self.staged.mkdir()
        (self.staged / 'Info.plist').write_bytes(plistlib.dumps(probe.reader_info(self.bundle)))
        (self.staged / 'PVTReadback').write_bytes(b'this-test-does-not-run-native-code')
        self.app = root / self.udid / 'Bundle' / 'PVTReadback.app'
        self.data = root / self.udid / 'Data'
        self.report = {'commands': [], 'owned_udid': self.udid, 'probe_bundle_id': self.bundle}
        self.installs, self.app_queries = 0, 0

    def publish(self):
        self.app.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(self.staged, self.app)
        self.data.mkdir()

    def run(self, name, argv, **kwargs):
        if str(argv[0]) == '/usr/bin/log':
            self.report['commands'].append({'name': name, 'argv': list(map(str, argv)), 'exit_code': 0})
            return subprocess.CompletedProcess(argv, 0, b'', b'')
        argv = list(map(str, argv)); action = argv[2]
        record = {'name': name, 'argv': argv, 'timeout_seconds': kwargs.get('timeout')}
        self.report['commands'].append(record)
        self._result = subprocess.CompletedProcess(argv, 0, b'', b'')
        if action == 'listapps':
            delay = self.inventory_delay if not self.app.exists() else self.post_install_inventory_delay
            if not self.ready or delay > kwargs['timeout']:
                record['outcome'] = 'uncertain-after-timeout'
                raise probe.Blocked('inventory service timeout')
            apps = {self.bundle: {'CFBundleIdentifier': self.bundle}} if self.app.exists() else {}
            self._result.stdout = json.dumps(apps).encode()
        elif action == 'install':
            self.installs += 1
            if self.behavior in ('success', 'timeout-installed'):
                self.publish()
            if self.behavior.startswith('timeout'):
                record['outcome'] = 'uncertain-after-timeout'
                record['exit_code'] = -15
                raise probe.Blocked('install client timeout; server result unknown')
            if self.behavior == 'error':
                self._result.returncode = 1
                self._result.stderr = b'actual native installation error'
        elif action == 'get_app_container':
            self.assert_owned(argv)
            if self.container_query_delay > kwargs['timeout']:
                record['outcome'] = 'uncertain-after-timeout'
                raise probe.Blocked('container service timeout')
            if argv[-1] == 'app':
                self.app_queries += 1
                if self.behavior == 'timeout-late' and self.app_queries == 2:
                    self.publish()
            path = self.app if argv[-1] == 'app' else self.data
            self._result.stdout = (str(path) + '\n').encode() if path.exists() else b''
            self._result.returncode = 0 if path.exists() else 2
        elif action != 'spawn':
            raise AssertionError('unexpected action: ' + str(argv))
        record['exit_code'] = self._result.returncode
        return self._result

    def assert_owned(self, argv):
        assert argv[3] == self.udid and argv[4] == self.bundle

    def install(self):
        with patch.object(probe.Path, 'home', return_value=self.root), patch.object(
                probe.subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, b'', b'')):
            return probe.install_owned_reader(self.run, self.udid, self.bundle, self.staged,
                                              self.work, self.report, pause=lambda _: None)


class SimulatorInstallTests(unittest.TestCase):
    def test_success_requires_actual_matching_bundle_and_both_containers(self):
        with tempfile.TemporaryDirectory() as raw:
            model = SimulatorInstallModel(Path(raw))
            facts = model.install()
            self.assertEqual(facts['status'], 'installed-verified')
            self.assertEqual(model.installs, 1)
            self.assertEqual(facts['installed_binary_sha256'], probe.sha(model.staged / 'PVTReadback'))

    def test_timeout_is_reconciled_without_repeating_the_install(self):
        for behavior in ('timeout-installed', 'timeout-late'):
            with self.subTest(behavior=behavior), tempfile.TemporaryDirectory() as raw:
                model = SimulatorInstallModel(Path(raw), behavior)
                facts = model.install()
                self.assertEqual(facts['status'], 'installed-verified-after-client-timeout')
                self.assertEqual(model.installs, 1)
                self.assertTrue(any(c.get('outcome') == 'uncertain-after-timeout' for c in model.report['commands']))

    def test_unverified_timeout_stays_unknown_and_never_reinstalls(self):
        with tempfile.TemporaryDirectory() as raw:
            model = SimulatorInstallModel(Path(raw), 'timeout-absent')
            with self.assertRaises(probe.Blocked): model.install()
            self.assertEqual(model.installs, 1)
            self.assertEqual(model.report['reader_installation']['status'], 'not-verified-after-client-timeout')
            self.assertLessEqual(model.app_queries, 3)

    def test_service_readiness_is_bounded_and_never_starts_an_install_when_unknown(self):
        with tempfile.TemporaryDirectory() as raw:
            model = SimulatorInstallModel(Path(raw), ready=False)
            with self.assertRaises(probe.Blocked): model.install()
            self.assertEqual(model.installs, 0)
            self.assertEqual(sum(c['argv'][2] == 'listapps' for c in model.report['commands']), 3)

    def test_slow_startup_inventory_can_finish_before_installation(self):
        for delay in (30, 78.5):
            with self.subTest(delay=delay), tempfile.TemporaryDirectory() as raw:
                model = SimulatorInstallModel(Path(raw), inventory_delay=delay)
                facts = model.install()
                self.assertEqual(facts['status'], 'installed-verified')
                self.assertEqual(model.installs, 1)
                startup = [o for o in facts['observations'] if o['phase'] == 'inventory']
                self.assertEqual(len(startup), 1)
                self.assertEqual(startup[0]['state'], 'service-answered')
                self.assertEqual(facts['installed_binary_sha256'], probe.sha(model.staged / 'PVTReadback'))

    def test_slow_post_install_services_are_verified_without_reinstalling(self):
        with tempfile.TemporaryDirectory() as raw:
            model = SimulatorInstallModel(Path(raw), inventory_delay=78.5,
                                          post_install_inventory_delay=78.5, container_query_delay=78.5)
            facts = model.install()
            self.assertEqual(facts['status'], 'installed-verified')
            self.assertEqual(model.installs, 1)
            self.assertEqual(model.app_queries, 1)
            self.assertEqual(facts['installed_binary_sha256'], probe.sha(model.staged / 'PVTReadback'))
            queries = [cmd for cmd in model.report['commands']
                       if cmd['name'] == 'install-readiness-after-command'
                       or cmd['name'].startswith('install-state-')]
            self.assertEqual(len(queries), 3)
            self.assertTrue(all(cmd['timeout_seconds'] >= 78.5 for cmd in queries))

    def test_startup_inventory_deadline_still_blocks_installation(self):
        with tempfile.TemporaryDirectory() as raw:
            model = SimulatorInstallModel(Path(raw), inventory_delay=121)
            with self.assertRaisesRegex(probe.Blocked, 'app-inventory service'):
                model.install()
            self.assertEqual(model.installs, 0)
            self.assertEqual(model.report['reader_installation']['status'], 'installation-service-not-ready')
            inventory = [c for c in model.report['commands'] if c['argv'][2] == 'listapps']
            self.assertEqual(len(inventory), 3)
            self.assertTrue(all(0 < c['timeout_seconds'] <= 120 for c in inventory))
            self.assertTrue(all(c['outcome'] == 'uncertain-after-timeout' for c in inventory))

    def test_service_failure_diagnostics_do_not_need_the_stalled_device_spawn_service(self):
        with tempfile.TemporaryDirectory() as raw:
            model = SimulatorInstallModel(Path(raw), ready=False)
            with self.assertRaisesRegex(probe.Blocked, 'app-inventory service'):
                model.install()
            self.assertEqual(model.installs, 0)
            self.assertEqual(model.report['reader_installation']['status'], 'installation-service-not-ready')
            self.assertFalse(any(c['argv'][:3] == ['xcrun', 'simctl', 'spawn'] for c in model.report['commands']))
            self.assertTrue(any(c['argv'][:2] == ['/usr/bin/log', 'show'] for c in model.report['commands']))
            self.assertEqual(model.report['host_simulator_diagnostics']['owned_udid'], model.udid)

    @unittest.skipUnless(os.name == 'posix', 'CoreSimulator host-file reads require POSIX descriptors')
    def test_host_observations_are_bounded_owned_facts_not_application_readiness(self):
        with tempfile.TemporaryDirectory() as raw:
            model = SimulatorInstallModel(Path(raw))
            device = model.root / 'Library/Developer/CoreSimulator/Devices' / model.udid
            device.mkdir(parents=True)
            (device / 'device.plist').write_bytes(plistlib.dumps(
                {'UDID': model.udid, 'state': 3, 'runtime': 'runtime-under-test', 'private_other_field': 'never-copy-this'}))
            log = model.root / 'Library/Logs/CoreSimulator' / model.udid / 'system.log'
            log.parent.mkdir(parents=True)
            log.write_bytes(b'DISCARDED-PREFIX' + b'x' * 70000 + b'LAST-ERROR')
            processes = b'42 1 42 S 00:20 00:01 3.0 1234 /Apple/CoreSimulatorService\n43 1 43 S 00:20 00:01 0.0 500 /private/unrelated-secret\n'
            def host_query(argv, **kwargs):
                output = processes if argv[0] == '/bin/ps' else (
                    b'Mach Virtual Memory Statistics: (page size of 16384 bytes)\nPages free: 100.\nPages active: 200.\n')
                return subprocess.CompletedProcess(argv, 0, output, b'')
            with patch.object(probe.Path, 'home', return_value=model.root), patch.object(
                    probe.subprocess, 'run', side_effect=host_query):
                facts = probe.capture_host_simulator_diagnostics(model.run, model.udid, model.report)
            self.assertEqual(facts['device']['state'], 3)
            self.assertEqual(facts['processes'][0]['command'], 'CoreSimulatorService')
            self.assertEqual(len(facts['processes']), 1)
            self.assertEqual(facts['processes'][0]['rss_kib'], 1234)
            self.assertEqual(facts['memory'], {'query_exit_code': 0, 'page_size_bytes': 16384,
                                              'page_counts': {'Pages free': 100, 'Pages active': 200}})
            self.assertTrue(facts['system_log']['truncated'])
            self.assertLessEqual(len(facts['system_log']['text'].encode()), 65536)
            self.assertTrue(facts['system_log']['text'].endswith('LAST-ERROR'))
            text = json.dumps(facts)
            for private in ('DISCARDED-PREFIX', 'never-copy-this', 'unrelated-secret'):
                self.assertNotIn(private, text)
            self.assertEqual(model.installs, 0)
            self.assertNotIn('installed-verified', text)

    def test_host_diagnostic_failures_do_not_replace_the_original_service_failure(self):
        with tempfile.TemporaryDirectory() as raw:
            model = SimulatorInstallModel(Path(raw), ready=False)
            original = model.run
            def run(name, argv, **kwargs):
                if str(argv[0]) == '/usr/bin/log':
                    raise probe.Blocked('host log client timeout')
                return original(name, argv, **kwargs)
            with patch.object(probe.Path, 'home', return_value=model.root), patch.object(
                    probe.subprocess, 'run', side_effect=subprocess.TimeoutExpired(['ps'], 5)):
                with self.assertRaisesRegex(probe.Blocked, 'app-inventory service'):
                    probe.install_owned_reader(run, model.udid, model.bundle, model.staged,
                                               model.work, model.report, pause=lambda _: None)
            facts = model.report['host_simulator_diagnostics']
            self.assertIn('process_error', facts)
            self.assertIn('host_log_error', facts)
            self.assertEqual(model.installs, 0)

    @unittest.skipUnless(os.name == 'posix', 'CoreSimulator host-file reads require POSIX descriptors')
    def test_host_diagnostics_refuse_foreign_identity_and_file_aliases(self):
        with tempfile.TemporaryDirectory() as raw:
            model = SimulatorInstallModel(Path(raw))
            with self.assertRaises(probe.Blocked):
                probe.capture_host_simulator_diagnostics(model.run, '00000000-0000-0000-0000-000000000000', model.report)
            external = model.root / 'external'; external.write_text('never-read-foreign-log')
            log = model.root / 'Library/Logs/CoreSimulator' / model.udid / 'system.log'
            log.parent.mkdir(parents=True); log.symlink_to(external)
            with patch.object(probe.Path, 'home', return_value=model.root), patch.object(
                    probe.subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, b'', b'')):
                facts = probe.capture_host_simulator_diagnostics(model.run, model.udid, model.report)
            self.assertIn('error', facts['system_log'])
            self.assertNotIn('never-read-foreign-log', json.dumps(facts))

    def test_existing_correct_install_is_verified_before_any_new_mutation(self):
        with tempfile.TemporaryDirectory() as raw:
            model = SimulatorInstallModel(Path(raw)); model.publish()
            facts = model.install()
            self.assertEqual(facts['status'], 'already-installed-verified')
            self.assertEqual(model.installs, 0)

    def test_changed_existing_binary_is_preserved_and_not_replaced(self):
        with tempfile.TemporaryDirectory() as raw:
            model = SimulatorInstallModel(Path(raw)); model.publish()
            binary = model.app / 'PVTReadback'; binary.write_bytes(b'other-existing-binary')
            with self.assertRaises(probe.Blocked): model.install()
            self.assertEqual(model.installs, 0)
            self.assertEqual(binary.read_bytes(), b'other-existing-binary')

    def test_owned_container_alias_is_compared_by_actual_path_identity(self):
        with tempfile.TemporaryDirectory() as raw:
            model = SimulatorInstallModel(Path(raw)); model.publish()
            original = model.app
            alias = model.root / model.udid / 'bundle-alias'
            alias.symlink_to(original, target_is_directory=True); model.app = alias
            facts = model.install()
            self.assertEqual(facts['app_container'], str(original.resolve()))
            self.assertEqual(model.installs, 0)

    def test_foreign_container_and_wrong_bundle_metadata_cannot_confirm_installation(self):
        for mutation in ('foreign-container', 'wrong-bundle'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as raw:
                model = SimulatorInstallModel(Path(raw)); model.publish()
                if mutation == 'foreign-container':
                    foreign = model.root / 'foreign' / 'PVTReadback.app'
                    foreign.parent.mkdir(); shutil.copytree(model.app, foreign); model.app = foreign
                else:
                    path = model.app / 'Info.plist'
                    value = plistlib.loads(path.read_bytes()); value['CFBundleIdentifier'] = 'other.app'
                    path.write_bytes(plistlib.dumps(value))
                with self.assertRaises(probe.Blocked): model.install()
                self.assertEqual(model.installs, 0)
                self.assertTrue(any(o.get('error') for o in model.report['reader_installation']['observations']))

    def test_plist_inventory_is_supported_but_malformed_success_is_not_readiness(self):
        for payload, accepted in ((plistlib.dumps({}), True), (b'[]', False)):
            with self.subTest(accepted=accepted), tempfile.TemporaryDirectory() as raw:
                model = SimulatorInstallModel(Path(raw)); original = model.run
                def run(name, argv, **kwargs):
                    result = original(name, argv, **kwargs)
                    if argv[2] == 'listapps': result.stdout = payload
                    return result
                if accepted:
                    facts = probe.install_owned_reader(run, model.udid, model.bundle, model.staged,
                                                       model.work, model.report, pause=lambda _: None)
                    self.assertEqual(facts['status'], 'installed-verified')
                else:
                    with self.assertRaises(probe.Blocked):
                        probe.install_owned_reader(run, model.udid, model.bundle, model.staged,
                                                   model.work, model.report, pause=lambda _: None)
                    self.assertEqual(model.installs, 0)

    def test_explicit_native_install_error_is_retained_without_retry(self):
        with tempfile.TemporaryDirectory() as raw:
            model = SimulatorInstallModel(Path(raw), 'error')
            with self.assertRaises(probe.Blocked): model.install()
            self.assertEqual(model.installs, 1)
            self.assertEqual(model.report['reader_installation']['install_exit_code'], 1)


if __name__ == '__main__':
    unittest.main()
