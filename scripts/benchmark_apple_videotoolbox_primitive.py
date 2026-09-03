#!/usr/bin/env python3
"""Measure the existing Apple VideoToolbox Main10 adapter primitive.

This is intentionally not compared as an alternative UHDR Gain Map encoder.
The current primitive belongs to the Photographic Styles Linear Thumbnail path
and is validated as Main10 / YUV 4:2:0 / 10-bit. Timings include one-shot helper
launch, RGB8->BGRA staging, VTCompressionSession setup, encode, and output file
writes. The adapter explicitly allows hardware acceleration and reports the
actual encoder selection as framework facts; software fallback remains valid.
A future persistent-helper benchmark can separate transport startup.
"""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
import statistics
import subprocess
import sys
import tempfile
import time
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_VERSION = 2


def percentile(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    if len(ordered) == 1:
        return ordered[0]
    position = fraction * (len(ordered) - 1)
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    weight = position - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def git_head() -> str:
    return subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
    ).strip()


def smooth_rgb(width: int, height: int) -> bytes:
    data = bytearray(width * height * 3)
    x_denominator = max(width - 1, 1)
    y_denominator = max(height - 1, 1)
    for y in range(height):
        y_component = y * 255 // y_denominator
        row = y * width * 3
        for x in range(width):
            x_component = x * 255 // x_denominator
            base = (x_component + y_component) // 2
            offset = row + x * 3
            data[offset] = base
            data[offset + 1] = (base + 11) & 0xFF
            data[offset + 2] = (base + 23) & 0xFF
    return bytes(data)


def parse_hvcc(payload: bytes) -> dict[str, int | str]:
    if len(payload) <= 18 or payload[0] != 1:
        raise RuntimeError("VideoToolbox hvcC is truncated or has an unsupported version")
    profile_idc = payload[1] & 0x1F
    chroma_idc = payload[16] & 0x03
    chroma = {0: "400", 1: "420", 2: "422", 3: "444"}[chroma_idc]
    luma_bit_depth = (payload[17] & 0x07) + 8
    chroma_bit_depth = (payload[18] & 0x07) + 8
    return {
        "general_profile_idc": profile_idc,
        "chroma": chroma,
        "luma_bit_depth": luma_bit_depth,
        "chroma_bit_depth": chroma_bit_depth,
    }


