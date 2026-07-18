#!/usr/bin/env python3
"""Compile/run the Swift CLI on one real sample and assert ImageIO pixel format."""

from __future__ import annotations

import argparse
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


PIXEL_FORMAT_INSPECTOR = r'''
import Foundation
import ImageIO

guard CommandLine.arguments.count == 3 else { exit(64) }
let url = URL(fileURLWithPath: CommandLine.arguments[1])
let expected = CommandLine.arguments[2]
guard let data = try? Data(contentsOf: url),
      let source = CGImageSourceCreateWithData(data as CFData, nil),
      let info = CGImageSourceCopyAuxiliaryDataInfoAtIndex(
          source,
          0,
          kCGImageAuxiliaryDataTypeISOGainMap
      ) as? [CFString: Any],
      let description = info[kCGImageAuxiliaryDataInfoDataDescription] as? [CFString: Any],
      let number = description[kCGImagePropertyPixelFormat] as? NSNumber else {
    fputs("no ImageIO ISO gain-map pixel format\n", stderr)
    exit(1)
}
let value = number.uint32Value
let bytes = [
    UInt8((value >> 24) & 0xff),
    UInt8((value >> 16) & 0xff),
    UInt8((value >> 8) & 0xff),
    UInt8(value & 0xff),
]
let actual = String(bytes: bytes, encoding: .ascii) ?? "????"
guard actual == expected else {
    fputs("gain-map pixel format mismatch: expected \(expected), got \(actual)\n", stderr)
    exit(1)
}
print("gain-map pixel format: \(actual)")
'''


def run(command: list[str], *, cwd: Path) -> None:
    result = subprocess.run(command, cwd=cwd, text=True, capture_output=True, check=False)
    if result.stdout:
        print(result.stdout, end="")
    if result.stderr:
        print(result.stderr, end="", file=sys.stderr)
    if result.returncode != 0:
        raise SystemExit(result.returncode)


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--expected-pixel-format", required=True, choices=("444f", "420f", "420v", "x420", "L008"))
    parser.add_argument("--binary", type=Path, help="reuse an already built xdremux executable")
    parser.add_argument("--oppo-compatible", action="store_true")
    parser.add_argument("--in-place", action="store_true", help="convert a temporary copy in place")
    parser.add_argument("--validate-only", action="store_true", help="validate --input without converting it")
    return parser.parse_args()


def main() -> int:
    arguments = parse_arguments()
    repo = Path.cwd().resolve()
    input_path = arguments.input.expanduser().resolve()
    if not input_path.is_file():
        print(f"input sample not found: {input_path}", file=sys.stderr)
        return 2

    if arguments.validate_only:
        run(
            ["swift", "-e", PIXEL_FORMAT_INSPECTOR, str(input_path), arguments.expected_pixel_format],
            cwd=repo,
        )
        return 0

    with tempfile.TemporaryDirectory(prefix="xdremux-swift-sample-") as directory:
        temporary = Path(directory)
        if arguments.binary:
            binary = arguments.binary.expanduser().resolve()
            if not binary.is_file():
                print(f"Swift CLI binary not found: {binary}", file=sys.stderr)
                return 2
        else:
            run(["swift", "build", "--product", "xdremux"], cwd=repo)
            binary = repo / ".build" / "debug" / "xdremux"

        if arguments.in_place:
            output = temporary / input_path.name
            shutil.copy2(input_path, output)
            command = [str(binary), "convert", "--input", str(output)]
        else:
            output = temporary / "output.heic"
            command = [
                str(binary),
                "convert",
                "--input",
                str(input_path),
                "--output",
                str(output),
                "--language",
                "en",
                "--format",
                "jsonl",
            ]
        if arguments.oppo_compatible:
            command.append("--oppo-compatible")
        run(command, cwd=repo)
        run(
            ["swift", "-e", PIXEL_FORMAT_INSPECTOR, str(output), arguments.expected_pixel_format],
            cwd=repo,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
