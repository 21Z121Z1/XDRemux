"""Local lifecycle for the archived stage prototype, not a product runner.

No shared worktree registration, forced removal or recursive deletion. Owned
clones/outputs are retained for review after success, failure and interruption.
"""
from contextlib import contextmanager
from pathlib import Path
import json
import hashlib
import os
import signal
import subprocess
import tempfile
import time
import types

OWNER_PATH = 'scripts/performance/compare_apple_styles_revisions.py'
RUST_PATH = 'crates/xdremux-runtime/src/apple_styles.rs'


class Interrupted(Exception):
    pass


class UncertainProcess(RuntimeError):
    pass


@contextmanager
def shield_cleanup():
    """Defer handled signals only for the independently bounded stop/receipt."""
    mask = {signal.SIGINT, signal.SIGTERM, signal.SIGALRM}
    previous = signal.pthread_sigmask(signal.SIG_BLOCK, mask)
    try:
        yield
    finally:
        signal.pthread_sigmask(signal.SIG_SETMASK, previous)


@contextmanager
def budget(seconds):
    """Overall deadline; every subprocess stop has its own deadline as well."""
    handled = (signal.SIGINT, signal.SIGTERM, signal.SIGALRM)
    old_handlers = {s: signal.getsignal(s) for s in handled}
    old_timer = signal.getitimer(signal.ITIMER_REAL)
    started = time.monotonic()
    def interrupt(signum, frame):
        if signum == signal.SIGALRM:
            raise TimeoutError('overall stage-prototype deadline expired')
        raise Interrupted(f'received signal {signum}')
    for s in handled:
        signal.signal(s, interrupt)
    signal.setitimer(signal.ITIMER_REAL, seconds)
    try:
        yield
    finally:
        with shield_cleanup():
            signal.setitimer(signal.ITIMER_REAL, 0)
            for s, handler in old_handlers.items():
                signal.signal(s, handler)
            if old_timer[0]:
                signal.setitimer(signal.ITIMER_REAL,
                                 max(0.001, old_timer[0] - (time.monotonic() - started)),
                                 old_timer[1])


