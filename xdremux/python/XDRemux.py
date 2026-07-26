#!/usr/bin/env python3
"""XDRemux — Convert OPPO/OnePlus/realme ProXDR HEIC to ISO 21496-1 HDR HEIC.

Cross-platform Python implementation. Replaces Apple ImageIO / CoreGraphics
with pillow-heif + Pillow + numpy.

Usage:
    xdremux.py convert --input <file.heic> [--output <out.heic>] [--debug-dir <dir>] [--oppo-compatible]
    xdremux.py batch --input-dir <dir> [--output-dir <dir>] [--glob <pattern>] [--oppo-compatible] [--categorize]
    xdremux.py categorize --input <file-or-dir> [--input <file-or-dir> ...] --output-dir <dir> [--jobs <n>] [--dry-run]
"""

import argparse
import json
import os
import sys

if sys.version_info < (3, 11):
    sys.exit("error: XDRemux requires Python 3.11 or newer")

from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
    __package__ = "xdremux.python"

from . import categorize, container, edr, iso21496


def cmd_convert(args: argparse.Namespace) -> int:
    """Convert a single ProXDR HEIC file."""
    input_path = Path(args.input)

    if not input_path.exists():
        print(f"error: input not found: {input_path}", file=sys.stderr)
        return 1

    output_path = Path(args.output) if args.output else input_path

    if output_path != input_path:
        output_path.parent.mkdir(parents=True, exist_ok=True)

    try:
        lhdr = container.extract_lhdr(str(input_path))
    except Exception as e:
        print(f"error: {e}", file=sys.stderr)
        return 1

    try:
        if lhdr.mode == "uhdr":
            iso_meta = iso21496.build_iso21496_metadata_from_uhdr(lhdr.meta_floats)
            edr_scale = iso_meta.get("scale", 1.0)
        else:
            edr_scale = edr.edr_scale_calculator(list(lhdr.meta_floats))
            iso_meta = iso21496.build_iso21496_metadata(edr_scale)
    except (ValueError, OverflowError) as e:
        print(f"error: invalid HDR metadata in {input_path.name}: {e}", file=sys.stderr)
        return 1

    print(f"  mode: {lhdr.mode}")
    print(f"  edr_scale: {edr_scale:.4f}")
    print(f"  gainMapMax: {iso_meta['gainMapMax'][0]:.4f}")
    print(f"  hdrCapacityMax: {iso_meta['hdrCapacityMax']:.4f}")

    try:
        from . import gainmap, heif_io
        import numpy as np
        import io
        from PIL import Image

        base_image = None
        exif_data = None
        reencode = getattr(args, "reencode", False)
        passthrough = False if reencode else getattr(args, "passthrough", True)

        if not passthrough:
            from pillow_heif import open_heif

            data = heif_io.read_heic(str(input_path))
            base_image = data["base_image"]

            # Extract source EXIF for normal-mode re-encode.
            # Passthrough copies the original EXIF item at the ISOBMFF layer.
            try:
                src_heif = open_heif(str(input_path))
                exif_data = src_heif[0].info.get("exif") if hasattr(src_heif, '__getitem__') else src_heif.info.get("exif")
            except Exception:
                pass

            if base_image is None:
                print("error: HEIC decode failed — install pillow-heif for full conversion", file=sys.stderr)
                return 1

        # Resolve gain map
        if lhdr.mode == "uhdr":
            gm_data = lhdr.gainmap_data
            gm_img = None
            if gm_data:
                try:
                    gm_img = Image.open(io.BytesIO(gm_data))
                except Exception:
                    pass
            if gm_img is None:
                print("error: UHDR gain map JPEG is missing or undecodable", file=sys.stderr)
                return 1
        else:
            mask_data = lhdr.mask_data
            if mask_data is None:
                print("error: no mask data found", file=sys.stderr)
                return 1
            mask_np = np.array(Image.open(io.BytesIO(mask_data)).convert("L"))
            gm_img = gainmap.reconstruct(mask_np, edr_scale, lhdr.meta_floats[0])

        if passthrough:
            heif_io.write_heic_passthrough(
                str(input_path),
                str(output_path),
                gm_img,
                iso_meta,
                lhdr=lhdr,
                oppo_compat=args.oppo_compat,
            )
        else:
            heif_io.write_heic(
                str(output_path),
                base_image,
                gm_img,
                iso_meta,
                oppo_compat=args.oppo_compat,
                lhdr=lhdr,
                exif_data=exif_data,
            )

        if args.debug_dir:
            debug_dir = Path(args.debug_dir) / input_path.stem
            debug_dir.mkdir(parents=True, exist_ok=True)
            debug = {
                "input": str(input_path),
                "mode": lhdr.mode,
                "edr_scale": edr_scale,
                "iso_meta": iso_meta,
                "floats": list(lhdr.meta_floats),
            }
            (debug_dir / "meta.json").write_text(json.dumps(debug, indent=2))

    except ImportError as e:
        print(json.dumps({
            "mode": lhdr.mode,
            "edr_scale": edr_scale,
            "gainMapMax": iso_meta["gainMapMax"][0],
        }, indent=2))
        print(
            f"error: conversion requires pillow-heif + Pillow + numpy ({e}); "
            "no output was written",
            file=sys.stderr,
        )
        return 1

    verb = "overwritten" if output_path == input_path else f"-> {output_path}"
    print(f"converted {input_path.name} {verb}")
    return 0


