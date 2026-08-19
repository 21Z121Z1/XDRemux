#!/usr/bin/env python3
"""Build/run the SwiftPM CLI on one real sample and assert ImageIO pixel format."""

from __future__ import annotations

import argparse
import json
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


def run(
    command: list[str],
    *,
    cwd: Path,
    echo_output: bool = True,
) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(command, cwd=cwd, text=True, capture_output=True, check=False)
    if (echo_output or result.returncode != 0) and result.stdout:
        print(result.stdout, end="")
    if (echo_output or result.returncode != 0) and result.stderr:
        print(result.stderr, end="", file=sys.stderr)
    if result.returncode != 0:
        raise SystemExit(result.returncode)
    return result


def production_cli(repo: Path, explicit_binary: Path | None) -> Path:
    if explicit_binary is not None:
        binary = explicit_binary.expanduser().resolve()
        if not binary.is_file():
            raise FileNotFoundError(f"Swift CLI binary not found: {binary}")
        return binary

    run(["swift", "build", "--quiet", "--product", "xdremux"], cwd=repo)
    bin_path = run(
        ["swift", "build", "--show-bin-path"],
        cwd=repo,
        echo_output=False,
    ).stdout.strip()
    binary = Path(bin_path) / "xdremux"
    if not binary.is_file():
        raise FileNotFoundError(f"SwiftPM did not produce xdremux at {binary}")
    return binary


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--expected-pixel-format", required=True, choices=("444f", "420f", "420v", "x420", "L008"))
    parser.add_argument("--binary", type=Path, help="reuse an already built SwiftPM xdremux binary")
    parser.add_argument("--oppo-compatible", action="store_true")
    parser.add_argument("--apple-portrait", action="store_true")
    parser.add_argument(
        "--expect-direct-gain",
        action="store_true",
        help="require the one-pass direct Gain Map encoder diagnostic",
    )
    parser.add_argument(
        "--require-compressed-primary-preserved",
        action="store_true",
        help="assert ordinary conversion kept the source Base payload byte-identical",
    )
    parser.add_argument("--in-place", action="store_true", help="convert a temporary copy in place")
    parser.add_argument(
        "--validate-only",
        action="store_true",
        help="inspect --input as an already-converted output instead of converting it",
    )
    arguments = parser.parse_args()
    if arguments.validate_only:
        incompatible = [
            name
            for name, enabled in (
                ("--in-place", arguments.in_place),
                ("--oppo-compatible", arguments.oppo_compatible),
                ("--apple-portrait", arguments.apple_portrait),
                ("--expect-direct-gain", arguments.expect_direct_gain),
                (
                    "--require-compressed-primary-preserved",
                    arguments.require_compressed_primary_preserved,
                ),
            )
            if enabled
        ]
        if incompatible:
            parser.error(
                "--validate-only performs no conversion and cannot be combined with "
                + ", ".join(incompatible)
            )
    return arguments


def main() -> int:
    arguments = parse_arguments()
    repo = Path.cwd().resolve()
    input_path = arguments.input.expanduser().resolve()
    if not input_path.is_file():
        print(f"input sample not found: {input_path}", file=sys.stderr)
        return 2

    if arguments.validate_only:
        run(
            [
                "swift",
                "-e",
                PIXEL_FORMAT_INSPECTOR,
                str(input_path),
                arguments.expected_pixel_format,
            ],
            cwd=repo,
        )
        return 0

    try:
        binary = production_cli(repo, arguments.binary)
    except FileNotFoundError as error:
        print(error, file=sys.stderr)
        return 2

    with tempfile.TemporaryDirectory(prefix="xdremux-swift-sample-") as directory:
        temporary = Path(directory)
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
            ]
        if arguments.oppo_compatible:
            command.append("--oppo-compatible")
        if arguments.apple_portrait:
            command.append("--apple-portrait")
        conversion = run(command, cwd=repo)
        if arguments.expect_direct_gain:
            expected_diagnostic = (
                "portrait Gain Map encoder=private-vt-tile base=single-imageio-encode"
                if arguments.apple_portrait
                else "[direct-gain] preserved compressed Base; encoded"
            )
            diagnostic_stream = conversion.stdout + conversion.stderr
            if diagnostic_stream.count(expected_diagnostic) != 1:
                print(
                    f"expected exactly one direct Gain Map diagnostic: {expected_diagnostic}",
                    file=sys.stderr,
                )
                return 1
        run(
            ["swift", "-e", PIXEL_FORMAT_INSPECTOR, str(output), arguments.expected_pixel_format],
            cwd=repo,
        )
        if arguments.apple_portrait:
            run([str(binary), "validate-portrait", "--input", str(output)], cwd=repo)
        if arguments.require_compressed_primary_preserved:
            if arguments.apple_portrait:
                print(
                    "compressed-primary preservation compares ordinary inputs only",
                    file=sys.stderr,
                )
                return 2
            comparison = run(
                [
                    sys.executable,
                    "scripts/compare_oppo_heif_mutation.py",
                    "--json",
                    str(input_path),
                    str(output),
                ],
                cwd=repo,
                echo_output=False,
            )
            invariants = json.loads(comparison.stdout)["invariants"]
            required = (
                "primary_payloads_equal",
                "all_non_hdr_item_payloads_equal",
                "non_hdr_tail_entries_equal",
            )
            failed = [name for name in required if invariants.get(name) is not True]
            if failed:
                print(
                    "compressed Base preservation failed: " + ", ".join(failed),
                    file=sys.stderr,
                )
                return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
