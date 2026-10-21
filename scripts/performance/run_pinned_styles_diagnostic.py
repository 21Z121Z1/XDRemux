#!/usr/bin/env python3
"""Manual, pinned Styles diagnosis. This never supplies a product budget pass."""
from pathlib import Path, PurePosixPath
import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import signal
import stat
import subprocess
import sys
import tempfile
import time
import types

PRODUCT = 'e1072a90e31b12861dfe8b9cf25e07ce75c17c44'
HARNESS = '1a1698e47e008b688cf5c3fc492dfc307b04ae1a'
BASELINE = '89d806c8aec961a7991eb3e6d6e2b1370aed8791'
REVIEW_REF = 'refs/tags/review/styles-stage-diagnostic-20261021-v1'
MODE = 'styles-stage-attribution'
REPO = '21Z121Z1/XDRemux'
HARNESS_DIR = Path('docs/validation/reviews/20261020-semantic-integration/iterations/e1072a9-clone-object-independence')
REVIEWED_FILES = {
    'bounded_stage_lifecycle.py': '787b499bbd580d26a88a3cf991bfc4c728ba7f362d0120a5a54a834045d8f7b3',
    'e107-stage-attribution-prototype.py': 'c4a3bf872648454d6b7742bcf284a4cbdce1b766f8e3572b64846fedae3a1fa1',
}
PREPARATION_SECONDS = 15 * 60
NATIVE_SECONDS = 90 * 60  # The unmodified reviewed CLI's own budget.
BOOTSTRAP_COMMAND_SECONDS = 20
STOP_SECONDS = 2
MAX_TEXT_BYTES = 4 * 1024 * 1024
MAX_ARTIFACT_BYTES = 32 * 1024 * 1024
TEXT_PATHS = tuple(re.compile(p) for p in (
    r'driver\.json', r'bootstrap/report\.json', r'checkouts/[a-z-]+\.json',
    r'candidate-build\.json', r'prep/report\.json',
    r'(?:bootstrap|prep)/commands/[0-9]{3}\.stderr',
    r'prep/(?:rust-build|swift-build|candidate-features)\.log',
    r'harness/report\.json', r'harness/commands/[0-9]{3}\.stderr',
    r'harness/(?:reference-rust-build|reference-swift-build|reference-cargo-features|instrumented-baseline-build)\.log',
    r'harness/baseline-timers\.patch',
    r'harness/[0-9]{2}-(?:baseline|candidate)/(?:stderr|validate)\.log',
))


