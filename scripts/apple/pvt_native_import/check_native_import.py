"""Drive whole-PVT system sharing in one owned simulator; never use host Photos."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import plistlib
import re
import shutil
import signal
import stat
import struct
import subprocess
import sys
import tempfile
import time
import uuid

if __package__:
    from .observations import PVTStages, parse_share_observations, check_reader_observation
    from .owned_cleanup import cleanup_owned_simulator
else:
    from observations import PVTStages, parse_share_observations, check_reader_observation
    from owned_cleanup import cleanup_owned_simulator


class Blocked(RuntimeError):
    def __init__(self, message, *, timed_out=False):
        super().__init__(message)
        self.timed_out = timed_out


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run_owned_process(command, cwd, stdout_path, stderr_path, *, timeout, stop_marker=None, diagnostic_path=None):
    """Bound only this invocation's compiler/XCTest session and retain progress."""
    started = time.monotonic()
    facts = {}
    snapshots = []
    next_snapshot = 0.
    with stdout_path.open('wb') as stdout, stderr_path.open('wb') as stderr:
        process = subprocess.Popen(command, cwd=cwd, stdout=stdout, stderr=stderr, start_new_session=True)
        facts['owned_process_group'] = process.pid
        try:
            while True:
                if stop_marker and stop_marker in stdout_path.read_bytes():
                    facts['outcome'] = 'native-save-activity-unavailable'
                    break
                if process.poll() is not None:
                    break
                if time.monotonic() - started >= timeout:
                    facts['outcome'] = 'uncertain-after-timeout'
                    break
                if diagnostic_path and time.monotonic() - started >= next_snapshot:
                    snapshot = {'elapsed_seconds': time.monotonic() - started, 'processes': []}
                    try:
                        # No argv/environment collection: retain only this
                        # private group's compiler names, CPU and lifecycle.
                        observed = subprocess.run(['ps', '-axo', 'pid=,ppid=,pgid=,stat=,etime=,time=,pcpu=,comm='],
                                                  capture_output=True, text=True, timeout=5)
                        for line in observed.stdout.splitlines():
                            fields = line.split(None, 7)
                            if len(fields) == 8 and int(fields[2]) == process.pid:
                                snapshot['processes'].append(dict(zip(
                                    ('pid', 'ppid', 'pgid', 'state', 'elapsed', 'cpu_time', 'cpu_percent', 'command'),
                                    [*map(int, fields[:3]), *fields[3:]])))
                        if observed.returncode:
                            snapshot['error'] = observed.stderr[-1000:]
                    except (OSError, ValueError, subprocess.TimeoutExpired) as error:
                        snapshot['error'] = str(error)
                    snapshots.append(snapshot)
                    diagnostic_path.write_text(json.dumps(snapshots, indent=2) + '\n')
                    next_snapshot = time.monotonic() - started + 10
                time.sleep(.1)
        finally:
            # The new session belongs solely to this invocation.
            # Never kill by executable name or touch another simulator/worker.
            if process.poll() is None or facts.get('outcome'):
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    pass
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait(timeout=5)
                facts['owned_process_group_stopped'] = True
            facts['exit_code'] = process.wait()
    facts['duration_seconds'] = time.monotonic() - started
    return facts


def run_share_ui(command, cwd, stdout_path, stderr_path, *, timeout, source_head=None, session_id=None):
    # Missing Save is a failed native precondition, never import success.
    try:
        facts = run_owned_process(command, cwd, stdout_path, stderr_path, timeout=timeout,
                                  stop_marker=b'PVT_SHARE_UI_BLOCKED:')
    except OSError as error:
        facts = {'outcome': 'not-started', 'error': str(error), 'exit_code': None, 'duration_seconds': 0}
    text = stdout_path.read_text(errors='replace')
    observations = parse_share_observations(text, source_head=source_head, session_id=session_id,
                                            outcome=facts.get('outcome'))
    if facts.get('outcome') != 'not-started':
        invocation = {'phase': 'test_runner', 'state': 'started', 'boundary': 'invocation_started',
                      'elapsed_seconds': 0, 'clock': 'driver-monotonic'}
        observations['events'].insert(0, invocation)
        if observations['last_observed_boundary'] is None:
            observations['last_observed_boundary'] = invocation
    runner = observations['phases']['test_runner']
    runner.update(state='not_started' if facts.get('outcome') == 'not-started'
                  else 'unknown_after_timeout' if facts.get('outcome') == 'uncertain-after-timeout'
                  else 'blocked' if facts.get('outcome') or facts['exit_code'] else 'completed',
                  started_elapsed_seconds=0, completed_elapsed_seconds=facts['duration_seconds'], clock='driver-monotonic')
    observations['missing_required_stages'] = [phase for phase, row in observations['phases'].items()
                                               if row['state'] != 'completed']
    observations['stdout_sha256'] = sha(stdout_path)
    facts['stage_observations'] = observations
    facts['ui_observations'] = observations['ui_observations']
    return facts


