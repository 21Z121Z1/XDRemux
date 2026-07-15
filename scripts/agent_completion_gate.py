#!/usr/bin/env python3
"""Run and verify HEAD-bound completion evidence for XDRemux agents."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
from pathlib import Path
import subprocess
import sys
import time
from typing import Any


SCHEMA_VERSION = 1
CHECK_KINDS = {"static", "regression", "functional", "integration", "device"}
FUNCTIONAL_KINDS = {"functional", "integration", "device"}
PRODUCTION_PREFIXES = (
    "xdremux/",
    "apps/macos/XDRemuxApp/Sources/",
)
SOURCE_SUFFIXES = {".swift", ".py", ".sh", ".c", ".cc", ".cpp", ".h", ".m", ".mm"}
OUTPUT_TAIL_LIMIT = 16_000


class GateConfigurationError(RuntimeError):
    pass


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat()


def run_process(
    command: list[str],
    *,
    cwd: Path,
    timeout_seconds: int | None = None,
    env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        cwd=cwd,
        env=env,
        text=True,
        capture_output=True,
        timeout=timeout_seconds,
        check=False,
    )


def git(repo: Path, *arguments: str, check: bool = True) -> str:
    result = run_process(["git", *arguments], cwd=repo)
    if check and result.returncode != 0:
        detail = (result.stderr or result.stdout).strip()
        raise GateConfigurationError(f"git {' '.join(arguments)} failed: {detail}")
    return result.stdout.strip()


def repository_root() -> Path:
    result = run_process(["git", "rev-parse", "--show-toplevel"], cwd=Path.cwd())
    if result.returncode != 0:
        raise GateConfigurationError("completion gate must run inside a Git repository")
    return Path(result.stdout.strip()).resolve()


def resolve_commit(repo: Path, revision: str) -> str:
    return git(repo, "rev-parse", "--verify", f"{revision}^{{commit}}")


def changed_files(repo: Path, base_commit: str, head_commit: str) -> list[str]:
    merge_base = git(repo, "merge-base", base_commit, head_commit)
    output = git(
        repo,
        "diff",
        "--name-only",
        "--diff-filter=ACMRTUXB",
        f"{merge_base}...{head_commit}",
    )
    return [line for line in output.splitlines() if line]


def tracked_status(repo: Path) -> list[str]:
    output = git(repo, "status", "--porcelain=v1", "--untracked-files=no")
    return [line for line in output.splitlines() if line]


def load_plan(path: Path) -> dict[str, Any]:
    try:
        plan = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise GateConfigurationError(f"cannot read verification plan {path}: {error}") from error
    if not isinstance(plan, dict):
        raise GateConfigurationError("verification plan must be a JSON object")
    if plan.get("schema_version") != SCHEMA_VERSION:
        raise GateConfigurationError(f"verification plan schema_version must be {SCHEMA_VERSION}")
    if not isinstance(plan.get("scope"), str) or not plan["scope"].strip():
        raise GateConfigurationError("verification plan requires a non-empty scope")
    checks = plan.get("checks")
    if not isinstance(checks, list) or not checks:
        raise GateConfigurationError("verification plan requires at least one check")

    names: set[str] = set()
    for index, check in enumerate(checks):
        if not isinstance(check, dict):
            raise GateConfigurationError(f"check {index} must be a JSON object")
        name = check.get("name")
        kind = check.get("kind")
        command = check.get("command")
        timeout_seconds = check.get("timeout_seconds", 300)
        if not isinstance(name, str) or not name.strip():
            raise GateConfigurationError(f"check {index} requires a non-empty name")
        if name in names:
            raise GateConfigurationError(f"duplicate check name: {name}")
        names.add(name)
        if kind not in CHECK_KINDS:
            raise GateConfigurationError(f"check {name} has unsupported kind: {kind}")
        if (
            not isinstance(command, list)
            or not command
            or not all(isinstance(value, str) and value for value in command)
        ):
            raise GateConfigurationError(f"check {name} command must be a non-empty string array")
        if not isinstance(timeout_seconds, int) or not 1 <= timeout_seconds <= 3600:
            raise GateConfigurationError(f"check {name} timeout_seconds must be between 1 and 3600")
        check_env = check.get("env", {})
        if not isinstance(check_env, dict) or not all(
            isinstance(key, str) and isinstance(value, str) for key, value in check_env.items()
        ):
            raise GateConfigurationError(f"check {name} env must be a string-to-string object")
    return plan


def enforce_evidence_policy(plan: dict[str, Any], files: list[str]) -> dict[str, bool]:
    kinds = {check["kind"] for check in plan["checks"]}
    production_changed = any(path.startswith(PRODUCTION_PREFIXES) for path in files)
    source_changed = any(Path(path).suffix in SOURCE_SUFFIXES for path in files)
    if source_changed and "regression" not in kinds:
        raise GateConfigurationError("source changes require at least one regression check")
    if production_changed and not kinds.intersection(FUNCTIONAL_KINDS):
        raise GateConfigurationError(
            "production changes require at least one functional, integration, or device check"
        )
    return {
        "production_changed": production_changed,
        "source_changed": source_changed,
    }


def output_tail(value: str) -> str:
    return value[-OUTPUT_TAIL_LIMIT:]


def execute_check(repo: Path, check: dict[str, Any]) -> dict[str, Any]:
    started_at = utc_now()
    started = time.monotonic()
    environment = os.environ.copy()
    environment["AGENT_COMPLETION_GATE"] = "1"
    environment.update(check.get("env", {}))
    timed_out = False
    try:
        result = run_process(
            check["command"],
            cwd=repo,
            timeout_seconds=check.get("timeout_seconds", 300),
            env=environment,
        )
        return_code = result.returncode
        stdout = result.stdout
        stderr = result.stderr
    except subprocess.TimeoutExpired as error:
        timed_out = True
        return_code = 124
        stdout = error.stdout or ""
        stderr = error.stderr or ""
        if isinstance(stdout, bytes):
            stdout = stdout.decode(errors="replace")
        if isinstance(stderr, bytes):
            stderr = stderr.decode(errors="replace")
    except OSError as error:
        return_code = 127
        stdout = ""
        stderr = str(error)
    return {
        "name": check["name"],
        "kind": check["kind"],
        "command": check["command"],
        "started_at": started_at,
        "finished_at": utc_now(),
        "duration_seconds": round(time.monotonic() - started, 3),
        "timeout_seconds": check.get("timeout_seconds", 300),
        "timed_out": timed_out,
        "return_code": return_code,
        "passed": return_code == 0 and not timed_out,
        "stdout_tail": output_tail(stdout),
        "stderr_tail": output_tail(stderr),
    }


def default_receipt_path(repo: Path, head_commit: str) -> Path:
    return repo / ".codex" / "verification-receipts" / f"{head_commit}.json"


def write_receipt(path: Path, receipt: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(path)


def run_gate(arguments: argparse.Namespace) -> int:
    repo = repository_root()
    plan = load_plan(arguments.plan.resolve())
    base_commit = resolve_commit(repo, arguments.base)
    head_commit = resolve_commit(repo, "HEAD")
    files = changed_files(repo, base_commit, head_commit)
    if not files:
        raise GateConfigurationError("HEAD has no changes relative to the selected base")
    policy = enforce_evidence_policy(plan, files)
    initial_status = tracked_status(repo)
    diff_check = run_process(["git", "diff", "--check", f"{base_commit}...{head_commit}"], cwd=repo)

    results = [execute_check(repo, check) for check in plan["checks"]]
    final_head = resolve_commit(repo, "HEAD")
    final_status = tracked_status(repo)
    builtins = {
        "initial_tracked_tree_clean": not initial_status,
        "diff_check_passed": diff_check.returncode == 0,
        "head_unchanged": final_head == head_commit,
        "final_tracked_tree_clean": not final_status,
    }
    passed = all(builtins.values()) and all(result["passed"] for result in results)
    receipt = {
        "schema_version": SCHEMA_VERSION,
        "generated_at": utc_now(),
        "passed": passed,
        "scope": plan["scope"],
        "repository": str(repo),
        "base_revision": arguments.base,
        "base_commit": base_commit,
        "head_commit": head_commit,
        "changed_files": files,
        "policy": policy,
        "builtins": builtins,
        "initial_tracked_status": initial_status,
        "final_tracked_status": final_status,
        "diff_check_output": output_tail(diff_check.stdout + diff_check.stderr),
        "checks": results,
    }
    receipt_path = arguments.receipt.resolve() if arguments.receipt else default_receipt_path(repo, head_commit)
    write_receipt(receipt_path, receipt)

    for result in results:
        state = "PASS" if result["passed"] else "FAIL"
        print(f"{state} [{result['kind']}] {result['name']} ({result['duration_seconds']:.3f}s)")
    state = "PASS" if passed else "FAIL"
    print(f"{state} completion gate for {head_commit}")
    print(f"receipt: {receipt_path}")
    return 0 if passed else 1


def verify_receipt(arguments: argparse.Namespace) -> int:
    repo = repository_root()
    try:
        receipt = json.loads(arguments.receipt.resolve().read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise GateConfigurationError(f"cannot read receipt {arguments.receipt}: {error}") from error
    current_head = resolve_commit(repo, "HEAD")
    expected_files = changed_files(repo, receipt.get("base_commit", ""), current_head)
    checks = receipt.get("checks", [])
    builtins = receipt.get("builtins", {})
    required_builtins = {
        "initial_tracked_tree_clean",
        "diff_check_passed",
        "head_unchanged",
        "final_tracked_tree_clean",
    }
    valid = (
        receipt.get("schema_version") == SCHEMA_VERSION
        and receipt.get("passed") is True
        and receipt.get("head_commit") == current_head
        and receipt.get("changed_files") == expected_files
        and not tracked_status(repo)
        and isinstance(checks, list)
        and bool(checks)
        and all(check.get("passed") is True for check in checks)
        and isinstance(builtins, dict)
        and set(builtins) == required_builtins
        and all(value is True for value in builtins.values())
    )
    if not valid:
        print("FAIL receipt is stale, incomplete, failed, or does not match the current clean HEAD")
        return 1
    print(f"PASS verified completion receipt for {current_head}")
    return 0


def make_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    run_parser = subparsers.add_parser("run", help="execute a verification plan and write a receipt")
    run_parser.add_argument("--base", required=True, help="base revision for the completed change")
    run_parser.add_argument("--plan", required=True, type=Path, help="verification-plan JSON")
    run_parser.add_argument("--receipt", type=Path, help="override the default HEAD-bound receipt path")
    run_parser.set_defaults(handler=run_gate)

    verify_parser = subparsers.add_parser("verify", help="verify a receipt against the current HEAD")
    verify_parser.add_argument("receipt", type=Path)
    verify_parser.set_defaults(handler=verify_receipt)
    return parser


def main() -> int:
    parser = make_parser()
    arguments = parser.parse_args()
    try:
        return arguments.handler(arguments)
    except GateConfigurationError as error:
        print(f"configuration error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
