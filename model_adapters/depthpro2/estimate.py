#!/usr/bin/env python3
"""Depth Pro 2 camera FOV primitive: return pinhole observations to the Rust runtime."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


ADAPTER_SCHEMA_VERSION = 2
REPO_REVISION = "4e607a682f993bbf95035054fc78ba9ffc81faf2"


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def resolve_adapter(explicit: Path | None) -> Path:
    if explicit is not None:
        resolved = explicit.resolve(strict=True)
        if not resolved.is_file() or not os.access(resolved, os.X_OK):
            raise ValueError(f"Apple adapter is not executable: {resolved}")
        return resolved

    env_path = os.environ.get("XDREMUX_APPLE_ADAPTER")
    if env_path:
        candidate = Path(env_path).expanduser().resolve(strict=True)
        if not candidate.is_file() or not os.access(candidate, os.X_OK):
            raise ValueError(f"XDREMUX_APPLE_ADAPTER is not executable: {candidate}")
        return candidate

    self_dir = Path(__file__).resolve().parent
    for sibling in (
        self_dir / "xdremux-apple-adapter",
        self_dir / "../libexec/xdremux-apple-adapter",
        self_dir / "../Helpers/xdremux-apple-adapter",
    ):
        if sibling.is_file() and os.access(sibling, os.X_OK):
            return sibling.resolve()

    found = shutil.which("xdremux-apple-adapter")
    if found:
        return Path(found).resolve()

    raise FileNotFoundError(
        "could not locate xdremux-apple-adapter; set --adapter or XDREMUX_APPLE_ADAPTER"
    )


def estimate(
    input_path: Path,
    count: int,
    *,
    adapter: Path | None = None,
    models_dir: Path | None = None,
) -> dict:
    if not 3 <= count <= 32:
        raise ValueError("sample count must be between 3 and 32")

    adapter_bin = resolve_adapter(adapter)
    source_hash = digest(input_path)

    request = {
        "schema_version": ADAPTER_SCHEMA_VERSION,
        "operation": "depthpro2-camera-estimate",
        "input_path": str(input_path),
        "depthpro2_camera": {"sample_count": count},
    }
    env = os.environ.copy()
    if models_dir is not None:
        env["XDREMUX_MODEL_DIR"] = str(models_dir)

    completed = subprocess.run(
        [str(adapter_bin)],
        input=(json.dumps(request) + "\n").encode("utf-8"),
        env=env,
        capture_output=True,
        timeout=300,
        check=False,
    )
    if completed.returncode != 0:
        detail = completed.stderr.decode("utf-8", errors="replace").strip()
        raise RuntimeError(f"xdremux-apple-adapter failed ({completed.returncode}): {detail}")

    response = json.loads(completed.stdout.decode("utf-8"))
    if response.get("schema_version") != ADAPTER_SCHEMA_VERSION:
        raise ValueError("unexpected xdremux-apple-adapter schema_version")
    facts = response.get("depthpro2_camera")
    if not isinstance(facts, dict):
        raise ValueError("adapter response omitted depthpro2_camera")

    samples = facts.get("samples")
    if not isinstance(samples, list) or len(samples) < 3:
        raise ValueError("Depth Pro 2 FOV requires at least 3 video frame samples")

    formatted_samples = [
        {
            "time_seconds": sample["time_seconds"],
            "intrinsics": {
                "image_size": sample["image_size"],
                "focal_length_pixels": sample["focal_length_pixels"],
                "principal_point_pixels": sample["principal_point_pixels"],
                "focal_uncertainty_pixels": sample["focal_uncertainty_pixels"],
            },
        }
        for sample in samples
    ]

    shared = facts.get("shared_intrinsics")
    formatted_shared = (
        {
            "image_size": shared["image_size"],
            "focal_length_pixels": shared["focal_length_pixels"],
            "principal_point_pixels": shared["principal_point_pixels"],
            "focal_uncertainty_pixels": shared["focal_uncertainty_pixels"],
        }
        if isinstance(shared, dict)
        else None
    )

    if digest(input_path) != source_hash:
        raise ValueError("input file changed during camera estimation")

    return {
        "schema_version": 1,
        "source_sha256": source_hash,
        "estimator": {
            "name": facts.get("estimator_name", "depthpro2-fov"),
            "revision": facts.get("revision", REPO_REVISION),
            "weights_sha256": facts["weights_sha256"],
        },
        "samples": formatted_samples,
        "shared_intrinsics": formatted_shared,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--adapter", type=Path, default=None)
    parser.add_argument("--models-dir", type=Path, default=None)
    parser.add_argument("--samples", type=int, default=5)
    args = parser.parse_args()
    try:
        if args.output.exists():
            raise ValueError("refusing to replace an existing camera estimate")
        result = estimate(
            args.input.resolve(strict=True),
            args.samples,
            adapter=args.adapter,
            models_dir=args.models_dir,
        )
        encoded = json.dumps(result, indent=2, allow_nan=False) + "\n"
        temporary = None
        try:
            with tempfile.NamedTemporaryFile(
                mode="w",
                encoding="utf-8",
                dir=args.output.parent,
                prefix=".camera-estimate-",
                delete=False,
            ) as stream:
                temporary = Path(stream.name)
                stream.write(encoded)
            os.link(temporary, args.output)
        finally:
            if temporary is not None:
                temporary.unlink(missing_ok=True)
    except (ValueError, OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"Depth Pro 2 camera estimation failed: {error}", file=sys.stderr)
        return 1
    print(args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
