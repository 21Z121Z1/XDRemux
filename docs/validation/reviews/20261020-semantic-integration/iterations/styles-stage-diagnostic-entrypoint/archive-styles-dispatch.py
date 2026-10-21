from pathlib import Path
import datetime, hashlib, json, os, subprocess, sys
import yaml

ROOT = Path('/workspace/XDRemux')
AUDIT = Path('/workspace/xdremux-audit')
CONTROL = Path('/workspace/XDRemux-style-diagnostic-entrypoint')
EVIDENCE = Path('/workspace/XDRemux-review-evidence')
HEAD = '33bbe860ad6b1777839753a738ec17d3f9229b61'
TREE = 'bfd8a388308c0e3b6e3e59a6065abb54247e37cc'
PRODUCT = 'e1072a90e31b12861dfe8b9cf25e07ce75c17c44'
MAIN = '89d806c8aec961a7991eb3e6d6e2b1370aed8791'
REF = 'refs/tags/review/styles-stage-diagnostic-20261021-v1'
URL = 'https://github.com/21Z121Z1/XDRemux.git'
HELPER = EVIDENCE / 'docs/validation/reviews/20261020-semantic-integration/iterations/e1072a9-clone-object-independence/bounded_stage_lifecycle.py'
assert hashlib.sha256(HELPER.read_bytes()).hexdigest() == '787b499bbd580d26a88a3cf991bfc4c728ba7f362d0120a5a54a834045d8f7b3'
sys.path.insert(0, str(HELPER.parent))
from bounded_stage_lifecycle import Commands, require_local_objects

phase = sys.argv[1]
assert phase in ('preflight', 'publish')
out = AUDIT / ('styles-dispatch-' + phase + '-raw')
out.mkdir(exist_ok=False)
report = {'phase': phase, 'entrypoint': HEAD, 'tree': TREE, 'ref': REF,
          'product': PRODUCT, 'native_experiments': 0, 'operation_enabled': False,
          'captured_at': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'status': 'started'}
