#!/opt/codex/runtimes/codex-primary-runtime/dependencies/python/bin/python3
"""Temporary guard for a reviewed repository with no active original hooks."""
import hashlib
from pathlib import Path
import subprocess
import sys

MANIFEST = '/workspace/xdremux-audit/cleanup-guard/hook-race-repair/local-fixtures-74goh9xg/config-change-rejected/fixture-approved.json'
MANIFEST_SHA256 = '32902f8c1285e5455aeaa06f22ec9f8083e5c573bf867e200ef65b11c9671693'
GUARD = '/workspace/xdremux-audit/cleanup-guard/hook-race-repair/guard.py'
GUARD_SHA256 = '29e4d565f92c6e6e4041949309e7a8b79ad4864b4e2af8feebb2c770aff156c2'
CONTEXT = {'git_directory': '/workspace/xdremux-audit/cleanup-guard/hook-race-repair/local-fixtures-74goh9xg/config-change-rejected/client.git',
 'git_identity': {'device': 27,
                  'gid': 1000,
                  'inode': 413454,
                  'mode': 448,
                  'uid': 1000},
 'original_hooks_directory': '/workspace/xdremux-audit/cleanup-guard/hook-race-repair/local-fixtures-74goh9xg/config-change-rejected/client.git/hooks',
 'original_identity': {'device': 27,
                       'gid': 1000,
                       'inode': 413456,
                       'mode': 448,
                       'uid': 1000}}
TEMPORARY_HOOKS = '/workspace/xdremux-audit/cleanup-guard/hook-race-repair/local-fixtures-74goh9xg/config-change-rejected/guarded-hooks'
TEMPORARY_IDENTITY = {'device': 27, 'gid': 1000, 'inode': 413513, 'mode': 448, 'uid': 1000}


try:
    payload = sys.stdin.buffer.read()
    guard_bytes = Path(GUARD).read_bytes()
    manifest_bytes = Path(MANIFEST).read_bytes()
    if hashlib.sha256(guard_bytes).hexdigest() != GUARD_SHA256:
        raise ValueError("guard SHA-256 differs from reviewed launcher")
    if hashlib.sha256(manifest_bytes).hexdigest() != MANIFEST_SHA256:
        raise ValueError("manifest SHA-256 differs from reviewed launcher")
    namespace = {"__name__": "pinned_cleanup_guard", "__file__": GUARD}
    # Compile the bytes just hashed. No original hook is forwarded or executed.
    exec(compile(guard_bytes, GUARD, "exec"), namespace)
    namespace["validate_environment"](CONTEXT, TEMPORARY_HOOKS, TEMPORARY_IDENTITY)
    refs = namespace["validate"](manifest_bytes, sys.argv[1:], payload)
    print("cleanup guard approved advertised deletion batch: " + ", ".join(refs), file=sys.stderr)
except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
    print(f"cleanup guard rejected: {error}", file=sys.stderr)
    sys.exit(1)
