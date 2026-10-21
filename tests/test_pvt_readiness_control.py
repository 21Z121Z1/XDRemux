"""Portable lifecycle models; no Apple service/import acceptance is claimed."""
import json
from pathlib import Path
import plistlib
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from scripts.apple.pvt_native_import import check_native_import as product
from scripts.apple.pvt_native_import import readiness_control as control


class ControlModel:
    uuid = '11111111-2222-3333-4444-555555555555'
    foreign = 'AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE'
    runtime = 'com.apple.CoreSimulator.SimRuntime.iOS-27-0'
    device = 'com.apple.CoreSimulator.SimDeviceType.iPhone-11'

    def __init__(self, root, failure=None, payload=None):
        self.root, self.failure, self.payload = root, failure, payload
        self.devices = root / 'devices'; self.devices.mkdir()
        (self.devices / self.foreign).mkdir()
        self.commands, self.probes = [], 0
        self.report = {'commands': [], 'product_source_sha': control.PRODUCT_SHA,
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
        return 0, {'head': control.PRODUCT_SHA, 'selected_runtime': self.runtime,
                   'selected_device_type': self.device,
                   'status': 'native-direct-pvt-import-and-readback-pass', 'cleanup_status': 'completed',
                   'synthetic_input_unchanged': True, 'package_bytes_unchanged': True}

    def assert_control_cleaned(self):
        assert not (self.devices / self.uuid).exists() and (self.devices / self.foreign).exists()

    def execute(self, probe=None):
        return control.run_control_and_probe(self.run, product.select_device, probe or self.probe,
                                             self.report, 'pvt-readiness-control-' + 'a' * 32, self.devices)


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

    def test_changed_product_tree_is_not_validated_by_the_old_pinned_probe(self):
        with patch.object(control.subprocess, 'check_output', side_effect=['tree-one', 'tree-two']):
            with self.assertRaisesRegex(control.ReadinessBlocked, 'Cargo.toml'):
                control.product_equivalence(Path('/harness'), Path('/product'))

    def test_success_is_one_parsed_query_then_verified_cleanup_then_one_original_probe(self):
        for payload in (None, plistlib.dumps({'com.apple.mobileslideshow': {}}), b'{ openstep = {}; }'):
            with self.subTest(payload=payload), tempfile.TemporaryDirectory() as raw:
                m = ControlModel(Path(raw), payload=payload); m.execute()
                self.assertEqual(m.report['status'], 'native-direct-pvt-import-and-readback-pass')
                self.assertEqual(m.probes, 1)
                queries = [c for c in m.commands if c[0] == 'control-listapps']
                self.assertEqual(len(queries), 1); self.assertEqual(queries[0][2], 20)
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
        for changes, exit_code in [({'status': 'blocked'}, 2), ({'head': 'f' * 40}, 0),
                                   ({'selected_device_type': 'different'}, 0),
                                   ({'cleanup_status': 'failed'}, 0), ({'package_bytes_unchanged': False}, 0)]:
            with self.subTest(changes=changes), tempfile.TemporaryDirectory() as raw:
                m = ControlModel(Path(raw))
                def probe():
                    code, result = m.probe(); return exit_code, {**result, **changes}
                m.execute(probe)
                self.assertEqual(m.probes, 1)
                self.assertEqual(m.report['status'], 'product-probe-not-accepted')


if __name__ == '__main__':
    unittest.main()