def write_json(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    pending = path.with_suffix(path.suffix + '.pending')
    pending.write_text(json.dumps(data, indent=2) + '\n')
    pending.replace(path)


def digest(path):
    with path.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()


def validate_request(event, env, expected):
    repository = event.get('repository', {})
    inputs = event.get('inputs', {})
    if not re.fullmatch('[0-9a-f]{40}', expected):
        raise ValueError('a full reviewed entrypoint SHA is required')
    if (env.get('GITHUB_ACTIONS') != 'true' or env.get('GITHUB_EVENT_NAME') != 'workflow_dispatch'
            or env.get('GITHUB_REPOSITORY') != REPO or repository.get('full_name') != REPO
            or repository.get('private') is not False or repository.get('visibility') != 'public'
            or env.get('GITHUB_REF') != REVIEW_REF or env.get('GITHUB_SHA') != expected
            or inputs.get('mode') != MODE or inputs.get('reviewed_entrypoint_sha') != expected):
        raise ValueError('request is not the reviewed public-repository diagnostic dispatch')


class Bootstrap:
    """Bounded Git/host reads before any checked-out harness Python executes."""
    def __init__(self, raw, deadline):
        self.root = raw / 'bootstrap'
        (self.root / 'commands').mkdir(parents=True)
        self.rows = []
        self.deadline = deadline

    def check_deadline(self):
        if time.monotonic() >= self.deadline:
            raise TimeoutError('preparation deadline exhausted')

    def read(self, command, cwd):
        self.check_deadline()
        n = len(self.rows)
        stdout = self.root / f'commands/{n:03d}.stdout'
        stderr = self.root / f'commands/{n:03d}.stderr'
        row = {'argv': list(command), 'cwd': str(cwd), 'status': 'starting',
               'stdout_path': str(stdout), 'stderr_path': str(stderr)}
        self.rows.append(row)
        child = None
        try:
            write_json(self.root / 'report.json', {'commands': self.rows})
            with stdout.open('wb') as out, stderr.open('wb') as err:
                child = subprocess.Popen(command, cwd=cwd, stdout=out, stderr=err, start_new_session=True)
                row['owned_process_group'] = child.pid
                row['budget_seconds'] = min(BOOTSTRAP_COMMAND_SECONDS, self.deadline - time.monotonic())
                if row['budget_seconds'] <= 0:
                    raise TimeoutError('preparation deadline exhausted')
                row['exit_code'] = child.wait(timeout=row['budget_seconds'])
            row['status'] = 'exited'
            try:
                os.killpg(child.pid, 0)
                row['cleanup_uncertain'] = True
                raise RuntimeError('bootstrap command left an unconfirmed process group')
            except ProcessLookupError:
                row['process_group_absent'] = True
            if row['exit_code']:
                raise RuntimeError('bootstrap command failed; retained stderr and exit code')
            return stdout.read_bytes().decode('utf-8')
        except BaseException as error:
            row.update(status='failed-or-interrupted', error_type=type(error).__name__)
            previous = signal.pthread_sigmask(signal.SIG_BLOCK, {signal.SIGINT, signal.SIGTERM, signal.SIGALRM})
            try:
                if child is not None and child.returncode is None:
                    try:
                        os.killpg(child.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    except OSError:
                        row['cleanup_uncertain'] = True
                    try:
                        row['stopped_leader_exit'] = child.wait(timeout=STOP_SECONDS)
                    except BaseException:
                        row['cleanup_uncertain'] = True
                    try:
                        os.killpg(child.pid, 0)
                        row['cleanup_uncertain'] = True
                    except ProcessLookupError:
                        row['process_group_absent'] = True
                elif child is None:
                    row['start_outcome_uncertain'] = True
                write_json(self.root / 'report.json', {'commands': self.rows})
            finally:
                signal.pthread_sigmask(signal.SIG_SETMASK, previous)
            raise
        finally:
            write_json(self.root / 'report.json', {'commands': self.rows})

    def git(self, root, *args):
        return self.read(['git', '-C', str(root), *args], root).strip()


def verify_checkout(root, expected, boot):
    root = root.resolve(strict=True)
    if any(os.environ.get(k) for k in ('GIT_DIR', 'GIT_WORK_TREE', 'GIT_COMMON_DIR',
                                       'GIT_OBJECT_DIRECTORY', 'GIT_ALTERNATE_OBJECT_DIRECTORIES')):
        raise ValueError('Git object/worktree redirection is not supported')
    if Path(boot.git(root, 'rev-parse', '--show-toplevel')).resolve() != root:
        raise ValueError('checkout root identity differs')
    if boot.git(root, 'rev-parse', 'HEAD') != expected:
        raise ValueError('checkout SHA differs from pin')
    if boot.git(root, 'status', '--porcelain=v1', '--untracked-files=all'):
        raise ValueError('checkout is not clean')
    if boot.git(root, 'rev-parse', '--show-object-format') != 'sha1':
        raise ValueError('unexpected Git object format')
    objects = Path(boot.git(root, 'rev-parse', '--path-format=absolute', '--git-path', 'objects'))
    for name in ('alternates', 'http-alternates'):
        p = objects / 'info' / name
        if p.exists() or p.is_symlink():
            raise ValueError('borrowed object store refused')
    git_dir = boot.git(root, 'rev-parse', '--absolute-git-dir')
    tree = boot.read(['git', '-C', str(root), 'ls-tree', '-rz', '--full-tree', expected], root)
    rows = []
    for record in tree.split('\0'):
        if not record:
            continue
        boot.check_deadline()
        metadata, name = record.split('\t', 1)
        mode, kind, oid = metadata.split()
        relative = PurePosixPath(name)
        if relative.is_absolute() or '..' in relative.parts or kind != 'blob':
            raise ValueError('unsupported tracked tree entry')
        file = root / name
        if mode == '120000':
            if not file.is_symlink():
                raise ValueError('tracked symlink identity differs')
            data = os.fsencode(os.readlink(file))
            actual = hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()
            sha256 = hashlib.sha256(data).hexdigest()
            size = len(data)
        elif mode in ('100644', '100755') and stat.S_ISREG(file.lstat().st_mode):
            if bool(file.stat().st_mode & 0o111) != (mode == '100755'):
                raise ValueError('tracked executable mode differs')
            size = file.stat().st_size
            sha1 = hashlib.sha1(b'blob ' + str(size).encode() + b'\0')
            sha = hashlib.sha256()
            with file.open('rb') as f:
                for block in iter(lambda: f.read(1024 * 1024), b''):
                    boot.check_deadline()
                    sha1.update(block)
                    sha.update(block)
            actual, sha256 = sha1.hexdigest(), sha.hexdigest()
        else:
            raise ValueError('tracked file kind differs')
        if actual != oid:
            raise ValueError('actual tracked bytes differ: ' + name)
        rows.append({'path': name, 'mode': mode, 'git_blob': oid, 'sha256': sha256, 'bytes': size})
    return {'root': str(root), 'head': expected, 'git_directory': git_dir,
            'object_directory': str(objects), 'no_alternates': True, 'actual_files': rows}


def load_reviewed(harness):
    codes = {}
    for name, expected in REVIEWED_FILES.items():
        content = (harness / HARNESS_DIR / name).read_bytes()
        if hashlib.sha256(content).hexdigest() != expected:
            raise ValueError('reviewed harness bytes differ')
        codes[name] = content
    if 'bounded_stage_lifecycle' in sys.modules:
        raise ValueError('unexpected preloaded lifecycle module')
    helper = types.ModuleType('bounded_stage_lifecycle')
    helper.__file__ = str(harness / HARNESS_DIR / 'bounded_stage_lifecycle.py')
    exec(compile(codes['bounded_stage_lifecycle.py'], helper.__file__, 'exec'), helper.__dict__)
    sys.modules[helper.__name__] = helper
    prototype = types.ModuleType('xdremux_pinned_styles_prototype')
    prototype.__file__ = str(harness / HARNESS_DIR / 'e107-stage-attribution-prototype.py')
    exec(compile(codes['e107-stage-attribution-prototype.py'], prototype.__file__, 'exec'), prototype.__dict__)
    return helper, prototype


def fresh_directory(root, name):
    path = root / name
    path.mkdir(exist_ok=False)
    if any(path.iterdir()):
        raise ValueError('fresh build directory is not empty')
    return path


def build_candidate(product, raw, helper, report):
    if any(os.environ.get(k) for k in ('RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_TARGET',
                                      'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS')):
        raise ValueError('custom compiler wrappers/targets/flags refused for the pinned diagnostic')
    owned = Path(tempfile.mkdtemp(prefix='xdremux-styles-fresh-build-', dir=os.environ['RUNNER_TEMP'])).resolve()
    receipt = {'source': PRODUCT, 'owned_build_directory': str(owned), 'retained': True}
    report['candidate_build'] = receipt
    write_json(raw / 'candidate-build.json', receipt)
    target = fresh_directory(owned, 'cargo-target')
    scratch = fresh_directory(owned, 'swift-scratch')
    cargo_home = fresh_directory(owned, 'cargo-home')
    env = {'CARGO_TARGET_DIR': str(target), 'CARGO_HOME': str(cargo_home), 'CARGO_INCREMENTAL': '0',
           'RUSTUP_TOOLCHAIN': 'stable', 'MACOSX_DEPLOYMENT_TARGET': '27.0', 'HOMEBREW_NO_AUTO_UPDATE': '1'}
    os.environ.update(env)
    prep = raw / 'prep'
    prep.mkdir()
    receipt.update(fresh_empty_directories=[str(target), str(scratch), str(cargo_home)], environment=env)
    commands = helper.Commands(prep, receipt)
    def logged(argv, name, timeout):
        with (prep / name).open('w') as stream:
            code = commands.process(argv, product, os.environ.copy(), stream, subprocess.STDOUT, timeout)
        if code:
            raise RuntimeError('candidate build command failed; see ' + name)
    commands.capture(['brew', 'install', 'libheif'], product, timeout=180)
    prefix = commands.capture(['brew', '--prefix', 'libheif'], product).strip()
    os.environ['PKG_CONFIG_PATH'] = prefix + '/lib/pkgconfig'
    receipt['environment']['PKG_CONFIG_PATH'] = os.environ['PKG_CONFIG_PATH']
    commands.capture(['rustup', 'toolchain', 'install', 'stable', '--profile', 'minimal', '--no-self-update'], product, timeout=180)
    receipt['toolchain'] = {name: commands.capture(argv, product).strip() for name, argv in {
        'rustc': ['rustc', '--version'], 'cargo': ['cargo', '--version'], 'swift': ['swift', '--version'],
        'libheif': ['pkg-config', '--modversion', 'libheif']}.items()}
    receipt['tool_paths'] = {name: shutil.which(name) for name in ('git', 'brew', 'rustup', 'cargo', 'rustc', 'swift', 'pkg-config', 'xcrun')}
    logged(['cargo', 'build', '--locked', '--workspace', '--all-targets', '--release'], 'rust-build.log', 600)
    swift = ['swift', 'build', '--package-path', str(product), '--scratch-path', str(scratch), '-c', 'release']
    logged([*swift, '--product', 'xdremux-apple-adapter'], 'swift-build.log', 300)
    bin_root = Path(commands.capture([*swift, '--show-bin-path'], product).strip()).resolve()
    cli = (target / 'release/xdremux').resolve(strict=True)
    adapter = (bin_root / 'xdremux-apple-adapter').resolve(strict=True)
    if not cli.is_relative_to(target) or not adapter.is_relative_to(scratch):
        raise ValueError('product binary escaped its fresh build directories')
    for name, path in [('cli', cli), ('adapter', adapter)]:
        architecture = commands.capture(['xcrun', 'lipo', '-archs', str(path)], product).strip()
        if architecture != 'arm64':
            raise ValueError('fresh product binary is not arm64')
        receipt[name] = {'path': str(path), 'sha256': digest(path), 'bytes': path.stat().st_size,
                         'architecture': architecture}
    logged(['cargo', 'tree', '--locked', '--workspace', '--edges', 'features'], 'candidate-features.log', 120)
    receipt['feature_graph'] = {'path': 'prep/candidate-features.log', 'sha256': digest(prep / 'candidate-features.log')}
    write_json(raw / 'candidate-build.json', receipt)
    return receipt


def allowed_text(relative):
    return any(pattern.fullmatch(relative) for pattern in TEXT_PATHS)


def export_text(raw, artifact, env):
    manifest, total, withheld = [], 0, False
    secrets = [v for k, v in env.items() if len(v) >= 8 and re.search('TOKEN|SECRET|PASSWORD', k, re.I)]
    for source in sorted(raw.rglob('*')):
        relative = source.relative_to(raw).as_posix()
        if not allowed_text(relative):
            continue
        entry = {'source': relative}
        try:
            if source.is_symlink() or not source.resolve().is_relative_to(raw.resolve()):
                raise ValueError('symlink/escape refused')
            if not stat.S_ISREG(source.lstat().st_mode) or source.stat().st_size > MAX_TEXT_BYTES:
                raise ValueError('not a bounded regular text file')
            with source.open('rb') as stream:
                data = stream.read(MAX_TEXT_BYTES + 1)
            if len(data) > MAX_TEXT_BYTES or total + len(data) > MAX_ARTIFACT_BYTES or b'\0' in data:
                raise ValueError('text size/type boundary exceeded')
            text = data.decode('utf-8')
            check = text + (json.dumps(json.loads(text), ensure_ascii=False) if source.suffix == '.json' else '')
            if any(secret in check for secret in secrets) or re.search(r'https?://[^\s/"<>]+@', check):
                raise ValueError('possible credential text withheld; raw retained locally')
            name = relative.replace('/', '__')
            if not name.endswith('.json'):
                name += '.log'
            (artifact / name).write_bytes(data)
            total += len(data)
            entry.update(artifact=name, bytes=len(data), sha256=hashlib.sha256(data).hexdigest())
        except (ValueError, UnicodeError, OSError) as error:
            withheld = True
            entry['withheld'] = str(error)
        manifest.append(entry)
    write_json(artifact / 'export-manifest.json', {'files': manifest, 'bytes': total, 'withheld': withheld,
               'raw_retained': str(raw), 'policy': 'flat explicit text allowlist; no checkout/clone/media/stdout dump'})
    return not withheld


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('control-source', 'product-source', 'harness-source', 'artifact-dir'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--expected-control-sha', required=True)
    args = parser.parse_args()
    artifact = args.artifact_dir.resolve()
    runner_temp = Path(os.environ['RUNNER_TEMP']).resolve(strict=True)
    if not artifact.is_relative_to(runner_temp):
        raise ValueError('artifact directory must be owned runner-temporary data')
    artifact.mkdir(exist_ok=False)
    write_json(artifact / 'status.json', {'status': 'starting', 'native_executed': False})
    if os.environ.get('GITHUB_OUTPUT'):
        with Path(os.environ['GITHUB_OUTPUT']).open('a') as out:
            out.write('artifact_ready=true\n')
    raw = Path(tempfile.mkdtemp(prefix='styles-stage-raw-', dir=runner_temp)).resolve()
    report = {'status': 'starting', 'product': PRODUCT, 'harness': HARNESS, 'diagnostic_only': True,
              'absolute_gate': 'not executed; previous failures remain', 'product_build_started': False, 'harness_started': False,
              'raw_retained': str(raw), 'limits': ['Broad operation spans; release scopes and logging overhead differ.',
              'Do not sum nested scene spans or stage medians; no cold-host or cause claim.']}
    old_handlers, old_argv = {}, sys.argv
    try:
        event = json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
        validate_request(event, os.environ, args.expected_control_sha)
        report['execution_context'] = {k: os.environ.get(k) for k in ('GITHUB_REPOSITORY', 'GITHUB_REF', 'GITHUB_SHA', 'GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT', 'GITHUB_JOB', 'RUNNER_NAME')}
        if platform.system() != 'Darwin' or platform.machine() != 'arm64':
            raise RuntimeError('requires the standard arm64 macOS27 host; no native command started')
        def interrupt(signum, frame):
            if signum == signal.SIGALRM:
                raise TimeoutError('15-minute preparation budget exhausted')
            raise KeyboardInterrupt('diagnostic interrupted')
        old_handlers = {s: signal.getsignal(s) for s in (signal.SIGTERM, signal.SIGALRM)}
        for s in old_handlers:
            signal.signal(s, interrupt)
        started = time.monotonic()
        signal.setitimer(signal.ITIMER_REAL, PREPARATION_SECONDS)
        boot = Bootstrap(raw, started + PREPARATION_SECONDS)
        roots = [p.resolve(strict=True) for p in (args.control_source, args.product_source, args.harness_source)]
        if len(set(roots)) != 3:
            raise ValueError('control/product/harness checkouts must be separate')
        checkouts = {}
        for name, root, sha in zip(('control', 'product', 'harness'), roots,
                                   (args.expected_control_sha, PRODUCT, HARNESS)):
            checkouts[name] = verify_checkout(root, sha, boot)
            write_json(raw / f'checkouts/{name}.json', checkouts[name])
        if len({x['git_directory'] for x in checkouts.values()}) != 3 or len({x['object_directory'] for x in checkouts.values()}) != 3:
            raise ValueError('checkouts share Git/object storage')
        report['host'] = {name: boot.read(command, roots[0]).strip() for name, command in {
            'macos': ['sw_vers', '-productVersion'], 'xcode': ['xcodebuild', '-version'],
            'sdk': ['xcrun', '--sdk', 'macosx', '--show-sdk-version'],
            'architecture': ['uname', '-m'],
            'hardware': ['sysctl', 'hw.model', 'machdep.cpu.brand_string', 'hw.physicalcpu', 'hw.logicalcpu', 'hw.memsize'],
            'load': ['uptime']}.items()}
        if report['host']['macos'].split('.')[0] != '27' or not re.match(r'^Xcode 27(?:\.|\s|$)', report['host']['xcode']) or report['host']['sdk'].split('.')[0] != '27':
            raise ValueError('actual OS/Xcode/SDK is outside macOS27 product scope')
        boot.git(roots[1], 'cat-file', '-e', BASELINE + '^{commit}')
        helper, prototype = load_reviewed(roots[2])
        report['phase'] = 'fresh-product-build'
        report['product_build_started'] = True
        write_json(raw / 'driver.json', report)
        build = build_candidate(roots[1], raw, helper, report)
        write_json(raw / 'checkouts/product-after-build.json', verify_checkout(roots[1], PRODUCT, boot))
        boot.check_deadline()
        signal.setitimer(signal.ITIMER_REAL, 0)
        report.update(phase='reviewed-harness', harness_started=True,
                      preparation_seconds=time.monotonic() - started,
                      execution_condition='source verification and fresh builds precede interleaving; caches may be warm')
        write_json(raw / 'driver.json', report)
        sys.argv = ['reviewed-styles-prototype', '--source', str(roots[1]), '--cli', build['cli']['path'],
                    '--apple-adapter', build['adapter']['path'], '--cli-sha256', build['cli']['sha256'],
                    '--adapter-sha256', build['adapter']['sha256'], '--output-dir', str(raw / 'harness')]
        # In-process CLI invocation preserves its signal/process cleanup. No
        # supervising SIGKILL can strand the harness's separate child sessions.
        code = prototype.main()
        result = json.loads((raw / 'harness/report.json').read_text())
        if result.get('status') == 'interrupted':
            raise KeyboardInterrupt('reviewed harness was interrupted; partial receipt retained')
        if code or result.get('status') != 'completed-with-retained-workspace' or result.get('process_cleanup_uncertain'):
            raise RuntimeError('reviewed harness failed or remained incomplete; partial evidence retained')
        boot.deadline = time.monotonic() + 60
        write_json(raw / 'checkouts/product-after-harness.json', verify_checkout(roots[1], PRODUCT, boot))
        for kind in ('cli', 'adapter'):
            if digest(Path(build[kind]['path'])) != build[kind]['sha256']:
                raise ValueError('fresh binary changed during diagnosis')
        report.update(status='completed-diagnostic-only', harness_status=result['status'])
        return_code = 0
    except BaseException as error:
        report.update(status='interrupted' if isinstance(error, KeyboardInterrupt)
                      else 'timed-out' if isinstance(error, (TimeoutError, subprocess.TimeoutExpired)) else 'failed',
                      error_type=type(error).__name__, error=str(error))
        return_code = 2
    finally:
        sys.argv = old_argv
        if old_handlers:
            signal.setitimer(signal.ITIMER_REAL, 0)
            for s, handler in old_handlers.items():
                signal.signal(s, handler)
        write_json(raw / 'driver.json', report)
        if 'candidate_build' in report:
            write_json(raw / 'candidate-build.json', report['candidate_build'])
        exported = export_text(raw, artifact, os.environ)
        if not exported:
            return_code = 2
        write_json(artifact / 'status.json', {'status': report['status'] if exported else 'failed-text-export', 'exit_code': return_code,
                   'product_build_started': report['product_build_started'],
                   'harness_started': report['harness_started'], 'diagnostic_only': True})
    return return_code


if __name__ == '__main__':
    raise SystemExit(main())
