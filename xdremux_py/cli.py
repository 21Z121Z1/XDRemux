#!/usr/bin/env python3
"""XDRemux — HDR and Motion Photo conversion CLI.

The Python front end is cross-platform. Motion Photo -> Apple Live Photo uses
only Python/container code plus pillow-heif/Pillow; macOS Apple frameworks are
used by CI solely as an independent compatibility oracle.
"""

import argparse
import json
import os
import sys

if sys.version_info < (3, 11):
    sys.exit("error: XDRemux requires Python 3.11 or newer")

from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    __package__ = "xdremux_py"

from . import categorize, live_photo, pipeline
from .commands import DEFAULT_BATCH_GLOB, BatchCommand, CategorizeCommand, ConvertCommand
from .motion_photo import MotionPhotoError, parse_motion_photo
from .pipeline import ConversionAnalysis, ConversionConfiguration, ConversionError, MissingImagingDependencies


def _report_analysis(analysis: ConversionAnalysis) -> None:
    print(f"  mode: {analysis.mode}")
    print(f"  edr_scale: {analysis.edr_scale:.4f}")
    print(f"  gainMapMax: {analysis.gain_map_max:.4f}")
    print(f"  hdrCapacityMax: {analysis.hdr_capacity_max:.4f}")


def _report_metadata_summary(analysis: ConversionAnalysis) -> None:
    print(json.dumps({
        "mode": analysis.mode,
        "edr_scale": analysis.edr_scale,
        "gainMapMax": analysis.gain_map_max,
    }, indent=2))


def _convert_one(input_path: Path, output_path: Path, configuration: ConversionConfiguration) -> int:
    try:
        result = pipeline.convert_file(input_path, output_path, configuration, on_analysis=_report_analysis)
    except MissingImagingDependencies as exc:
        _report_metadata_summary(exc.analysis)
        print(f"error: {exc}", file=sys.stderr)
        return 1
    except ConversionError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    verb = "overwritten" if result.overwritten_in_place else f"-> {result.output_path}"
    print(f"converted {result.input_path.name} {verb}")
    return 0


def _motion_configuration_is_default(configuration: ConversionConfiguration) -> bool:
    return not configuration.oppo_compat and not configuration.reencode and configuration.debug_dir is None


def _convert_motion(input_path: Path, output_image: Path) -> int:
    try:
        result = live_photo.convert_motion_photo(input_path, output_image)
    except live_photo.LivePhotoConversionError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    print(f"converted Motion Photo {input_path.name}")
    print(f"  still -> {result.image_path}")
    print(f"  video -> {result.video_path}")
    print(f"  content identifier: {result.content_identifier}")
    print(f"  still-image-time: {result.still_time_seconds:.6f} s")
    for diagnostic in result.diagnostics:
        print(f"  note: {diagnostic}")
    return 0


def _probe_motion(path: Path):
    if path.suffix.lower() not in {".jpg", ".jpeg", ".heic", ".heif"}:
        return None
    return parse_motion_photo(path)