class Commands:
    def __init__(self, output, report, *, stop_seconds=2, waiter=None, reaper=None):
        self.output = Path(output)
        self.logs = self.output / 'commands'
        self.logs.mkdir(exist_ok=False)
        self.report = report
        self.report['commands'] = []
        self.stop_seconds = stop_seconds
        self.waiter = waiter or (lambda child, timeout: child.wait(timeout=timeout))
        self.reaper = reaper or (lambda child, timeout: child.wait(timeout=timeout))
        self.children = []

    def checkpoint(self):
        pending = self.output / 'report.pending.json'
        pending.write_text(json.dumps(self.report, indent=2) + '\n')
        pending.replace(self.output / 'report.json')

    @staticmethod
    def group_gone(pid):
        try:
            os.killpg(pid, 0)
            return False
        except ProcessLookupError:
            return True
        except PermissionError:
            return False

    def stop(self, child):
        started = time.monotonic()
        result = {'budget_seconds': self.stop_seconds, 'status': 'uncertain'}
        # The unreaped session leader keeps its PID reserved. Do not signal a
        # group after this Popen already reaped its leader (possible PID reuse).
        if child.returncode is None:
            try:
                os.killpg(child.pid, signal.SIGKILL)
                result['signal'] = 'SIGKILL to this command session only'
            except ProcessLookupError:
                result['signal'] = 'group already absent'
            except OSError as error:
                result['signal_error'] = str(error)
        else:
            result['signal'] = 'not sent: leader already reaped'
        try:
            result['leader_exit_code'] = self.reaper(child, self.stop_seconds)
        except BaseException as error:
            result['reap_error'] = type(error).__name__ + ': ' + str(error)
        result['process_group_absent'] = self.group_gone(child.pid)
        if 'leader_exit_code' in result and result['process_group_absent']:
            result['status'] = 'confirmed'
        result['duration_seconds'] = time.monotonic() - started
        return result

    def process(self, command, cwd, env, stdout, stderr, timeout):
        if os.name != 'posix':
            raise RuntimeError('owned process groups require POSIX; no command started')
        row = {'argv': list(command), 'cwd': str(cwd), 'budget_seconds': timeout,
               'status': 'starting', 'stdout_path': str(getattr(stdout, 'name', 'external stream')),
               'stderr_path': str(getattr(stderr, 'name', 'external stream'))}
        self.report['commands'].append(row)
        self.checkpoint()
        started = time.monotonic()
        child = None
        try:
            # No Popen context manager: its implicit unbounded wait is unsafe
            # after a timed-out or interrupted command.
            child = subprocess.Popen(command, cwd=cwd, env=env, stdout=stdout,
                                     stderr=stderr, start_new_session=True)
            self.children.append(child)
            row.update(status='running', owned_process_group=child.pid)
            self.checkpoint()
            code = self.waiter(child, timeout)
            row.update(status='exited', exit_code=code,
                       process_group_absent=self.group_gone(child.pid))
            if not row['process_group_absent']:
                row['cleanup_status'] = 'uncertain'
                raise UncertainProcess('exited leader left an unconfirmed process group')
            row['cleanup_status'] = 'confirmed'
            return code
        except BaseException as error:
            row['status'] = ('interrupted' if isinstance(error, (KeyboardInterrupt, Interrupted))
                             else 'timed-out' if isinstance(error, (TimeoutError, subprocess.TimeoutExpired))
                             else 'failed')
            row['error_type'] = type(error).__name__
            row['error'] = str(error)
            with shield_cleanup():
                if child is not None:
                    row['termination'] = self.stop(child)
                    row['cleanup_status'] = row['termination']['status']
                else:
                    # An interruption inside Popen may happen after fork but
                    # before a handle is returned. Do not invent a PID or
                    # report zero effects from absence of a Python handle.
                    row['cleanup_status'] = 'uncertain'
                    row['start_outcome_uncertain'] = True
                row['duration_seconds'] = time.monotonic() - started
                self.checkpoint()
            raise
        finally:
            # This final receipt has no git/rmtree/communicate/wait operation.
            with shield_cleanup():
                row['duration_seconds'] = time.monotonic() - started
                self.checkpoint()

    def capture(self, command, cwd, *, timeout=30):
        number = len(self.report['commands'])
        out = self.logs / f'{number:03d}.stdout'
        err = self.logs / f'{number:03d}.stderr'
        with out.open('w') as stdout, err.open('w') as stderr:
            code = self.process(command, cwd, os.environ.copy(), stdout, stderr, timeout)
        if code:
            raise subprocess.CalledProcessError(code, command,
                                                output=out.read_bytes().decode('utf-8'), stderr=err.read_bytes().decode('utf-8'))
        return out.read_bytes().decode('utf-8')

    def git(self, root, *arguments):
        return self.capture(['git', '-C', str(root), *arguments], root).strip()


def verify_source(source, commands, expected):
    source = Path(source).resolve(strict=True)
    top = Path(commands.git(source, 'rev-parse', '--show-toplevel')).resolve(strict=True)
    if top != source:
        raise ValueError('source argument is not the actual checkout root')
    head = commands.git(source, 'rev-parse', 'HEAD')
    if head != expected:
        raise ValueError('candidate SHA differs from fixed source')
    if commands.git(source, 'status', '--porcelain=v1', '--untracked-files=all'):
        raise ValueError('candidate working tree is dirty')
    return {'path': str(source), 'head': head,
            'git_directory': commands.git(source, 'rev-parse', '--absolute-git-dir'),
            'tracked_and_untracked_status': ''}


def load_verified_owner(source, commands, expected):
    commands.report['owner_execution_attempted'] = False
    commands.report['owner_execution_completed'] = False
    commands.checkpoint()
    identity = verify_source(source, commands, expected)
    source = Path(identity['path'])
    # Read executable code and candidate Rust byte-for-byte from the fixed
    # commit, not the mutable worktree. Do not strip whitespace from blobs.
    def blob(path):
        return commands.capture(['git', '-C', str(source), 'show', expected + ':' + path],
                                source, timeout=30)
    owner_text = blob(OWNER_PATH)
    candidate_rust = blob(RUST_PATH)
    # A writer between the first checks and blob reads must also stop execution.
    if verify_source(source, commands, expected) != identity:
        raise ValueError('candidate checkout identity changed before owner execution')
    commands.report['source_identity'] = identity
    commands.report['fixed_source_hashes'] = {
        'owner_sha256': hashlib.sha256(owner_text.encode()).hexdigest(),
        'candidate_rust_sha256': hashlib.sha256(candidate_rust.encode()).hexdigest()}
    commands.report['owner_execution_attempted'] = True
    commands.checkpoint()
    module = types.ModuleType('xdremux_verified_comparison_owner')
    module.__file__ = str(Path(identity['path']) / OWNER_PATH)
    exec(compile(owner_text, module.__file__, 'exec'), module.__dict__)
    # All commands used by the retained owner helpers become bounded. Its
    # original reference_build/worktree cleanup is deliberately never invoked.
    module.git = commands.git
    module.run_process = commands.process
    module.ROOT = Path(identity['path'])
    commands.report['owner_execution_completed'] = True
    commands.checkpoint()
    return module, identity, owner_text, candidate_rust


