#!/usr/bin/env python3
"""Render reviewable temporary hooks; never configure a repository or push."""
import argparse
import hashlib
import os
from pathlib import Path
import pprint
import shlex
import sys


def prepare(manifest, output, original_hooks):
    manifest, output, original_hooks = map(lambda p: Path(p).resolve(),
                                            (manifest, output, original_hooks))
    if output.exists() or output == original_hooks or output.is_relative_to(original_hooks):
        raise ValueError("use a new temporary directory outside the original hooks directory")
    if not original_hooks.is_dir():
        raise ValueError("original hooks directory must be inspected explicitly")
    guard = Path(__file__).with_name("guard.py").resolve()
    values = {"PYTHON": sys.executable, "MANIFEST": str(manifest),
              "MANIFEST_SHA256": hashlib.sha256(manifest.read_bytes()).hexdigest(),
              "GUARD": str(guard), "GUARD_SHA256": hashlib.sha256(guard.read_bytes()).hexdigest(),
              "ORIGINAL_HOOKS_DIRECTORY": str(original_hooks)}
    entries = list(original_hooks.iterdir())
    if any(not p.is_file() for p in entries):
        raise ValueError("original hook entry is not an ordinary file; review manually")
    values["ORIGINAL_HOOKS"] = {
        p.name: {"sha256": hashlib.sha256(p.read_bytes()).hexdigest(),
                 "executable": os.access(p, os.X_OK)} for p in entries}
    template = Path(__file__).with_name("pre-push.template.py").read_text()
    for key, value in values.items():
        template = template.replace("@" + key + "@", value if key == "PYTHON" else pprint.pformat(value))
    output.mkdir(parents=True)
    hook = output / "pre-push"
    hook.write_text(template)
    hook.chmod(0o755)
    # core.hooksPath redirects every hook. Forward the others to their original
    # paths as well, preserving stdin, arguments, cwd and exit/signal semantics.
    for path in entries:
        if path.name != "pre-push":
            shim = output / path.name
            shim.write_text("#!/bin/sh\nexec " + shlex.quote(str(path)) + ' "$@"\n')
            shim.chmod(0o755 if os.access(path, os.X_OK) else 0o644)
    return hook


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--original-hooks", type=Path, required=True)
    args = parser.parse_args()
    print(prepare(args.manifest, args.output, args.original_hooks))
