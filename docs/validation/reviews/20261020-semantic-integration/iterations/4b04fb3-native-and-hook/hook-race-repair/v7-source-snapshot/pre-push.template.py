#!@PYTHON@
"""Temporary pinned pre-push chain. It does not push or modify Git config."""
import hashlib
import os
from pathlib import Path
import signal
import subprocess
import sys

MANIFEST = @MANIFEST@
MANIFEST_SHA256 = @MANIFEST_SHA256@
GUARD = @GUARD@
GUARD_SHA256 = @GUARD_SHA256@
ORIGINAL_HOOKS_DIRECTORY = @ORIGINAL_HOOKS_DIRECTORY@
ORIGINAL_HOOKS = @ORIGINAL_HOOKS@


def hook_snapshot(directory):
    root = Path(directory)
    if not root.is_dir():
        raise ValueError("original hooks directory disappeared")
    return {p.name: {"sha256": hashlib.sha256(p.read_bytes()).hexdigest(),
                     "executable": os.access(p, os.X_OK)}
            for p in root.iterdir() if p.is_file()}


try:
    payload = sys.stdin.buffer.read()
    if hook_snapshot(ORIGINAL_HOOKS_DIRECTORY) != ORIGINAL_HOOKS:
        raise ValueError("original hooks changed after preparation; re-review the chain")
    original = Path(ORIGINAL_HOOKS_DIRECTORY) / "pre-push"
    if original.exists():
        if not original.is_file() or not os.access(original, os.X_OK):
            raise ValueError("existing pre-push is not an executable file; do not bypass it")
        result = subprocess.run([str(original), *sys.argv[1:]], input=payload)
        if result.returncode < 0:
            if -result.returncode not in (signal.SIGKILL, signal.SIGSTOP):
                signal.signal(-result.returncode, signal.SIG_DFL)
            os.kill(os.getpid(), -result.returncode)
        if result.returncode:
            sys.exit(result.returncode)
    guard_bytes = Path(GUARD).read_bytes()
    manifest_bytes = Path(MANIFEST).read_bytes()
    if hashlib.sha256(guard_bytes).hexdigest() != GUARD_SHA256:
        raise ValueError("guard SHA-256 differs from reviewed launcher")
    if hashlib.sha256(manifest_bytes).hexdigest() != MANIFEST_SHA256:
        raise ValueError("manifest SHA-256 differs from reviewed launcher")
    namespace = {"__name__": "pinned_cleanup_guard", "__file__": GUARD}
    # Execute precisely the bytes just hashed, rather than reopening a changed path.
    exec(compile(guard_bytes, GUARD, "exec"), namespace)
    refs = namespace["validate"](manifest_bytes, sys.argv[1:], payload)
    print("cleanup guard approved advertised deletion batch: " + ", ".join(refs), file=sys.stderr)
except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
    print(f"cleanup guard rejected: {error}", file=sys.stderr)
    sys.exit(1)
