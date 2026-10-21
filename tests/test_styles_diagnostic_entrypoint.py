"""Static dispatch and portable boundary evidence. No native job is executed."""
from pathlib import Path
import ast
import copy
import hashlib
import importlib.util
import json
import os
import platform
import re
import subprocess
import sys
import tempfile
import time
import types
import unittest
from unittest import mock
import yaml

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('pinned_styles_driver', ROOT / 'scripts/performance/run_pinned_styles_diagnostic.py')
d = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(d)
WORKFLOW = yaml.load((ROOT / '.github/workflows/performance.yml').read_text(), Loader=yaml.BaseLoader)
OLD = yaml.load((ROOT / 'tests/fixtures/performance-e107.yml').read_text(), Loader=yaml.BaseLoader)


def expression(text, context):
    body = text.removeprefix('${{').removesuffix('}}').strip()
    pattern = r"'[^']*'|github\.[a-z_.]+|inputs\.[a-z_]+|&&|\|\||==|!=|\(|\)|false|true"
    tokens = re.findall(pattern, body)
    if re.sub(r'\s+', '', ''.join(tokens)) != re.sub(r'\s+', '', body):
        raise ValueError('unsupported GitHub expression in static checker')
    parts = []
    for token in tokens:
        if token.startswith(('github.', 'inputs.')):
            parts.append(repr(context.get(token, '')))
        else:
            parts.append({'&&': ' and ', '||': ' or ', 'false': 'False', 'true': 'True'}.get(token, token))
    tree = ast.parse(''.join(parts), mode='eval')
    permitted = (ast.Expression, ast.BoolOp, ast.Compare, ast.Constant, ast.And, ast.Or, ast.Eq, ast.NotEq)
    if any(not isinstance(node, permitted) for node in ast.walk(tree)):
        raise ValueError('expression outside static comparison subset')
    return bool(eval(compile(tree, '<workflow-if>', 'eval'), {'__builtins__': {}}, {}))


def request():
    sha = 'a' * 40
    event = {'repository': {'full_name': d.REPO, 'private': False, 'visibility': 'public'},
             'inputs': {'mode': d.MODE, 'reviewed_entrypoint_sha': sha}}
    env = {'GITHUB_ACTIONS': 'true', 'GITHUB_EVENT_NAME': 'workflow_dispatch',
           'GITHUB_REPOSITORY': d.REPO, 'GITHUB_REF': d.REVIEW_REF, 'GITHUB_SHA': sha}
    return sha, event, env


