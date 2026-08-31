#!/usr/bin/env python3
from __future__ import annotations

import json
import subprocess
import tempfile
from pathlib import Path

from xdremux_py.motion_photo import ByteRange, copy_range

ROOT = Path(__file__).resolve().parents[1]
RUST_ORACLE = ROOT / "target" / "debug" / "examples" / "payload_conformance"


def run_rust(
    source: Path,
    byte_range: ByteRange,
    destination: Path,
    *,
    max_bytes: int,
    buffer_size: int,
) -> dict:
    completed = subprocess.run(
        [
            str(RUST_ORACLE),
            str(source),
            str(byte_range.start),
            str(byte_range.end),
            str(destination),
            str(max_bytes),
            str(buffer_size),
        ],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(completed.stdout)


def assert_success_case(name: str, source_bytes: bytes, byte_range: ByteRange) -> None:
    with tempfile.TemporaryDirectory(prefix="xdremux-payload-python-rust-") as directory:
        root = Path(directory)
        source = root / "source.bin"
        python_destination = root / "python" / f"{name}.bin"
        rust_destination = root / "rust" / f"{name}.bin"
        source.write_bytes(source_bytes)

        copy_range(source, byte_range, python_destination, chunk_size=113)
        result = run_rust(
            source,
            byte_range,
            rust_destination,
            max_bytes=max(1, byte_range.length),
            buffer_size=113,
        )
        if result.get("status") != "ok":
            raise AssertionError(f"{name}: Rust payload copy failed: {result!r}")

        python_bytes = python_destination.read_bytes()
        rust_bytes = rust_destination.read_bytes()
        expected = source_bytes[byte_range.start:byte_range.end]
        if python_bytes != expected or rust_bytes != expected:
            raise AssertionError(
                f"{name}: payload bytes diverged: "
                f"python={len(python_bytes)} rust={len(rust_bytes)} expected={len(expected)}"
            )


def main() -> None:
    if not RUST_ORACLE.exists():
        raise SystemExit(f"Rust payload oracle is not built: {RUST_ORACLE}")

    source = bytes(index & 0xFF for index in range(8192))
    assert_success_case("subrange", source, ByteRange(31, 4097))
    assert_success_case("empty", source, ByteRange(512, 512))
    print("Python/Rust Motion Photo payload conformance: PASS")


if __name__ == "__main__":
    main()