def capture_host_simulator_diagnostics(run, owned_udid, report):
    """Observe an unresponsive owned device without its simctl spawn service.

    These host facts never establish installation or Photos readiness.
    """
    if str(uuid.UUID(owned_udid)).upper() != owned_udid or report.get('owned_udid') != owned_udid:
        raise Blocked('host diagnostics target is not the simulator created by this invocation')
    facts = {'owned_udid': owned_udid, 'processes': [],
             'process_scope': 'selected host service names; per-process device ownership is not inferred',
             'scope': 'read-only host observations; device/service readiness remains unproven'}
    report['host_simulator_diagnostics'] = facts
    home = Path.home().resolve(strict=True)
    try:
        facts['host_load_average'] = list(os.getloadavg())
    except (OSError, AttributeError) as error:
        facts['host_load_error'] = str(error)

    def read_owned(relative, *, tail=False):
        # Walk under the canonical home with no-follow directory descriptors.
        # A symlink/pipe must not turn diagnostics into an unrelated read/hang.
        directory = os.open(home, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        try:
            for component in relative.parts[:-1]:
                child = os.open(component, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=directory)
                os.close(directory)
                directory = child
            fd = os.open(relative.name, os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW, dir_fd=directory)
            with os.fdopen(fd, 'rb') as stream:
                info = os.fstat(stream.fileno())
                size = info.st_size
                if not stat.S_ISREG(info.st_mode) or (not tail and size > 65536):
                    raise ValueError('host diagnostic is not a bounded regular file')
                if tail:
                    stream.seek(max(0, size - 65536))
                data = stream.read(65536)
                return data, {'bytes_total': size, 'bytes_read': len(data), 'truncated': size > len(data)}
        finally:
            os.close(directory)

    try:
        data, _ = read_owned(Path('Library/Developer/CoreSimulator/Devices') / owned_udid / 'device.plist')
        device = plistlib.loads(data)
        if not isinstance(device, dict) or device.get('UDID', '').upper() != owned_udid:
            raise ValueError('host device plist identity does not match the owned UUID')
        facts['device'] = {key: device[key] for key in ('UDID', 'name', 'state', 'runtime', 'deviceType', 'isDeleted')
                           if type(device.get(key)) in (str, int, bool)}
    except (OSError, ValueError, AttributeError) as error:
        facts['device_error'] = str(error)
    try:
        data, metadata = read_owned(Path('Library/Logs/CoreSimulator') / owned_udid / 'system.log', tail=True)
        facts['system_log'] = {**metadata, 'text': data.decode(errors='replace')}
    except (OSError, ValueError, AttributeError) as error:
        facts['system_log'] = {'error': str(error)}
    try:
        observed = subprocess.run(['/bin/ps', '-ww', '-axo', 'pid=,ppid=,pgid=,stat=,etime=,time=,pcpu=,rss=,comm='],
                                  capture_output=True, timeout=5)
        facts['process_query_exit_code'] = observed.returncode
        relevant = {'CoreSimulatorService', 'com.apple.CoreSimulator.CoreSimulatorService',
                    'SimulatorTrampoline', 'launchd_sim', 'simdiskimaged',
                    'SpringBoard', 'backboardd', 'installd', 'installcoordinationd'}
        for line in observed.stdout.decode(errors='replace').splitlines():
            fields = line.split(None, 8)
            if len(fields) == 9 and Path(fields[-1]).name in relevant:
                facts['processes'].append(dict(zip(
                    ('pid', 'ppid', 'pgid', 'state', 'elapsed', 'cpu_time', 'cpu_percent', 'rss_kib', 'command'),
                    [*map(int, fields[:3]), *fields[3:7], int(fields[7]), Path(fields[-1]).name])))
                if len(facts['processes']) == 64:
                    facts['process_list_truncated'] = True
                    break
    except (OSError, ValueError, subprocess.TimeoutExpired) as error:
        facts['process_error'] = str(error)
    try:
        memory = subprocess.run(['/usr/bin/vm_stat'], capture_output=True, timeout=5)
        text = memory.stdout.decode(errors='replace')
        page_size = re.search(r'page size of (\d+) bytes', text)
        facts['memory'] = {'query_exit_code': memory.returncode,
                           'page_size_bytes': int(page_size[1]) if page_size else None,
                           'page_counts': {key: int(value) for key, value in re.findall(
                               r'(?m)^(Pages (?:free|active|inactive|speculative|wired down|occupied by compressor)):\s+(\d+)\.', text)}}
    except (OSError, ValueError, subprocess.TimeoutExpired) as error:
        facts['memory_error'] = str(error)
    try:
        # Host logd is independent of the device's hung spawn/install services.
        # Restrict messages to this exact UUID; never dump global logs/env/argv.
        result = run('host-simulator-log', ['/usr/bin/log', 'show', '--last', '2m', '--style', 'compact',
                     '--predicate', '(process == "CoreSimulatorService" OR process == "com.apple.CoreSimulator.CoreSimulatorService") '
                                    f'AND eventMessage CONTAINS[c] "{owned_udid}"'],
                     timeout=5, required=False)
        facts['host_log_exit_code'] = result.returncode
    except Blocked as error:
        facts['host_log_error'] = str(error)
    return facts


def install_owned_reader(run, owned_udid, bundle, app, work, report, *, pause=time.sleep):
    """Install at most once; reconcile registered bytes after a client timeout.

    Booted, installer exit0 and a readable installed app are separate facts.
    Read-only readiness/result polls never repeat an uncertain installation.
    """
    if (str(uuid.UUID(owned_udid)).upper() != owned_udid or report.get('owned_udid') != owned_udid
            or report.get('probe_bundle_id') != bundle
            or not re.fullmatch(r'org\.xdremux\.audit\.PVTReadback\.u[0-9a-f]{32}', bundle)):
        raise Blocked('installation target is not the simulator/bundle created by this invocation')
    staged_info = plistlib.loads((app / 'Info.plist').read_bytes())
    if staged_info.get('CFBundleIdentifier') != bundle or staged_info.get('CFBundleExecutable') != 'PVTReadback':
        raise Blocked('staged reader identity does not match this invocation')
    expected_binary = sha(app / 'PVTReadback')
    facts = {'status': 'preparing', 'install_attempts': 0, 'observations': [],
             'readiness_attempt_limit': 3, 'readiness_query_timeout_seconds': 120,
             'reconcile_attempt_limit': 3, 'install_timeout_seconds': 120}
    report['reader_installation'] = facts

    def fail(status, reason, *, timed_out=False):
        facts['status'] = status
        try:
            capture_host_simulator_diagnostics(run, owned_udid, report)
        except (Blocked, OSError, ValueError) as error:
            facts['host_diagnostic_error'] = str(error)
        raise Blocked(reason, timed_out=timed_out)

    def inventory(attempt, *, timeout=20):
        name = f'install-readiness-{attempt}'
        result = run(name, ['xcrun', 'simctl', 'listapps', owned_udid], timeout=timeout, required=False)
        if result.returncode:
            raise Blocked('installed-app inventory service returned nonzero; readiness not established')
        try:
            value = json.loads(result.stdout)
        except (ValueError, UnicodeDecodeError):
            try:
                value = plistlib.loads(result.stdout)
            except (ValueError, plistlib.InvalidFileException):
                # simctl also emits OpenStep plists. Preserve the raw reply and
                # use the platform plist reader; never infer readiness from text.
                raw = work / (name + '.stdout')
                raw.write_bytes(result.stdout)
                normalized = run(name + '-plist', ['plutil', '-convert', 'json', '-o', '-', raw], timeout=10)
                value = json.loads(normalized.stdout)
        if not isinstance(value, dict) or any(not isinstance(k, str) or not isinstance(v, dict) for k, v in value.items()):
            raise Blocked('installed-app inventory is not a bundle dictionary')
        return value

    def installed_state(attempt):
        observation = {'phase': 'container-readback', 'attempt': attempt, 'state': 'not-verified'}
        facts['observations'].append(observation)
        try:
            paths = {}
            for kind in ('app', 'data'):
                result = run(f'install-state-{attempt}-{kind}',
                             ['xcrun', 'simctl', 'get_app_container', owned_udid, bundle, kind], timeout=10, required=False)
                observation[kind + '_query_exit_code'] = result.returncode
                if result.returncode:
                    continue
                path = Path(result.stdout.decode().strip())
                if not path.is_absolute() or owned_udid.lower() not in {p.lower() for p in path.parts}:
                    raise Blocked('installed container query points outside the owned simulator')
                path = path.resolve(strict=True)
                if owned_udid.lower() not in {p.lower() for p in path.parts} or not path.is_dir():
                    raise Blocked('resolved installed container is outside the owned simulator')
                paths[kind] = path
            if set(paths) != {'app', 'data'} or paths['app'] == paths['data']:
                return None
            info_path, binary = paths['app'] / 'Info.plist', paths['app'] / 'PVTReadback'
            if info_path.is_symlink() or binary.is_symlink() or not binary.is_file():
                raise Blocked('installed reader metadata or binary is not a regular owned file')
            info = plistlib.loads(info_path.read_bytes())
            if info.get('CFBundleIdentifier') != bundle or info.get('CFBundleExecutable') != 'PVTReadback':
                raise Blocked('installed reader bundle identity differs from the staged app')
            actual_binary = sha(binary)
            if actual_binary != expected_binary:
                raise Blocked('installed reader binary differs from the signed staged binary')
            observation['state'] = 'installed-verified'
            return {'app_container': str(paths['app']), 'data_container': str(paths['data']),
                    'installed_binary_sha256': actual_binary, 'installed_info_sha256': sha(info_path)}
        except (Blocked, OSError, ValueError) as error:
            observation['error'] = str(error)
            return None

    ready = None
    for attempt in range(1, 4):
        try:
            ready = inventory(attempt, timeout=facts['readiness_query_timeout_seconds'])
            facts['observations'].append({'phase': 'inventory', 'attempt': attempt, 'state': 'service-answered',
                'registered_app_count': len(ready), 'reader_registered': bundle in ready,
                'inventory_contains_com_apple_mobileslideshow': 'com.apple.mobileslideshow' in ready})
            break
        except (Blocked, ValueError) as error:
            facts['observations'].append({'phase': 'inventory', 'attempt': attempt, 'state': 'unknown', 'error': str(error)})
            if attempt < 3:
                pause(1)
    if ready is None:
        fail('installation-service-not-ready', 'owned simulator app-inventory service did not become readable; no installation attempted',
             timed_out=report['commands'][-1].get('outcome') == 'uncertain-after-timeout')
    if bundle not in ready:
        facts['install_attempts'] = 1
        try:
            result = run('install-owned-reader', ['xcrun', 'simctl', 'install', owned_udid, app], timeout=120, required=False)
            facts['install_exit_code'] = result.returncode
        except Blocked as error:
            record = next(c for c in reversed(report['commands']) if c['name'] == 'install-owned-reader')
            facts['install_exit_code'] = record.get('exit_code')
            facts['install_client_outcome'] = record.get('outcome', 'failed-without-known-result')
            facts['install_client_error'] = str(error)
    if facts['install_attempts']:
        try:
            after_command = inventory('after-command')
            facts['observations'].append({'phase': 'registration-after-command', 'state': 'service-answered',
                                          'reader_registered': bundle in after_command})
        except (Blocked, ValueError) as error:
            facts['observations'].append({'phase': 'registration-after-command', 'state': 'unknown', 'error': str(error)})
    for attempt in range(1, 4):
        verified = installed_state(attempt)
        explicit_error = (facts['install_attempts'] and facts.get('install_exit_code') is not None and facts['install_exit_code'] != 0
                          and facts.get('install_client_outcome') != 'uncertain-after-timeout')
        if explicit_error:
            fail('native-install-error', 'native installer failed; actual command and installed-state observations retained')
        if facts['install_attempts'] and facts.get('install_exit_code') is None and facts.get('install_client_outcome') != 'uncertain-after-timeout':
            fail('unknown-install-client-result', 'installer client result is unknown; actual installed-state observations retained')
        if verified:
            facts.update(verified)
            if not facts['install_attempts']:
                facts['status'] = 'already-installed-verified'
            elif facts.get('install_client_outcome') == 'uncertain-after-timeout':
                facts['status'] = 'installed-verified-after-client-timeout'
            else:
                facts['status'] = 'installed-verified'
            return facts
        if attempt < 3:
            pause(1)
    status = ('not-verified-after-client-timeout' if facts.get('install_client_outcome') == 'uncertain-after-timeout'
              else 'registered-reader-not-verified')
    fail(status, 'owned reader installation could not be verified; no second install or Photos admission assumed',
         timed_out=(facts.get('install_client_outcome') == 'uncertain-after-timeout'
                    or report['commands'][-1].get('outcome') == 'uncertain-after-timeout'))


def print_console_receipt(work, report):
    # Keep the first native failure inspectable when artifact transfer is not
    # available. Only this owned probe's fixed text outputs are eligible.
    print("PVT native receipt:\n" + json.dumps(report, indent=2, sort_keys=True))
    for name in ('compile-generator.stderr', 'generate-synthetic.stderr',
                 'compile-sdk-canary.stderr', 'compile-reader.stderr',
                 'compile-sdk-canary.processes.json', 'compile-reader.processes.json',
                 'boot-owned-simulator.stdout', 'boot-owned-status.stdout',
                 'install-owned-reader.stdout', 'install-owned-reader.stderr', 'install-owned-reader.processes.json',
                 'host-simulator-log.stdout', 'host-simulator-log.stderr',
                 'grant-owned-photos.stderr', 'baseline.json',
                 'build-share-ui.stderr', 'test-share-ui.stdout', 'test-share-ui.stderr',
                 'share.json', 'share-transfer.json', 'share-types.json',
                 'after.json', 'shutdown-owned.stderr', 'delete-owned.stderr'):
        path = work / name
        if not path.is_file() or path.is_symlink():
            continue
        content = path.read_text(errors='replace')
        if len(content) > 12000:
            content = content[:6000] + '\n[console excerpt; complete text retained in artifact]\n' + content[-6000:]
        print(f"PVT diagnostic {name}:\n{content}")


def observe_media_preservation(report, motion, package):
    # Observe success and failure before the owned simulator is cleaned up.
    if motion is not None:
        try:
            report['synthetic_input_sha256_after'] = sha(motion)
            report['synthetic_input_unchanged'] = report['synthetic_input_sha256_after'] == report.get('synthetic_input_sha256')
        except OSError as error:
            report['synthetic_input_preservation_error'] = str(error)
            report['synthetic_input_unchanged'] = False
    if package is not None:
        try:
            if not package.exists() and report.get('package_sha256_before_import') is None:
                return
            if package.is_symlink():
                raise OSError('package path became a symlink')
            report['package_sha256_after_import'] = {
                path.name: sha(path) if path.is_file() and not path.is_symlink() else '<non-regular-entry>'
                for path in package.iterdir()}
            report['package_bytes_unchanged'] = report['package_sha256_after_import'] == report.get('package_sha256_before_import')
        except OSError as error:
            report['package_preservation_error'] = str(error)
            report['package_bytes_unchanged'] = False


def wrap_motion_photo(jpeg, movie):
    if not jpeg.startswith(b'\xff\xd8') or not jpeg.endswith(b'\xff\xd9'):
        raise Blocked('generated still is not one complete JPEG')
    # Explicit synthetic source facts: a 60-frame 30fps source and frame 30 cover.
    xmp = ('<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">'
           '<rdf:Description xmlns:GCamera="http://ns.google.com/photos/1.0/camera/" '
           'GCamera:MicroVideo="1" GCamera:MicroVideoVersion="1" '
           f'GCamera:MicroVideoOffset="{len(movie)}" GCamera:MicroVideoPresentationTimestampUs="1000000"/>'
           '</rdf:RDF></x:xmpmeta>').encode()
    packet = b'http://ns.adobe.com/xap/1.0/\x00' + xmp
    if len(packet) + 2 > 65535:
        raise Blocked('synthetic XMP exceeds APP1')
    return jpeg[:2] + b'\xff\xe1' + struct.pack('>H', len(packet) + 2) + packet + jpeg[2:] + movie


def select_device(runtimes, types):
    candidates = [item for item in runtimes['runtimes'] if item.get('isAvailable') is True
                  and '.iOS-' in item.get('identifier', '') and int(item['version'].split('.')[0]) >= 17]
    candidates.sort(key=lambda item: tuple(int(x) for x in item['version'].split('.')), reverse=True)
    for runtime in candidates:
        supported = {item['identifier'] for item in runtime.get('supportedDeviceTypes', [])}
        parts = [int(x) for x in runtime['version'].split('.')]
        encoded = parts[0] * 65536 + (parts[1] if len(parts) > 1 else 0) * 256 + (parts[2] if len(parts) > 2 else 0)
        compatible = [item for item in types['devicetypes'] if item.get('productFamily') == 'iPhone'
                      and item['identifier'] in supported
                      and item.get('minRuntimeVersion', 0) <= encoded <= item.get('maxRuntimeVersion', 2**32)]
        if compatible:
            return runtime['identifier'], compatible[-1]['identifier']
    raise Blocked('no installed available iOS runtime explicitly supports a compatible catalogued iPhone; no runtime installation attempted')


def check_readback(baseline, after, expected_id, expected_hashes, cover_seconds):
    if baseline.get('status') != 'baseline' or not isinstance(baseline.get('asset_identifiers'), list):
        raise Blocked('new owned simulator did not present an authorized asset-identifier baseline')
    added = set(after.get('asset_identifiers', [])) - set(baseline['asset_identifiers'])
    removed = set(baseline['asset_identifiers']) - set(after.get('asset_identifiers', []))
    if after.get('status') != 'readback-complete' or len(added) != 1 or removed or not after.get('photo_live'):
        raise Blocked('direct PVT import did not yield exactly one readable photoLive asset')
    if after.get('asset_local_identifier') not in added:
        raise Blocked('readback did not inspect the newly imported asset')
    photos = [resource for resource in after['resources'] if resource['is_original_photo']]
    videos = [resource for resource in after['resources'] if resource['is_original_paired_video']]
    if len(photos) != 1 or len(videos) != 1:
        raise Blocked('asset does not have exactly one original photo and one original pairedVideo')
    if photos[0].get('maker_apple_17') != expected_id or videos[0].get('content_identifiers') != [expected_id]:
        raise Blocked('native still MakerApple17 / video content identifier does not match current CLI output')
    covers = videos[0].get('still_image_time_samples', [])
    if not any(sample['timescale'] > 0 and sample['epoch'] == 0
               and abs(sample['value'] / sample['timescale'] - cover_seconds) <= 1e-9 for sample in covers):
        raise Blocked('native timed metadata does not contain the expected exact cover timestamp')
    matches = {'photo': photos[0]['sha256'] == expected_hashes['photo'],
               'paired_video': videos[0]['sha256'] == expected_hashes['paired_video']}
    return {'one_live_photo_asset': True, 'pair_identifier_matches': True, 'cover_time_matches': True,
            'original_resource_hashes_match': matches, 'original_resource_bytes_match': all(matches.values())}


def prepare_share_tests(source, work, bundle, *, source_head):
    if not re.fullmatch(r'org\.xdremux\.audit\.PVTReadback\.u[0-9a-f]{32}', bundle):
        raise Blocked('share test bundle does not belong to this probe')
    target = work / 'share-tests'
    target.mkdir()
    shutil.copytree(source / 'PVTShareTests.xcodeproj', target / 'PVTShareTests.xcodeproj')
    template = (source / 'PVTShareUITests.swift.in').read_text()
    if template.count('@@BUNDLE_ID@@') != 1 or template.count('@@SOURCE_HEAD@@') != 1 or not re.fullmatch(r'[0-9a-f]{40}', source_head):
        raise Blocked('share UI test source/session binding is not singular and complete')
    (target / 'PVTShareUITests.swift').write_text(template.replace('@@BUNDLE_ID@@', bundle).replace('@@SOURCE_HEAD@@', source_head))
    return target / 'PVTShareTests.xcodeproj'


def check_share_receipt(shared, transfer, package, expected_members, *, source_head=None, session_id=None):
    if source_head is not None:
        check_reader_observation(shared, source_head=source_head, session_id=session_id, phase='share')
        check_reader_observation(transfer, source_head=source_head, session_id=session_id, phase='share-transfer')
    if (shared.get('status') != 'completed' or shared.get('activity_completed') is not True
            or shared.get('is_save_to_camera_roll') is not True):
        raise Blocked('native Photos save activity did not complete; no admission assumed')
    native_url = Path(transfer.get('whole_package_url', ''))
    representation = transfer.get('representation')
    phase = transfer.get('share_phase')
    valid_representation = (representation == 'file' and phase == 'share' and transfer.get('type_identifier') == 'com.apple.live-photo'
                            or representation == 'whole-package-url' and phase == 'share-url')
    if (not valid_representation or transfer.get('is_in_place') is not False
            or shared.get('share_phase') != phase
            or not native_url.is_absolute() or native_url.name != package.name
            or transfer.get('member_sha256') != expected_members):
        raise Blocked('system share did not receive the exact whole PVT input')


def reader_info(bundle):
    return {'CFBundleIdentifier': bundle, 'CFBundleExecutable': 'PVTReadback', 'CFBundleName': 'PVTReadback',
            'CFBundlePackageType': 'APPL', 'CFBundleVersion': '1', 'CFBundleShortVersionString': '1.0',
            'MinimumOSVersion': '17.0', 'LSRequiresIPhoneOS': True, 'UILaunchScreen': {},
            'CFBundleSupportedPlatforms': ['iPhoneSimulator'], 'UIDeviceFamily': [1],
            'UIApplicationSceneManifest': {'UIApplicationSupportsMultipleScenes': False,
                'UISceneConfigurations': {'UIWindowSceneSessionRoleApplication': [
                    {'UISceneConfigurationName': 'PVT audit'}]}},
            'NSPhotoLibraryUsageDescription': 'Read and verify synthetic media in this disposable audit simulator.',
            'NSPhotoLibraryAddUsageDescription': 'Save only the synthetic whole PVT through the system activity in this disposable simulator.'}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--cli', type=Path, required=True)
    parser.add_argument('--expected-head', required=True)
    parser.add_argument('--output-root', type=Path, required=True)
    parser.add_argument('--execute-owned-simulator', action='store_true')
    args = parser.parse_args()
    args.repo = args.repo.resolve()
    args.cli = args.cli.resolve()
    args.output_root = args.output_root.resolve()
    source = Path(__file__).resolve().parent
    session_id = uuid.uuid4().hex
    stages = PVTStages(args.expected_head, session_id, ('preparation', 'readiness', 'baseline', 'save_callback', 'readback'))
    args.output_root.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix='pvt-native-', dir=args.output_root))
    report = {'status': 'pending-native-execution', 'work': str(work), 'host_system': platform.system(),
              'host_architecture': platform.machine(), 'host_version': platform.mac_ver()[0],
              'commands': [], 'session_id': session_id, 'stage_observations': stages.facts,
              'direct_package_import_only': True, 'media_uploaded': False,
              'native_route': 'iOS Simulator system Photos save activity; whole package URL or live-photo file representation',
              'not_exercised': ['macOS Photos library import', 'physical-device Photos or wallpaper'],
              'known_unsupported_route_not_repeated': 'simctl addmedia rejects PVT directories (NSPOSIXErrorDomain22)',
              'probe_source_sha256': {str(path.relative_to(source)): sha(path) for path in source.rglob('*')
                                      if path.is_file() and (path.name.endswith(('.py', '.swift', '.swift.in', '.xcscheme'))
                                                             or path.name == 'project.pbxproj')}}
    owned_udid = None
    motion = None
    package = None

    def run(name, command, *, timeout=60, required=True):
        record = {'name': name, 'argv': list(map(str, command)),
                  'started_elapsed_seconds': stages.clock() - stages.started, 'timeout_seconds': timeout}
        report['commands'].append(record)
        started = time.monotonic()
        try:
            if name in ('compile-sdk-canary', 'compile-reader', 'install-owned-reader', 'host-simulator-log'):
                progress = None if name == 'host-simulator-log' else work / (name + '.processes.json')
                if progress:
                    record['progress_file'] = progress.name
                record['timeout_seconds'] = timeout
                record.update(run_owned_process(record['argv'], args.repo, work / (name + '.stdout'),
                                                work / (name + '.stderr'), timeout=timeout, diagnostic_path=progress))
                if record.get('outcome'):
                    raise Blocked(f'{name}: {record["outcome"]}; owned process stages and stdout/stderr retained',
                                  timed_out=record['outcome'] == 'uncertain-after-timeout')
                result = subprocess.CompletedProcess(record['argv'], record['exit_code'],
                    (work / (name + '.stdout')).read_bytes(), (work / (name + '.stderr')).read_bytes())
            else:
                result = subprocess.run(record['argv'], cwd=args.repo, capture_output=True, timeout=timeout)
            (work / (name + '.stdout')).write_bytes(result.stdout)
            (work / (name + '.stderr')).write_bytes(result.stderr)
            record['exit_code'] = result.returncode
            if required and result.returncode:
                raise Blocked(f'{name} exited {result.returncode}; original stdout/stderr retained')
            return result
        except (OSError, subprocess.TimeoutExpired) as error:
            record['error'] = str(error)
            record['outcome'] = 'uncertain-after-timeout' if isinstance(error, subprocess.TimeoutExpired) else 'not-started'
            if isinstance(error, subprocess.TimeoutExpired):
                (work / (name + '.stdout')).write_bytes(error.stdout or b'')
                (work / (name + '.stderr')).write_bytes(error.stderr or b'')
            raise Blocked(f'{name}: {error}', timed_out=isinstance(error, subprocess.TimeoutExpired)) from error
        finally:
            record['duration_seconds'] = time.monotonic() - started

    def app_container(phase, bundle):
        container = Path(run('container-' + phase, ['xcrun', 'simctl', 'get_app_container', owned_udid, bundle, 'data']).stdout.decode().strip())
        if not container.is_absolute() or owned_udid.lower() not in {part.lower() for part in container.parts}:
            raise Blocked('reader container path is not within the owned simulator')
        return container

    def read_report(phase, bundle):
        stage = 'baseline' if phase == 'baseline' else 'readback'
        stages.record(stage, 'started', 'photokit_launch_submitted')
        run('launch-' + phase, ['xcrun', 'simctl', 'launch', owned_udid, bundle, '--phase', phase,
                                '--source-head', args.expected_head, '--session-id', session_id])
        container = app_container(phase, bundle)
        deadline = time.monotonic() + 45
        path = container / 'Documents' / (phase + '.json')
        while not path.is_file() and time.monotonic() < deadline:
            time.sleep(0.2)
        if not path.is_file():
            raise Blocked('PhotoKit report timeout; no authorization or readback success assumed', timed_out=True)
        shutil.copy2(path, work / (phase + '.json'))
        value = json.loads(path.read_text())
        check_reader_observation(value, source_head=args.expected_head, session_id=session_id, phase=phase)
        stages.record(stage, 'observed', 'photokit_report_received', report_sha256=sha(path), native_status=value.get('status'))
        if phase == 'after' and (container / 'Documents/readback').exists():
            # These are exports after direct import, never import inputs.
            shutil.copytree(container / 'Documents/readback', work / 'readback')
        run('terminate-' + phase, ['xcrun', 'simctl', 'terminate', owned_udid, bundle], required=False)
        return value

    try:
        stages.record('preparation', 'started', 'source_check_started')
        head = run('head', ['git', 'rev-parse', 'HEAD']).stdout.decode().strip()
        status = run('source-status', ['git', 'status', '--porcelain']).stdout
        report['head'] = head
        if head != args.expected_head or status:
            raise Blocked('expected committed HEAD and clean checkout are required')
        if platform.system() != 'Darwin':
            # Retain the actual missing-tool error, not a synthetic native success.
            run('simulator-sdk-path', ['xcrun', '--sdk', 'iphonesimulator', '--show-sdk-path'])
            raise Blocked('native probe requires macOS')
        run('xcode-version', ['xcodebuild', '-version'])
        run('swift-version', ['xcrun', '--sdk', 'iphonesimulator', 'swiftc', '--version'])
        run('macos-version', ['sw_vers'])
        privacy = run('simctl-privacy-help', ['xcrun', 'simctl', 'help', 'privacy'], required=False)
        help_text = (privacy.stdout + privacy.stderr).decode(errors='replace')
        privacy_available = privacy.returncode == 0 and bool(re.search(r'\bgrant\b', help_text)) and bool(re.search(r'\bphotos\b', help_text))
        report['official_owned_photos_grant_available'] = privacy_available
        runtime_data = json.loads(run('simctl-runtimes', ['xcrun', 'simctl', 'list', 'runtimes', '--json']).stdout)
        types_data = json.loads(run('simctl-device-types', ['xcrun', 'simctl', 'list', 'devicetypes', '--json']).stdout)
        runtime, device = select_device(runtime_data, types_data)
        report['selected_runtime'] = runtime
        report['selected_device_type'] = device
        selected_runtime = next(item for item in runtime_data['runtimes'] if item.get('identifier') == runtime)
        selected_device = next(item for item in types_data['devicetypes'] if item.get('identifier') == device)
        report['selected_runtime_facts'] = {key: value for key, value in selected_runtime.items()
                                           if key in ('identifier', 'version', 'buildversion', 'isAvailable', 'supportedArchitectures', 'supportedDeviceTypes')}
        report['selected_device_type_facts'] = {key: value for key, value in selected_device.items()
                                               if key in ('identifier', 'name', 'productFamily', 'modelIdentifier', 'minRuntimeVersion',
                                                          'maxRuntimeVersion', 'minRuntimeVersionString', 'maxRuntimeVersionString')}
        sdk_result = run('simulator-sdk-path', ['xcrun', '--sdk', 'iphonesimulator', '--show-sdk-path'], required=False)
        sdk = sdk_result.stdout.decode().strip() if sdk_result.returncode == 0 else None
        run('simulator-sdk-version', ['xcrun', '--sdk', 'iphonesimulator', '--show-sdk-version'], required=False)
        if not args.execute_owned_simulator:
            raise Blocked('capability discovery completed; owned-simulator execution not selected')
        build = run('build-current-cli', ['cargo', 'build', '--locked', '-p', 'xdremux-cli', '--message-format=json-render-diagnostics'], timeout=300)
        artifacts = [json.loads(line) for line in build.stdout.decode().splitlines() if line.startswith('{')]
        executables = [item['executable'] for item in artifacts if item.get('reason') == 'compiler-artifact'
                       and item.get('target', {}).get('name') == 'xdremux' and item.get('executable')]
        if len(executables) != 1 or Path(executables[0]).resolve() != args.cli.resolve():
            raise Blocked('--cli does not name the executable freshly produced by the current checkout')
        if run('built-source-status', ['git', 'status', '--porcelain']).stdout or run('built-head', ['git', 'rev-parse', 'HEAD']).stdout.decode().strip() != head:
            raise Blocked('checkout changed during current CLI build')
        report['binary_sha256'] = sha(args.cli)
        run('cli-version', [args.cli, '--version'])
        generator = work / 'generate-synthetic'
        run('compile-generator', ['xcrun', '--sdk', 'macosx', 'swiftc', '-swift-version', '5', '-parse-as-library', source / 'GenerateSyntheticMotion.swift', '-o', generator], timeout=120)
        run('generate-synthetic', [generator, work])
        motion = work / 'synthetic-motion.jpg'
        motion.write_bytes(wrap_motion_photo((work / 'synthetic.jpg').read_bytes(), (work / 'synthetic.mp4').read_bytes()))
        report['synthetic_input_sha256'] = sha(motion)
        run('cli-inspect', [args.cli, 'inspect', motion, '--json'])
        package = work / 'capture.pvt'
        run('cli-convert-pvt', [args.cli, 'convert', '--input', motion, '--output', package, '--live-photo-output', 'pvt'], timeout=120)
        report['package_sha256_before_import'] = {path.name: sha(path) for path in package.iterdir()}
        validation = json.loads(run('cli-validate-pvt', [args.cli, 'validate', package, '--json']).stdout)
        if not validation.get('valid') or validation.get('kind') != 'live-photo':
            raise Blocked('current CLI PVT validation failed')
        pair = validation['details']['value']
        report['expected_identifier'] = pair['content_identifier']
        metadata = plistlib.loads((package / 'metadata.plist').read_bytes())
        if metadata != {'PFVideoComplementMetadataVersionKey': '1'}:
            raise Blocked('CLI package metadata differs from the string-version contract')
        token = session_id
        bundle = 'org.xdremux.audit.PVTReadback.u' + token
        report['probe_bundle_id'] = bundle
        report['owned_device_name'] = 'pvt-audit-' + token
        app = work / 'PVTReadback.app'
        app.mkdir()
        (app / 'Info.plist').write_bytes(plistlib.dumps(reader_info(bundle)))
        if not privacy_available or not sdk:
            raise Blocked('documented owned photos grant or simulator SDK unavailable; no alternate permission setup')
        architecture = platform.machine()
        if architecture != 'arm64':
            raise Blocked('complete native PVT probe requires the arm64 product runner')
        compile_command = ['xcrun', '--sdk', 'iphonesimulator', 'swiftc', '-swift-version', '5', '-parse-as-library',
                           '-target', architecture + '-apple-ios17.0-simulator', '-sdk', sdk,
                           '-module-cache-path', work / 'sdk-module-cache',
                           '-Xfrontend', '-debug-time-function-bodies',
                           '-Xfrontend', '-warn-long-function-bodies=500']
        run('compile-sdk-canary', [*compile_command, source / 'SDKCompileProbe.swift', '-o', work / 'sdk-canary'], timeout=240)
        run('compile-reader', [*compile_command, source / 'ReadbackCore.swift', source / 'PVTReadback.swift',
                               '-o', app / 'PVTReadback'], timeout=120)
        run('sign-owned-reader', ['codesign', '--force', '--sign', '-', app])
        report['reader_binary_sha256'] = sha(app / 'PVTReadback')
        report['reader_info_sha256'] = sha(app / 'Info.plist')
        stages.record('preparation', 'completed', 'reader_signed_and_package_validated')
        stages.record('readiness', 'started', 'simulator_create_submitted')
        result = run('create-owned-simulator', ['xcrun', 'simctl', 'create', 'pvt-audit-' + token, device, runtime])
        owned_udid = str(uuid.UUID(result.stdout.decode().strip())).upper()
        report['owned_udid'] = owned_udid
        (work / 'owned-simulator.json').write_text(json.dumps({'udid': owned_udid, 'name': 'pvt-audit-' + token, 'bundle': bundle}))
        run('boot-owned-simulator', ['xcrun', 'simctl', 'boot', owned_udid])
        run('boot-owned-status', ['xcrun', 'simctl', 'bootstatus', owned_udid, '-b'], timeout=180)
        install_owned_reader(run, owned_udid, bundle, app, work, report)
        # Only the exact created UDID and installed bundle; no host TCC/SQLite.
        run('grant-owned-photos', ['xcrun', 'simctl', 'privacy', owned_udid, 'grant', 'photos', bundle])
        stages.record('readiness', 'completed', 'reader_installed_and_photos_granted', owned_udid=owned_udid)
        baseline = read_report('baseline', bundle)
        if baseline.get('status') != 'baseline' or not isinstance(baseline.get('asset_identifiers'), list):
            raise Blocked('owned simulator did not provide an authorized baseline identifier set')
        stages.record('baseline', 'completed', 'authorized_asset_baseline_validated')
        container = app_container('share', bundle)
        transported = container / 'Documents/capture.pvt'
        # Transport the complete validated directory unchanged. No resource
        # list is passed to PhotoKit, and no new pair is created by this probe.
        shutil.copytree(package, transported)
        expected_members = report['package_sha256_before_import']
        before_share = {path.name: sha(path) for path in transported.iterdir()}
        if before_share != expected_members:
            raise Blocked('whole-package transport changed source bytes before sharing')
        report['transported_package_sha256_before_share'] = before_share
        # Activity availability control only: never save this still to Photos.
        shutil.copy2(work / 'synthetic.jpg', container / 'Documents/control.jpg')
        project = prepare_share_tests(source, work, bundle, source_head=head)
        report['ui_test_source_sha256'] = sha(project.parent / 'PVTShareUITests.swift')
        ui_command = ['xcodebuild', '-project', project, '-scheme', 'PVTShareUITests',
                      '-destination', 'id=' + owned_udid, '-derivedDataPath', work / 'ui-build',
                      'CODE_SIGNING_ALLOWED=NO', 'PRODUCT_BUNDLE_IDENTIFIER=' + bundle + '.uitests']
        run('build-share-ui', [*ui_command, 'build-for-testing'], timeout=300)
        report['native_share_ui_test_invoked'] = True
        try:
            command = list(map(str, [*ui_command, '-resultBundlePath', work / 'share-results.xcresult',
                                    '-parallel-testing-enabled', 'NO',
                                    '-only-testing:PVTShareUITests/PVTShareUITests/testWholePackageSystemShare',
                                    'test-without-building']))
            record = {'name': 'test-share-ui', 'argv': command}
            report['commands'].append(record)
            record.update(run_share_ui(command, args.repo, work / 'test-share-ui.stdout',
                                       work / 'test-share-ui.stderr', timeout=180, source_head=head, session_id=session_id))
            report['share_observations'] = record['stage_observations']
            if record.get('outcome') == 'native-save-activity-unavailable':
                try:
                    after = read_report('after', bundle)
                    report['unavailable_save_asset_delta'] = {
                        'added': sorted(set(after.get('asset_identifiers', [])) - set(baseline['asset_identifiers'])),
                        'removed': sorted(set(baseline['asset_identifiers']) - set(after.get('asset_identifiers', []))),
                        'readback_status': after.get('status')}
                except Blocked as error:
                    report['unavailable_save_readback_error'] = str(error)
                raise Blocked('whole-package URL and live-photo provider expose no identifiable Photos save activity; native import remains unaccepted')
            if record.get('outcome') or record['exit_code']:
                raise Blocked('test-share-ui: ' + record.get('outcome', 'exit ' + str(record['exit_code'])),
                              timed_out=record.get('outcome') == 'uncertain-after-timeout')
            if (record['stage_observations']['format'] != 'structured-events'
                    or any(record['stage_observations']['phases'][phase]['state'] != 'completed'
                           for phase in ('test_setup', 'test_body', 'save_callback'))):
                raise Blocked('test-share-ui ended without complete source-bound setup/body/save observations')
        finally:
            # Retain native callbacks on UI failure as well as success.
            for phase in ('share', 'share-transfer', 'share-types'):
                path = container / 'Documents' / (phase + '.json')
                if path.is_file() and not path.is_symlink():
                    shutil.copy2(path, work / (phase + '.json'))
            report['transported_package_sha256_after_share'] = {
                path.name: sha(path) if path.is_file() and not path.is_symlink() else '<non-regular-entry>'
                for path in transported.iterdir()} if transported.is_dir() and not transported.is_symlink() else None
        shared = json.loads((work / 'share.json').read_text())
        transfer = json.loads((work / 'share-transfer.json').read_text())
        check_reader_observation(shared, source_head=head, session_id=session_id, phase='share')
        stages.record('save_callback', 'observed', 'native_callback_receipt_received',
                      report_sha256=sha(work / 'share.json'), native_status=shared.get('status'),
                      native_observation=shared['observation'])
        check_share_receipt(shared, transfer, transported, expected_members, source_head=head, session_id=session_id)
        stages.record('save_callback', 'completed', 'whole_package_save_callback_validated')
        if report['transported_package_sha256_after_share'] != expected_members:
            raise Blocked('native sharing changed the transported package bytes')
        report['system_save_activity_completed'] = True
        report['whole_pvt_input_representation'] = transfer['representation']
        report['whole_pvt_representation_requested'] = transfer['representation'] == 'file'
        after = read_report('after', bundle)
        added = set(after.get('asset_identifiers', [])) - set(baseline['asset_identifiers'])
        report['single_live_asset_admission_verified'] = (len(added) == 1 and after.get('asset_local_identifier') in added and after.get('photo_live') is True)
        expected = {'photo': sha(Path(pair['image'])), 'paired_video': sha(Path(pair['video']))}
        report['native_facts'] = check_readback(baseline, after, pair['content_identifier'], expected, pair['still_time_seconds'])
        stages.record('readback', 'observed', 'photokit_resource_relationships_validated')
        # Exported bytes are post-import readback, never an alternate import path.
        exported = after['resources']
        photo = next(item for item in exported if item['is_original_photo'])
        video = next(item for item in exported if item['is_original_paired_video'])
        checked = work / 'readback-pair'
        checked.mkdir()
        shutil.copy2(work / 'readback' / photo['exported_name'], checked / 'capture.heic')
        shutil.copy2(work / 'readback' / video['exported_name'], checked / 'capture.mov')
        run('cli-validate-exported-pair', [args.cli, 'validate', checked / 'capture.heic', '--json'])
        if not report['native_facts']['original_resource_bytes_match']:
            raise Blocked('native admission and pairing verified, but original resource byte preservation failed')
        stages.record('readback', 'completed', 'exported_resources_validated_and_bytes_preserved')
        stages.facts['admission'] = 'verified'
        report['status'] = 'native-direct-pvt-import-and-readback-pass'
    except Blocked as error:
        report['status'] = 'blocked'
        report['reason'] = str(error)
        stages.stop_active(timed_out=error.timed_out)
    except Exception as error:
        report['status'] = 'probe-error'
        report['reason'] = repr(error)
    finally:
        stages.stop_active()
        observe_media_preservation(report, motion, package)
        if report['status'].endswith('-pass') and (report.get('synthetic_input_unchanged') is not True or report.get('package_bytes_unchanged') is not True):
            report['status'] = 'native-validation-passed-source-preservation-failed'
            report['reason'] = 'source/package byte preservation failed or could not be observed'
        report['native_validation_status_before_cleanup'] = report['status']
        cleanup_failed = False
        if owned_udid:
            cleanup = cleanup_owned_simulator(run, owned_udid,
                {'owned_udid': owned_udid, 'source_head': args.expected_head, 'session_id': session_id},
                Path.home() / 'Library/Developer/CoreSimulator/Devices', prefix='product', command_records=report['commands'])
            report['cleanup_observations'] = cleanup
            for verb, row in cleanup['commands'].items():
                for field in ('exit_code', 'error'):
                    if field in row:
                        report[verb + '-owned-' + ('exit' if field == 'exit_code' else 'error')] = row[field]
            report['cleanup_absence_verified'] = cleanup['absence']['verified_absent']
            cleanup_failed = not cleanup['accepted']
            report['cleanup_status'] = 'failed' if cleanup_failed else 'completed'
        else:
            creation = next((record for record in report['commands'] if record['name'] == 'create-owned-simulator'), None)
            report['cleanup_status'] = 'creation-outcome-uncertain-no-udid' if creation else 'not-created'
        if cleanup_failed and report['status'].endswith('-pass'):
            report['status'] = 'native-validation-passed-cleanup-failed'
            report['reason'] = 'owned simulator cleanup commands or independent absence verification failed; exact observations retained'
        report['probe_driver_sha256'] = sha(Path(__file__))
        (work / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
        print_console_receipt(work, report)
        print(json.dumps({'status': report['status'], 'report': str(work / 'result.json'), 'reason': report.get('reason')}))
    return 0 if report['status'].endswith('-pass') else 2


if __name__ == '__main__':
    sys.exit(main())
