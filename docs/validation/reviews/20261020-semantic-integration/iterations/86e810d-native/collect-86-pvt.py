from pathlib import Path
import hashlib
import json

root = Path('/workspace/xdremux-audit')
receipt = json.loads((root / '86e810d-pvt-native-receipt.json').read_text())
assert receipt['head'] == '86e810dfb52cbe2516d816bc0ac11b94de84d0e7'
assert receipt['status'] == 'blocked'
install = receipt['reader_installation']
assert install['status'] == 'installation-service-not-ready' and install['install_attempts'] == 0
observations = install['observations']
assert len(observations) == 3 and all(o['phase'] == 'inventory' and o['state'] == 'unknown' for o in observations)
commands = {c['name']: c for c in receipt['commands']}
assert 'install-owned-reader' not in commands
assert 'grant-owned-photos' not in commands and not receipt.get('native_share_ui_test_invoked')
assert receipt['synthetic_input_unchanged'] is True and receipt['package_bytes_unchanged'] is True
assert receipt['cleanup_status'] == 'completed'
assert commands['shutdown-owned']['exit_code'] == commands['delete-owned']['exit_code'] == 0
result = {
    'source': receipt['head'], 'run': 39551734230, 'job': 119062245441,
    'conclusion': 'failure', 'probe_status': receipt['status'],
    'reason': receipt['reason'],
    'native_setup': {name: commands[name] for name in commands if name.startswith(('compile-sdk-', 'compile-reader', 'sign-reader', 'boot-owned', 'install-', 'shutdown-owned', 'delete-owned'))},
    'reader_installation': install,
    'selected_runtime': receipt['selected_runtime'], 'selected_device_type': receipt['selected_device_type'],
    'input_and_package_unchanged': True, 'owned_simulator_cleanup': 'completed',
    'never_reached': ['installation', 'Photos authorization', 'baseline', 'three whole-package sharing paths', 'independent Live asset/resources/IDs/cover/hash readback'],
    'artifact': {'id': 12325600964, 'bytes': 32102, 'api_digest': 'sha256:0c3757b660baecef5fff137f63dd5e744544bc9f6a6b0a17954d959e431836de', 'local_bytes_verified': False,
                 'local_read': 'Connector returned a reusable file reference; supported signed-URL download in the execution environment failed with proxy Tunnel connection failed:403 Forbidden. No bypass/retry used.'},
    'log_sha256': hashlib.sha256((root / '86e810d-pvt-job119062245441.log').read_bytes()).hexdigest(),
    'limits': ['No Apple media rejection or Photos admission inferred from the service-query timeouts.', 'No new source change, rerun, second installation or timeout increase based on this result.', 'This head did not reach the installed-container reconciliation branch; behavioral model tests are separate.', 'Runtime/device compatibility uses the existing min/max selection. iPhone11 selection alone does not prove incompatibility; raw runtime/devicetype inventory is retained in the small artifact, not independently materialized locally.', 'Physical Photos, macOS Photos and wallpaper remain untested.']
}
(root / '86e810d-pvt-native-observation.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'source': result['source'], 'status': result['probe_status'], 'install_attempts': 0, 'preserved': True, 'cleanup': 'completed', 'artifact': 12325600964}))
