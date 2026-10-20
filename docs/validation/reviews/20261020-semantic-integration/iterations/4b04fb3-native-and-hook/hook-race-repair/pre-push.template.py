#!@PYTHON@
"""Temporary guard for a reviewed repository with no active original hooks."""
import hashlib
from pathlib import Path
import subprocess
import sys

MANIFEST = @MANIFEST@
MANIFEST_SHA256 = @MANIFEST_SHA256@
GUARD = @GUARD@
GUARD_SHA256 = @GUARD_SHA256@
CONTEXT = @CONTEXT@
TEMPORARY_HOOKS = @TEMPORARY_HOOKS@
TEMPORARY_IDENTITY = @TEMPORARY_IDENTITY@


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
