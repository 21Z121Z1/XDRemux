#!/usr/bin/env python3
"""Compare Swift and Python categorization against the same real photos."""

from __future__ import annotations

import argparse
import hashlib
import re
import struct
import subprocess
import sys
import tempfile
from pathlib import Path


RESULT_PATTERN = re.compile(r"^(\S+) \[(.+)] (.+) -> (.+?)(?: error=.*)?$")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def output_manifest(root: Path) -> dict[str, str]:
    return {
        str(path.relative_to(root)): sha256(path)
        for path in sorted(root.rglob("*"))
        if path.is_file()
    }


def run(
    command: list[str],
    cwd: Path,
    allowed_returncodes: tuple[int, ...] = (0,),
) -> subprocess.CompletedProcess[str]:
    completed = subprocess.run(command, cwd=cwd, text=True, capture_output=True)
    if completed.returncode not in allowed_returncodes:
        raise subprocess.CalledProcessError(
            completed.returncode,
            command,
            output=completed.stdout,
            stderr=completed.stderr,
        )
    return completed


def result_manifest(stdout: str, output_root: Path) -> list[tuple[str, str, str]]:
    results: list[tuple[str, str, str]] = []
    for line in stdout.splitlines():
        match = RESULT_PATTERN.match(line)
        if not match:
            continue
        disposition, mode, _, destination = match.groups()
        relative = str(Path(destination).relative_to(output_root))
        results.append((disposition, mode, relative))
    return results


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", action="append", required=True)
    parser.add_argument("--swift-executable", default=".build/debug/xdremux")
    args = parser.parse_args()

    repo = Path(__file__).resolve().parents[2]
    inputs = [str(Path(value).resolve()) for value in args.input]
    swift_executable = str((repo / args.swift_executable).resolve())
    python_cli = str(repo / "xdremux/python/XDRemux.py")

    with tempfile.TemporaryDirectory(prefix="xdremux-categorization-cross-") as temporary:
        root = Path(temporary)
        swift_output = root / "swift"
        python_output = root / "python"
        common = [part for value in inputs for part in ("--input", value)]
        swift_dry_output = root / "swift-dry"
        python_dry_output = root / "python-dry"
        swift_dry = run(
            [swift_executable, "categorize", *common, "--output-dir", str(swift_dry_output), "--jobs", "3", "--dry-run"],
            repo,
            (0, 1),
        )
        python_dry = run(
            [sys.executable, python_cli, "categorize", *common, "--output-dir", str(python_dry_output), "--jobs", "3", "--dry-run"],
            repo,
            (0, 1),
        )
        swift_dry_results = result_manifest(swift_dry.stdout, swift_dry_output)
        python_dry_results = result_manifest(python_dry.stdout, python_dry_output)
        if not swift_dry_results or swift_dry_results != python_dry_results:
            raise AssertionError("Swift and Python dry-run plans differ")
        if any(item[0] != "dry-run" for item in swift_dry_results):
            raise AssertionError(f"dry-run returned unexpected dispositions: {swift_dry_results}")
        if swift_dry_output.exists() or python_dry_output.exists():
            raise AssertionError("dry-run created an output directory")

        malformed = root / "malformed.jpg"
        payload = b"ASCII\0\0\0not-an-oppo-comment"
        header = b"II" + struct.pack("<H", 42) + struct.pack("<I", 8)
        ifd0 = struct.pack("<H", 1) + struct.pack("<HHII", 0x8769, 4, 1, 26) + struct.pack("<I", 0)
        exif = struct.pack("<H", 1) + struct.pack("<HHII", 0x9286, 7, len(payload), 44) + struct.pack("<I", 0)
        malformed.write_bytes(header + ifd0 + exif + payload)
        for label, command, output in (
            (
                "Swift",
                [swift_executable, "categorize", "--input", str(malformed), "--output-dir", str(root / "swift-malformed")],
                root / "swift-malformed",
            ),
            (
                "Python",
                [sys.executable, python_cli, "categorize", "--input", str(malformed), "--output-dir", str(root / "python-malformed")],
                root / "python-malformed",
            ),
        ):
            completed = subprocess.run(command, cwd=repo, text=True, capture_output=True)
            if completed.returncode == 0:
                raise AssertionError(f"{label} accepted a malformed UserComment without reporting failure")
            if (output / malformed.name).read_bytes() != malformed.read_bytes():
                raise AssertionError(f"{label} did not preserve the malformed photo in the output root")

        swift = run(
            [swift_executable, "categorize", *common, "--output-dir", str(swift_output), "--jobs", "2"],
            repo,
            (0, 1),
        )
        python = run(
            [sys.executable, python_cli, "categorize", *common, "--output-dir", str(python_output), "--jobs", "2"],
            repo,
            (0, 1),
        )

        swift_results = result_manifest(swift.stdout, swift_output)
        python_results = result_manifest(python.stdout, python_output)
        if swift_results != python_results:
            raise AssertionError(f"result mismatch:\nSwift: {swift_results}\nPython: {python_results}")
        if output_manifest(swift_output) != output_manifest(python_output):
            raise AssertionError("relative output paths or SHA-256 values differ")
        if not swift_results:
            raise AssertionError("no categorized results were reported")

        first_manifest = output_manifest(swift_output)
        swift_repeat = run(
            [swift_executable, "categorize", *common, "--output-dir", str(swift_output)],
            repo,
            (0, 1),
        )
        python_repeat = run(
            [sys.executable, python_cli, "categorize", *common, "--output-dir", str(python_output)],
            repo,
            (0, 1),
        )
        for label, completed, output in (
            ("Swift", swift_repeat, swift_output),
            ("Python", python_repeat, python_output),
        ):
            repeated = result_manifest(completed.stdout, output)
            if not repeated or any(item[0] != "duplicate" for item in repeated):
                raise AssertionError(f"{label} repeat was not duplicate-only: {repeated}")
        if first_manifest != output_manifest(swift_output):
            raise AssertionError("Swift repeat changed the output tree")
        if first_manifest != output_manifest(python_output):
            raise AssertionError("Python repeat changed the output tree")

    print(f"categorization cross-implementation check passed for {len(inputs)} input(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
