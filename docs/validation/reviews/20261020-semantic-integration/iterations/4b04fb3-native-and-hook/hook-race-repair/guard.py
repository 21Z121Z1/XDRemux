"""Check only the advertised deletion batch; this module never changes refs."""
import json
import os
from pathlib import Path
import re
import stat
import subprocess


def git_output(arguments, repository=None):
    command = ["git"] + (["-C", str(repository)] if repository is not None else [])
    return subprocess.check_output(command + arguments).decode().strip()


def hooks_config(repository=None):
    command = ["git"] + (["-C", str(repository)] if repository is not None else [])
    result = subprocess.run(command + ["config", "--null", "--show-scope", "--show-origin",
                                       "--get-all", "core.hooksPath"], capture_output=True)
    if result.returncode == 1 and not result.stdout:
        return []
    result.check_returncode()
    fields = result.stdout.decode().split("\0")
    if fields.pop() != "" or len(fields) % 3:
        raise ValueError("unexpected scoped core.hooksPath output")
    return [fields[index:index + 3] for index in range(0, len(fields), 3)]


def directory_identity(path):
    info = Path(path).lstat()
    if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.getuid():
        raise ValueError(f"directory must be ordinary and owned by the current user: {path}")
    if info.st_mode & 0o022:
        raise ValueError(f"directory is writable by group or others: {path}")
    return {"device": info.st_dev, "inode": info.st_ino, "uid": info.st_uid,
            "gid": info.st_gid, "mode": stat.S_IMODE(info.st_mode)}


def inspect_original_hooks(directory):
    identity = directory_identity(directory)
    active = []
    samples = []
    with os.scandir(directory) as entries:
        for entry in entries:
            # Git does not invoke *.sample. Reject even non-executable unknown
            # entries and all symlinks rather than emulate a possible hook chain.
            info = entry.stat(follow_symlinks=False)
            if not stat.S_ISREG(info.st_mode) or not entry.name.endswith(".sample"):
                active.append(entry.name)
            else:
                samples.append(entry.name)
    if directory_identity(directory) != identity:
        raise ValueError("original hooks directory identity changed during inspection")
    return {"identity": identity, "active": sorted(active), "samples": sorted(samples)}


def require_no_active_hooks(snapshot):
    if snapshot["active"]:
        raise ValueError("active or unknown original hooks require parent review: "
                         + ", ".join(snapshot["active"]))


def prepare_context(repository):
    if hooks_config(repository):
        raise ValueError("this repair supports only an unset original core.hooksPath")
    git_directory = git_output(["rev-parse", "--absolute-git-dir"], repository)
    original = git_output(["rev-parse", "--path-format=absolute", "--git-path", "hooks"], repository)
    snapshot = inspect_original_hooks(original)
    require_no_active_hooks(snapshot)
    return {"git_directory": git_directory, "git_identity": directory_identity(git_directory),
            "original_hooks_directory": original, "original_identity": snapshot["identity"]}


def validate_environment(context, temporary_hooks, temporary_identity):
    if git_output(["rev-parse", "--absolute-git-dir"]) != context["git_directory"]:
        raise ValueError("push is executing in a different repository")
    if directory_identity(context["git_directory"]) != context["git_identity"]:
        raise ValueError("repository identity or permissions changed after preparation")
    if hooks_config() != [["command", "command line:", temporary_hooks]]:
        raise ValueError("core.hooksPath changed; require only the reviewed command-scoped override")
    effective = git_output(["rev-parse", "--path-format=absolute", "--git-path", "hooks"])
    if effective != temporary_hooks or directory_identity(effective) != temporary_identity:
        raise ValueError("effective temporary hooks path, identity or permissions changed")
    snapshot = inspect_original_hooks(context["original_hooks_directory"])
    # Decide from the observed snapshot, never exists()/exec() a hook afterward.
    require_no_active_hooks(snapshot)
    if snapshot["identity"] != context["original_identity"]:
        raise ValueError("original hooks directory identity or permissions changed")


def validate(manifest_bytes, arguments, payload):
    manifest = json.loads(manifest_bytes)
    if manifest.get("schema_version") != 1 or manifest.get("approval") != "approved":
        raise ValueError("manifest is not an approved schema-1 deletion batch")
    remote = manifest["remote"]
    if arguments != [remote["name"], remote["url"]]:
        raise ValueError("remote name/location differs from approved manifest")
    # Atomicity is per transport. Refuse origin configured with multiple push URLs.
    urls = subprocess.check_output(
        ["git", "remote", "get-url", "--push", "--all", remote["name"]], text=True
    ).splitlines()
    if urls != [remote["url"]]:
        raise ValueError("named remote must have exactly the approved push URL")
    approved = manifest["approved_deletions"]
    if not isinstance(approved, dict) or not approved:
        raise ValueError("approved deletion batch is empty")
    for ref, item in approved.items():
        if not ref.startswith("refs/heads/") or ref == "refs/heads/main":
            raise ValueError("manifest may approve only non-main branches")
        subprocess.run(["git", "check-ref-format", ref], check=True)
        if not re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", item["oid"]):
            raise ValueError("manifest must contain a full nonzero object ID")
        if set(item["oid"]) == {"0"}:
            raise ValueError("approved backup head cannot be zero")
        if not item["backup_ref"].startswith("refs/tags/backup/"):
            raise ValueError("approved head needs its reviewed durable backup ref")
    seen = set()
    lines = payload.decode("ascii").splitlines()
    if not lines:
        raise ValueError("pre-push received no advertised update batch")
    for line in lines:
        fields = line.split()
        if len(fields) != 4:
            raise ValueError("pre-push must supply exactly four fields per update")
        local_ref, local_oid, remote_ref, remote_oid = fields
        if local_ref != "(delete)" or not re.fullmatch(r"0{40}|0{64}", local_oid):
            raise ValueError("only deletion updates with all-zero local OIDs are allowed")
        if not remote_ref.startswith("refs/heads/") or remote_ref == "refs/heads/main":
            raise ValueError("main, tags and other namespaces may not be deleted")
        if remote_ref not in approved or remote_ref in seen:
            raise ValueError("unapproved or repeated remote ref")
        if remote_oid != approved[remote_ref]["oid"] or len(remote_oid) != len(local_oid):
            raise ValueError(f"advertised head changed: {remote_ref}: {remote_oid}")
        seen.add(remote_ref)
    if seen != set(approved):
        raise ValueError("advertised deletion set differs from the complete approved batch")
    return sorted(seen)
