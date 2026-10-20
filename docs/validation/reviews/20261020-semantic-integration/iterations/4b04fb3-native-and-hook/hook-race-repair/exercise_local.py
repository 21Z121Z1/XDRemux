#!/usr/bin/env python3
"""Focused real local-bare transport regression; no production ref writes."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent
V7 = ROOT.parent
SEED = V7 / "local-fixtures-wum6z4xv/seed.git"
MAIN = "89d806c8aec961a7991eb3e6d6e2b1370aed8791"
BACKED = "1b2c07da2cb2b0c89c64ca546148fe5d68d75a97"
A, B = "refs/heads/fixture/a", "refs/heads/fixture/b"
BACKUP = "refs/tags/backup/semantic-review-20261020/integration/1b2c07d"
ENV = dict(os.environ, GIT_ALLOW_PROTOCOL="file", GIT_CONFIG_NOSYSTEM="1",
           GIT_CONFIG_GLOBAL="/dev/null", PYTHONDONTWRITEBYTECODE="1",
           GIT_CONFIG_COUNT="1", GIT_CONFIG_KEY_0="pack.threads", GIT_CONFIG_VALUE_0="1")
RUN = Path(tempfile.mkdtemp(prefix="local-fixtures-", dir=ROOT))
RESULTS = []


def run(arguments, *, cwd=None, success=True, env=None):
    result = subprocess.run(list(map(str, arguments)), cwd=cwd, env=env or ENV, capture_output=True)
    record = {"command": list(map(str, arguments)), "cwd": str(cwd or Path.cwd()),
              "exit_code": result.returncode, "stdout": result.stdout.decode(errors="replace"),
              "stderr": result.stderr.decode(errors="replace")}
    with (RUN / "commands.jsonl").open("a") as log:
        log.write(json.dumps(record) + "\n")
    if success and result.returncode:
        raise RuntimeError(record)
    return result


def git(repository, *arguments, **kwargs):
    assert Path(repository).resolve().is_relative_to(RUN), "local fixture writes only"
    return run(["git", "-C", repository, *arguments], **kwargs)


def refs(repository):
    output = git(repository, "for-each-ref", "--format=%(refname) %(objectname)").stdout.decode()
    return dict(line.split() for line in output.splitlines())


def hook_code(marker, exit_code):
    return (f"#!{sys.executable}\nfrom pathlib import Path\nimport sys\n"
            f"Path({str(marker)!r}).write_text('EXECUTED\\n')\n"
            f"sys.stdin.buffer.read()\nsys.exit({exit_code})\n")


def instrument(hook, case, target, trigger_line, old_hook, action):
    """Fixture-only tracing: interleave a writer at the exact observed race window."""
    saved = case / "reviewed-launcher.py"
    shutil.copyfile(hook, saved)
    target = saved if target is None else target
    replacement = hook_code(case / "replacement-executed.txt", 0)
    hook.write_text(f'''#!{sys.executable}
import os, runpy, sys
from pathlib import Path
target = {str(target)!r}
def trace(frame, event, arg):
 if event == "line" and frame.f_code.co_filename == target and frame.f_lineno == {trigger_line}:
  sys.settrace(None)
  previous = Path({str(old_hook)!r})
  if {action!r} == "delete":
   previous.unlink()
  else:
   replacement = previous.with_name("replacement.tmp")
   replacement.write_text({replacement!r})
   replacement.chmod(0o755)
   os.replace(replacement, previous)
  Path({str(case / 'race-interleaving.txt')!r}).write_text(
    "Observed original hook snapshot; fixture writer {action} before subsequent decision\\n")
 return trace
sys.settrace(trace)
runpy.run_path({str(saved)!r}, run_name="__main__")
''')
    hook.chmod(0o755)


def line_number(path, fragment):
    matches = [number for number, line in enumerate(path.read_text().splitlines(), 1)
               if fragment in line]
    assert len(matches) == 1, (path, fragment, matches)
    return matches[0]


def exercise(name):
    case = RUN / name
    case.mkdir()
    client, remote = case / "client.git", case / "remote.git"
    for destination in (client, remote):
        run(["git", "clone", "--bare", "--shared", SEED, destination])
    git(client, "remote", "set-url", "origin", remote)
    before = refs(remote)
    assert before[A] == before[B] == BACKED and before["refs/heads/main"] == MAIN
    manifest_path = case / "fixture-approved.json"
    manifest_path.write_text(json.dumps({"schema_version": 1, "approval": "approved",
        "remote": {"name": "origin", "url": str(remote)},
        "approved_deletions": {ref: {"oid": BACKED, "backup_ref": BACKUP} for ref in (A, B)}}, indent=2) + "\n")
    hooks = case / "guarded-hooks"
    original = client / "hooks"
    old_hook = original / "pre-push"
    old_source = hook_code(case / "original-executed.txt", 37)
    old_version = name.startswith("v7-red-")
    if old_version or name == "prepare-active-hook-rejected":
        old_hook.write_text(old_source)
        old_hook.chmod(0o755)
    source = V7 if old_version else ROOT
    command = [sys.executable, source / "prepare.py", "--manifest", manifest_path, "--output", hooks]
    command += ["--original-hooks", original] if old_version else ["--repository", client]
    preparation = run(command, success=name != "prepare-active-hook-rejected")
    if name == "prepare-active-hook-rejected":
        assert preparation.returncode != 0 and not hooks.exists()
        assert b"active or unknown original hooks require parent review" in preparation.stderr
        after = refs(remote)
        assert after == before
        assert not (case / "original-executed.txt").exists()
        record = {"name": name, "passed": True, "prepare_exit": preparation.returncode,
                  "git_push_executed": False, "before": before, "after": after}
        (case / "result.json").write_text(json.dumps(record, indent=2) + "\n")
        RESULTS.append(record)
        print(name, "PASS (no push)", flush=True)
        return
    hook = hooks / "pre-push"
    original_mode = hook.stat().st_mode & 0o777
    if not old_version:
        assert original_mode == 0o644, "renderer must leave the hook disabled"
    if name.startswith("green-"):
        old_hook.write_text(old_source)
        old_hook.chmod(0o755)
    if old_version:
        trigger = line_number(hook, "    original = Path(")
        instrument(hook, case, None, trigger, old_hook, name.rsplit("-", 1)[1])
    elif name.startswith("green-"):
        trigger = line_number(ROOT / "guard.py", "    # Decide from the observed snapshot") + 1
        instrument(hook, case, ROOT / "guard.py", trigger, old_hook, name.rsplit("-", 1)[1])
    elif name == "config-change-rejected":
        git(client, "config", "core.hooksPath", original)
    elif name == "permissions-change-rejected":
        original.chmod(0o500)
    elif name == "directory-replacement-rejected":
        original.rename(case / "original-hooks-before-replacement")
        original.mkdir(mode=0o755)
    # Explicit activation occurs only inside these disposable local fixtures.
    hook.chmod(0o755)
    result = git(client, "-c", f"core.hooksPath={hooks}", "push", "--atomic", "origin", "--delete", A, B,
                 success=False, env=dict(ENV, GIT_TRACE_PACKET=str(case / "transport-packets.log")))
    (case / "push.stdout").write_bytes(result.stdout)
    (case / "push.stderr").write_bytes(result.stderr)
    after = refs(remote)
    markers = {key: (case / filename).exists() for key, filename in
               (("original_executed", "original-executed.txt"),
                ("replacement_executed", "replacement-executed.txt"),
                ("interleaving_executed", "race-interleaving.txt"))}
    if old_version or name == "matching-empty-hooks":
        assert result.returncode == 0, result.stderr
        assert after == {ref: oid for ref, oid in before.items() if ref not in (A, B)}
        if old_version:
            assert markers["interleaving_executed"] and not markers["original_executed"]
            assert markers["replacement_executed"] == name.endswith("replace")
    else:
        assert result.returncode != 0 and after == before, (name, result.stderr, before, after)
        assert not markers["original_executed"] and not markers["replacement_executed"]
        if name.startswith("green-"):
            assert markers["interleaving_executed"]
            assert b"active or unknown original hooks require parent review: pre-push" in result.stderr
        elif name == "config-change-rejected":
            assert b"core.hooksPath changed" in result.stderr
        else:
            assert b"original hooks directory identity or permissions changed" in result.stderr
    assert after["refs/heads/main"] == MAIN and after["refs/tags/control"] == MAIN
    # Restore only the two intentionally deleted fixture refs using known OIDs.
    if after != before:
        for ref in (A, B):
            git(remote, "update-ref", ref, BACKED, "0" * 40)
    if name == "permissions-change-rejected":
        original.chmod(0o755)
    if name == "config-change-rejected":
        git(client, "config", "--unset", "core.hooksPath")
    if name == "directory-replacement-rejected":
        original.rmdir()
        (case / "original-hooks-before-replacement").rename(original)
    restored = refs(remote)
    assert restored == before, "fixture restoration differs from original refs"
    (case / "restore.json").write_text(json.dumps({"before": before, "after_push": after,
        "restored": restored, "restored_exactly": True, "new_commit_created": False}, indent=2) + "\n")
    record = {"name": name, "passed": True, "demonstrates_v7_bug": old_version,
              "git_push_exit": result.returncode, "before": before, "after": after,
              "restored_exactly": True, "rendered_initial_mode": oct(original_mode), **markers,
              "manifest_sha256": hashlib.sha256(manifest_path.read_bytes()).hexdigest(),
              "effective_test_hook_sha256": hashlib.sha256(hook.read_bytes()).hexdigest()}
    (case / "result.json").write_text(json.dumps(record, indent=2) + "\n")
    RESULTS.append(record)
    print(name, "PASS", "(v7 BUG reproduced)" if old_version else "", "push exit", result.returncode, flush=True)


try:
    for name in ("v7-red-delete", "v7-red-replace", "matching-empty-hooks",
                 "prepare-active-hook-rejected", "green-delete", "green-replace",
                 "config-change-rejected", "permissions-change-rejected", "directory-replacement-rejected"):
        exercise(name)
    output = run(["git", "--git-dir", SEED, "cat-file", "--batch-all-objects",
                  "--batch-check=%(objectname) %(objecttype)"]).stdout.decode()
    commit_objects = {line.split()[0] for line in output.splitlines() if line.endswith(" commit")}
    allowed = set(run(["git", "--git-dir", SEED, "rev-list", MAIN, BACKED]).stdout.decode().splitlines())
    assert commit_objects == allowed and len(allowed) == 1314
    (RUN / "fixture-object-scope.json").write_text(json.dumps({"allowed_tips": [MAIN, BACKED],
        "commit_object_count": len(commit_objects), "matches_backed_ancestor_closure": True,
        "new_commit_count": 0, "read_only_seed": str(SEED)}, indent=2) + "\n")
finally:
    (RUN / "results.json").write_text(json.dumps({"fixture_root": str(RUN),
        "git_version": run(["git", "--version"]).stdout.decode().strip(),
        "tested_source_sha256": {name: hashlib.sha256((ROOT / name).read_bytes()).hexdigest()
                                 for name in ("guard.py", "prepare.py", "pre-push.template.py", "exercise_local.py")},
        "completed": RESULTS, "all_nine_passed": len(RESULTS) == 9,
        "production_executed": False, "github_contacted": False}, indent=2) + "\n")
    print("Evidence:", RUN, flush=True)