def cmd_batch(args: argparse.Namespace) -> int:
    """Batch convert ProXDR HEIC files."""
    input_dir = Path(args.input_dir)

    if not input_dir.is_dir():
        print(f"error: input dir not found: {input_dir}", file=sys.stderr)
        return 1

    output_dir = Path(args.output_dir) if args.output_dir else input_dir
    if output_dir != input_dir:
        output_dir.mkdir(parents=True, exist_ok=True)

    glob_pattern = args.glob or "*.heic"
    files = sorted(input_dir.glob(glob_pattern))
    categorized_outputs = (
        categorize.batch_destinations(files, output_dir)
        if getattr(args, "categorize_output", False)
        else {}
    )
    converted, failed = 0, 0
    for f in files:
        out = categorized_outputs.get(f, output_dir / f.name)
        args2 = argparse.Namespace(input=str(f), output=str(out),
                                    debug_dir=args.debug_dir,
                                    oppo_compat=args.oppo_compat,
                                    passthrough=args.passthrough,
                                    reencode=args.reencode)
        ret = cmd_convert(args2)
        if ret == 0:
            converted += 1
        else:
            failed += 1

    print(f"batch complete: {converted} converted, {failed} failed")
    return 0 if failed == 0 else 1


def cmd_categorize(args: argparse.Namespace) -> int:
    """Copy photos into shooting-mode directories without modifying sources."""
    try:
        plan = categorize.make_plan([Path(value) for value in args.input], Path(args.output_dir))
    except (OSError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    results = categorize.execute_plan(plan, jobs=args.jobs, dry_run=args.dry_run)
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


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Convert OPPO ProXDR HEIC to ISO 21496-1 HDR HEIC",
    )
    sub = parser.add_subparsers(dest="command")

    c = sub.add_parser("convert")
    c.add_argument("--input", required=True)
    c.add_argument("--output")
    c.add_argument("--debug-dir")
    c.set_defaults(oppo_compat=False, passthrough=True, reencode=False)
    c.add_argument("--oppo-compatible", "--oppo-compat", action="store_true", dest="oppo_compat",
                   help="Add OPPO Gallery compatibility metadata")
    c.add_argument("--no-oppo-compat", action="store_false", dest="oppo_compat",
                   help=argparse.SUPPRESS)
    c.add_argument("--passthrough", action="store_true", dest="passthrough",
                   help=argparse.SUPPRESS)
    c.add_argument("--reencode", action="store_true", dest="reencode",
                   help="Decode and re-encode the base image instead of preserving source HEVC")

    b = sub.add_parser("batch")
    b.add_argument("--input-dir", required=True)
    b.add_argument("--output-dir")
    b.add_argument("--glob")
    b.add_argument("--debug-dir")
    b.set_defaults(oppo_compat=False, passthrough=True, reencode=False)
    b.add_argument("--oppo-compatible", "--oppo-compat", action="store_true", dest="oppo_compat",
                   help="Add OPPO Gallery compatibility metadata")
    b.add_argument("--no-oppo-compat", action="store_false", dest="oppo_compat",
                   help=argparse.SUPPRESS)
    b.add_argument("--passthrough", action="store_true", dest="passthrough",
                   help=argparse.SUPPRESS)
    b.add_argument("--reencode", action="store_true", dest="reencode",
                   help="Decode and re-encode the base image instead of preserving source HEVC")
    b.add_argument("--categorize", action="store_true", dest="categorize_output",
                   help="Write outputs under Chinese shooting-mode directories")

    category = sub.add_parser("categorize")
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
        return cmd_convert(args)
    elif args.command == "batch":
        return cmd_batch(args)
    elif args.command == "categorize":
        if args.jobs < 1:
            parser.error("--jobs must be greater than zero")
        return cmd_categorize(args)
    else:
        parser.print_help()
        return 1


if __name__ == "__main__":
    sys.exit(main())
