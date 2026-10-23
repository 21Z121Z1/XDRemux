"""One same-job readiness control, then at most one current-HEAD PVT probe."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import plistlib
import re
import subprocess
import sys
import tempfile
import time
import uuid

if __package__:
    from .observations import PVTStages
    from .owned_cleanup import cleanup_owned_simulator
else:
    from observations import PVTStages
    from owned_cleanup import cleanup_owned_simulator

NATIVE_PASS = 'native-direct-pvt-import-and-readback-pass'


class ReadinessBlocked(RuntimeError):
    pass


def run_control(run, choose_device, report, name, device_root, *, capture_diagnostics=None):
    stages = PVTStages(report.get('expected_head'), name, ('readiness',))
    facts = {'name': name, 'source_head': report.get('expected_head'), 'status': 'pending', 'listapps_attempts': 0,
             'cleanup_status': 'not-created', 'stage_observations': stages.facts}
    report['control'] = facts
    owned = None
    ready = False
    create_submitted = False
    try:
        if not re.fullmatch(r'pvt-readiness-control-[0-9a-f]{32}', name):
            raise ReadinessBlocked('control name does not belong to this invocation')
        stages.record('readiness', 'started', 'runtime_catalog_requested')
        runtimes = json.loads(run('control-runtimes', ['xcrun', 'simctl', 'list', 'runtimes', '--json']).stdout)
        types = json.loads(run('control-device-types', ['xcrun', 'simctl', 'list', 'devicetypes', '--json']).stdout)
        runtime, device = choose_device(runtimes, types)
        supported = next(r for r in runtimes['runtimes'] if r['identifier'] == runtime).get('supportedDeviceTypes', [])
        if not any(row.get('identifier') == device for row in supported):
            raise ReadinessBlocked('selected runtime does not explicitly list the selected device type')
        facts.update(selected_runtime=runtime, selected_device_type=device)
        stages.record('readiness', 'observed', 'supported_pair_selected', runtime=runtime, device_type=device)
        create_submitted = True
        raw = run('control-create', ['xcrun', 'simctl', 'create', name, device, runtime]).stdout.decode().strip()
        parsed = str(uuid.UUID(raw)).upper()
        if raw.upper() != parsed:
            raise ReadinessBlocked('control create did not return one canonical UUID')
        owned = parsed
        facts['owned_udid'] = owned
        stages.record('readiness', 'observed', 'owned_uuid_received', owned_udid=owned)
        run('control-boot', ['xcrun', 'simctl', 'boot', owned])
        run('control-bootstatus', ['xcrun', 'simctl', 'bootstatus', owned, '-b'], timeout=180)
        stages.record('readiness', 'observed', 'bootstatus_completed')
        facts['listapps_attempts'] = 1
        stages.record('readiness', 'observed', 'inventory_submitted')
        result = run('control-listapps', ['xcrun', 'simctl', 'listapps', owned], timeout=20)
        if result.returncode:
            raise ReadinessBlocked('control inventory returned nonzero')
        try:
            bundles = json.loads(result.stdout)
        except (ValueError, UnicodeDecodeError):
            try:
                bundles = plistlib.loads(result.stdout)
            except (ValueError, plistlib.InvalidFileException):
                # This is formatting of the same reply, never another query.
                normalized = run('control-inventory-plist', ['plutil', '-convert', 'json', '-o', '-',
                                  report['control_stdout_path']], timeout=10)
                bundles = json.loads(normalized.stdout)
        if not isinstance(bundles, dict) or any(not isinstance(k, str) or not isinstance(v, dict) for k, v in bundles.items()):
            raise ReadinessBlocked('control inventory is not a bundle dictionary')
        facts.update(bundle_count=len(bundles), inventory_stdout_sha256=hashlib.sha256(result.stdout).hexdigest())
        stages.record('readiness', 'completed', 'inventory_parsed', bundle_count=len(bundles))
        ready = True
    except (ReadinessBlocked, RuntimeError, OSError, ValueError, KeyError, StopIteration) as error:
        facts['reason'] = str(error)
    finally:
        stages.stop_active(timed_out=any(row.get('outcome') == 'uncertain-after-timeout' for row in report.get('commands', [])))
        if owned:
            if not ready and capture_diagnostics is not None:
                try:
                    capture_diagnostics(run, owned, facts, include_host_processes=False)
                except Exception as error:
                    # Optional observations must not replace the failure or skip cleanup.
                    facts['host_diagnostic_error'] = str(error)
            cleanup = cleanup_owned_simulator(run, owned,
                {'owned_udid': owned, 'source_head': report.get('expected_head'), 'session_id': name},
                device_root, prefix='control', command_records=report.get('commands', []))
            facts['cleanup_observations'] = cleanup
            facts['cleanup_absence_verified'] = cleanup['absence']['verified_absent']
            facts['cleanup_status'] = 'verified-absent' if cleanup['accepted'] else 'uncertain'
        elif create_submitted:
            # No UUID means no safe target to guess, delete or retry.
            facts['cleanup_status'] = 'creation-outcome-uncertain'
    facts['status'] = 'passed-cleaned' if ready and facts['cleanup_status'] == 'verified-absent' else 'blocked'
    return facts


def run_control_and_probe(run, choose_device, invoke_probe, report, control_name, device_root, *, expected_head, capture_diagnostics=None):
    report.update(control_prewarms_core_simulator=True, product_probe_invocations=0,
                  experimental_condition='control may warm CoreSimulator/runtime; not a cold-start fix or fresh-VM claim')
    control = run_control(run, choose_device, report, control_name, device_root,
                          capture_diagnostics=capture_diagnostics)
    if control['status'] != 'passed-cleaned':
        report.update(status='control-environment-blocked', reason=control.get('reason', 'control cleanup is not confirmed'))
        return
    report.update(product_probe_invocations=1, status='product-probe-running')
    try:
        code, native = invoke_probe(control['selected_runtime'], control['selected_device_type'])
        report['product_probe_exit_code'] = code
        report['product_result'] = native
        if not isinstance(native, dict):
            raise ReadinessBlocked('product receipt is not an object')
        valid = (code == 0 and native.get('head') == expected_head and native.get('status') == NATIVE_PASS
                 and native.get('cleanup_status') == 'completed'
                 and native.get('synthetic_input_unchanged') is True and native.get('package_bytes_unchanged') is True
                 and native.get('selected_runtime') == control['selected_runtime']
                 and native.get('selected_device_type') == control['selected_device_type'])
        report.update(status=NATIVE_PASS if valid else 'product-probe-not-accepted',
                      product_native_status=native.get('status'), product_native_reason=native.get('reason'))
        report['product_observations'] = {key: native[key] for key in ('session_id', 'stage_observations', 'share_observations')
                                          if key in native}
    except (ReadinessBlocked, OSError, ValueError) as error:
        report.update(status='product-probe-not-accepted', reason=str(error))


def checkout_facts(repo, expected):
    if not re.fullmatch(r'[0-9a-f]{40}', expected):
        raise ReadinessBlocked('expected source must be a complete commit SHA')
    def git(*args):
        return subprocess.check_output(['git', '-C', str(repo), *args], text=True, timeout=10).strip()
    head, tree, status = git('rev-parse', 'HEAD'), git('rev-parse', 'HEAD^{tree}'), git('status', '--porcelain')
    if head != expected or status:
        raise ReadinessBlocked('expected source/clean checkout did not match: ' + str(repo))
    return {'head': head, 'tree': tree, 'status': status}


def source_bindings(repo):
    trees = {}
    for path in ('Cargo.toml', 'Cargo.lock', 'Package.swift', 'crates', 'Sources', 'apps', 'fixtures'):
        trees[path] = subprocess.check_output(['git', '-C', str(repo), 'rev-parse', 'HEAD:' + path],
                                              text=True, timeout=10).strip()
    paths = subprocess.check_output(['git', '-C', str(repo), 'ls-files', 'scripts/apple/pvt_native_import'],
                                    text=True, timeout=10).splitlines()
    return {'product_trees': trees,
            'probe_files_sha256': {path: hashlib.sha256((repo / path).read_bytes()).hexdigest() for path in paths}}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--expected-head', required=True)
    parser.add_argument('--output-root', type=Path, required=True)
    parser.add_argument('--product-output-root', type=Path, required=True)
    parser.add_argument('--execute-owned-simulator', action='store_true')
    args = parser.parse_args()
    args.repo = args.repo.resolve()
    args.output_root = args.output_root.resolve()
    args.product_output_root = args.product_output_root.resolve()
    for output in (args.output_root, args.product_output_root):
        if output.is_relative_to(args.repo):
            parser.error('receipts must be outside the source checkout')
    args.output_root.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix='control-', dir=args.output_root))
    report = {'status': 'not-executed', 'commands': [], 'expected_head': args.expected_head,
              'control_stdout_path': str(work / 'control-listapps.stdout'), 'work': str(work),
              'host': {'node': platform.node(), 'os': platform.system(), 'arch': platform.machine(), 'pid': os.getpid()},
              'job': {key: os.environ.get(key) for key in ('GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT', 'GITHUB_JOB', 'RUNNER_NAME')},
              'media_uploaded': False, 'product_probe_invocations': 0}
    def save_report():
        pending = work / 'result.pending.json'
        pending.write_text(json.dumps(report, indent=2) + '\n')
        pending.replace(work / 'result.json')
    try:
        report['source'] = checkout_facts(args.repo, args.expected_head)
        report['source_bindings'] = source_bindings(args.repo)
        if args.product_output_root.exists() or args.product_output_root.is_symlink():
            raise ReadinessBlocked('product output root already exists; never reuse an old receipt')
        probe_path = args.repo / 'scripts/apple/pvt_native_import/check_native_import.py'
        report['product_probe_sha256'] = hashlib.sha256(probe_path.read_bytes()).hexdigest()
        module_name = __package__ + '.pvt_current_product_probe' if __package__ else 'pvt_current_product_probe'
        spec = importlib.util.spec_from_file_location(module_name, probe_path)
        probe = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(probe)
        if not args.execute_owned_simulator or platform.system() != 'Darwin' or platform.machine() != 'arm64':
            raise ReadinessBlocked('owned execution requires the authorized arm64/macOS host')

        def run(name, argv, *, timeout=60, required=True):
            argv = list(map(str, argv)); record = {'name': name, 'argv': argv, 'timeout_seconds': timeout,
                                                  'started_monotonic_seconds': time.monotonic()}
            report['commands'].append(record)
            save_report()
            try:
                record.update(probe.run_owned_process(argv, args.repo, work / (name + '.stdout'),
                                                     work / (name + '.stderr'), timeout=timeout))
            except OSError as error:
                record.update(outcome='not-started', error=str(error))
                raise ReadinessBlocked(name + ': ' + str(error)) from error
            finally:
                save_report()
            if record.get('outcome') or (required and record['exit_code']):
                raise ReadinessBlocked(name + ': ' + record.get('outcome', 'exit ' + str(record['exit_code'])))
            return subprocess.CompletedProcess(argv, record['exit_code'],
                (work / (name + '.stdout')).read_bytes(), (work / (name + '.stderr')).read_bytes())

        def invoke_probe(runtime, device):
            save_report()
            args.product_output_root.mkdir(parents=True, exist_ok=False)
            command = [sys.executable, str(probe_path), '--repo', str(args.repo), '--cli',
                       str(args.repo / 'target/debug/xdremux'), '--expected-head', args.expected_head,
                       '--output-root', str(args.product_output_root), '--execute-owned-simulator',
                       '--required-runtime', runtime, '--required-device-type', device]
            report['product_command'] = command
            started = time.monotonic()
            with (work / 'product-probe.stdout').open('wb') as stdout, (work / 'product-probe.stderr').open('wb') as stderr:
                result = subprocess.run(command, cwd=args.repo, env={**os.environ, 'CARGO_TARGET_DIR': str(args.repo / 'target')},
                                        stdout=stdout, stderr=stderr)
            report['product_duration_seconds'] = time.monotonic() - started
            receipts = list(args.product_output_root.glob('pvt-native-*/result.json'))
            if len(receipts) != 1 or receipts[0].is_symlink() or receipts[0].stat().st_size > 2000000:
                raise ReadinessBlocked('one fresh bounded product receipt was not produced')
            report['product_receipt'] = str(receipts[0])
            return result.returncode, json.loads(receipts[0].read_text())

        run_control_and_probe(run, probe.select_device, invoke_probe, report,
                              'pvt-readiness-control-' + uuid.uuid4().hex,
                              Path.home() / 'Library/Developer/CoreSimulator/Devices',
                              expected_head=args.expected_head, capture_diagnostics=probe.capture_host_simulator_diagnostics)
    except (ReadinessBlocked, OSError, ValueError, RuntimeError) as error:
        report.update(status='experiment-blocked', reason=str(error))
    except Exception as error:
        report.update(status='experiment-error', reason=repr(error))
    finally:
        save_report()
        print('Same-host readiness receipt:\n' + json.dumps(report, indent=2))
        for path in sorted(work.glob('*.stdout')) + sorted(work.glob('*.stderr')):
            text = path.read_text(errors='replace')
            if len(text) > 12000:
                text = text[:6000] + '\n[console excerpt; full text in artifact]\n' + text[-6000:]
            print('Readiness diagnostic ' + path.name + ':\n' + text)
    return 0 if report['status'] == NATIVE_PASS else 2


if __name__ == '__main__':
    sys.exit(main())