def cmd_convert(command: ConvertCommand) -> int:
    try:
        motion = _probe_motion(command.input_path)
    except (OSError, MotionPhotoError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    if motion is not None:
        if not _motion_configuration_is_default(command.configuration):
            print(
                "error: ProXDR-only conversion switches cannot be combined with Motion Photo conversion",
                file=sys.stderr,
            )
            return 1
        output = command.output_path if command.output_explicit else live_photo.default_output_image(command.input_path)
        return _convert_motion(command.input_path, output)
    if command.input_path.suffix.lower() in {".jpg", ".jpeg"}:
        print("error: JPEG input is not a supported Motion Photo", file=sys.stderr)
        return 1
    return _convert_one(command.input_path, command.output_path, command.configuration)


def _default_batch_candidates(input_dir: Path) -> list[Path]:
    allowed = {".heic", ".heif", ".jpg", ".jpeg"}
    return sorted(path for path in input_dir.iterdir() if path.is_file() and path.suffix.lower() in allowed)


def _motion_output_for(source: Path, command: BatchCommand) -> Path:
    parent = command.output_dir
    if command.categorize_output:
        destination = categorize.batch_destinations([source], command.output_dir).get(source)
        if destination is not None:
            parent = destination.parent
    if source.suffix.lower() in {".heic", ".heif"}:
        return parent / f"{source.stem}.live.heic"
    return parent / f"{source.stem}.heic"


def cmd_batch(command: BatchCommand) -> int:
    if not command.input_dir.is_dir():
        print(f"error: input dir not found: {command.input_dir}", file=sys.stderr)
        return 1
    command.output_dir.mkdir(parents=True, exist_ok=True)
    candidates = (
        sorted(path for path in command.input_dir.glob(command.glob) if path.is_file())
        if command.glob_explicit else _default_batch_candidates(command.input_dir)
    )
    motion_inputs: list[Path] = []
    normal_inputs: list[Path] = []
    failed = 0
    for path in candidates:
        try:
            motion = _probe_motion(path)
        except (OSError, MotionPhotoError) as exc:
            print(f"error: {path.name}: {exc}", file=sys.stderr)
            failed += 1
            continue
        if motion is not None:
            motion_inputs.append(path)
        elif path.suffix.lower() in {".heic", ".heif"}:
            # On rerun, generated Apple Live Photo stills must not fall into ProXDR conversion.
            if live_photo.existing_pair_is_valid(path, live_photo.companion_video_path(path)):
                continue
            normal_inputs.append(path)
        elif command.glob_explicit:
            print(f"error: {path.name}: JPEG input is not a supported Motion Photo", file=sys.stderr)
            failed += 1
        # Ordinary JPEG is intentionally ignored by default discovery.

    converted = 0
    if normal_inputs:
        destinations = categorize.batch_destinations(normal_inputs, command.output_dir) if command.categorize_output else {}
        for input_path in normal_inputs:
            output_path = destinations.get(input_path, command.output_dir / input_path.name)
            if _convert_one(input_path, output_path, command.configuration) == 0:
                converted += 1
            else:
                failed += 1

    if motion_inputs and not _motion_configuration_is_default(command.configuration):
        print(
            "error: ProXDR-only conversion switches cannot be combined with Motion Photo batch inputs",
            file=sys.stderr,
        )
        failed += len(motion_inputs)
    else:
        for input_path in motion_inputs:
            output_image = _motion_output_for(input_path, command)
            output_video = live_photo.companion_video_path(output_image)
            if live_photo.existing_pair_is_valid(output_image, output_video):
                print(f"skipped existing Live Photo pair {output_image.name} + {output_video.name}")
                continue
            if _convert_motion(input_path, output_image) == 0:
                converted += 1
            else:
                failed += 1

    print(f"batch complete: {converted} converted, {failed} failed")
    return 0 if failed == 0 else 1


def cmd_categorize(command: CategorizeCommand) -> int:
    try:
        plan = categorize.make_plan(command.input_paths, command.output_dir)
    except (OSError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    results = categorize.execute_plan(plan, jobs=command.jobs, dry_run=command.dry_run)
    for item in results:
        mode = item.classification.mode.folder_name if item.classification.mode else f"根目录 ({item.classification.status})"
        detail = f" error={item.error}" if item.error else ""
        print(f"{item.disposition} [{mode}] {item.source} -> {item.destination}{detail}")
    copied = sum(item.disposition == "copied" for item in results)
    dry_run = sum(item.disposition == "dry-run" for item in results)
    duplicate = sum(item.disposition == "duplicate" for item in results)
    categorized = sum(item.classification.mode is not None for item in results)
    root = len(results) - categorized
    failed = sum(
        item.disposition == "failed" or item.classification.status in {"malformed-user-comment", "unreadable-image"}
        for item in results
    )
    print(
        f"categorize complete: {categorized} categorized, {root} kept at root, "
        f"{copied} copied, {dry_run} dry-run, {duplicate} duplicate, {failed} failed"
    )
    return 0 if failed == 0 else 1


def _conversion_options_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(add_help=False)
    parser.set_defaults(oppo_compat=False, passthrough=True, reencode=False)
    parser.add_argument("--debug-dir", help="Write a per-file metadata dump under this directory")
    parser.add_argument("--oppo-compatible", "--oppo-compat", action="store_true", dest="oppo_compat",
                        help="Add OPPO Gallery compatibility metadata")
    parser.add_argument("--no-oppo-compat", action="store_false", dest="oppo_compat", help=argparse.SUPPRESS)
    parser.add_argument("--passthrough", action="store_true", dest="passthrough", help=argparse.SUPPRESS)
    parser.add_argument("--reencode", action="store_true", dest="reencode",
                        help="Decode and re-encode the base image instead of preserving source HEVC")
    return parser


def build_parser() -> argparse.ArgumentParser:
    conversion_options = _conversion_options_parser()
    parser = argparse.ArgumentParser(description="Convert ProXDR HDR photos and Android Motion Photos")
    sub = parser.add_subparsers(dest="command")
    c = sub.add_parser("convert", parents=[conversion_options], help="Convert one photo")
    c.add_argument("--input", required=True, help="Input photo")
    c.add_argument(
        "--output",
        help=(
            "Output photo; ProXDR overwrites the input when omitted. Motion Photo writes "
            "a new HEIC+MOV pair and always preserves the source"
        ),
    )
    b = sub.add_parser("batch", parents=[conversion_options], help="Convert photos in a directory")
    b.add_argument("--input-dir", required=True, help="Input directory")
    b.add_argument("--output-dir", help="Output directory; writes into the input directory when omitted")
    b.add_argument(
        "--glob",
        help=(
            f"Explicit file match pattern. Without --glob, discover HEIC/HEIF and classify "
            f"JPEG/JPG Motion Photos (legacy ProXDR pattern was {DEFAULT_BATCH_GLOB})"
        ),
    )
    b.add_argument("--categorize", action="store_true", dest="categorize_output",
                   help="Write outputs under Chinese shooting-mode directories")
    category = sub.add_parser("categorize", help="Copy photos into shooting-mode directories")
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
    if args.command == "batch":
        return cmd_batch(BatchCommand.from_namespace(args))
    if args.command == "categorize":
        if args.jobs < 1:
            parser.error("--jobs must be greater than zero")
        return cmd_categorize(CategorizeCommand.from_namespace(args))
    parser.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
