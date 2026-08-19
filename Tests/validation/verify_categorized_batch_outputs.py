#!/usr/bin/env python3
"""Run real Swift/Python batches with --categorize and validate their outputs."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


def run(command: list[str], cwd: Path) -> None:
    subprocess.run(command, cwd=cwd, text=True, check=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True)
    parser.add_argument("--swift-executable", default=".build/debug/xdremux")
    args = parser.parse_args()

    repo = Path(__file__).resolve().parents[2]
    sys.path.insert(0, str(repo))
    from xdremux_py import categorize

    source = Path(args.input).resolve()
    source_comment = categorize.extract_user_comment(source)
    classification = categorize.classify_path(source)
    relative_folder = categorize.FolderProjection.relative_directory(classification)
    swift_executable = str((repo / args.swift_executable).resolve())

    with tempfile.TemporaryDirectory(prefix="xdremux-categorized-batch-") as temporary:
        root = Path(temporary)
        outputs: list[Path] = []
        for implementation in ("swift", "python"):
            input_dir = root / implementation / "input"
            output_dir = root / implementation / "output"
            input_dir.mkdir(parents=True)
            local_input = input_dir / source.name
            shutil.copy2(source, local_input)
            if implementation == "swift":
                run(
                    [
                        swift_executable, "batch", "--input-dir", str(input_dir),
                        "--output-dir", str(output_dir), "--glob", "*.heic",
                        "--jobs", "1", "--no-resume", "--categorize",
                    ],
                    repo,
                )
            else:
                run(
                    [
                        sys.executable, "-m", "xdremux_py", "batch",
                        "--input-dir", str(input_dir), "--output-dir", str(output_dir),
                        "--glob", "*.heic", "--categorize",
                    ],
                    repo,
                )
            output = output_dir / relative_folder / source.name
            if not output.is_file():
                raise AssertionError(f"{implementation} categorized output missing: {output}")
            if categorize.extract_user_comment(output) != source_comment:
                raise AssertionError(f"{implementation} did not preserve UserComment")
            try:
                from pillow_heif import open_heif
                heif = open_heif(str(output), convert_hdr_to_8bit=False)
                if len(heif) < 1:
                    raise AssertionError(f"{implementation} output has no readable image")
            except ImportError as exc:
                raise AssertionError("pillow-heif is required for the functional batch check") from exc
            outputs.append(output)

    print(f"categorized Swift and Python batch outputs passed for {source.name} in {relative_folder}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
