"""Portable fixture and POSIX lifecycle tests; no Apple/product execution."""
from pathlib import Path
import hashlib
import importlib.util
import json
import os
import platform
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import types
import unittest
from unittest import mock

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from bounded_stage_lifecycle import (Commands, Interrupted, budget, load_verified_owner,
                                     install_timers, owned_workspace, reference_clone)
spec = importlib.util.spec_from_file_location('stage_prototype', HERE / 'e107-stage-attribution-prototype.py')
p = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)
BASE = (HERE / 'fixtures/baseline-apple-styles.rs').read_text()
CAND = (HERE / 'fixtures/candidate-apple-styles.rs').read_text()


def fault_worker(mode, output):
    output.mkdir()
    report = dict(status='starting', experiment='portable lifecycle fault injection', native_executed=False)
    reaper = None
    waiter = None
    if mode == 'uncertain':
        def reaper(child, seconds):
            raise subprocess.TimeoutExpired('injected wait-result uncertainty', seconds)
    if mode == 'keyboard':
        def waiter(child, timeout):
            raise KeyboardInterrupt('injected after actual Popen')
    commands = Commands(output, report, stop_seconds=0.15, reaper=reaper, waiter=waiter)
    workspace = owned_workspace(report, commands)
    note = workspace / 'simulated-user-change.txt'
    note.write_text('retain this exact change\n')
    script = 'import time; time.sleep(60)'
    timeout = 0.08
    if mode == 'failure':
        script = 'import sys; print("first error",file=sys.stderr); sys.exit(7)'
    if mode == 'interrupt':
        timeout = 10
    try:
        with budget(0.08 if mode == 'alarm' else 15):
            code = commands.capture([sys.executable, '-c', script], workspace, timeout=timeout)
            report.update(status='completed', stdout=code)
    except (KeyboardInterrupt, Interrupted) as error:
        report.update(status='interrupted', error_type=type(error).__name__)
    except BaseException as error:
        report.update(status='timed-out' if isinstance(error, (TimeoutError, subprocess.TimeoutExpired))
                      else 'failed', error_type=type(error).__name__)
    finally:
        # Test-only drain after the injected wait-result uncertainty. The
        # production receipt remains uncertain, never promoted by this drain.
        if mode == 'uncertain':
            for child in commands.children:
                child.wait(timeout=2)
            report['test_only_child_drain'] = True
        report['retained_note_sha256'] = hashlib.sha256(note.read_bytes()).hexdigest()
        report['process_cleanup_uncertain'] = any(row.get('cleanup_status') == 'uncertain' for row in report['commands'])
        (output / 'retained-note.txt').write_bytes(note.read_bytes())
        commands.checkpoint()
    return 0