class WorkflowTests(unittest.TestCase):
    def test_event_mode_matrix_uses_actual_workflow_expressions(self):
        context = {'github.event_name': 'workflow_dispatch', 'inputs.mode': d.MODE,
                   'github.repository': d.REPO, 'github.ref': d.REVIEW_REF,
                   'github.sha': 'a' * 40, 'inputs.reviewed_entrypoint_sha': 'a' * 40,
                   'github.event.repository.visibility': 'public', 'github.event.repository.private': False}
        cases = [
            ('reviewed-diagnostic', {}, False, True),
            ('push-main', {'github.event_name': 'push', 'github.ref': 'refs/heads/main'}, True, False),
            ('ordinary-pr', {'github.event_name': 'pull_request', 'github.ref': 'refs/pull/71/merge'}, True, False),
            ('tag-push-job-guard', {'github.event_name': 'push'}, False, False),
            ('standard-dispatch-main', {'inputs.mode': 'standard', 'github.ref': 'refs/heads/main'}, True, False),
            ('standard-on-review-tag', {'inputs.mode': 'standard'}, False, False),
            ('unknown-mode', {'inputs.mode': 'unknown'}, False, False),
            ('missing-mode', {'inputs.mode': ''}, False, False),
            ('missing-reviewed-sha', {'inputs.reviewed_entrypoint_sha': ''}, False, False),
            ('different-reviewed-sha', {'inputs.reviewed_entrypoint_sha': 'b' * 40}, False, False),
            ('wrong-ref', {'github.ref': 'refs/heads/main'}, False, False),
            ('other-repository', {'github.repository': 'somebody/else'}, False, False),
            ('private-event', {'github.event.repository.private': True, 'github.event.repository.visibility': 'private'}, False, False),
            ('missing-public-proof', {'github.event.repository.visibility': ''}, False, False),
        ]
        rows = []
        for name, patch, legacy, diagnostic in cases:
            ctx = {**context, **patch}
            observed = [expression(WORKFLOW['jobs'][key]['if'], ctx) for key in ('rust-performance', 'styles-stage-diagnostic')]
            with self.subTest(case=name):
                self.assertEqual(observed, [legacy, diagnostic])
                self.assertFalse(all(observed))
            rows.append({'case': name, 'context': ctx, 'legacy_job': observed[0], 'diagnostic_job': observed[1]})
        output = os.environ.get('XDREMUX_DISPATCH_STATIC_RECEIPTS')
        if output:
            out = Path(output);out.mkdir(parents=True, exist_ok=True)
            (out / 'event-mode-matrix.json').write_text(json.dumps({'scope': 'Static expression subset, not GitHub execution; path filters assumed matched', 'cases': rows}, indent=2) + '\n')

    def test_normal_job_steps_budget_and_other_triggers_are_unchanged(self):
        old = copy.deepcopy(OLD['jobs']['rust-performance'])
        actual = copy.deepcopy(WORKFLOW['jobs']['rust-performance'])
        actual.pop('if')
        self.assertEqual(actual.pop('concurrency'), OLD['concurrency'])
        self.assertEqual(actual, old)
        for event in ('push', 'pull_request'):
            self.assertEqual(WORKFLOW['on'][event], OLD['on'][event])
        self.assertNotIn('concurrency', WORKFLOW)

    def test_diagnostic_has_only_fixed_checkouts_read_permissions_and_independent_concurrency(self):
        job = WORKFLOW['jobs']['styles-stage-diagnostic']
        self.assertEqual(job['runs-on'], 'xcode-27')
        self.assertEqual(job['permissions'], {'contents': 'read'})
        self.assertEqual(WORKFLOW['permissions'], {'contents': 'read'})
        self.assertEqual(job['concurrency']['cancel-in-progress'], 'false')
        self.assertIn('github.run_id', job['concurrency']['group'])
        self.assertNotEqual(job['concurrency']['group'], WORKFLOW['jobs']['rust-performance']['concurrency']['group'])
        checkouts = [step['with'] for step in job['steps'] if step.get('uses') == 'actions/checkout@v7']
        self.assertEqual([x['ref'] for x in checkouts], ['${{ github.sha }}', d.PRODUCT, d.HARNESS])
        self.assertEqual(len({x['path'] for x in checkouts}), 3)
        self.assertTrue(all(x['persist-credentials'] == 'false' and x['fetch-depth'] == '0' for x in checkouts))
        self.assertNotIn('needs', job)
        self.assertNotIn('research.yml', json.dumps(job))
        self.assertFalse(any('cache' in step.get('uses', '') for step in job['steps']))

    def test_deadlines_and_flat_text_upload_boundary(self):
        job = WORKFLOW['jobs']['styles-stage-diagnostic']
        run = next(s for s in job['steps'] if s.get('id') == 'diagnostic')
        self.assertEqual(job['timeout-minutes'], '120')
        self.assertEqual(run['timeout-minutes'], '110')
        self.assertLess(d.PREPARATION_SECONDS + d.NATIVE_SECONDS + 60 + d.STOP_SECONDS, 110 * 60)
        uploads = [s for s in job['steps'] if s.get('uses') == 'actions/upload-artifact@v7']
        self.assertEqual(len(uploads), 2)
        for step in uploads:
            self.assertEqual(step['with']['retention-days'], '1')
            self.assertEqual(step['timeout-minutes'], '5')
            self.assertIn('always()', step['if'])
            self.assertNotIn('**', step['with']['path'])
            self.assertNotIn('product-source', step['with']['path'])
            self.assertNotIn('reviewed-harness', step['with']['path'])
        self.assertEqual(set(WORKFLOW['on']['workflow_dispatch']['inputs']), {'mode', 'reviewed_entrypoint_sha'})

    def test_initial_checkout_receipt_is_static_python_and_contains_both_pins(self):
        job = WORKFLOW['jobs']['styles-stage-diagnostic']
        step = next(s for s in job['steps'] if s.get('id') == 'initial')
        python = step['run'].split("python3 - <<'PYINIT'\n", 1)[1].rsplit('\nPYINIT', 1)[0]
        ast.parse(python)
        self.assertIn(d.PRODUCT, python)
        self.assertIn(d.HARNESS, python)
        self.assertIn('out.mkdir(exist_ok=False)', python)

    def test_all_repository_push_workflows_exclude_tags(self):
        rows = []
        for path in sorted(p for p in (ROOT / '.github/workflows').iterdir() if p.suffix in ('.yml', '.yaml')):
            doc = yaml.load(path.read_text(), Loader=yaml.BaseLoader)
            push = doc.get('on', {}).get('push')
            self.assertIsInstance(push, dict)
            self.assertEqual(push.get('branches'), ['main'])
            self.assertNotIn('tags', push)
            self.assertNotIn('create', doc['on'])
            self.assertNotIn('workflow_run', doc['on'])
            rows.append({'file': path.name, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'push': push})
        output = os.environ.get('XDREMUX_DISPATCH_STATIC_RECEIPTS')
        if output:
            out = Path(output);out.mkdir(parents=True, exist_ok=True)
            (out / 'push-trigger-inventory.json').write_text(json.dumps({'ref': d.REVIEW_REF, 'eligible_repository_push_workflows': [], 'workflows': rows}, indent=2) + '\n')


