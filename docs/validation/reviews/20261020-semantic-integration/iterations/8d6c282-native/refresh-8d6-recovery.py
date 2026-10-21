from pathlib import Path
import json
import subprocess
from datetime import datetime, timezone

root = Path('/workspace/xdremux-audit')
manifest = json.loads((root / 'all-38-backup-manifest.json').read_text())
repository = Path('/workspace/XDRemux')
restore = root / 'restore-test.git'

def git(cwd, *args):
    return subprocess.check_output(['git', '-C', str(cwd), *args], text=True).strip()

heads = {ref: oid for oid, ref in (line.split() for line in git(repository, 'ls-remote', '--heads', 'origin').splitlines())}
expected_heads = {row['branch']: row['head'] for row in manifest['branch_coverage']}
assert heads == expected_heads
tags = {ref: oid for oid, ref in (line.split() for line in git(repository, 'ls-remote', '--refs', 'origin', 'refs/tags/backup/semantic-review-20261020/*').splitlines())}
assert len(tags) == 38 and set(tags) == {r['ref'] for r in manifest['recovery_refs']}
assert git(restore, 'rev-parse', '--is-shallow-repository') == 'false'
for row in manifest['recovery_refs']:
    assert tags[row['ref']] == row['head']
    assert git(restore, 'rev-parse', row['ref'] + '^{commit}') == row['head']
    assert git(restore, 'rev-parse', row['ref'] + '^{tree}') == row['tree']
result = subprocess.run(['git', '-C', str(restore), 'fsck', '--full', '--strict'], capture_output=True, text=True)
(root / '8d6c282-recovery-refresh-fsck.log').write_text(result.stdout + result.stderr)
assert result.returncode == 0
report = {
    'repository': '21Z121Z1/XDRemux', 'source': manifest['source'],
    'captured_at': datetime.now(timezone.utc).isoformat(),
    'remote_heads_match_manifest': heads, 'remote_backup_ref_count': len(tags),
    'all_commit_tree_matches': True, 'non_shallow_independent_restore': True,
    'full_strict_fsck_exit': result.returncode,
    'manifest': 'all-38-backup-manifest.json',
    'limits': 'Fresh read-only equality/integrity check of the already independently fetched/restored remote refs. No merge/delete approval or execution; native checks are separate.'
}
(root / '8d6c282-recovery-refresh.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({'refs': len(tags), 'heads': len(heads), 'fsck': result.returncode, 'source': manifest['source']}))
