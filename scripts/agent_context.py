#!/usr/bin/env python3
"""Print a small, reproducible XDRemux context bundle for repository agents."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
MAP_PATH = ROOT / "docs" / "agent-map.json"


def load_map() -> dict[str, Any]:
    return json.loads(MAP_PATH.read_text(encoding="utf-8"))


def git(*args: str, check: bool = True) -> str:
    completed = subprocess.run(
        ["git", *args],
        cwd=ROOT,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if check and completed.returncode != 0:
        message = completed.stderr.strip() or completed.stdout.strip()
        raise RuntimeError(message or f"git {' '.join(args)} failed")
    return completed.stdout.strip()


def current_branch() -> str | None:
    branch = git("symbolic-ref", "--short", "-q", "HEAD", check=False)
    return branch or None


def resolve_ref(name: str) -> str | None:
    # Prefer the fetched remote-tracking ref when present. This still reflects
    # local repository state; callers that require remote freshness must fetch.
    for candidate in (f"refs/remotes/origin/{name}", f"refs/heads/{name}", name):
        if git("rev-parse", "--verify", "--quiet", f"{candidate}^{{commit}}", check=False):
            return candidate
    return None


def git_status(base_override: str | None) -> dict[str, Any]:
    data = load_map()
    branch = current_branch()
    head = git("rev-parse", "HEAD")
    dirty = bool(git("status", "--porcelain=v1", "--untracked-files=normal"))
    role = data["branch_roles"].get(branch or "")
    intended_base = base_override or (role or {}).get("intended_base")

    result: dict[str, Any] = {
        "branch": branch or "(detached)",
        "head": head,
        "dirty": dirty,
        "role": (role or {}).get("role", "unregistered"),
        "intended_base": intended_base,
        "base_ref": None,
        "base_commit": None,
        "merge_base": None,
        "ahead": None,
        "behind": None,
        "freshness": "Git divergence uses locally available refs; fetch before relying on remote freshness.",
    }

    if intended_base:
        base_ref = resolve_ref(intended_base)
        result["base_ref"] = base_ref
        if base_ref:
            result["base_commit"] = git("rev-parse", f"{base_ref}^{{commit}}")
            result["merge_base"] = git("merge-base", base_ref, "HEAD")
            counts = git("rev-list", "--left-right", "--count", f"{base_ref}...HEAD").split()
            if len(counts) == 2:
                result["behind"] = int(counts[0])
                result["ahead"] = int(counts[1])
        else:
            result["base_note"] = (
                f"Base {intended_base!r} is not available locally. Fetch it or pass --base."
            )
    elif branch != "main":
        result["base_note"] = "No intended base is registered for this branch; pass --base explicitly."

    return result


def emit_status(args: argparse.Namespace) -> None:
    status = git_status(args.base)
    if args.json:
        print(json.dumps(status, ensure_ascii=False, indent=2, sort_keys=True))
        return

    print(f"branch: {status['branch']}")
    print(f"HEAD: {status['head']}")
    print(f"worktree: {'dirty' if status['dirty'] else 'clean'}")
    print(f"role: {status['role']}")
    if status["intended_base"]:
        print(f"intended base: {status['intended_base']}")
    if status["base_ref"]:
        print(f"base ref: {status['base_ref']} @ {status['base_commit']}")
    if status["merge_base"]:
        print(f"merge base: {status['merge_base']}")
        print(f"ahead/behind: +{status['ahead']} / -{status['behind']}")
    if status.get("base_note"):
        print(f"note: {status['base_note']}")
    print(f"freshness: {status['freshness']}")


def capability_by_id(identifier: str, data: dict[str, Any]) -> dict[str, Any]:
    for capability in data["capabilities"]:
        if capability["id"] == identifier:
            return capability
    known = ", ".join(item["id"] for item in data["capabilities"])
    raise KeyError(f"unknown capability {identifier!r}; known capabilities: {known}")


def routed_capability(identifier: str) -> dict[str, Any]:
    data = load_map()
    capability = capability_by_id(identifier, data).copy()
    context = data["path_context"]
    capability["rust_owner_branch"] = context["rust_owner_branch"]
    capability["reference_owner_branch"] = (
        context["styles_research_branch"]
        if capability["layer"] == "research"
        else context["released_reference_branch"]
    )
    return capability


def emit_capability(args: argparse.Namespace) -> None:
    try:
        capability = routed_capability(args.identifier)
    except KeyError as error:
        raise SystemExit(str(error)) from error

    if args.json:
        print(json.dumps(capability, ensure_ascii=False, indent=2, sort_keys=True))
        return

    print(f"capability: {capability['id']}")
    layer = capability["layer"]
    if capability.get("secondary_layer") is not None:
        layer = f"{layer} (+ {capability['secondary_layer']})"
    print(f"layer: {layer}")
    print(f"summary: {capability['summary']}")
    rust_owner = ", ".join(capability["rust_owner"]) or "not promoted"
    print(f"Rust owner [{capability['rust_owner_branch']}]: {rust_owner}")
    reference_owner = ", ".join(capability["reference_owner"]) or "none"
    print(f"reference owner [{capability['reference_owner_branch']}]: {reference_owner}")
    print("evidence: " + "; ".join(capability["evidence"]))


def emit_branches(args: argparse.Namespace) -> None:
    branches = load_map()["branch_roles"]
    if args.json:
        print(json.dumps(branches, ensure_ascii=False, indent=2, sort_keys=True))
        return
    for name, metadata in branches.items():
        base = metadata.get("intended_base") or "(root)"
        print(f"{name}: {metadata['role']} [base={base}]")


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(
        description=(
            "Derive local Git state and stable capability routing without scanning the whole repository."
        )
    )
    subparsers = root.add_subparsers(dest="command", required=True)

    status = subparsers.add_parser("status", help="print current branch, HEAD, base, and divergence")
    status.add_argument("--base", help="override the intended base for an unregistered branch")
    status.add_argument("--json", action="store_true", help="emit machine-readable JSON")
    status.set_defaults(func=emit_status)

    capability = subparsers.add_parser("capability", help="route one stable capability")
    capability.add_argument("identifier")
    capability.add_argument("--json", action="store_true", help="emit machine-readable JSON")
    capability.set_defaults(func=emit_capability)

    branches = subparsers.add_parser("branches", help="print registered long-lived branch roles")
    branches.add_argument("--json", action="store_true", help="emit machine-readable JSON")
    branches.set_defaults(func=emit_branches)
    return root


def main() -> int:
    args = parser().parse_args()
    try:
        args.func(args)
    except RuntimeError as error:
        print(f"agent context error: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
