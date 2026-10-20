"""Check only the advertised deletion batch; this module never changes refs."""
import json
import re
import subprocess


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
