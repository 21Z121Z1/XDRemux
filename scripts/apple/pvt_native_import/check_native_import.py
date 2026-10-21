"""Verify direct PVT import in one disposable simulator; never use host Photos."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import plistlib
import re
import shutil
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


def print_console_receipt(work, report):
    # Keep the first native failure inspectable when artifact transfer is not
    # available. Only this owned probe's fixed text outputs are eligible.
    print("PVT native receipt:\n" + json.dumps(report, indent=2, sort_keys=True))
    for name in ('simctl-addmedia-help.stdout', 'compile-generator.stderr',
                 'generate-synthetic.stderr', 'compile-reader.stderr',
                 'grant-owned-photos.stderr', 'baseline.json',
                 'direct-pvt-addmedia.stdout', 'direct-pvt-addmedia.stderr',
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
                  and '.iOS-' in item.get('identifier', '')]
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
              'commands': [], 'direct_package_import_only': True, 'media_uploaded': False,
              'probe_source_sha256': {path.name: sha(path) for path in source.glob('*.swift')}}
    owned_udid = None
    motion = None
    package = None

    def run(name, command, *, timeout=60, required=True):
        record = {'name': name, 'argv': list(map(str, command))}
        report['commands'].append(record)
        try:
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

    def read_report(phase, bundle):
        run('launch-' + phase, ['xcrun', 'simctl', 'launch', owned_udid, bundle, '--phase', phase])
        container = Path(run('container-' + phase, ['xcrun', 'simctl', 'get_app_container', owned_udid, bundle, 'data']).stdout.decode().strip())
        if not container.is_absolute() or owned_udid.lower() not in {part.lower() for part in container.parts}:
            raise Blocked('reader container path is not within the owned simulator')
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
            run('simctl-addmedia-help', ['xcrun', 'simctl', 'help', 'addmedia'])
            raise Blocked('native probe requires macOS')
        run('xcode-version', ['xcodebuild', '-version'])
        run('simctl-addmedia-help', ['xcrun', 'simctl', 'help', 'addmedia'])
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
        result = run('create-owned-simulator', ['xcrun', 'simctl', 'create', 'pvt-audit-' + token, device, runtime])
        owned_udid = str(uuid.UUID(result.stdout.decode().strip())).upper()
        report['owned_udid'] = owned_udid
        (work / 'owned-simulator.json').write_text(json.dumps({'udid': owned_udid, 'name': 'pvt-audit-' + token, 'bundle': bundle}))
        run('boot-owned-simulator', ['xcrun', 'simctl', 'boot', owned_udid])
        run('boot-owned-status', ['xcrun', 'simctl', 'bootstatus', owned_udid, '-b'], timeout=180)
        app = work / 'PVTReadback.app'
        app.mkdir()
        info = {'CFBundleIdentifier': bundle, 'CFBundleExecutable': 'PVTReadback', 'CFBundleName': 'PVTReadback',
                'CFBundlePackageType': 'APPL', 'CFBundleVersion': '1', 'CFBundleShortVersionString': '1.0',
                'MinimumOSVersion': '17.0', 'LSRequiresIPhoneOS': True, 'UILaunchScreen': {},
                'CFBundleSupportedPlatforms': ['iPhoneSimulator'], 'UIDeviceFamily': [1],
                'NSPhotoLibraryUsageDescription': 'Read only the synthetic media in this disposable audit simulator.'}
        (app / 'Info.plist').write_bytes(plistlib.dumps(info))
        baseline = None
        try:
            if not privacy_available or not sdk:
                raise Blocked('documented owned photos grant or simulator SDK unavailable; no alternate permission setup')
            architecture = platform.machine()
            if architecture not in ('arm64', 'x86_64'):
                raise Blocked('unrecognized simulator host architecture')
            run('compile-reader', ['xcrun', '--sdk', 'iphonesimulator', 'swiftc', '-swift-version', '5', '-parse-as-library',
                                  '-target', architecture + '-apple-ios17.0-simulator', '-sdk', sdk,
                                  source / 'PVTReadback.swift', '-o', app / 'PVTReadback'], timeout=120)
            run('sign-owned-reader', ['codesign', '--force', '--sign', '-', app])
            run('install-owned-reader', ['xcrun', 'simctl', 'install', owned_udid, app])
            # Explicitly authorized setup, only this created UDID and installed bundle.
            # Never host TCC, SQLite, globally selected simulator, or unrelated app IDs.
            run('grant-owned-photos', ['xcrun', 'simctl', 'privacy', owned_udid, 'grant', 'photos', bundle])
            baseline = read_report('baseline', bundle)
            if baseline.get('status') != 'baseline' or not isinstance(baseline.get('asset_identifiers'), list):
                raise Blocked('owned simulator did not provide an authorized baseline identifier set')
        except Blocked as error:
            report['readback_setup_blocked'] = str(error)
            baseline = None
        # The sole import call: exactly the complete directory, no child paths.
        report['direct_import_attempted'] = True
        result = run('direct-pvt-addmedia', ['xcrun', 'simctl', 'addmedia', owned_udid, package], timeout=90, required=False)
        report['direct_import_exit_code'] = result.returncode
        if result.returncode:
            raise Blocked('native whole-directory PVT import rejected; no pair fallback attempted')
        report['direct_import_command_succeeded'] = True
        if baseline is None:
            raise Blocked('whole-directory addmedia exited zero; asset admission remains unconfirmed because readback setup failed')
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
