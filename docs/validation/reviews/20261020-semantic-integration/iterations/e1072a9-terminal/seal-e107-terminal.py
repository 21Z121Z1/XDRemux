from pathlib import Path
import datetime as dt
import hashlib
import json
import re
import subprocess

A = Path('/workspace/xdremux-audit')
R = Path('/workspace/XDRemux-integration')
RESTORE = A / 'restore-test.git'
H = 'e1072a90e31b12861dfe8b9cf25e07ce75c17c44'
URL = 'https://github.com/21Z121Z1/XDRemux.git'
NOW = dt.datetime.now(dt.timezone.utc).isoformat()
def save(name, value):
    (A / name).write_text(json.dumps(value, indent=2) + '\n')
def git(*args, cwd=R, raw=False):
    r = subprocess.run(['git', *args], cwd=cwd, capture_output=True, check=True)
    return r.stdout if raw else r.stdout.decode().strip()
def clean_log(name):
    return re.sub(r'(?m)^\d{4}-\d\d-\d\dT\S+ ', '', (A / name).read_text())
def object_after(text, marker):
    start = text.index(marker) + len(marker)
    return json.JSONDecoder().raw_decode(text[start:].lstrip())[0]

assert git('rev-parse', 'HEAD') == H
assert not git('status', '--porcelain')
checks = json.loads((A / 'e1072a9-checks-terminal-raw.json').read_text())['check_runs']
assert len(checks) == 20 and all(c['head_sha'] == H and c['status'] == 'completed' for c in checks)
assert sum(c['conclusion'] == 'success' for c in checks) == 18
assert {c['name'] for c in checks if c['conclusion'] == 'failure'} == {'pvt-native-import', 'rust-performance'}
save('e1072a9-ci-terminal.json', dict(source=H, captured_at=NOW, all_terminal=True,
    counts=dict(success=18, failure=2, total=20), checks=[{k:c.get(k) for k in
    ('id', 'name', 'head_sha', 'status', 'conclusion', 'started_at', 'completed_at', 'html_url')} for c in checks]))
pvt = object_after(clean_log('e1072a9-pvt-job119111773529.log'), 'Same-host readiness receipt:\n')
assert pvt['status'] == 'control-environment-blocked' and pvt['product_probe_invocations'] == 0
assert pvt['harness_source']['head'] == H and pvt['control']['cleanup_status'] == 'uncertain'
save('e1072a9-pvt-log-receipt.json', pvt)
perf_text = clean_log('e1072a9-performance-job119111773150.log')
perf = object_after(perf_text, '\n{\n  "distinct_revisions"') if False else None
start = perf_text.index('{\n  "distinct_revisions"')
perf = json.JSONDecoder().raw_decode(perf_text[start:])[0]
assert perf['candidate_profile']['status'] == 'validated'
perf['source'] = H
perf['captured_at'] = NOW
perf['absolute_gate'] = dict(status='failure', median_seconds=14.458, median_limit_seconds=11.732,
    p95_seconds=16.156, p95_limit_seconds=15.406, max_rss_mib=1084.3,
    run_id=39567065898, job_id=119111773150, attempt=1)
perf['samples'] = [json.loads(line)['sample'] for line in perf_text.splitlines() if line.startswith('{"sample":')]
assert len(perf['samples']) == 12
assert abs(perf['summary']['candidate_over_baseline_median'] - 1.1825749932607268) < 1e-12
save('e1072a9-performance-terminal.json', perf)

# Refresh remote facts and independently restore only already-existing durable refs.
def remote(pattern):
    return {ref:sha for sha,ref in (line.split() for line in git('ls-remote', 'origin', pattern).splitlines())}
old = json.loads((A / 'all-39-backup-manifest.json').read_text())
heads = remote('refs/heads/*')
assert heads == {x['branch']:x['head'] for x in old['branch_coverage']}
tags = remote('refs/tags/backup/semantic-review-20261020/*')
assert tags == {x['ref']:x['head'] for x in old['recovery_refs']}
assert git('remote','get-url','--push','origin') == URL
log = []
def restore(*args):
    r = subprocess.run(['git', '--git-dir='+str(RESTORE), *args], capture_output=True, text=True)
    log.append('$ git --git-dir=<independent-restore> '+' '.join(args)+'\n'+r.stdout+r.stderr+'\nexit='+str(r.returncode)+'\n')
    (A/'e1072a9-terminal-restore.log').write_text('\n'.join(log))
    assert r.returncode == 0
    return r.stdout.strip()
