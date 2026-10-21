"""Portable installation state tests; these do not run an Apple simulator."""
import json
from pathlib import Path
import plistlib
import shutil
import subprocess
import tempfile
import unittest

from scripts.apple.pvt_native_import import check_native_import as probe


class SimulatorInstallModel:
    udid = '12345678-ABCD-1234-ABCD-123456789ABC'
    bundle = 'org.xdremux.audit.PVTReadback.u' + 'a' * 32

    def __init__(self, root, behavior='success', ready=True):
        self.root, self.behavior, self.ready = root, behavior, ready
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
        argv = list(map(str, argv)); action = argv[2]
        record = {'name': name, 'argv': argv}; self.report['commands'].append(record)
        self._result = subprocess.CompletedProcess(argv, 0, b'', b'')
        if action == 'listapps':
            if not self.ready:
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