def run_once(
    adapter: Path,
    input_path: Path,
    annex_path: Path,
    hvcc_path: Path,
    width: int,
    height: int,
    quality: float,
) -> tuple[float, int, dict[str, int | str], dict[str, Any]]:
    request = {
        "schema_version": SCHEMA_VERSION,
        "operation": "videotoolbox-encode-main10",
        "input_path": str(input_path),
        "output_path": str(annex_path),
        "video_toolbox_main10": {
            "raw_width": width,
            "raw_height": height,
            "raw_bytes_per_row": width * 3,
            "quality": quality,
            "hvcc_path": str(hvcc_path),
        },
    }
    started = time.perf_counter()
    completed = subprocess.run(
        [str(adapter)],
        cwd=ROOT,
        input=json.dumps(request).encode("utf-8"),
        capture_output=True,
        check=False,
    )
    elapsed = time.perf_counter() - started
    if completed.returncode != 0:
        raise RuntimeError(
            f"Apple adapter exited {completed.returncode}: "
            f"{completed.stderr.decode('utf-8', errors='replace')}"
        )
    try:
        response = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError(f"Apple adapter returned invalid JSON: {error}") from error
    if response.get("schema_version") != SCHEMA_VERSION:
        raise RuntimeError(f"unexpected adapter schema: {response!r}")
    facts = response.get("video_toolbox_main10")
    if not isinstance(facts, dict):
        raise RuntimeError("Apple adapter omitted video_toolbox_main10 facts")
    if facts.get("width") != width or facts.get("height") != height:
        raise RuntimeError(f"Apple adapter changed raster geometry: {facts!r}")
    if facts.get("hardware_acceleration_allowed") is not True:
        raise RuntimeError(
            "VideoToolbox primitive did not preserve the hardware-acceleration preference"
        )
    hardware_used = facts.get("using_hardware_accelerated_encoder")
    if hardware_used is not None and not isinstance(hardware_used, bool):
        raise RuntimeError(f"invalid hardware encoder fact: {hardware_used!r}")
    encoder_id = facts.get("encoder_id")
    if encoder_id is not None and not isinstance(encoder_id, str):
        raise RuntimeError(f"invalid VideoToolbox encoder_id fact: {encoder_id!r}")

    annex = annex_path.read_bytes()
    hvcc = hvcc_path.read_bytes()
    if not annex or not hvcc:
        raise RuntimeError("Apple VideoToolbox primitive produced an empty resource")
    if facts.get("annex_b_length") != len(annex) or facts.get("hvcc_length") != len(hvcc):
        raise RuntimeError("Apple adapter resource lengths disagree with written files")
    profile = parse_hvcc(hvcc)
    if profile != {
        "general_profile_idc": 2,
        "chroma": "420",
        "luma_bit_depth": 10,
        "chroma_bit_depth": 10,
    }:
        raise RuntimeError(f"unexpected VideoToolbox Main10 storage profile: {profile!r}")
    encoder_facts = {
        "hardware_acceleration_allowed": True,
        "using_hardware_accelerated_encoder": hardware_used,
        "encoder_id": encoder_id,
    }
    return elapsed, len(annex) + len(hvcc), profile, encoder_facts


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--apple-adapter", type=Path, required=True)
    parser.add_argument("--width", type=int, default=1024)
    parser.add_argument("--height", type=int, default=768)
    parser.add_argument("--quality", type=float, default=0.85)
    parser.add_argument("--warmup", type=int, default=1)
    parser.add_argument("--iterations", type=int, default=5)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    adapter = args.apple_adapter.resolve()
    if not adapter.is_file():
        parser.error(f"--apple-adapter is not a file: {adapter}")
    if args.width <= 0 or args.height <= 0:
        parser.error("--width and --height must be positive")
    if not math.isfinite(args.quality) or not 0.0 <= args.quality <= 1.0:
        parser.error("--quality must be finite and within 0 through 1")
    if args.warmup < 0 or args.iterations < 3:
        parser.error("--warmup must be non-negative and --iterations must be at least 3")

    raster = smooth_rgb(args.width, args.height)
    samples: list[float] = []
    encoder_samples: list[dict[str, Any]] = []
    encoded_bytes = 0
    profile: dict[str, int | str] | None = None
    with tempfile.TemporaryDirectory(prefix="xdremux-vt-bench-") as raw_work:
        work = Path(raw_work)
        input_path = work / "input.rgb"
        input_path.write_bytes(raster)
        for index in range(args.warmup + args.iterations):
            annex_path = work / f"sample-{index}.hevc"
            hvcc_path = work / f"sample-{index}.hvcc"
            elapsed, encoded_bytes, observed_profile, encoder_facts = run_once(
                adapter,
                input_path,
                annex_path,
                hvcc_path,
                args.width,
                args.height,
                args.quality,
            )
            profile = observed_profile
            if index >= args.warmup:
                samples.append(elapsed)
                encoder_samples.append(encoder_facts)

    assert profile is not None
    observed_encoder_ids = sorted(
        {
            sample["encoder_id"]
            for sample in encoder_samples
            if isinstance(sample.get("encoder_id"), str)
        }
    )
    hardware_sample_count = sum(
        sample.get("using_hardware_accelerated_encoder") is True
        for sample in encoder_samples
    )
    unknown_hardware_sample_count = sum(
        sample.get("using_hardware_accelerated_encoder") is None
        for sample in encoder_samples
    )
    report = {
        "schema_version": 1,
        "measurement_layer": "apple-adapter-primitive-e2e",
        "head": git_head(),
        "operation": "videotoolbox-encode-main10",
        "contract": "photographic-styles-linear-thumbnail",
        "hardware_acceleration_allowed": True,
        "hardware_accelerated_samples": hardware_sample_count,
        "unknown_hardware_acceleration_samples": unknown_hardware_sample_count,
        "observed_encoder_ids": observed_encoder_ids,
        "encoder_samples": encoder_samples,
        "includes_helper_launch": True,
        "includes_rgb_to_bgra_staging": True,
        "includes_session_creation": True,
        "includes_output_file_io": True,
        "width": args.width,
        "height": args.height,
        "input_format": "rgb8",
        "raw_raster_bytes": len(raster),
        "quality": args.quality,
        "warmup": args.warmup,
        "iterations": args.iterations,
        "storage_profile": profile,
        "encoded_resource_bytes": encoded_bytes,
        "samples_seconds": samples,
        "median_seconds": statistics.median(samples),
        "p95_seconds": percentile(samples, 0.95),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(
        f"VideoToolbox Main10 adapter primitive: median={report['median_seconds']:.3f}s "
        f"p95={report['p95_seconds']:.3f}s profile={profile} "
        f"hardware_samples={hardware_sample_count}/{args.iterations} "
        f"encoder_ids={observed_encoder_ids}",
        flush=True,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