def install_timers(file, original, modified, owned, report, commands):
    """Retain the exact old inode, then exclusively create the patched file."""
    backup_dir = Path(tempfile.mkdtemp(prefix='pre-timers-', dir=owned))
    backup = backup_dir / 'source.rs'
    receipt = {'file': str(file), 'backup': str(backup), 'status': 'planned'}
    report['instrumentation_source_backup'] = receipt
    commands.checkpoint()
    try:
        os.rename(file, backup)
        receipt['status'] = 'original retained'
        commands.checkpoint()
        if backup.is_symlink() or backup.read_bytes() != original:
            raise ValueError('owned source changed; retained current bytes without overwriting')
        # Another writer creating the leaf after rename must cause refusal.
        with file.open('xb') as stream:
            stream.write(modified)
        receipt['status'] = 'patched file created exclusively; original retained'
    except BaseException as error:
        receipt['error'] = type(error).__name__ + ': ' + str(error)
        raise
    finally:
        commands.checkpoint()


def owned_workspace(report, commands):
    root = Path(tempfile.mkdtemp(prefix='xdremux-stage-retained-')).resolve()
    # Keep media/clone/targets out of text-evidence and normal artifact trees.
    out = commands.output.resolve()
    if root == out or root in out.parents or out in root.parents:
        raise ValueError('owned workspace and text evidence must not overlap')
    report['owned_workspace'] = {'path': str(root), 'retained': True,
        'deletion_attempted': False, 'shared_worktree_registry_touched': False,
        'reason': 'retained by design on success, failure or interrupt; manual review required'}
    commands.checkpoint()
    return root


def reference_clone(c, revision, owned, output, report, commands):
    source = owned / 'reference-source'
    cargo_target = owned / 'reference-cargo-target'
    swift_scratch = owned / 'reference-swift-scratch'
    # A fresh independent clone has no shared worktree registry or alternates.
    # Do not overwrite, reset, force-remove or recursively delete any path.
    if source.exists() or source.is_symlink():
        raise ValueError('reference destination already exists')
    commands.capture(['git', 'clone', '--no-checkout', '--no-hardlinks', '--',
                      str(c.ROOT), str(source)], c.ROOT, timeout=120)
    commands.capture(['git', '-C', str(source), 'checkout', '--detach', revision],
                     source, timeout=120)
    if commands.git(source, 'rev-parse', 'HEAD') != revision:
        raise ValueError('reference clone revision mismatch')
    if commands.git(source, 'status', '--porcelain=v1', '--untracked-files=all'):
        raise ValueError('fresh reference clone is dirty')
    report['reference_clone'] = {'path': str(source), 'head': revision, 'retained': True}
    commands.checkpoint()
    env = {**os.environ, 'CARGO_TARGET_DIR': str(cargo_target)}
    c.run_logged(c.CARGO_RELEASE_COMMAND, source, env, output / 'reference-rust-build.log', 1800)
    swift = ['swift', 'build', '--package-path', str(source), '--scratch-path',
             str(swift_scratch), '-c', 'release']
    c.run_logged([*swift, '--product', 'xdremux-apple-adapter'], source, env,
                 output / 'reference-swift-build.log', 1800)
    binary_root = commands.capture([*swift, '--show-bin-path'], source, timeout=30).strip()
    return {'sha': revision, 'tracked_status': commands.git(source, 'status', '--porcelain=v1'),
            'cli': c.binary(cargo_target / 'release/xdremux'),
            'adapter': c.binary(Path(binary_root) / 'xdremux-apple-adapter'),
            'fixture_sha256': c.checked_fixture(source),
            'feature_graph': c.feature_graph(source, env, output, 'reference')}