class PortableTests(unittest.TestCase):
    def test_fixed_snapshot_insertion_is_reversible_without_a_checkout(self):
        modified, proof = p.insert_timers(BASE, CAND)
        self.assertTrue(proof['removed_insertions_restore_exact_original'])
        self.assertEqual(modified.count('    profile_stage(&mut profile,'), 13)
        self.assertEqual(tuple(stage for stage, _, _ in p.ANCHORS), p.COMMON)

    def test_changed_or_already_instrumented_source_is_rejected(self):
        modified, _ = p.insert_timers(BASE, CAND)
        for bad in (modified, BASE.replace('let scene_raster = downsample_raster', 'let scene_raster = wrong'), BASE + BASE):
            with self.subTest(case=bad[-30:]), self.assertRaises(ValueError):
                p.insert_timers(bad, CAND)

    def test_each_arm_requires_its_complete_stage_receipt(self):
        def receipt(labels):
            return ''.join(f'xdremux_styles_profile stage={s} seconds=0.125\n' for s in labels)
        baseline = receipt(p.COMMON)
        candidate = receipt(p.COMMON[:4] + p.DETAIL + p.COMMON[4:])
        self.assertEqual(len(p.spans(baseline, 'baseline')), 13)
        self.assertEqual(len(p.spans(candidate, 'candidate')), 15)
        for text, arm in [(baseline, 'candidate'), (candidate, 'baseline'), (baseline + baseline, 'baseline'), ('', 'baseline'), (baseline.replace('0.125', 'nan', 1), 'baseline')]:
            with self.subTest(arm=arm), self.assertRaises(ValueError):
                p.spans(text, arm)

    def test_summary_excludes_nested_spans_and_requires_all_readbacks(self):
        rows = []
        for i in range(5):
            for arm in ('baseline', 'candidate'):
                rows.append(dict(arm=arm, warmup=False, status='validated', stages=[dict(stage=s, wall_seconds=1 if arm == 'baseline' else 2) for s in p.COMMON] + [dict(stage=s, wall_seconds=999) for s in p.DETAIL]))
        result = p.summary(rows)
        self.assertEqual(tuple(result), p.COMMON)
        self.assertTrue(all(r['candidate_minus_baseline_seconds'] == 1 for r in result.values()))
        rows[-1]['status'] = 'converted'
        with self.assertRaises(ValueError):
            p.summary(rows)
        with self.assertRaises(ValueError):
            p.summary(rows[:-2])

    def test_cli_has_an_explicit_source_argument_on_every_host(self):
        r = subprocess.run([sys.executable, str(HERE / 'e107-stage-attribution-prototype.py')], capture_output=True, text=True, timeout=5)
        self.assertEqual(r.returncode, 2)
        self.assertIn('--source', r.stderr)

    @unittest.skipIf(platform.system() == 'Darwin', 'actual non-macOS refusal; not a macOS execution test')
    def test_actual_non_macos_entry_refuses_before_native_or_owner_execution(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            out = root / 'receipt'
            r = subprocess.run([sys.executable, str(HERE / 'e107-stage-attribution-prototype.py'), '--source', str(root), '--cli', str(root / 'missing-cli'), '--apple-adapter', str(root / 'missing-adapter'), '--cli-sha256', '0' * 64, '--adapter-sha256', '0' * 64, '--output-dir', str(out)], capture_output=True, text=True, timeout=5)
            self.assertEqual(r.returncode, 2)
            receipt = json.loads((out / 'report.json').read_text())
            self.assertEqual(receipt['status'], 'blocked')
            self.assertIn('zero builds or native conversions', receipt['error'])
            self.assertEqual(receipt['commands'], [])

    @unittest.skipUnless(os.name == 'posix', 'signal-based native prototype requires POSIX')
    def test_keyboard_interrupt_has_an_explicit_main_terminal_receipt(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            args = ['stage', '--source', str(root), '--cli', 'none', '--apple-adapter', 'none', '--cli-sha256', '0' * 64, '--adapter-sha256', '0' * 64, '--output-dir', str(root / 'out')]
            with mock.patch.object(sys, 'argv', args), mock.patch.object(p.platform, 'system', return_value='Darwin'), mock.patch.object(p, 'run_native', side_effect=KeyboardInterrupt), mock.patch('builtins.print'):
                self.assertEqual(p.main(), 2)
            receipt = json.loads((root / 'out/report.json').read_text())
            self.assertEqual(receipt['status'], 'interrupted')
            self.assertEqual(receipt['error_type'], 'KeyboardInterrupt')
            self.assertFalse(receipt['process_cleanup_uncertain'])


@unittest.skipUnless(os.name == 'posix', 'owned process-group/interrupt tests require POSIX')
class LifecycleTests(unittest.TestCase):
    def run_fault(self, mode):
        destination = os.environ.get('XDREMUX_STAGE_TEST_RECEIPTS')
        parent = Path(destination) if destination else Path(tempfile.mkdtemp(prefix='stage-test-receipts-'))
        parent.mkdir(parents=True, exist_ok=True)
        out = parent / (mode + '-' + str(time.time_ns()))
        started = time.monotonic()
        child = subprocess.Popen([sys.executable, str(Path(__file__).resolve()), '--worker', mode, str(out)], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, start_new_session=True)
        try:
            if mode == 'interrupt':
                until = time.monotonic() + 3
                while time.monotonic() < until:
                    path = out / 'report.json'
                    if path.exists():
                        data = json.loads(path.read_text())
                        if data['commands'] and data['commands'][-1]['status'] == 'running':
                            break
                    time.sleep(0.01)
                else:
                    self.fail('fault worker did not reach an owned command')
                os.kill(child.pid, signal.SIGINT)
            _, stderr = child.communicate(timeout=5)
            self.assertEqual(child.returncode, 0, stderr.decode())
        finally:
            if child.poll() is None:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait(timeout=2)
        self.assertLess(time.monotonic() - started, 5)
        receipt = json.loads((out / 'report.json').read_text())
        workspace = Path(receipt['owned_workspace']['path'])
        self.assertEqual((workspace / 'simulated-user-change.txt').read_bytes(), b'retain this exact change\n')
        self.assertFalse(receipt['owned_workspace']['deletion_attempted'])
        self.assertFalse(receipt['native_executed'])
        return receipt

    def test_real_command_timeout_is_bounded_and_retains_workspace(self):
        r = self.run_fault('timeout')
        self.assertEqual(r['status'], 'timed-out')
        self.assertEqual(r['commands'][-1]['termination']['status'], 'confirmed')

    def test_real_nonzero_failure_preserves_stderr_and_retained_workspace(self):
        r = self.run_fault('failure')
        self.assertEqual(r['status'], 'failed')
        self.assertEqual(r['commands'][-1]['exit_code'], 7)
        self.assertIn('first error', Path(r['commands'][-1]['stderr_path']).read_text())

    def test_real_signal_interrupt_has_a_terminal_and_independent_stop_receipt(self):
        r = self.run_fault('interrupt')
        self.assertEqual(r['status'], 'interrupted')
        self.assertEqual(r['commands'][-1]['termination']['status'], 'confirmed')

    def test_expired_overall_alarm_does_not_remove_the_stop_deadline(self):
        r = self.run_fault('alarm')
        self.assertEqual(r['status'], 'timed-out')
        self.assertEqual(r['commands'][-1]['termination']['budget_seconds'], 0.15)
        self.assertEqual(r['commands'][-1]['termination']['status'], 'confirmed')

    def test_injected_reap_uncertainty_is_recorded_without_waiting_forever(self):
        r = self.run_fault('uncertain')
        self.assertEqual(r['commands'][-1]['cleanup_status'], 'uncertain')
        self.assertTrue(r['test_only_child_drain'])
        self.assertTrue(r['process_cleanup_uncertain'])

    def test_keyboard_interrupt_after_popen_has_a_bounded_cleanup_receipt(self):
        r = self.run_fault('keyboard')
        self.assertEqual(r['status'], 'interrupted')
        self.assertEqual(r['error_type'], 'KeyboardInterrupt')
        self.assertEqual(r['commands'][-1]['termination']['status'], 'confirmed')

    def test_timer_install_preserves_concurrent_bytes_and_refuses_a_new_leaf(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            out = root / 'out'
            out.mkdir()
            commands = Commands(out, {})
            old = root / 'source.rs'
            old.write_bytes(b'original')
            rename = os.rename
            def concurrent_writer(src, dst):
                Path(src).write_bytes(b'user changed bytes')
                rename(src, dst)
            report = {}
            with mock.patch('bounded_stage_lifecycle.os.rename', side_effect=concurrent_writer), self.assertRaisesRegex(ValueError, 'retained current bytes'):
                install_timers(old, b'original', b'patched', root, report, commands)
            self.assertEqual(Path(report['instrumentation_source_backup']['backup']).read_bytes(), b'user changed bytes')
            old.write_bytes(b'original')
            def new_leaf(src, dst):
                rename(src, dst)
                Path(src).write_bytes(b'user created leaf')
            report = {}
            with mock.patch('bounded_stage_lifecycle.os.rename', side_effect=new_leaf), self.assertRaises(FileExistsError):
                install_timers(old, b'original', b'patched', root, report, commands)
            self.assertEqual(old.read_bytes(), b'user created leaf')
            self.assertEqual(Path(report['instrumentation_source_backup']['backup']).read_bytes(), b'original')

    def test_interrupted_start_without_a_handle_is_uncertain_and_never_guesses_pid(self):
        with tempfile.TemporaryDirectory() as raw:
            out = Path(raw)
            report = {}
            commands = Commands(out, report)
            with mock.patch('bounded_stage_lifecycle.subprocess.Popen', side_effect=KeyboardInterrupt), mock.patch('bounded_stage_lifecycle.os.killpg') as kill:
                with self.assertRaises(KeyboardInterrupt):
                    commands.capture(['unused-test-command'], out, timeout=0.05)
                kill.assert_not_called()
            row = report['commands'][-1]
            self.assertEqual(row['status'], 'interrupted')
            self.assertEqual(row['cleanup_status'], 'uncertain')
            self.assertTrue(row['start_outcome_uncertain'])


@unittest.skipUnless(os.name == 'posix' and shutil.which('git'), 'isolated Git identity fixtures require POSIX git')
class IdentityTests(unittest.TestCase):
    def fixture(self, root):
        repo = root / 'source'
        repo.mkdir()
        def git(*args):
            return subprocess.check_output(['git', *args], cwd=repo, text=True, timeout=5).strip()
        git('init', '-q')
        script = repo / 'scripts/performance/compare_apple_styles_revisions.py'
        script.parent.mkdir(parents=True)
        marker = root / 'owner-executed'
        script.write_text('from pathlib import Path\nPath(' + repr(str(marker)) + ').write_text("executed")\n')
        rust = repo / 'crates/xdremux-runtime/src/apple_styles.rs'
        rust.parent.mkdir(parents=True)
        rust.write_text('fixed candidate bytes\n\n')
        git('add', '.')
        git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'fixture')
        return repo, git('rev-parse', 'HEAD'), marker, script, rust

    def test_wrong_sha_and_dirty_tree_never_execute_owner(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            repo, head, marker, script, rust = self.fixture(root)
            for label, expected in [('wrong-sha', '0' * 40), ('dirty', head)]:
                if label == 'dirty':
                    rust.write_text('user modification\n')
                out = root / label
                out.mkdir()
                commands = Commands(out, {})
                with self.subTest(label=label), self.assertRaises(ValueError):
                    load_verified_owner(repo, commands, expected)
                self.assertFalse(marker.exists())
                self.assertFalse(commands.report['owner_execution_attempted'])
                self.assertEqual(rust.read_text(), 'user modification\n' if label == 'dirty' else 'fixed candidate bytes\n\n')

    def test_blob_bytes_are_exact_and_owner_load_follows_identity_checks(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            repo, head, marker, script, rust = self.fixture(root)
            out = root / 'out'
            out.mkdir()
            commands = Commands(out, {})
            owner, identity, code, candidate = load_verified_owner(repo, commands, head)
            self.assertTrue(marker.exists())
            self.assertEqual(candidate.encode(), b'fixed candidate bytes\n\n')
            self.assertEqual(identity['head'], head)

    def test_writer_during_blob_read_stops_owner_execution(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            repo, head, marker, script, rust = self.fixture(root)
            out = root / 'out'
            out.mkdir()
            commands = Commands(out, {})
            capture = commands.capture
            def mutate(*args, **kwargs):
                result = capture(*args, **kwargs)
                if any(head + ':crates/' in x for x in args[0]):
                    script.write_text('changed by another writer\n')
                return result
            with mock.patch.object(commands, 'capture', side_effect=mutate), self.assertRaises(ValueError):
                load_verified_owner(repo, commands, head)
            self.assertFalse(marker.exists())
            self.assertEqual(script.read_text(), 'changed by another writer\n')

    def test_wrong_checkout_root_never_executes_owner(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            repo, head, marker, script, rust = self.fixture(root)
            out = root / 'out'
            out.mkdir()
            commands = Commands(out, {})
            with self.assertRaisesRegex(ValueError, 'actual checkout root'):
                load_verified_owner(repo / 'scripts', commands, head)
            self.assertFalse(marker.exists())

    def test_late_worktree_drift_cannot_replace_fixed_executable_or_rust_bytes(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            repo, head, marker, script, rust = self.fixture(root)
            out = root / 'out'
            out.mkdir()
            commands = Commands(out, {})
            import bounded_stage_lifecycle as lifecycle
            verify = lifecycle.verify_source
            calls = []
            changed_marker = root / 'changed-owner-executed'
            def late_writer(*args):
                result = verify(*args)
                calls.append(1)
                if len(calls) == 2:
                    script.write_text('from pathlib import Path\nPath(' + repr(str(changed_marker)) + ').write_text("wrong")\n')
                    rust.write_text('late changed Rust bytes\n')
                return result
            with mock.patch.object(lifecycle, 'verify_source', side_effect=late_writer):
                module, identity, owner_text, candidate = load_verified_owner(repo, commands, head)
            self.assertTrue(marker.exists())
            self.assertFalse(changed_marker.exists())
            self.assertEqual(candidate.encode(), b'fixed candidate bytes\n\n')
            self.assertEqual(rust.read_text(), 'late changed Rust bytes\n')

    def test_failed_build_retains_independent_clone_and_does_not_touch_source_registry(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            repo, head, marker, script, rust = self.fixture(root)
            note = repo / 'user-change.txt'
            note.write_text('do not overwrite\n')
            before = subprocess.check_output(['git', 'worktree', 'list', '--porcelain'], cwd=repo, timeout=5)
            out = root / 'out'
            out.mkdir()
            report = {}
            commands = Commands(out, report)
            workspace = owned_workspace(report, commands)
            def fail_build(*args):
                raise RuntimeError('deliberate test build failure; no Rust/native execution')
            owner = types.SimpleNamespace(ROOT=repo, CARGO_RELEASE_COMMAND=['unused-cargo'], run_logged=fail_build)
            with self.assertRaisesRegex(RuntimeError, 'deliberate test build failure'):
                reference_clone(owner, head, workspace, out, report, commands)
            clone = workspace / 'reference-source'
            self.assertTrue((clone / '.git').is_dir())
            self.assertFalse((clone / '.git/objects/info/alternates').exists())
            self.assertEqual(note.read_text(), 'do not overwrite\n')
            after = subprocess.check_output(['git', 'worktree', 'list', '--porcelain'], cwd=repo, timeout=5)
            self.assertEqual(before, after)
            self.assertFalse(any('worktree' in row['argv'] for row in report['commands']))
            destination = os.environ.get('XDREMUX_STAGE_TEST_RECEIPTS')
            if destination:
                target = Path(destination) / ('clone-failure-' + str(time.time_ns()))
                target.mkdir(parents=True)
                (target / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
                (target / 'retained-user-note.txt').write_bytes(note.read_bytes())

    def test_rejects_real_borrowing_source_before_clone_or_owner(self):
        root = Path(tempfile.mkdtemp(prefix='stage-real-borrowed-source-'))
        lender, head, marker, script, rust = self.fixture(root)
        borrower = root / 'borrower'
        destination = os.environ.get('XDREMUX_STAGE_TEST_RECEIPTS')
        output_parent = Path(destination) if destination else root
        out = output_parent / ('borrowed-source-' + str(time.time_ns()))
        out.mkdir(parents=True)
        report = {'native_executed': False, 'fixture': {'root': str(root), 'lender': str(lender),
                  'borrower': str(borrower), 'head': head, 'retained': True}}
        commands = Commands(out, report)
        commands.capture(['git', 'clone', '--shared', '--', str(lender), str(borrower)], root, timeout=10)
        alternates = borrower / '.git/objects/info/alternates'
        self.assertTrue(alternates.is_file())
        self.assertEqual(commands.git(borrower, 'rev-parse', 'HEAD'), head)
        commands.capture(['git', '-C', str(borrower), 'cat-file', '-e', head], borrower, timeout=5)
        self.assertFalse((borrower / '.git/objects' / head[:2] / head[2:]).exists())
        report['fixture'].update(alternates=alternates.read_text(),
                                 borrowed_commit_available=True, commit_loose_object_local=False)
        owned = owned_workspace(report, commands)
        def forbid_build(*args):
            clone = owned / 'reference-source'
            report['unexpected_build_boundary_reached'] = True
            report['unexpected_clone_inherits_alternates'] = (clone / '.git/objects/info/alternates').is_file()
            commands.checkpoint()
            raise RuntimeError('test sentinel: missing borrowed-object guard reached build boundary')
        owner = types.SimpleNamespace(ROOT=borrower, CARGO_RELEASE_COMMAND=['unused-cargo'], run_logged=forbid_build)
        before = len(report['commands'])
        try:
            with self.assertRaisesRegex(ValueError, 'borrowed'):
                reference_clone(owner, head, owned, out, report, commands)
            self.assertFalse((owned / 'reference-source').exists())
            self.assertFalse(report.get('unexpected_build_boundary_reached', False))
            self.assertFalse(any('clone' in row['argv'] for row in report['commands'][before:]))
            with self.assertRaisesRegex(ValueError, 'borrowed'):
                load_verified_owner(borrower, commands, head)
            self.assertFalse(marker.exists())
        finally:
            report['fixture']['lender_and_borrower_preserved'] = lender.is_dir() and borrower.is_dir() and alternates.is_file()
            commands.checkpoint()


if __name__ == '__main__':
    if len(sys.argv) > 1 and sys.argv[1] == '--worker':
        raise SystemExit(fault_worker(sys.argv[2], Path(sys.argv[3])))
    unittest.main()
