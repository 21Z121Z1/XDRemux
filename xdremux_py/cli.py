#!/usr/bin/env python3
"""XDRemux — Convert OPPO/OnePlus/realme ProXDR HEIC to ISO 21496-1 HDR HEIC.

Cross-platform Python implementation. Replaces Apple ImageIO / CoreGraphics
with pillow-heif + Pillow + numpy.

This module is the CLI layer only: argument parsing, terminal output, and exit
codes. Conversion behavior lives in ``pipeline``, and parsed arguments become
the typed models in ``commands``.

Usage:
    xdremux-py convert --input <file.heic> [--output <out.heic>] [--debug-dir <dir>] [--oppo-compatible]
    xdremux-py batch --input-dir <dir> [--output-dir <dir>] [--glob <pattern>] [--oppo-compatible] [--categorize]
    xdremux-py categorize --input <file-or-dir> [--input <file-or-dir> ...] --output-dir <dir> [--jobs <n>] [--dry-run]

Without installing the package, run the same commands from the repository root
with ``python3 -m xdremux_py`` or ``python3 xdremux/python/XDRemux.py``.
"""

import argparse
import json
import os
import sys
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    __package__ = "xdremux_py"

from . import categorize, pipeline
from .commands import (
    DEFAULT_BATCH_GLOB,
    BatchCommand,
    CategorizeCommand,
    ConvertCommand,
)
from .pipeline import ConversionAnalysis, ConversionConfiguration, ConversionError


def _report_analysis(analysis: ConversionAnalysis) -> None:
    """Print the HDR parameters as soon as they are known."""
    print(f"  mode: {analysis.mode}")
    print(f"  edr_scale: {analysis.edr_scale:.4f}")
    print(f"  gainMapMax: {analysis.gain_map_max:.4f}")
    print(f"  hdrCapacityMax: {analysis.hdr_capacity_max:.4f}")


def _report_metadata_only(analysis: ConversionAnalysis) -> None:
    """Print the analysis-only summary used when imaging deps are missing."""
    print("Metadata extraction only (install pillow-heif + Pillow + numpy for full conversion)")
    print(json.dumps({
        "mode": analysis.mode,
        "edr_scale": analysis.edr_scale,
        "gainMapMax": analysis.gain_map_max,
    }, indent=2))


def _convert_one(
    input_path: Path,
    output_path: Path,
    configuration: ConversionConfiguration,
) -> int:
    """Convert one file and report it; shared by ``convert`` and ``batch``."""
    try:
        result = pipeline.convert_file(
            input_path,
            output_path,
            configuration,
            on_analysis=_report_analysis,
        )
    except ConversionError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    if not result.encoded:
        _report_metadata_only(result.analysis)

    verb = "overwritten" if result.overwritten_in_place else f"-> {result.output_path}"
    print(f"converted {result.input_path.name} {verb}")
    return 0


def cmd_convert(command: ConvertCommand) -> int:
    """Convert a single ProXDR HEIC file."""
    return _convert_one(command.input_path, command.output_path, command.configuration)


def cmd_batch(command: BatchCommand) -> int:
    """Batch convert ProXDR HEIC files."""
    try:
        entries = pipeline.plan_batch(
            command.input_dir,
            command.output_dir,
            command.glob,
            command.categorize_output,
        )
    except ConversionError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    converted, failed = 0, 0
    for entry in entries:
        if _convert_one(entry.input_path, entry.output_path, command.configuration) == 0:
            converted += 1
        else:
            failed += 1

    print(f"batch complete: {converted} converted, {failed} failed")
    return 0 if failed == 0 else 1


def cmd_categorize(command: CategorizeCommand) -> int:
    """Copy photos into shooting-mode directories without modifying sources."""
    try:
        plan = categorize.make_plan(command.input_paths, command.output_dir)
    except (OSError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    results = categorize.execute_plan(plan, jobs=command.jobs, dry_run=command.dry_run)
    for item in results:
        mode = (
            item.classification.mode.folder_name
            if item.classification.mode
            else f"根目录 ({item.classification.status})"
        )
        detail = f" error={item.error}" if item.error else ""
        print(f"{item.disposition} [{mode}] {item.source} -> {item.destination}{detail}")
    copied = sum(item.disposition == "copied" for item in results)
    dry_run = sum(item.disposition == "dry-run" for item in results)
    duplicate = sum(item.disposition == "duplicate" for item in results)
    categorized = sum(item.classification.mode is not None for item in results)
    root = len(results) - categorized
    failed = sum(
        item.disposition == "failed"
        or item.classification.status in {"malformed-user-comment", "unreadable-image"}
        for item in results
    )
    print(
        f"categorize complete: {categorized} categorized, {root} kept at root, "
        f"{copied} copied, {dry_run} dry-run, {duplicate} duplicate, {failed} failed"
    )
    return 0 if failed == 0 else 1


def _conversion_options_parser() -> argparse.ArgumentParser:
    """Options shared by ``convert`` and ``batch``."""
    parser = argparse.ArgumentParser(add_help=False)
    parser.set_defaults(oppo_compat=False, passthrough=True, reencode=False)
    parser.add_argument("--debug-dir",
                        help="Write a per-file metadata dump under this directory")
    parser.add_argument("--oppo-compatible", "--oppo-compat", action="store_true", dest="oppo_compat",
                        help="Add OPPO Gallery compatibility metadata")
    parser.add_argument("--no-oppo-compat", action="store_false", dest="oppo_compat",
                        help=argparse.SUPPRESS)
    parser.add_argument("--passthrough", action="store_true", dest="passthrough",
                        help=argparse.SUPPRESS)
    parser.add_argument("--reencode", action="store_true", dest="reencode",
                        help="Decode and re-encode the base image instead of preserving source HEVC")
    return parser


def build_parser() -> argparse.ArgumentParser:
    conversion_options = _conversion_options_parser()
    parser = argparse.ArgumentParser(
        description="Convert OPPO ProXDR HEIC to ISO 21496-1 HDR HEIC",
    )
    sub = parser.add_subparsers(dest="command")

    c = sub.add_parser("convert", parents=[conversion_options],
                       help="Convert one photo")
    c.add_argument("--input", required=True, help="Input photo")
    c.add_argument("--output", help="Output photo; overwrites the input when omitted")

    b = sub.add_parser("batch", parents=[conversion_options],
                       help="Convert every matching photo in a directory")
    b.add_argument("--input-dir", required=True, help="Input directory")
    b.add_argument("--output-dir",
                   help="Output directory; writes into the input directory when omitted")
    b.add_argument("--glob", help=f"File match pattern (default {DEFAULT_BATCH_GLOB})")
    b.add_argument("--categorize", action="store_true", dest="categorize_output",
                   help="Write outputs under Chinese shooting-mode directories")

    category = sub.add_parser("categorize",
                              help="Copy photos into shooting-mode directories")
    category.add_argument("--input", action="append", required=True,
                          help="Input photo or directory; may be repeated")
    category.add_argument("--output-dir", required=True)
    category.add_argument("--jobs", type=int, default=min(os.cpu_count() or 1, 4))
    category.add_argument("--dry-run", action="store_true")
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.command == "convert":
        return cmd_convert(ConvertCommand.from_namespace(args))
    elif args.command == "batch":
        return cmd_batch(BatchCommand.from_namespace(args))
    elif args.command == "categorize":
        if args.jobs < 1:
            parser.error("--jobs must be greater than zero")
        return cmd_categorize(CategorizeCommand.from_namespace(args))
    else:
        parser.print_help()
        return 1


if __name__ == "__main__":
    sys.exit(main())
