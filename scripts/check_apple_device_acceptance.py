#!/usr/bin/env python3
"""Validate SHA-bound physical Apple Photos acceptance evidence.

This checker deliberately cannot create acceptance evidence. A human must test
the exact Rust-produced assets on a representative physical Apple device and
record the observations in the bundle manifest. The checker only verifies that
the result is complete, exact-head, and still bound to the bytes that were
actually tested.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sys
from typing import Any


REQUIRED_OUTPUTS = ("portrait", "photographic_styles")
REQUIRED_TRUE_FIELDS = (
    "photos_imported",
    "feature_recognized",
    "editing_ui_available",
    "edited_render_matches_expected_behavior",
    "revert_or_round_trip_succeeds",
)


class AcceptanceError(RuntimeError):
    pass


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_manifest(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise AcceptanceError(f"cannot read acceptance manifest {path}: {error}") from error
    if not isinstance(value, dict):
        raise AcceptanceError("acceptance manifest must be a JSON object")
    return value


def nonempty_string(value: Any, context: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise AcceptanceError(f"{context} must be a non-empty string")
    return value


def asset_path(bundle_root: Path, raw_path: Any, context: str) -> Path:
    relative = Path(nonempty_string(raw_path, f"{context}.path"))
    if relative.is_absolute() or ".." in relative.parts:
        raise AcceptanceError(f"{context}.path must stay inside the validation bundle")
    return bundle_root / relative


def validate_acceptance(manifest_path: Path, expected_head: str) -> None:
    manifest_path = manifest_path.resolve()
    manifest = load_manifest(manifest_path)
    if manifest.get("manifest_schema_version") != 1:
        raise AcceptanceError(
            f"unsupported manifest_schema_version {manifest.get('manifest_schema_version')!r}"
        )
    expected_head = nonempty_string(expected_head, "expected head")
    if manifest.get("head") != expected_head:
        raise AcceptanceError(
            "device evidence is not exact-head: "
            f"expected {expected_head}, manifest={manifest.get('head')!r}"
        )

    outputs = manifest.get("outputs")
    if not isinstance(outputs, dict):
        raise AcceptanceError("manifest outputs must be an object")
    bundle_root = manifest_path.parent

    for name in REQUIRED_OUTPUTS:
        record = outputs.get(name)
        if not isinstance(record, dict):
            raise AcceptanceError(f"missing output record {name!r}")
        path = asset_path(bundle_root, record.get("path"), f"outputs.{name}")
        if not path.is_file():
            raise AcceptanceError(f"outputs.{name} asset is missing: {path}")
        size = record.get("size_bytes")
        if isinstance(size, bool) or not isinstance(size, int) or size <= 0:
            raise AcceptanceError(f"outputs.{name}.size_bytes must be a positive integer")
        if path.stat().st_size != size:
            raise AcceptanceError(
                f"outputs.{name} size changed: manifest={size}, actual={path.stat().st_size}"
            )
        expected_sha = nonempty_string(record.get("sha256"), f"outputs.{name}.sha256")
        actual_sha = sha256(path)
        if actual_sha != expected_sha:
            raise AcceptanceError(
                f"outputs.{name} SHA-256 changed: manifest={expected_sha}, actual={actual_sha}"
            )

        acceptance = record.get("device_acceptance")
        if not isinstance(acceptance, dict):
            raise AcceptanceError(f"outputs.{name}.device_acceptance must be an object")
        if acceptance.get("tested") is not True:
            raise AcceptanceError(f"outputs.{name} has not been marked physically tested")
        nonempty_string(acceptance.get("device"), f"outputs.{name}.device_acceptance.device")
        nonempty_string(
            acceptance.get("os_version"),
            f"outputs.{name}.device_acceptance.os_version",
        )
        for field in REQUIRED_TRUE_FIELDS:
            if acceptance.get(field) is not True:
                raise AcceptanceError(
                    f"outputs.{name}.device_acceptance.{field} must be true"
                )
        notes = acceptance.get("notes")
        if notes is not None and not isinstance(notes, str):
            raise AcceptanceError(f"outputs.{name}.device_acceptance.notes must be text or null")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--expected-head", required=True)
    args = parser.parse_args()
    validate_acceptance(args.manifest, args.expected_head)
    print(
        f"PASS physical Apple Photos acceptance for {args.expected_head}: {args.manifest}",
        flush=True,
    )
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except AcceptanceError as error:
        print(f"Apple Photos acceptance failed: {error}", file=sys.stderr)
        sys.exit(1)
