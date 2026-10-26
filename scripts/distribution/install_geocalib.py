#!/usr/bin/env python3
"""Install a local model helper using an existing GeoCalib environment and weights."""

from __future__ import annotations

import argparse
import hashlib
import os
from pathlib import Path
import shlex
import tempfile

ROOT = Path(__file__).resolve().parents[2]
WEIGHTS_SHA256 = "86d6aeacd8bbd974c59ce39f61854e00d36911c732ad89be471476fd708722ac"


def install(python: Path, weights: Path, bin_dir: Path) -> Path:
    # Keep the venv entry point rather than resolving its symlink to system Python.
    python = python.absolute()
    weights = weights.resolve(strict=True)
    bin_dir = bin_dir.resolve(strict=True)
    if not python.is_file() or not os.access(python, os.X_OK):
        raise ValueError("--python must name the executable in the GeoCalib environment")
    with weights.open("rb") as stream:
        if hashlib.file_digest(stream, "sha256").hexdigest() != WEIGHTS_SHA256:
            raise ValueError("GeoCalib weights SHA-256 does not match the pinned release")
    source = (ROOT / "model_adapters/geocalib/estimate.py").read_text(encoding="utf-8")
    delimiter = "XDREMUX_GEOCALIB_MODEL_SOURCE"
    if delimiter in source.splitlines():
        raise ValueError("unexpected model source delimiter")
    # Isolated mode excludes CWD, PYTHONPATH and user-site imports while retaining
    # packages in the selected venv. Keep the venv interpreter path above intact.
    # One self-contained helper; no live dependency on the source checkout.
    launcher = (f"#!/bin/sh\nexec {shlex.quote(str(python))} -I - --weights "
                f"{shlex.quote(str(weights))} \"$@\" <<'{delimiter}'\n{source}\n{delimiter}\n")
    output = bin_dir / "xdremux-geocalib"
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=bin_dir,
                                         prefix=".geocalib-install-", delete=False) as stream:
            temporary = Path(stream.name)
            stream.write(launcher)
        temporary.chmod(0o700)
        os.link(temporary, output)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)
    return output


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--python", type=Path, required=True)
    parser.add_argument("--weights", type=Path, required=True)
    parser.add_argument("--bin-dir", type=Path, required=True)
    args = parser.parse_args()
    try:
        print(install(args.python, args.weights, args.bin_dir))
    except (OSError, ValueError) as error:
        parser.exit(1, f"GeoCalib installation failed: {error}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
