#!/usr/bin/env python3
"""Measure the canonical release CLI on representative real product workloads.

The benchmark intentionally measures end-to-end process behavior rather than
isolated helpers. It therefore includes source I/O, parsing, codec work,
publication, and (for Apple features) adapter process/framework overhead.

HDR cases name both the source family and the resolved product Gain Map layout.
This is deliberate: Standard LHDR is monochrome 4:0:0 while Standard UHDR is
RGB 4:4:4, and OPPO-compatible output is RGB 4:2:0 for both source families.
Those workloads are not interchangeable performance samples.

This script records evidence; it does not invent a regression budget. Commit a
baseline only after repeated runs on a stable runner demonstrate normal
variance, then add or update the separate comparison gate.
"""

from __future__ import annotations

import argparse
import json
import math
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass


ROOT = Path(__file__).resolve().parents[1]
TIME = Path("/usr/bin/time")
RSS_RE = re.compile(r"^\s*(\d+)\s+maximum resident set size\s*$", re.MULTILINE)
MIB = 1024 * 1024


@dataclass(frozen=True)
class Case:
    name: str
    inputs: tuple[Path, ...]
    args: tuple[str, ...]
    expected: tuple[str, ...]
    apple: bool = False
    source_family: str | None = None
    gain_map_channels: str | None = None
    gain_map_chroma: str | None = None
    codec_path: str | None = None

    @property
    def input_bytes(self) -> int:
        return sum(path.stat().st_size for path in self.inputs)


def percentile(values: list[float], fraction: float) -> float:
    if not values:
        raise ValueError("percentile requires at least one value")
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


def run_timed(command: list[str], env: dict[str, str]) -> tuple[float, int, str, str]:
    if not TIME.is_file():
        raise RuntimeError("/usr/bin/time is required for peak-RSS measurement")
    started = time.perf_counter()
    completed = subprocess.run(
        [str(TIME), "-l", *command],
        cwd=ROOT,
        env=env,
        text=True,
        capture_output=True,
        check=False,
    )
    elapsed = time.perf_counter() - started
    if completed.returncode != 0:
        raise RuntimeError(
            "command failed with exit code "
            f"{completed.returncode}: {' '.join(command)}\n"
            f"stdout:\n{completed.stdout}\nstderr:\n{completed.stderr}"
        )
    match = RSS_RE.search(completed.stderr)
    if match is None:
        raise RuntimeError(
            "could not parse maximum resident set size from /usr/bin/time output:\n"
            + completed.stderr
        )
    return elapsed, int(match.group(1)), completed.stdout, completed.stderr


def cases(cli: Path, apple_adapter: Path | None) -> list[Case]:
    del cli  # The CLI path belongs to execution, not workload identity.
    lhdr = ROOT / "fixtures/proxdr/oppo/find-x7-ultra/lhdr-v2-01.heic"
    uhdr = ROOT / "fixtures/proxdr/oppo/find-x9-ultra/uhdr-hr-01.heic"
    portrait = ROOT / "fixtures/proxdr/oppo/find-x9-ultra/uhdr-portrait-01.heic"
    uhdr2 = ROOT / "fixtures/proxdr/oppo/find-x9-ultra/uhdr-master-v1-01.heic"
    jpeg_motion = ROOT / "fixtures/motion-photo/samsung/jpeg-ultrahdr-01.jpg"
    heif_motion = ROOT / "fixtures/motion-photo/samsung/heif-ultrahdr-01.heic"
    required = (lhdr, uhdr, portrait, uhdr2, jpeg_motion, heif_motion)
    missing = [str(path) for path in required if not path.is_file()]
    if missing:
        raise RuntimeError(f"benchmark fixtures are missing: {missing}")

    result = [
        Case(
            "hdr-lhdr-mono400",
            (lhdr,),
            ("convert", "--input", str(lhdr), "--output", "{work}/output.heic"),
            ("output.heic",),
            source_family="lhdr",
            gain_map_channels="mono",
            gain_map_chroma="400",
            codec_path="portable-libheif",
        ),
        Case(
            "hdr-uhdr-rgb444",
            (uhdr,),
            ("convert", "--input", str(uhdr), "--output", "{work}/output.heic"),
            ("output.heic",),
            source_family="uhdr",
            gain_map_channels="rgb",
            gain_map_chroma="444",
            codec_path="portable-libheif",
        ),
        Case(
            "oppo-compatible-lhdr-rgb420",
            (lhdr,),
            (
                "convert",
                "--input",
                str(lhdr),
                "--output",
                "{work}/output.heic",
                "--oppo-compatible",
            ),
            ("output.heic",),
            source_family="lhdr",
            gain_map_channels="rgb",
            gain_map_chroma="420",
            codec_path="portable-libheif",
        ),
        Case(
            "oppo-compatible-uhdr-rgb420",
            (uhdr,),
            (
                "convert",
                "--input",
                str(uhdr),
                "--output",
                "{work}/output.heic",
                "--oppo-compatible",
            ),
            ("output.heic",),
            source_family="uhdr",
            gain_map_channels="rgb",
            gain_map_chroma="420",
            codec_path="portable-libheif",
        ),
        Case(
            "motion-jpeg",
            (jpeg_motion,),
            ("convert", "--input", str(jpeg_motion), "--output", "{work}/live.heic"),
            ("live.heic", "live.mov"),
        ),
        Case(
            "motion-heif",
            (heif_motion,),
            ("convert", "--input", str(heif_motion), "--output", "{work}/live.heic"),
            ("live.heic", "live.mov"),
        ),
        Case(
            "batch-3",
            (uhdr, portrait, uhdr2),
            (
                "batch",
                "--input",
                str(uhdr),
                "--input",
                str(portrait),
                "--input",
                str(uhdr2),
                "--output-dir",
                "{work}/batch",
                "--jobs",
                "3",
                "--json",
            ),
            ("batch",),
        ),
    ]
    if apple_adapter is not None:
        result.extend(
            [
                Case(
                    "apple-portrait",
                    (portrait,),
                    (
                        "convert",
                        "--input",
                        str(portrait),
                        "--output",
                        "{work}/portrait.heic",
                        "--apple-portrait",
                    ),
                    ("portrait.heic",),
                    apple=True,
                    source_family="uhdr",
                    codec_path="product-e2e-with-apple-adapter",
                ),
                Case(
                    "apple-styles",
                    (portrait,),
                    (
                        "convert",
                        "--input",
                        str(portrait),
                        "--output",
                        "{work}/styles.heic",
                        "--apple-styles",
                    ),
                    ("styles.heic",),
                    apple=True,
                    source_family="uhdr",
                    codec_path="product-e2e-with-apple-adapter",
                ),
            ]
        )
    return result