class BoundaryTests(unittest.TestCase):
    def test_request_rejects_wrong_event_private_repo_ref_or_sha(self):
        sha, event, env = request()
        d.validate_request(event, env, sha)
        for patch in ({'GITHUB_EVENT_NAME': 'push'}, {'GITHUB_REF': 'refs/heads/main'}, {'GITHUB_SHA': 'b' * 40}, {'GITHUB_REPOSITORY': 'other/repo'}):
            with self.subTest(patch=patch), self.assertRaises(ValueError):
                d.validate_request(event, {**env, **patch}, sha)
        for patch in ({'private': True}, {'visibility': 'private'}):
            bad = copy.deepcopy(event);bad['repository'].update(patch)
            with self.assertRaises(ValueError):d.validate_request(bad, env, sha)
        with self.assertRaises(ValueError):d.validate_request(event, env, 'short')

    def test_export_allowlist_excludes_media_clone_and_stdout_dumps(self):
        for good in ('driver.json', 'checkouts/product-after-build.json', 'prep/rust-build.log', 'harness/03-candidate/stderr.log', 'harness/report.json'):
            self.assertTrue(d.allowed_text(good), good)
        for bad in ('driverXjson', 'owned/reference-source/data.json', 'harness/03-candidate/stdout.log', 'photo.heic', 'movie.mov', 'harness/03-candidate/styles.heic', 'prep/other.log'):
            self.assertFalse(d.allowed_text(bad), bad)

    def test_export_withholds_secrets_binary_and_symlink_while_retaining_partial_text(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw);source = root / 'raw';artifact = root / 'artifact'
            source.mkdir();artifact.mkdir()
            (source / 'driver.json').write_text('{"status":"failed"}')
            (source / 'harness').mkdir();(source / 'harness/report.json').write_text('{"error":"secret-value-123456"}')
            (source / 'harness/01-candidate').mkdir();(source / 'harness/01-candidate/stderr.log').write_bytes(b'\0binary')
            (source / 'photo.heic').write_bytes(b'private-media')
            (source / 'prep').mkdir();outside = root / 'outside-secret';outside.write_text('outside data')
            (source / 'prep/rust-build.log').symlink_to(outside)
            self.assertFalse(d.export_text(source, artifact, {'TEST_SECRET': 'secret-value-123456'}))
            self.assertTrue((artifact / 'driver.json').exists())
            self.assertFalse(any(b'secret-value-123456' in x.read_bytes() or b'private-media' in x.read_bytes() for x in artifact.iterdir()))
            self.assertEqual(outside.read_text(), 'outside data')
            self.assertTrue(json.loads((artifact / 'export-manifest.json').read_text())['withheld'])

    def test_actual_tracked_bytes_detect_assume_unchanged_modification(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw);repo = root / 'repo';repo.mkdir();out = root / 'raw';out.mkdir()
            def git(*args):return subprocess.check_output(['git', *args], cwd=repo, text=True, timeout=5).strip()
            git('init', '-q');(repo / 'data.txt').write_text('fixed\n');git('add', '.')
            git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'fixture')
            sha = git('rev-parse', 'HEAD');boot = d.Bootstrap(out, time.monotonic() + 10)
            receipt = d.verify_checkout(repo, sha, boot)
            self.assertEqual(receipt['actual_files'][0]['sha256'], hashlib.sha256(b'fixed\n').hexdigest())
            git('update-index', '--assume-unchanged', 'data.txt');(repo / 'data.txt').write_text('different\n')
            self.assertEqual(git('status', '--porcelain'), '')
            with self.assertRaisesRegex(ValueError, 'actual tracked bytes'):
                d.verify_checkout(repo, sha, boot)

    def test_fresh_directories_refuse_reuse_and_reviewed_bytes_refuse_mutation(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw);path = d.fresh_directory(root, 'target')
            (path / 'old-cli').write_bytes(b'old')
            with self.assertRaises(FileExistsError):d.fresh_directory(root, 'target')
            harness = root / 'harness';(harness / d.HARNESS_DIR).mkdir(parents=True)
            for name in d.REVIEWED_FILES:(harness / d.HARNESS_DIR / name).write_text('raise RuntimeError("must not execute")')
            with self.assertRaisesRegex(ValueError, 'reviewed harness bytes'):d.load_reviewed(harness)
            self.assertNotIn('bounded_stage_lifecycle', sys.modules)

    def test_modeled_build_receipt_uses_fresh_targets_and_hashes_actual_new_bytes(self):
        """Compiler calls/architecture are modeled; no Cargo/Swift is invoked."""
        with tempfile.TemporaryDirectory() as raw, mock.patch.dict(os.environ, {}, clear=True):
            root = Path(raw);out = root / 'raw';out.mkdir();product = root / 'source';product.mkdir()
            os.environ['RUNNER_TEMP'] = str(root)
            class ModelCommands:
                def __init__(self, output, report):self.report = report;report['commands'] = []
                def process(self, argv, cwd, env, stdout, stderr, timeout):
                    self.report['commands'].append({'argv': argv, 'timeout': timeout})
                    if argv[:2] == ['cargo', 'build']:
                        file = Path(env['CARGO_TARGET_DIR']) / 'release/xdremux';file.parent.mkdir();file.write_bytes(b'modeled fresh CLI')
                    elif argv[:2] == ['swift', 'build']:
                        folder = Path(argv[argv.index('--scratch-path') + 1]) / 'bin';folder.mkdir();(folder / 'xdremux-apple-adapter').write_bytes(b'modeled fresh adapter')
                    stdout.write('modeled build/feature output\n')
                    return 0
                def capture(self, argv, cwd, timeout=30):
                    self.report['commands'].append({'argv': argv, 'timeout': timeout})
                    if argv[:2] == ['brew', '--prefix']:return str(root / 'brew')
                    if '--show-bin-path' in argv:return str(Path(argv[argv.index('--scratch-path') + 1]) / 'bin')
                    if '-archs' in argv:return 'arm64'
                    return 'modeled tool receipt'
            receipt = d.build_candidate(product, out, types.SimpleNamespace(Commands=ModelCommands), {})
            for name, data in [('cli', b'modeled fresh CLI'), ('adapter', b'modeled fresh adapter')]:
                self.assertEqual(receipt[name]['sha256'], hashlib.sha256(data).hexdigest())
                self.assertTrue(Path(receipt[name]['path']).is_relative_to(Path(receipt['owned_build_directory'])))
            self.assertEqual(len(set(receipt['fresh_empty_directories'])), 3)
            self.assertIn('CARGO_HOME', receipt['environment'])
            self.assertTrue((out / 'candidate-build.json').is_file())

    def test_bootstrap_timeout_model_uses_an_independent_bounded_stop(self):
        with tempfile.TemporaryDirectory() as raw:
            out = Path(raw);boot = d.Bootstrap(out, time.monotonic() + 10)
            waits = []
            class Child:
                pid = 999999
                returncode = None
                def wait(self, timeout):
                    waits.append(timeout)
                    if len(waits) == 1:raise subprocess.TimeoutExpired('modeled bootstrap', timeout)
                    self.returncode = -9
                    return -9
            def group(pid, sig):
                if sig == 0:raise ProcessLookupError
            with mock.patch.object(d.subprocess, 'Popen', return_value=Child()), mock.patch.object(d.os, 'killpg', side_effect=group), mock.patch.object(d.signal, 'pthread_sigmask', return_value=set()):
                with self.assertRaises(subprocess.TimeoutExpired):boot.read(['modeled-git-read'], out)
            self.assertLessEqual(waits[0], d.BOOTSTRAP_COMMAND_SECONDS)
            self.assertEqual(waits[1], d.STOP_SECONDS)
            self.assertEqual(boot.rows[0]['stopped_leader_exit'], -9)

    @unittest.skipIf(platform.system() == 'Darwin', 'actual Linux/non-macOS refusal only; not native evidence')
    def test_real_driver_entry_refuses_host_before_checkout_build_or_harness(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw);sha, event, env = request();event_file = root / 'event.json';event_file.write_text(json.dumps(event))
            output_file = root / 'output';artifact = root / 'artifact'
            command = [sys.executable, str(ROOT / 'scripts/performance/run_pinned_styles_diagnostic.py'), '--control-source', str(root / 'control'), '--product-source', str(root / 'product'), '--harness-source', str(root / 'harness'), '--expected-control-sha', sha, '--artifact-dir', str(artifact)]
            result = subprocess.run(command, env={**os.environ, **env, 'RUNNER_TEMP': str(root), 'GITHUB_EVENT_PATH': str(event_file), 'GITHUB_OUTPUT': str(output_file)}, capture_output=True, text=True, timeout=5)
            self.assertEqual(result.returncode, 2, result.stderr)
            status = json.loads((artifact / 'status.json').read_text())
            self.assertFalse(status['product_build_started']);self.assertFalse(status['harness_started'])
            self.assertEqual(output_file.read_text(), 'artifact_ready=true\n')


if __name__ == '__main__':
    unittest.main()
