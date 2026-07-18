#!/usr/bin/env python3
"""Verify default cleanup and explicit debug retention on a real Styles conversion."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import sys
import tempfile


def run(command: list[str], *, cwd: Path) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(command, cwd=cwd, text=True, capture_output=True, check=False)
    if result.stdout:
        print(result.stdout, end="")
    if result.stderr:
        print(result.stderr, end="", file=sys.stderr)
    if result.returncode:
        raise RuntimeError(f"command failed with status {result.returncode}: {command}")
    return result


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--binary", type=Path, help="reuse an already built xdremux-dev executable")
    return parser.parse_args()


def conversion_command(
    binary: Path,
    input_path: Path,
    output_path: Path,
    *,
    debug_root: Path | None = None,
) -> list[str]:
    command = [
        str(binary),
        "convert",
        "--input",
        str(input_path),
        "--output",
        str(output_path),
        "--apple-photographic-styles",
    ]
    if debug_root is not None:
        command.extend(["--diagnostics-dir", str(debug_root)])
    return command


def assert_output_directory_is_clean(output_directory: Path, expected: set[Path]) -> None:
    actual = set(output_directory.iterdir())
    unexpected = sorted(str(path) for path in actual - expected)
    missing = sorted(str(path) for path in expected - actual)
    if unexpected or missing:
        raise RuntimeError(
            f"output artifact lifecycle mismatch; unexpected={unexpected}, missing={missing}"
        )


def validate_output(binary: Path, output: Path, *, cwd: Path) -> None:
    command = [str(binary), "validate-apple", "--input", str(output)]
    report = json.loads(run(command, cwd=cwd).stdout)
    if report.get("passed") is not True:
        raise RuntimeError(f"validate-apple rejected {output}")


def main() -> int:
    args = parse_arguments()
    repo = Path.cwd().resolve()
    input_path = args.input.expanduser().resolve()
    if not input_path.is_file():
        print(f"input sample not found: {input_path}", file=sys.stderr)
        return 2

    with tempfile.TemporaryDirectory(prefix="xdremux-artifact-lifecycle-") as directory:
        temporary = Path(directory)
        output_directory = temporary / "outputs"
        output_directory.mkdir()
        if args.binary:
            binary = args.binary.expanduser().resolve()
            if not binary.is_file():
                print(f"Swift CLI binary not found: {binary}", file=sys.stderr)
                return 2
        else:
            run(["swift", "build", "--product", "xdremux-dev"], cwd=repo)
            binary = repo / ".build" / "debug" / "xdremux-dev"

        default_output = output_directory / "default.heic"
        temporary_evidence_before = set(
            Path(tempfile.gettempdir()).glob("xdremux-photographic-styles-*")
        )
        run(
            conversion_command(
                binary,
                input_path,
                default_output,
            ),
            cwd=repo,
        )
        validate_output(binary, default_output, cwd=repo)
        assert_output_directory_is_clean(output_directory, {default_output})
        temporary_evidence_after = set(
            Path(tempfile.gettempdir()).glob("xdremux-photographic-styles-*")
        )
        leaked_temporary_evidence = temporary_evidence_after - temporary_evidence_before
        if leaked_temporary_evidence:
            raise RuntimeError(
                "default conversion leaked temporary Styles evidence: "
                + ", ".join(sorted(str(path) for path in leaked_temporary_evidence))
            )

        debug_root = temporary / "debug"
        debug_output = output_directory / "debug.heic"
        run(
            conversion_command(
                binary,
                input_path,
                debug_output,
                debug_root=debug_root,
            ),
            cwd=repo,
        )
        validate_output(binary, debug_output, cwd=repo)
        assert_output_directory_is_clean(output_directory, {default_output, debug_output})

        diagnostic_directory = debug_root / input_path.stem
        styles_directory = diagnostic_directory / "photographic-styles"
        latest_path = styles_directory / "latest.json"
        latest = json.loads(latest_path.read_text())
        manifest_path = Path(latest["manifest"])
        if not manifest_path.is_file() or manifest_path.parent.parent.parent != styles_directory:
            raise RuntimeError("debug Styles manifest does not resolve inside the retained evidence directory")
        print(
            json.dumps(
                {
                    "schema": "xdremux-apple-feature-artifact-lifecycle-v1",
                    "defaultOutputOnly": True,
                    "debugEvidenceRetained": True,
                    "debugManifest": str(manifest_path),
                },
                indent=2,
                sort_keys=True,
            )
        )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (KeyError, OSError, RuntimeError, ValueError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
