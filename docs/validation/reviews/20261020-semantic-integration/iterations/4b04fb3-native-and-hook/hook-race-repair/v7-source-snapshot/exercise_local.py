#!/usr/bin/env python3
"""Actual local Git transport exercises; every writable repo is under this directory."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent
SOURCE = Path("/workspace/XDRemux/.git")
MAIN = "89d806c8aec961a7991eb3e6d6e2b1370aed8791"
BACKED = "1b2c07da2cb2b0c89c64ca546148fe5d68d75a97"
A, B = "refs/heads/fixture/a", "refs/heads/fixture/b"
BACKUP = "refs/tags/backup/semantic-review-20261020/integration/1b2c07d"
ENV = dict(os.environ, GIT_ALLOW_PROTOCOL="file", GIT_CONFIG_NOSYSTEM="1",
           GIT_CONFIG_GLOBAL="/dev/null", PYTHONDONTWRITEBYTECODE="1",
           GIT_CONFIG_COUNT="1", GIT_CONFIG_KEY_0="pack.threads", GIT_CONFIG_VALUE_0="1")
RUN = Path(tempfile.mkdtemp(prefix="local-fixtures-", dir=ROOT))
COMMANDS = []


def run(args, cwd=None, *, env=None, input=None, success=True):
    result = subprocess.run(list(map(str, args)), cwd=cwd, env=env or ENV,
                            input=input, capture_output=True)
    record = {"command": list(map(str, args)), "cwd": str(cwd or Path.cwd()),
              "exit_code": result.returncode, "stdout": result.stdout.decode(errors="replace"),
              "stderr": result.stderr.decode(errors="replace")}
    COMMANDS.append(record)
    with (RUN / "commands.jsonl").open("a") as log:
        log.write(json.dumps(record) + "\n")
    if success and result.returncode:
        raise RuntimeError(record)
    return result


def git(repo, *args, **kwargs):
    assert Path(repo).resolve().is_relative_to(RUN), "writes are restricted to local fixtures"
    return run(["git", "-C", repo, *args], **kwargs)


def refs(repo):
    text = git(repo, "for-each-ref", "--format=%(refname) %(objectname)").stdout.decode()
    return dict(line.split() for line in text.splitlines())


seed = RUN / "seed.git"
if len(sys.argv) == 3 and sys.argv[1] == "--seed":
    previous_seed = Path(sys.argv[2]).resolve()
    assert previous_seed.is_relative_to(ROOT) and previous_seed.name == "seed.git"
    run(["git", "clone", "--bare", "--shared", previous_seed, seed])
else:
    assert len(sys.argv) == 1, "only --seed <prior local fixture seed> is supported"
    run(["git", "init", "--bare", "--initial-branch=main", seed])
    # Read objects only from the local user repository. No GitHub URL is contacted.
    git(seed, "fetch", "--no-tags", str(SOURCE), MAIN, BACKED)
    for ref, oid in (("refs/heads/main", MAIN), (A, BACKED), (B, BACKED),
                     ("refs/heads/fixture/unapproved", BACKED), ("refs/tags/control", MAIN)):
        git(seed, "update-ref", ref, oid)
allowed_commits = set(git(seed, "rev-list", MAIN, BACKED).stdout.decode().splitlines())
RESULTS = []


def exercise(name):
    case = RUN / name
    case.mkdir()
    remote, client = case / "remote.git", case / "client.git"
    for destination in (remote, client):
        run(["git", "clone", "--bare", "--shared", seed, destination])
    git(client, "remote", "set-url", "origin", str(remote))
    assert str(remote).startswith(str(RUN))
    git(client, "config", "remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*")
    for short in ("a", "b"):
        git(client, "update-ref", "refs/remotes/origin/fixture/" + short, BACKED)
    sources = case / "reviewed-sources"
    sources.mkdir()
    for filename in ("guard.py", "prepare.py", "pre-push.template.py"):
        shutil.copyfile(ROOT / filename, sources / filename)
    original = case / "original-hooks"
    original.mkdir()
    previous = original / "pre-push"
    previous_code = f'''#!{sys.executable}
from pathlib import Path
import json, os, sys
Path({str(case / 'existing.stdin')!r}).write_bytes(sys.stdin.buffer.read())
Path({str(case / 'existing.args.json')!r}).write_text(json.dumps({{"args":sys.argv[1:],"cwd":os.getcwd()}}))
print("EXISTING_HOOK_EXECUTED", file=sys.stderr)
sys.exit({37 if name == 'existing-hook-rejects' else 0})
'''
    previous.write_text(previous_code)
    previous.chmod(0o755)
    # Ensure the temporary hooks directory does not bypass another existing hook.
    other = original / "reference-transaction"
    other.write_text(f'''#!{sys.executable}
from pathlib import Path
import sys
with Path({str(case / 'other-hook.log')!r}).open("a") as out:
 out.write(sys.argv[1] + "\\n")
sys.stdin.buffer.read()
''')
    other.chmod(0o755)
    manifest = {"schema_version": 1, "approval": "approved",
                "remote": {"name": "origin", "url": str(remote)},
                "approved_deletions": {r: {"oid": BACKED, "backup_ref": BACKUP} for r in (A, B)}}
    if name == "wrong-url":
        manifest["remote"]["url"] += ".not-approved"
    manifest_path = case / "approved.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
    hooks = case / "guarded-hooks"
    run([sys.executable, sources / "prepare.py", "--manifest", manifest_path,
         "--output", hooks, "--original-hooks", original])
    hook = hooks / "pre-push"
    if name == "manifest-hash-mismatch":
        manifest_path.write_text(manifest_path.read_text() + " ")
    if name == "guard-hash-mismatch":
        (sources / "guard.py").write_text((sources / "guard.py").read_text() + "\n# changed\n")
    if name == "existing-hook-changed":
        previous.write_text(previous_code + "\n# changed after review\n")
    if name == "change-before-advertisement":
        git(remote, "update-ref", A, MAIN, BACKED)
    if name == "atomic-unavailable":
        git(remote, "config", "receive.advertiseAtomic", "false")
    if name == "multiple-push-urls":
        git(client, "config", "--add", "remote.origin.pushurl", str(remote))
        git(client, "config", "--add", "remote.origin.pushurl", str(remote))
    if name == "change-after-advertisement":
        wrapped = case / "race-hooks"
        wrapped.mkdir()
        for entry in hooks.iterdir():
            if entry.name != "pre-push":
                shutil.copy2(entry, wrapped / entry.name)
        wrapper = wrapped / "pre-push"
        wrapper.write_text(f'''#!{sys.executable}
from pathlib import Path
import subprocess, sys
payload = sys.stdin.buffer.read()
result = subprocess.run([{str(hook)!r}, *sys.argv[1:]], input=payload)
if result.returncode:
 sys.exit(result.returncode)
# Fixture-only writer: the server has already advertised BACKED to this push.
subprocess.run(["git", "--git-dir={remote}", "update-ref", {A!r}, {MAIN!r}, {BACKED!r}], check=True)
Path({str(case / 'writer-after-guard.txt')!r}).write_text("guard approved BACKED, writer changed A to MAIN before client update request\\n")
''')
        wrapper.chmod(0o755)
        hooks = wrapped
    before = refs(remote)
    command = ["git", "-C", client, "-c", f"core.hooksPath={hooks}", "push", "--atomic", "origin", "--delete", A, B]
    if name == "main-rejected":
        command.append("refs/heads/main")
    if name == "tag-rejected":
        command.append("refs/tags/control")
    if name == "unapproved-rejected":
        command.append("refs/heads/fixture/unapproved")
    if name == "partial-batch-rejected":
        command.pop()
    if name == "non-deletion-rejected":
        command = command[:command.index("--delete")] + [":" + A, ":" + B, "refs/heads/main:refs/heads/fixture/new"]
    if name == "wrong-remote-name":
        git(client, "remote", "add", "not-origin", str(remote))
        command[command.index("origin")] = "not-origin"
    env = dict(ENV, GIT_TRACE_PACKET=str(case / "transport-packets.log"))
    result = run(command, env=env, success=False)
    after = refs(remote)
    (case / "push.stdout").write_bytes(result.stdout)
    (case / "push.stderr").write_bytes(result.stderr)
    if name == "matching-two-ref-delete":
        assert result.returncode == 0, result.stderr
        assert after == {r: oid for r, oid in before.items() if r not in (A, B)}, after
        assert (case / "other-hook.log").is_file(), "other existing hook was bypassed"
    elif name == "change-after-advertisement":
        assert result.returncode != 0 and (case / "writer-after-guard.txt").exists()
        assert after == dict(before, **{A: MAIN}), (before, after)
        assert "expected " + BACKED in result.stderr.decode(), result.stderr
        assert "atomic" in result.stderr.decode().lower(), result.stderr
        packets = (case / "transport-packets.log").read_text()
        assert f"{BACKED} {'0' * 40} {A}" in packets, packets
    else:
        assert result.returncode != 0, (name, result.stdout, result.stderr)
        assert after == before, (name, before, after)
        reasons = {
            "change-before-advertisement": "advertised head changed",
            "main-rejected": "main, tags and other namespaces",
            "tag-rejected": "main, tags and other namespaces",
            "non-deletion-rejected": "only deletion updates",
            "unapproved-rejected": "unapproved or repeated remote ref",
            "manifest-hash-mismatch": "manifest SHA-256 differs",
            "guard-hash-mismatch": "guard SHA-256 differs",
            "existing-hook-rejects": "EXISTING_HOOK_EXECUTED",
            "existing-hook-changed": "original hooks changed after preparation",
            "wrong-url": "remote name/location differs",
            "wrong-remote-name": "remote name/location differs",
            "multiple-push-urls": "exactly the approved push URL",
            "partial-batch-rejected": "complete approved batch",
            "atomic-unavailable": "does not support --atomic push",
        }
        assert reasons[name] in result.stderr.decode(), (name, result.stderr)
    assert after["refs/heads/main"] == MAIN and after["refs/tags/control"] == MAIN
    direct_exit = None
    if (case / "existing.stdin").exists():
        old_args = json.loads((case / "existing.args.json").read_text())
        assert old_args["args"] == ["not-origin" if name == "wrong-remote-name" else "origin", str(remote)]
        assert Path(old_args["cwd"]) == client
        if name in ("matching-two-ref-delete", "change-after-advertisement"):
            expected = "".join(f"(delete) {'0' * 40} {ref} {BACKED}\n" for ref in (A, B)).encode()
            assert (case / "existing.stdin").read_bytes() == expected
        if name == "existing-hook-rejects":
            direct = run([hook, "origin", remote], cwd=client, input=(case / "existing.stdin").read_bytes(), success=False)
            direct_exit = direct.returncode
            assert direct_exit == 37, direct
    commits = set(git(remote, "rev-list", "--all").stdout.decode().splitlines())
    assert commits <= allowed_commits, "fixture introduced an unbacked commit"
    record = {"name": name, "passed": True, "git_push_exit": result.returncode,
              "direct_chained_hook_exit": direct_exit, "before": before, "after": after,
              "manifest_sha256": hashlib.sha256(manifest_path.read_bytes()).hexdigest(),
              "launcher_sha256": hashlib.sha256(hook.read_bytes()).hexdigest()}
    (case / "result.json").write_text(json.dumps(record, indent=2) + "\n")
    RESULTS.append(record)
    print(name, "PASS", "git push exit", result.returncode, flush=True)


try:
    for case in ["matching-two-ref-delete", "change-before-advertisement", "change-after-advertisement",
                 "main-rejected", "tag-rejected", "non-deletion-rejected", "unapproved-rejected",
                 "manifest-hash-mismatch", "guard-hash-mismatch", "existing-hook-rejects",
                 "existing-hook-changed", "wrong-url", "wrong-remote-name", "multiple-push-urls",
                 "partial-batch-rejected", "atomic-unavailable"]:
        exercise(case)
finally:
    (RUN / "results.json").write_text(json.dumps({"fixture_root": str(RUN), "git_version": run(["git", "--version"]).stdout.decode().strip(),
        "allowed_tips": [MAIN, BACKED], "allowed_commit_closure_size": len(allowed_commits),
        "completed": RESULTS, "all_sixteen_passed": len(RESULTS) == 16,
        "production_executed": False}, indent=2) + "\n")
    print("Evidence:", RUN, flush=True)
