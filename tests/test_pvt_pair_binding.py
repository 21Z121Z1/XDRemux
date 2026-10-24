"""Synthetic catalog guards only; no simulator or Photos acceptance claim."""
from contextlib import redirect_stdout
from copy import deepcopy
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from scripts.apple.pvt_native_import import check_native_import as probe


class PairBindingTests(unittest.TestCase):
    runtime = 'com.apple.CoreSimulator.SimRuntime.iOS-27-0'
    device = 'com.apple.CoreSimulator.SimDeviceType.iPhone-11'

    def catalogs(self):
        return ({'runtimes': [{'identifier': self.runtime, 'version': '27.0', 'isAvailable': True,
                              'supportedDeviceTypes': [{'identifier': self.device}]}]},
                {'devicetypes': [{'identifier': self.device, 'productFamily': 'iPhone',
                                 'minRuntimeVersion': 17 * 65536}]})

    def test_added_preferred_runtime_cannot_reselect_control_pair(self):
        runtimes, types = self.catalogs()
        newer = deepcopy(runtimes['runtimes'][0])
        newer.update(identifier=self.runtime.replace('27', '28'), version='28.0')
        runtimes['runtimes'].append(newer)
        self.assertNotEqual(probe.select_device(runtimes, types)[0], self.runtime)
        with patch.object(probe, 'select_device', side_effect=AssertionError('must not reselect')):
            self.assertEqual(probe.validate_required_device(runtimes, types, self.runtime, self.device),
                             (self.runtime, self.device))

    def test_changed_catalog_blocks_product_main_before_build_create_or_native_import(self):
        changes = ('removed-runtime', 'unavailable', 'unsupported', 'removed-device', 'wrong-family',
                   'min-version', 'max-version', 'duplicate-runtime', 'duplicate-device')
        for change in changes:
            with self.subTest(change=change), tempfile.TemporaryDirectory() as raw:
                runtimes, types = self.catalogs()
                row, phone = runtimes['runtimes'][0], types['devicetypes'][0]
                if change == 'removed-runtime': runtimes['runtimes'] = []
                elif change == 'unavailable': row['isAvailable'] = False
                elif change == 'unsupported': row['supportedDeviceTypes'] = []
                elif change == 'removed-device': types['devicetypes'] = []
                elif change == 'wrong-family': phone['productFamily'] = 'iPad'
                elif change == 'min-version': phone['minRuntimeVersion'] = 28 * 65536
                elif change == 'max-version': phone['maxRuntimeVersion'] = 26 * 65536
                elif change == 'duplicate-runtime': runtimes['runtimes'].append(deepcopy(row))
                elif change == 'duplicate-device': types['devicetypes'].append(deepcopy(phone))
                # A perfectly valid alternative must not rescue the required pair.
                other_runtime, other_types = self.catalogs()
                other_runtime['runtimes'][0]['identifier'] += '-alternative'
                other_runtime['runtimes'][0]['supportedDeviceTypes'][0]['identifier'] += '-alternative'
                other_types['devicetypes'][0]['identifier'] += '-alternative'
                runtimes['runtimes'] += other_runtime['runtimes']
                types['devicetypes'] += other_types['devicetypes']
                commands = []
                def run(argv, **kwargs):
                    commands.append(argv)
                    if argv == ['git', 'rev-parse', 'HEAD']: output = b'a' * 40
                    elif argv == ['git', 'status', '--porcelain']: output = b''
                    elif argv[-3:] == ['list', 'runtimes', '--json']: output = json.dumps(runtimes).encode()
                    elif argv[-3:] == ['list', 'devicetypes', '--json']: output = json.dumps(types).encode()
                    elif argv in (['xcodebuild', '-version'], ['xcrun', '--sdk', 'iphonesimulator', 'swiftc', '--version'],
                                   ['sw_vers'], ['xcrun', 'simctl', 'help', 'privacy']): output = b''
                    else: raise AssertionError('unexpected execution before rejection: ' + repr(argv))
                    return subprocess.CompletedProcess(argv, 0, output, b'')
                root = Path(raw)
                argv = ['probe', '--repo', str(root), '--cli', str(root / 'xdremux'), '--expected-head', 'a' * 40,
                        '--output-root', str(root / 'receipts'), '--execute-owned-simulator',
                        '--required-runtime', self.runtime, '--required-device-type', self.device]
                with patch.object(sys, 'argv', argv), patch.object(probe.subprocess, 'run', side_effect=run), \
                     patch.object(probe.platform, 'system', return_value='Darwin'), \
                     patch.object(probe, 'select_device', side_effect=AssertionError('must not reselect')), \
                     redirect_stdout(io.StringIO()):
                    self.assertEqual(probe.main(), 2)
                receipt = json.loads(next((root / 'receipts').glob('pvt-native-*/result.json')).read_text())
                self.assertEqual(receipt['status'], 'blocked')
                self.assertEqual(receipt['cleanup_status'], 'not-created')
                self.assertFalse(any('create' in cmd or 'boot' in cmd or 'launch' in cmd or 'cargo' in cmd for cmd in commands))
                self.assertNotIn('owned_udid', receipt)

    def test_precreate_recheck_uses_fresh_catalog_and_never_changes_pair(self):
        calls = []
        runtimes, types = self.catalogs()
        def run(name, argv):
            calls.append(name)
            return subprocess.CompletedProcess(argv, 0, json.dumps(runtimes if 'runtimes' in name else types).encode())
        self.assertEqual(probe.read_required_device(run, self.runtime, self.device, prefix='precreate'),
                         (self.runtime, self.device))
        runtimes['runtimes'][0]['supportedDeviceTypes'] = []
        with self.assertRaises(probe.Blocked):
            probe.read_required_device(run, self.runtime, self.device, prefix='precreate')
        self.assertEqual(calls, ['precreate-runtimes', 'precreate-device-types'] * 2)


if __name__ == '__main__':
    unittest.main()
