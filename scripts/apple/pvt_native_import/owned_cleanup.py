"""Observe cleanup of the exact simulator UUID returned to this PVT run."""
import json
import hashlib
import time
import uuid


def check_binding(owned_udid, binding):
    if (str(uuid.UUID(owned_udid)).upper() != owned_udid or binding.get('owned_udid') != owned_udid
            or not binding.get('session_id') or not binding.get('source_head')):
        raise ValueError('cleanup target is not bound to this invocation')


def observe_owned_absence(run, owned_udid, binding, device_root, *, name):
    check_binding(owned_udid, binding)
    started = time.monotonic()
    absence = {**binding, 'state': 'started', 'verified_absent': None}
    try:
        result = run(name, ['xcrun', 'simctl', 'list', 'devices', '--json'], timeout=20)
        devices = json.loads(result.stdout)['devices']
        if not isinstance(devices, dict) or any(not isinstance(rows, list) for rows in devices.values()):
            raise ValueError('cleanup verification is not a device dictionary')
        ids = [str(uuid.UUID(row['udid'])).upper() for rows in devices.values() for row in rows]
        registry_absent = owned_udid not in ids
        directory_absent = not (device_root / owned_udid).exists() and not (device_root / owned_udid).is_symlink()
        absence.update(state='observed', registry_absent=registry_absent, directory_absent=directory_absent,
                       verified_absent=result.returncode == 0 and registry_absent and directory_absent)
    except (RuntimeError, OSError, ValueError, KeyError, TypeError, AttributeError) as error:
        absence.update(state='blocked', error=str(error))
    absence['duration_seconds'] = time.monotonic() - started
    return absence


def observe_receipt_absence(run, receipt_path, owned_udid, device_root):
    """Return supplemental facts bound to an unchanged original; never rewrite it."""
    if receipt_path.is_symlink() or not receipt_path.is_file() or receipt_path.stat().st_size > 2000000:
        raise ValueError('original receipt must be one bounded regular file')
    original = receipt_path.read_bytes()
    receipt = json.loads(original)
    owner = receipt.get('control', receipt)
    binding = {'source_head': receipt.get('head', receipt.get('expected_head')),
               'session_id': receipt.get('session_id', owner.get('name', receipt.get('probe_bundle_id'))),
               'owned_udid': owner.get('owned_udid'), 'original_receipt_sha256': hashlib.sha256(original).hexdigest()}
    check_binding(owned_udid, binding)
    result = observe_owned_absence(run, owned_udid, binding, device_root, name='supplemental-cleanup-verification')
    if receipt_path.read_bytes() != original:
        raise ValueError('original receipt changed during supplemental observation')
    result.update(original_native_status=receipt.get('status'), original_cleanup_status=owner.get('cleanup_status'),
                  original_receipt_unchanged=True)
    return result


def cleanup_owned_simulator(run, owned_udid, binding, device_root, *, prefix, command_records=()):
    check_binding(owned_udid, binding)
    if prefix not in ('control', 'product'):
        raise ValueError('unknown PVT cleanup owner')
    started = time.monotonic()
    facts = {**binding, 'clock': 'driver-monotonic', 'commands': {}}
    names = (('control-shutdown', 'control-delete', 'control-cleanup-verification') if prefix == 'control'
             else ('shutdown-owned', 'delete-owned', 'cleanup-owned-verification'))
    for name, verb in zip(names[:2], ('shutdown', 'delete')):
        # Shutdown includes the guest's service teardown, which can outlast the
        # generic 60-second command window on a newly booted hosted simulator.
        timeout = 180 if verb == 'shutdown' else 60
        observed = {'state': 'started', 'elapsed_seconds': time.monotonic() - started,
                    'timeout_seconds': timeout}
        facts['commands'][verb] = observed
        try:
            result = run(name, ['xcrun', 'simctl', verb, owned_udid], timeout=timeout, required=False)
            observed.update(state='completed', exit_code=result.returncode)
        except (RuntimeError, OSError, ValueError) as error:
            record = next((row for row in reversed(command_records) if row.get('name') == name), {})
            observed.update(state='unknown_after_timeout' if record.get('outcome') == 'uncertain-after-timeout' else 'blocked',
                            error=str(error), outcome=record.get('outcome'))
        observed['completed_elapsed_seconds'] = time.monotonic() - started
    facts['all_commands_succeeded'] = all(row.get('exit_code') == 0 for row in facts['commands'].values())
    elapsed = time.monotonic() - started
    facts['absence'] = observe_owned_absence(run, owned_udid, binding, device_root, name=names[2])
    facts['absence'].update(elapsed_seconds=elapsed, completed_elapsed_seconds=time.monotonic() - started)
    # Later absence cannot promote an uncertain command to accepted cleanup.
    facts['accepted'] = facts['all_commands_succeeded'] and facts['absence']['verified_absent'] is True
    return facts
