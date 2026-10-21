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
import struct
import subprocess
import sys
import tempfile
import time
import uuid


class Blocked(RuntimeError):
    pass


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


def run_share_ui(command, cwd, stdout_path, stderr_path, *, timeout):
    # Missing Save is a failed native precondition, never import success.
    facts = run_owned_process(command, cwd, stdout_path, stderr_path, timeout=timeout,
                              stop_marker=b'PVT_SHARE_UI_BLOCKED:')
    facts['ui_observations'] = [{'phase': phase, 'save_candidates': int(count)} for phase, count in re.findall(
        r'PVT_SHARE_UI_OBSERVATION: phase=(share(?:-control|-url)?) save_candidates=(\d+)',
        stdout_path.read_text(errors='replace'))]
    return facts


def print_console_receipt(work, report):
    # Keep the first native failure inspectable when artifact transfer is not
    # available. Only this owned probe's fixed text outputs are eligible.
    print("PVT native receipt:\n" + json.dumps(report, indent=2, sort_keys=True))
    for name in ('compile-generator.stderr', 'generate-synthetic.stderr',
                 'compile-sdk-canary.stderr', 'compile-reader.stderr',
                 'compile-sdk-canary.processes.json', 'compile-reader.processes.json',
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
        parts = [int(x) for x in runtime['version'].split('.')]
        encoded = parts[0] * 65536 + (parts[1] if len(parts) > 1 else 0) * 256 + (parts[2] if len(parts) > 2 else 0)
        compatible = [item for item in types['devicetypes'] if item.get('productFamily') == 'iPhone'
                      and item.get('minRuntimeVersion', 0) <= encoded <= item.get('maxRuntimeVersion', 2**32)]
        if compatible:
            return runtime['identifier'], compatible[-1]['identifier']
    raise Blocked('no installed available iOS runtime with a compatible iPhone device type; no runtime installation attempted')


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


def prepare_share_tests(source, work, bundle):
    if not re.fullmatch(r'org\.xdremux\.audit\.PVTReadback\.u[0-9a-f]{32}', bundle):
        raise Blocked('share test bundle does not belong to this probe')
    target = work / 'share-tests'
    target.mkdir()
    shutil.copytree(source / 'PVTShareTests.xcodeproj', target / 'PVTShareTests.xcodeproj')
    template = (source / 'PVTShareUITests.swift.in').read_text()
    if template.count('@@BUNDLE_ID@@') != 1:
        raise Blocked('share UI test bundle binding is not singular')
    (target / 'PVTShareUITests.swift').write_text(template.replace('@@BUNDLE_ID@@', bundle))
    return target / 'PVTShareTests.xcodeproj'


def check_share_receipt(shared, transfer, package, expected_members):
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
    args.output_root.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix='pvt-native-', dir=args.output_root))
    report = {'status': 'pending-native-execution', 'work': str(work), 'host_system': platform.system(),
              'host_architecture': platform.machine(), 'host_version': platform.mac_ver()[0],
              'commands': [], 'direct_package_import_only': True, 'media_uploaded': False,
              'native_route': 'iOS Simulator system Photos save activity; whole package URL or live-photo file representation',
              'not_exercised': ['macOS Photos library import', 'physical-device Photos or wallpaper'],
              'known_unsupported_route_not_repeated': 'simctl addmedia rejects PVT directories (NSPOSIXErrorDomain22)',
              'probe_source_sha256': {str(path.relative_to(source)): sha(path) for path in source.rglob('*')
                                      if path.is_file() and (path.name.endswith(('.swift', '.swift.in', '.xcscheme'))
                                                             or path.name == 'project.pbxproj')}}
    owned_udid = None
    motion = None
    package = None

    def run(name, command, *, timeout=60, required=True):
        record = {'name': name, 'argv': list(map(str, command))}
        report['commands'].append(record)
        started = time.monotonic()
        try:
            if name in ('compile-sdk-canary', 'compile-reader'):
                progress = work / (name + '.processes.json')
                record['progress_file'] = progress.name
                record['timeout_seconds'] = timeout
                record.update(run_owned_process(record['argv'], args.repo, work / (name + '.stdout'),
                                                work / (name + '.stderr'), timeout=timeout, diagnostic_path=progress))
                if record.get('outcome'):
                    raise Blocked(f'{name}: {record["outcome"]}; compiler process stages and stdout/stderr retained')
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
            raise Blocked(f'{name}: {error}') from error
        finally:
            record['duration_seconds'] = time.monotonic() - started

    def app_container(phase, bundle):
        container = Path(run('container-' + phase, ['xcrun', 'simctl', 'get_app_container', owned_udid, bundle, 'data']).stdout.decode().strip())
        if not container.is_absolute() or owned_udid.lower() not in {part.lower() for part in container.parts}:
            raise Blocked('reader container path is not within the owned simulator')
        return container

    def read_report(phase, bundle):
        run('launch-' + phase, ['xcrun', 'simctl', 'launch', owned_udid, bundle, '--phase', phase])
        container = app_container(phase, bundle)
        deadline = time.monotonic() + 45
        path = container / 'Documents' / (phase + '.json')
        while not path.is_file() and time.monotonic() < deadline:
            time.sleep(0.2)
        if not path.is_file():
            raise Blocked('PhotoKit report timeout; no authorization or readback success assumed')
        shutil.copy2(path, work / (phase + '.json'))
        value = json.loads(path.read_text())
        if phase == 'after' and (container / 'Documents/readback').exists():
            # These are exports after direct import, never import inputs.
            shutil.copytree(container / 'Documents/readback', work / 'readback')
        run('terminate-' + phase, ['xcrun', 'simctl', 'terminate', owned_udid, bundle], required=False)
        return value

    try:
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
        token = uuid.uuid4().hex
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
        # Compile before boot: cold SDK imports should not compete with an
        # idle simulator. Only cold module preparation gets a240s ceiling;
        # actual reader120s and UI180s remain separate bounded operations.
        run('compile-sdk-canary', [*compile_command, source / 'SDKCompileProbe.swift', '-o', work / 'sdk-canary'], timeout=240)
        run('compile-reader', [*compile_command, source / 'ReadbackCore.swift', source / 'PVTReadback.swift',
                               '-o', app / 'PVTReadback'], timeout=120)
        run('sign-owned-reader', ['codesign', '--force', '--sign', '-', app])
        report['reader_binary_sha256'] = sha(app / 'PVTReadback')
        report['reader_info_sha256'] = sha(app / 'Info.plist')
        result = run('create-owned-simulator', ['xcrun', 'simctl', 'create', 'pvt-audit-' + token, device, runtime])
        owned_udid = str(uuid.UUID(result.stdout.decode().strip())).upper()
        report['owned_udid'] = owned_udid
        (work / 'owned-simulator.json').write_text(json.dumps({'udid': owned_udid, 'name': 'pvt-audit-' + token, 'bundle': bundle}))
        run('boot-owned-simulator', ['xcrun', 'simctl', 'boot', owned_udid])
        run('boot-owned-status', ['xcrun', 'simctl', 'bootstatus', owned_udid, '-b'], timeout=180)
        run('install-owned-reader', ['xcrun', 'simctl', 'install', owned_udid, app])
        # Only the exact created UDID and installed bundle; no host TCC/SQLite.
        run('grant-owned-photos', ['xcrun', 'simctl', 'privacy', owned_udid, 'grant', 'photos', bundle])
        baseline = read_report('baseline', bundle)
        if baseline.get('status') != 'baseline' or not isinstance(baseline.get('asset_identifiers'), list):
            raise Blocked('owned simulator did not provide an authorized baseline identifier set')
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
        project = prepare_share_tests(source, work, bundle)
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
                                       work / 'test-share-ui.stderr', timeout=180))
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
                raise Blocked('test-share-ui: ' + record.get('outcome', 'exit ' + str(record['exit_code'])))
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
        check_share_receipt(shared, transfer, transported, expected_members)
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
        report['status'] = 'native-direct-pvt-import-and-readback-pass'
    except Blocked as error:
        report['status'] = 'blocked'
        report['reason'] = str(error)
    except Exception as error:
        report['status'] = 'probe-error'
        report['reason'] = repr(error)
    finally:
        observe_media_preservation(report, motion, package)
        if report['status'].endswith('-pass') and (report.get('synthetic_input_unchanged') is not True or report.get('package_bytes_unchanged') is not True):
            report['status'] = 'native-validation-passed-source-preservation-failed'
            report['reason'] = 'source/package byte preservation failed or could not be observed'
        report['native_validation_status_before_cleanup'] = report['status']
        cleanup_failed = False
        if owned_udid:
            # Only the exact UUID returned by this invocation is ever removed.
            for name, verb in [('shutdown-owned', 'shutdown'), ('delete-owned', 'delete')]:
                try:
                    result = run(name, ['xcrun', 'simctl', verb, owned_udid], required=False)
                    report[name + '-exit'] = result.returncode
                    cleanup_failed = cleanup_failed or result.returncode != 0
                except Exception as error:
                    report[name + '-error'] = str(error)
                    cleanup_failed = True
            report['cleanup_status'] = 'failed' if cleanup_failed else 'completed'
        else:
            creation = next((record for record in report['commands'] if record['name'] == 'create-owned-simulator'), None)
            report['cleanup_status'] = 'creation-outcome-uncertain-no-udid' if creation else 'not-created'
        if cleanup_failed and report['status'].endswith('-pass'):
            report['status'] = 'native-validation-passed-cleanup-failed'
            report['reason'] = 'owned simulator shutdown/delete failed; exact command receipts retained for cleanup'
        report['probe_driver_sha256'] = sha(Path(__file__))
        (work / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
        print_console_receipt(work, report)
        print(json.dumps({'status': report['status'], 'report': str(work / 'result.json'), 'reason': report.get('reason')}))
    return 0 if report['status'].endswith('-pass') else 2


if __name__ == '__main__':
    sys.exit(main())
