#!/usr/bin/env python3
"""Render a disabled review artifact; never change repository config or push."""
import argparse
import hashlib
from pathlib import Path
import pprint
import sys

from guard import directory_identity, prepare_context


def prepare(manifest, output, repository):
    manifest, output = Path(manifest).resolve(), Path(output).resolve()
    context = prepare_context(repository)
    original = Path(context["original_hooks_directory"])
    if output.exists() or output == original or output.is_relative_to(original):
        raise ValueError("use a new directory outside the original hooks directory")
    guard = Path(__file__).with_name("guard.py").resolve()
    values = {"PYTHON": sys.executable, "MANIFEST": str(manifest),
              "MANIFEST_SHA256": hashlib.sha256(manifest.read_bytes()).hexdigest(),
              "GUARD": str(guard), "GUARD_SHA256": hashlib.sha256(guard.read_bytes()).hexdigest(),
              "CONTEXT": context, "TEMPORARY_HOOKS": str(output)}
    template = Path(__file__).with_name("pre-push.template.py").read_text()
    output.mkdir(parents=True, mode=0o700)
    values["TEMPORARY_IDENTITY"] = directory_identity(output)
    for key, value in values.items():
        template = template.replace("@" + key + "@", value if key == "PYTHON" else pprint.pformat(value))
    hook = output / "pre-push"
    hook.write_text(template)
    hook.chmod(0o644)
    return hook


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--repository", type=Path, required=True)
    args = parser.parse_args()
    try:
        print(prepare(args.manifest, args.output, args.repository))
    except (OSError, ValueError) as error:
        parser.exit(1, f"cleanup preparation rejected: {error}\n")
