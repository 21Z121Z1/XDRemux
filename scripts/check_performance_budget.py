#!/usr/bin/env python3
"""Fail closed when end-to-end product performance regresses beyond runner noise.

The committed reference is deliberately a regression baseline, not a target.
Budgets combine relative and absolute headroom so tiny workloads are not made
flaky by scheduler noise. Apple helper launch counts are structural and therefore
use hard maxima rather than statistical tolerance.
"""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
import sys
from typing import Any


MIB = 1024 * 1024


def load_object(path: Path, label: str) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise RuntimeError(f"cannot read {label} {path}: {error}") from error
    if not isinstance(value, dict):
        raise RuntimeError(f"{label} must be a JSON object")
    if value.get("schema_version") != 1:
        raise RuntimeError(f"unsupported {label} schema_version {value.get('schema_version')!r}")
    return value


def finite_number(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise RuntimeError(f"{context} must be numeric")
    result = float(value)
    if not math.isfinite(result) or result < 0:
        raise RuntimeError(f"{context} must be finite and non-negative")
    return result


def integer_count(value: Any, context: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise RuntimeError(f"{context} must be a non-negative integer")
    return value


def budget(reference: float, fraction: float, absolute: float) -> float:
    return max(reference * (1.0 + fraction), reference + absolute)


def benchmark_cases(report: dict[str, Any]) -> dict[str, dict[str, Any]]:
    raw_cases = report.get("cases")
    if not isinstance(raw_cases, list):
        raise RuntimeError("benchmark cases must be an array")
    indexed: dict[str, dict[str, Any]] = {}
    for raw in raw_cases:
        if not isinstance(raw, dict) or not isinstance(raw.get("name"), str):
            raise RuntimeError("every benchmark case must be an object with a string name")
        name = raw["name"]
        if name in indexed:
            raise RuntimeError(f"benchmark contains duplicate case {name!r}")
        indexed[name] = raw
    return indexed


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--benchmark", type=Path, required=True)
    parser.add_argument("--launches", type=Path, required=True)
    parser.add_argument("--expected-head", required=True)
    args = parser.parse_args()

    baseline = load_object(args.baseline, "performance baseline")
    benchmark_report = load_object(args.benchmark, "benchmark report")
    launch_report = load_object(args.launches, "Apple launch report")

    benchmark_head = benchmark_report.get("head")
    launch_head = launch_report.get("head")
    if benchmark_head != args.expected_head or launch_head != args.expected_head:
        raise RuntimeError(
            "performance evidence is not exact-head: "
            f"expected {args.expected_head}, benchmark={benchmark_head!r}, launches={launch_head!r}"
        )
    if benchmark_report.get("machine") != "arm64":
        raise RuntimeError(
            f"performance budget is scoped to arm64, got {benchmark_report.get('machine')!r}"
        )
    if launch_report.get("metric") != "apple_adapter_process_launches_per_conversion":
        raise RuntimeError("Apple launch report has an unexpected metric")

    tolerances = baseline.get("tolerances")
    if not isinstance(tolerances, dict):
        raise RuntimeError("performance baseline tolerances must be an object")
    wall_fraction = finite_number(tolerances.get("wall_fraction"), "wall_fraction")
    wall_absolute = finite_number(
        tolerances.get("wall_absolute_seconds"), "wall_absolute_seconds"
    )
    rss_fraction = finite_number(tolerances.get("rss_fraction"), "rss_fraction")
    rss_absolute = finite_number(tolerances.get("rss_absolute_mib"), "rss_absolute_mib")

    reference_cases = baseline.get("cases")
    if not isinstance(reference_cases, dict) or not reference_cases:
        raise RuntimeError("performance baseline cases must be a non-empty object")
    current_cases = benchmark_cases(benchmark_report)

    failures: list[str] = []
    print("product performance regression budget:")
    for name, reference_raw in reference_cases.items():
        if not isinstance(name, str) or not isinstance(reference_raw, dict):
            raise RuntimeError("performance baseline case entries are malformed")
        current = current_cases.get(name)
        if current is None:
            failures.append(f"missing benchmark case {name}")
            continue

        checks = (
            (
                "median_wall_seconds",
                finite_number(reference_raw.get("median_wall_seconds"), f"{name} median reference"),
                finite_number(current.get("median_wall_seconds"), f"{name} median current"),
                wall_fraction,
                wall_absolute,
                "s",
            ),
            (
                "p95_wall_seconds",
                finite_number(reference_raw.get("p95_wall_seconds"), f"{name} p95 reference"),
                finite_number(current.get("p95_wall_seconds"), f"{name} p95 current"),
                wall_fraction,
                wall_absolute,
                "s",
            ),
            (
                "max_peak_rss_mib",
                finite_number(reference_raw.get("max_peak_rss_mib"), f"{name} RSS reference"),
                finite_number(current.get("max_peak_rss_bytes"), f"{name} RSS current") / MIB,
                rss_fraction,
                rss_absolute,
                "MiB",
            ),
        )
        for metric, reference, observed, fraction, absolute, unit in checks:
            limit = budget(reference, fraction, absolute)
            print(
                f"  {name} {metric}: observed={observed:.3f}{unit} "
                f"reference={reference:.3f}{unit} budget={limit:.3f}{unit}"
            )
            if observed > limit:
                failures.append(
                    f"{name} {metric} regressed: {observed:.3f}{unit} > {limit:.3f}{unit}"
                )

    reference_launches = baseline.get("apple_adapter_process_launches")
    current_launches = launch_report.get("cases")
    if not isinstance(reference_launches, dict) or not isinstance(current_launches, dict):
        raise RuntimeError("Apple launch-count baseline/report must contain case objects")
    print("Apple adapter structural launch budget:")
    for name, raw_limit in reference_launches.items():
        limit = integer_count(raw_limit, f"{name} launch limit")
        if name not in current_launches:
            failures.append(f"missing Apple launch-count case {name}")
            continue
        observed = integer_count(current_launches[name], f"{name} launch count")
        print(f"  {name}: observed={observed} max={limit}")
        if observed > limit:
            failures.append(f"{name} helper launches regressed: {observed} > {limit}")

    if failures:
        print("performance regression gate failed:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1

    print("performance regression gate passed")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except RuntimeError as error:
        print(f"performance regression gate error: {error}", file=sys.stderr)
        sys.exit(2)