c = Commands(out, report)
try:
    assert c.git(ROOT, 'remote', 'get-url', '--push', 'origin') == URL
    assert c.git(CONTROL, 'rev-parse', 'HEAD') == HEAD
    assert c.git(CONTROL, 'rev-parse', 'HEAD^{tree}') == TREE
    assert c.git(CONTROL, 'rev-parse', 'HEAD^') == PRODUCT
    assert not c.git(CONTROL, 'status', '--porcelain=v1', '--untracked-files=all')
    inventory = c.capture(['git', 'ls-remote', URL, 'refs/heads/*', REF], ROOT)
    heads = {line.split('\t')[1]:line.split('\t')[0] for line in inventory.splitlines()}
    assert heads['refs/heads/main'] == MAIN
    assert heads['refs/heads/integrate/semantic-review-20261020'] == PRODUCT
    assert REF not in heads, 'Existing ref: inspect outcome, never overwrite or force.'
    report['heads_before'] = heads
    if phase == 'preflight':
        inventories = []
        for role, sha in [('default-main', MAIN), ('entrypoint', HEAD), ('evidence-v27', '35562b9b5ed2c3c7901c7740edaf0317d0f76014')]:
            files = c.git(ROOT, 'ls-tree', '-r', '--name-only', sha, '.github/workflows').splitlines()
            workflows = []
            for name in files:
                if not name.endswith(('.yml', '.yaml')): continue
                source = c.capture(['git', 'show', sha + ':' + name], ROOT)
                doc = yaml.load(source, Loader=yaml.BaseLoader)
                events = doc['on']
                assert isinstance(events, dict)
                assert events['push']['branches'] == ['main']
                assert 'tags' not in events['push'] and 'tags-ignore' not in events['push']
                assert 'create' not in events and 'workflow_run' not in events
                if role == 'default-main' and name.endswith('/performance.yml'):
                    assert 'workflow_dispatch' in events
                workflows.append({'path': name, 'sha256': hashlib.sha256(source.encode()).hexdigest(), 'events': list(events), 'push': events['push']})
            assert len(workflows) == 6
            inventories.append({'role': role, 'sha': sha, 'eligible_tag_push_workflows': [], 'workflows': workflows})
        report['trigger_inventory'] = inventories
        report['tag_push_exclusion_basis'] = 'Exact committed workflows have only main branch push filters, no tag filters/create/workflow_run; GitHub undefined ref-type rule excludes tag pushes. Default CodeQL only default/protected branches.'
        report['sources'] = [
            'https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#onpushbranchestagsbranches-ignoretags-ignore',
            'https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#workflow_dispatch',
            'https://docs.github.com/en/code-security/concepts/code-scanning/setup-types']
        worktrees = c.git(ROOT, 'worktree', 'list', '--porcelain')
        report['local_worktrees'] = worktrees
        dirty = AUDIT / 'style-fit-accumulation-prototype/candidate-source'
        file = dirty / 'crates/xdremux-engine/src/apple_photographic_styles.rs'
        assert hashlib.sha256(file.read_bytes()).hexdigest() == 'b1f58170b7bc6504eb7e1957c481bb8cd6d32e91ccfbbbca5ee0213cb615cc81'
        patch = c.capture(['git', '-C', str(dirty), 'diff', '--binary'], ROOT)
        assert hashlib.sha256(patch.encode()).hexdigest() == '31be7f073e7ac635492d010910121789faa33e8c69f3b01b91e226b4207aa358'
        report['prior_dirty_sha256'] = hashlib.sha256(file.read_bytes()).hexdigest()
        report['prior_dirty_patch_sha256'] = hashlib.sha256(patch.encode()).hexdigest()
        for name in (ROOT, Path('/workspace/XDRemux-integration'), EVIDENCE):
            assert not c.git(name, 'status', '--porcelain=v1', '--untracked-files=all')
        report['status'] = 'passed-no-publication-yet'
    else:
        preflight = json.loads((AUDIT / 'styles-dispatch-preflight-raw/report.json').read_text())
        assert preflight['entrypoint'] == HEAD and preflight['status'] == 'passed-no-publication-yet'
        assert heads == preflight['heads_before'], 'Concurrent remote writer: stop for coordination.'
        report['publication_scope'] = 'One additive immutable review tag; no branch update, PR, dispatch or rerun.'
        c.capture(['git', 'push', 'origin', HEAD + ':' + REF], ROOT, timeout=60)
        observed = c.capture(['git', 'ls-remote', URL, REF], ROOT).strip()
        assert observed == HEAD + '\t' + REF
        restore = AUDIT / 'restore-test.git'
        require_local_objects(restore, c, 'independent-entrypoint-recovery')
        c.capture(['git', '--git-dir=' + str(restore), 'fetch', URL, REF + ':' + REF], ROOT, timeout=60)
        observed_head = c.capture(['git', '--git-dir=' + str(restore), 'rev-parse', REF + '^{commit}'], ROOT).strip()
        observed_tree = c.capture(['git', '--git-dir=' + str(restore), 'rev-parse', REF + '^{tree}'], ROOT).strip()
        assert observed_head == HEAD and observed_tree == TREE
        assert c.capture(['git', '--git-dir=' + str(restore), 'rev-parse', '--is-shallow-repository'], ROOT).strip() == 'false'
        c.capture(['git', '--git-dir=' + str(restore), 'fsck', '--full', '--strict', '--no-dangling'], ROOT, timeout=120)
        report.update(observed_commit=observed_head, observed_tree=observed_tree, fsck_exit_code=0, status='published-and-restored')
except BaseException as error:
    report.update(status='failed-or-uncertain', error_type=type(error).__name__, error=str(error))
    raise
finally:
    c.checkpoint()
print(json.dumps({k:v for k,v in report.items() if k in ('status', 'entrypoint', 'tree', 'ref', 'observed_commit', 'observed_tree', 'fsck_exit_code')}, indent=2))