def assert_outputs(work: Path, expected: tuple[str, ...]) -> None:
    for relative in expected:
        path = work / relative
        if not path.exists():
            raise RuntimeError(f"benchmark command did not create {path}")
        if path.is_file() and path.stat().st_size == 0:
            raise RuntimeError(f"benchmark command created empty output {path}")
        if path.is_dir() and not any(path.iterdir()):
            raise RuntimeError(f"benchmark command created empty output directory {path}")


def measure_case(
    case: Case,
    cli: Path,
    env: dict[str, str],
    warmup: int,
    iterations: int,
) -> dict[str, object]:
    samples: list[dict[str, float | int]] = []
    total_runs = warmup + iterations
    for index in range(total_runs):
        with tempfile.TemporaryDirectory(prefix=f"xdremux-bench-{case.name}-") as raw_work:
            work = Path(raw_work)
            args = [argument.format(work=work) for argument in case.args]
            elapsed, rss_bytes, _, _ = run_timed([str(cli), *args], env)
            assert_outputs(work, case.expected)
        if index >= warmup:
            samples.append(
                {
                    "wall_seconds": elapsed,
                    "peak_rss_bytes": rss_bytes,
                }
            )

    wall = [float(sample["wall_seconds"]) for sample in samples]
    rss = [int(sample["peak_rss_bytes"]) for sample in samples]
    median_wall = statistics.median(wall)
    return {
        "name": case.name,
        "measurement_layer": "product-e2e",
        "apple": case.apple,
        "source_family": case.source_family,
        "gain_map_channels": case.gain_map_channels,
        "gain_map_chroma": case.gain_map_chroma,
        "codec_path": case.codec_path,
        "input_bytes": case.input_bytes,
        "iterations": iterations,
        "samples": samples,
        "median_wall_seconds": median_wall,
        "p95_wall_seconds": percentile(wall, 0.95),
        "median_peak_rss_bytes": int(statistics.median(rss)),
        "max_peak_rss_bytes": max(rss),
        "median_input_mib_per_second": (case.input_bytes / MIB) / median_wall,
    }


def git_head() -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
    )
    return completed.stdout.strip()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--apple-adapter", type=Path)
    parser.add_argument("--warmup", type=int, default=1)
    parser.add_argument("--iterations", type=int, default=5)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    cli = args.cli.resolve()
    if not cli.is_file():
        parser.error(f"--cli is not a file: {cli}")
    apple_adapter = args.apple_adapter.resolve() if args.apple_adapter else None
    if apple_adapter is not None and not apple_adapter.is_file():
        parser.error(f"--apple-adapter is not a file: {apple_adapter}")
    if args.warmup < 0 or args.iterations < 3:
        parser.error("--warmup must be non-negative and --iterations must be at least 3")

    env = os.environ.copy()
    if apple_adapter is not None:
        env["XDREMUX_APPLE_ADAPTER"] = str(apple_adapter)

    report: dict[str, object] = {
        "schema_version": 1,
        "measurement_layer": "product-e2e",
        "head": git_head(),
        "platform": platform.platform(),
        "machine": platform.machine(),
        "python": platform.python_version(),
        "cli": str(cli),
        "apple_adapter": str(apple_adapter) if apple_adapter else None,
        "warmup": args.warmup,
        "iterations": args.iterations,
        "cases": [],
    }
    measured = report["cases"]
    assert isinstance(measured, list)
    for case in cases(cli, apple_adapter):
        print(f"measuring {case.name}...", flush=True)
        result = measure_case(case, cli, env, args.warmup, args.iterations)
        measured.append(result)
        print(
            f"{case.name}: median={result['median_wall_seconds']:.3f}s "
            f"p95={result['p95_wall_seconds']:.3f}s "
            f"max_rss={result['max_peak_rss_bytes'] / MIB:.1f}MiB",
            flush=True,
        )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
