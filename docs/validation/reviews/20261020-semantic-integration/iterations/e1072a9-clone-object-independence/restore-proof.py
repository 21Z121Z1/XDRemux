"""Read-only remote-tag recovery into a new retained independent bare repo."""
from pathlib import Path
import argparse
import json
import os
import shlex
from bounded_stage_lifecycle import Commands, require_local_objects

URL = 'https://github.com/21Z121Z1/XDRemux.git'
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--ref', required=True)
p.add_argument('--commit', required=True)
p.add_argument('--tree', required=True)
p.add_argument('--restore-dir', type=Path, required=True)
p.add_argument('--output-dir', type=Path, required=True)
a = p.parse_args()
if any(os.environ.get(name) for name in ('GIT_ALTERNATE_OBJECT_DIRECTORIES', 'GIT_OBJECT_DIRECTORY',
                                       'GIT_COMMON_DIR', 'GIT_DIR', 'GIT_WORK_TREE')):
    raise SystemExit('Git object/worktree redirection refused before restoration')
if not a.ref.startswith('refs/tags/review/semantic-integration-20261020-evidence-'):
    raise SystemExit('only this user repository review tags are supported')
if a.restore_dir.exists() or a.restore_dir.is_symlink():
    raise SystemExit('restore destination exists; do not overwrite or remove it')
out = a.output_dir.resolve()
out.mkdir(parents=True, exist_ok=False)
report = {'repository': URL, 'ref': a.ref, 'expected_commit': a.commit,
          'expected_tree': a.tree, 'restore_directory': str(a.restore_dir.resolve()),
          'status': 'starting', 'retained': True}
c = Commands(out, report)
try:
    remote = c.capture(['git', 'ls-remote', URL, a.ref], out, timeout=30).strip()
    if remote != a.commit + '\t' + a.ref:
        raise ValueError('advertised ref differs from exact expected commit')
    c.capture(['git', 'init', '--bare', str(a.restore_dir)], out, timeout=30)
    prefix = ['git', '--git-dir=' + str(a.restore_dir)]
    c.capture([*prefix, 'fetch', '--no-tags', URL, a.ref + ':' + a.ref], out, timeout=120)
    commit = c.capture([*prefix, 'rev-parse', a.ref + '^{commit}'], out, timeout=30).strip()
    tree = c.capture([*prefix, 'rev-parse', a.ref + '^{tree}'], out, timeout=30).strip()
    shallow = c.capture([*prefix, 'rev-parse', '--is-shallow-repository'], out, timeout=30).strip()
    if (commit, tree, shallow) != (a.commit, a.tree, 'false'):
        raise ValueError('restored identity or completeness differs')
    require_local_objects(a.restore_dir, c, 'fresh-independent-bare-restore')
    c.capture([*prefix, 'fsck', '--full', '--strict', '--no-dangling'], out, timeout=120)
    report.update(status='passed', observed_commit=commit, observed_tree=tree,
                  is_shallow=False, fsck_exit_code=0)
except BaseException as error:
    report.update(status='failed', error_type=type(error).__name__, error=str(error))
    raise
finally:
    c.checkpoint()
    chunks = []
    for command in report['commands']:
        chunks.extend(['$ ' + shlex.join(command['argv']), 'cwd=' + command['cwd'],
                       'budget_seconds=' + str(command['budget_seconds'])])
        for kind in ('stdout', 'stderr'):
            chunks.append(kind + ':')
            file = Path(command.get(kind + '_path', ''))
            chunks.append(file.read_text() if file.is_file() else '<no captured stream>')
        chunks.append('exit=' + str(command.get('exit_code', 'not returned')) + '\n')
    (out / 'commands.log').write_text('\n'.join(chunks))
print(json.dumps(report, indent=2))