restore('fetch', URL, 'refs/tags/backup/semantic-review-20261020/*:refs/tags/backup/semantic-review-20261020/*')
for row in old['recovery_refs']:
    assert restore('rev-parse',row['ref']+'^{commit}') == row['head']
    assert restore('rev-parse',row['ref']+'^{tree}') == row['tree']
assert restore('rev-parse','--is-shallow-repository') == 'false'
restore('fsck','--full','--strict','--no-dangling')
old.update(captured_at=NOW, restore='fresh fetch;39 exact commit/tree matches; non-shallow/full strict fsck0', restore_log='e1072a9-terminal-restore.log')
save('e1072a9-terminal-39-ref-manifest.json',old)

worktrees=[]
for block in git('worktree','list','--porcelain').split('\n\n'):
    facts=dict(line.split(' ',1) for line in block.splitlines() if ' ' in line)
    path=Path(facts['worktree'])
    row=dict(path=str(path),head=facts['HEAD'],branch=facts.get('branch'),status=git('status','--porcelain=v1',cwd=path))
    if row['status']:
        patch=git('diff','--binary',cwd=path,raw=True)
        row['tracked_patch_sha256']=hashlib.sha256(patch).hexdigest()
        row['tracked_files']={name:hashlib.sha256((path/name).read_bytes()).hexdigest() for name in git('diff','--name-only',cwd=path).splitlines()}
    worktrees.append(row)
dirty=[x for x in worktrees if x['status']]
assert len(dirty)==1
previous=json.loads((A/'e1072a9-local-dirty-recovery.json').read_text())
assert dirty[0]['path']==previous['dirty_worktree'] and dirty[0]['head']==previous['dirty_base']
assert dirty[0]['tracked_patch_sha256']==previous['recorded_patch_sha256']
assert dirty[0]['tracked_files']=={previous['file']:previous['file_sha256']}
ref=previous['backup_ref']
assert hashlib.sha256(subprocess.check_output(['git','--git-dir='+str(RESTORE),'show',ref+':'+previous['file']])).hexdigest()==previous['file_sha256']
patch=subprocess.check_output(['git','--git-dir='+str(RESTORE),'diff','--binary',previous['dirty_base'],ref,'--',previous['file']])
assert hashlib.sha256(patch).hexdigest()==previous['recorded_patch_sha256']
previous.update(captured_at=NOW,effect='read-only fresh verification: exact existing bytes and patch reconstructed from durable tag; no patch applied')
save('e1072a9-terminal-local-dirty-recovery.json',previous)
save('e1072a9-terminal-local-inventory.json',dict(source=H,captured_at=NOW,worktrees=worktrees,mutations='none'))

p=json.loads((A/'e1072a9-convergence-plan.json').read_text())
p.update(captured_at=NOW,backup_manifest='e1072a9-terminal-39-ref-manifest.json',local_inventory='e1072a9-terminal-local-inventory.json',local_dirty_recovery='e1072a9-terminal-local-dirty-recovery.json')
p['ordered_steps'][0]['blocking_now']='All20 exact e107checks terminal18success/2failure. PVT control environment blocked, cleanup uncertain, product calls0; absolute Styles median/p95 failed. No merge/delete approval.'
p['native_source']='Exact e107harness terminal. Pinned8d6product/probe unchanged; control stopped before product compilation/media generation, product calls0. No whole-PVT Photos admission or refusal observed.'
p['latest_evidence_note']='Terminal exacte10718success/2failure, not running. PVTcontrol listapps20.157s timeout and uncertain cleanup; product0. Styles14.458/16.156 above11.732/15.406; same-hostABBA12.434200333/14.704374375 +18.2575%. Existing39refs/9heads/WIP freshly restored; no branch mutation.'
p['controlled_preflight'].update(status='control-environment-blocked',cleanup_status='uncertain',product_probe_invocations=0,receipt='e1072a9-pvt-log-receipt.json',next_experiment='Blocked until new healthy isolated environment/service recovery evidence; no identical control rerun.')
p['performance']=dict(status='failed',receipt='e1072a9-performance-terminal.json',next_step='Same-host comparable two-arm stage attribution; current candidate-only profile cannot explain relative cost. Preserve all historical attempts and original limits.')
p['current_ci']='e1072a9-ci-terminal.json'
assert p['operation_enabled'] is False
save('e1072a9-terminal-convergence-plan.json',p)
print(json.dumps(dict(source=H,checks='18success/2failure terminal',refs=len(tags),heads=len(heads),dirty_count=len(dirty),product_probe_invocations=0),indent=2))
