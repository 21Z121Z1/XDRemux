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
import re
import statistics
import sys
import uuid
from typing import Any


MIB = 1024 * 1024

# Workload identity is owned by the benchmark, never by the measured report.
# Keep these contracts aligned with benchmark_rust_product.cases.
CASE_CONTRACTS = {
    "hdr-lhdr-mono400": (1, "iso-hdr-heif", ("output.heic",)),
    "hdr-uhdr-rgb444": (1, "iso-hdr-heif", ("output.heic",)),
    "oppo-compatible-lhdr-rgb420": (1, "iso-hdr-heif", ("output.heic",)),
    "oppo-compatible-uhdr-rgb420": (1, "iso-hdr-heif", ("output.heic",)),
    "motion-jpeg": (1, "live-photo", ("live.heic", "live.mov")),
    "motion-heif": (1, "live-photo", ("live.heic", "live.mov")),
    "batch-3": (3, "iso-hdr-heif", None),
    "apple-portrait": (1, "iso-hdr-heif", ("portrait.heic",)),
    "apple-styles": (1, "iso-hdr-heif", ("styles.heic",)),
}


def load_object(path: Path, label: str, version: int = 1) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise RuntimeError(f"cannot read {label} {path}: {error}") from error
    if not isinstance(value, dict):
        raise RuntimeError(f"{label} must be a JSON object")
    if value.get("schema_version") != version:
        if label == "current benchmark report":
            raise RuntimeError("current benchmark report requires schema_version 2")
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


def identity(value: Any, context: str) -> None:
    if not isinstance(value, dict):
        raise RuntimeError(f"{context} identity is missing")
    if integer_count(value.get("bytes"), f"{context} bytes") == 0:
        raise RuntimeError(f"{context} bytes must be positive")
    if not isinstance(value.get("sha256"), str) or not re.fullmatch(r"[0-9a-f]{64}", value["sha256"]):
        raise RuntimeError(f"{context} sha256 must identify the actual bytes")


def canonical_output(output: dict[str, Any], kind: str, name: str) -> str:
    report = output.get("report")
    if (not isinstance(report, dict) or report.get("command") != "validate"
            or type(report.get("schema_version")) is not int or report["schema_version"] != 1
            or report.get("valid") is not True or report.get("kind") != kind):
        raise RuntimeError(f"{name}: canonical validation receipt is missing or contradicts output")
    details = report.get("details")
    if not isinstance(details, dict) or details.get("kind") != kind:
        raise RuntimeError(f"{name}: canonical validation details disagree")
    value = details.get("value")
    path = value.get("input") if isinstance(value, dict) else None
    relative = Path(output["relative_path"])
    if (not isinstance(path, str) or not Path(path).is_absolute()
            or ".." in Path(path).parts or relative.is_absolute() or ".." in relative.parts
            or Path(path).parts[-len(relative.parts):] != relative.parts):
        raise RuntimeError(f"{name}: canonical validation input does not bind the output")
    if kind == "live-photo":
        still = Path(path).with_suffix(".heic")
        if (value.get("image") != str(still) or value.get("video") != str(still.with_suffix(".mov"))
                or not isinstance(value.get("content_identifier"), str)
                or not value["content_identifier"].strip()):
            raise RuntimeError(f"{name}: canonical Live Photo member roles disagree")
    return path


def batch_receipt(validation: dict[str, Any], inputs: list[dict[str, Any]],
                  paths: set[str], expected: int, name: str) -> None:
    receipt = validation.get("batch_receipt")
    if not isinstance(receipt, dict) or receipt.get("command") != "batch":
        raise RuntimeError(f"{name}: batch receipt is missing")
    for key, count in (("processed", expected), ("succeeded", expected),
                       ("failed", 0), ("skipped_existing", 0)):
        if integer_count(receipt.get(key), f"{name} batch {key}") != count:
            raise RuntimeError(f"{name}: batch receipt workload disagrees")
    successes = receipt.get("successes")
    if (not isinstance(successes, list) or len(successes) != expected
            or receipt.get("failures") != []):
        raise RuntimeError(f"{name}: batch receipt is incomplete")
    sources, outputs = set(), set()
    for item in successes:
        if not isinstance(item, dict) or item.get("status") != "converted":
            raise RuntimeError(f"{name}: batch receipt contains unconverted work")
        source, members = item.get("input"), item.get("outputs")
        if (not isinstance(source, str) or source in sources
                or not isinstance(members, list) or len(members) != 1
                or not isinstance(members[0], str) or members[0] in outputs):
            raise RuntimeError(f"{name}: batch receipt contains missing or duplicate work")
        sources.add(source)
        outputs.add(members[0])
    if sources != {item["path"] for item in inputs} or outputs != paths:
        raise RuntimeError(f"{name}: batch receipt does not bind all inputs and validated outputs")


def validated_samples(report: dict[str, Any], cases: dict[str, dict[str, Any]]) -> None:
    if report.get("status") != "complete" or report.get("errors") != []:
        raise RuntimeError("current benchmark report is incomplete or contains failures")
    iterations = integer_count(report.get("iterations"), "iterations")
    warmup = integer_count(report.get("warmup"), "warmup")
    if iterations < 5:
        raise RuntimeError("current benchmark report requires at least five completed measurements")
    identity(report.get("cli_identity"), "CLI")
    identity(report.get("cargo_lock_identity"), "Cargo.lock")
    if "adapter_identity" not in report:
        raise RuntimeError("current benchmark adapter identity is missing")
    if report.get("apple_adapter") is not None or any(case.get("apple") for case in cases.values()):
        identity(report["adapter_identity"], "Apple adapter")
    elif report["adapter_identity"] is not None:
        raise RuntimeError("Apple adapter identity has no measured adapter")
    execution = report.get("execution")
    if not isinstance(execution, dict):
        raise RuntimeError("current benchmark execution identity is missing")
    try:
        uuid.UUID(execution.get("id", ""))
    except (ValueError, TypeError, AttributeError) as error:
        raise RuntimeError("current benchmark execution id must be a UUID") from error
    if not isinstance(execution.get("host"), str) or not execution["host"].strip():
        raise RuntimeError("current benchmark execution host is missing")
    if not integer_count(execution.get("logical_cpus"), "execution logical_cpus"):
        raise RuntimeError("execution logical_cpus must be positive")
    for key in ("run_id", "run_attempt", "job", "runner"):
        if key not in execution or (execution[key] is not None
                and (not isinstance(execution[key], str) or not execution[key].strip())):
            raise RuntimeError(f"execution {key} must record its value or explicit null")
    source = report.get("source")
    if (not isinstance(source, dict) or source.get("head") != report.get("head")
            or source.get("tracked_tree_clean") is not True
            or not isinstance(source.get("tree"), str) or not re.fullmatch(r"[0-9a-f]{40}", source["tree"])):
        raise RuntimeError("current benchmark source must identify a clean exact-head tree")
    for name, case in cases.items():
        if case.get("status") != "complete" or case.get("errors") != []:
            raise RuntimeError(f"{name}: measurement is incomplete or contains failures")
        if (integer_count(case.get("iterations"), f"{name} iterations") != iterations
                or integer_count(case.get("warmup"), f"{name} warmup") != warmup
                or integer_count(case.get("warmup_completed"), f"{name} warmup_completed") != warmup):
            raise RuntimeError(f"{name}: declared run counts disagree")
        contract = CASE_CONTRACTS.get(name)
        if contract is None:
            raise RuntimeError(f"{name}: unknown workload contract")
        expected, kind, members = contract
        if integer_count(case.get("expected_work"), f"{name} expected_work") != expected:
            raise RuntimeError(f"{name}: expected work differs from fixed workload contract")
        inputs = case.get("inputs")
        if not expected or not isinstance(inputs, list) or len(inputs) != expected:
            raise RuntimeError(f"{name}: input count differs from expected work")
        input_paths = set()
        for item in inputs:
            identity(item, f"{name} input")
            path = item.get("path")
            if not isinstance(path, str) or not path or path in input_paths:
                raise RuntimeError(f"{name}: input identity is missing or duplicated")
            input_paths.add(path)
        samples = case.get("samples")
        if not isinstance(samples, list) or len(samples) != iterations:
            raise RuntimeError(f"{name}: actual sample count differs from declared measurements")
        wall, rss = [], []
        for index, sample in enumerate(samples):
            if not isinstance(sample, dict):
                raise RuntimeError(f"{name}: sample must be an object")
            if (type(sample.get("index")) is not int or sample["index"] != index
                    or sample.get("status") != "validated"
                    or type(sample.get("process_return_code")) is not int
                    or sample["process_return_code"] != 0):
                raise RuntimeError(f"{name}: sample {index} did not complete successfully")
            elapsed = finite_number(sample.get("wall_seconds"), f"{name} sample wall_seconds")
            if elapsed == 0:
                raise RuntimeError(f"{name}: sample wall_seconds must be positive")
            wall.append(elapsed)
            rss.append(integer_count(sample.get("peak_rss_bytes"), f"{name} sample peak_rss_bytes"))
            validation = sample.get("validation")
            if not isinstance(validation, dict):
                raise RuntimeError(f"{name}: sample validation is missing")
            for key, required in (("expected_work", expected), ("completed_work", expected),
                                  ("failed_work", 0), ("skipped_work", 0)):
                observed = integer_count(validation.get(key), f"{name} {key}")
                if observed != required:
                    raise RuntimeError(f"{name}: {key} differs from completed workload")
            outputs = validation.get("outputs")
            if not isinstance(outputs, list) or not outputs or any(not isinstance(item, dict) for item in outputs):
                raise RuntimeError(f"{name}: output validation is incomplete")
            if any(item.get("kind") != kind for item in outputs):
                raise RuntimeError(f"{name}: output contract requires {kind}")
            if len(outputs) != expected * (2 if kind == "live-photo" else 1):
                raise RuntimeError(f"{name}: output validation is incomplete")
            seen, canonical_paths, work_roots = set(), set(), set()
            pair_identifiers = set()
            for output in outputs:
                identity(output, f"{name} output")
                path = output.get("relative_path")
                if not isinstance(path, str) or not path or path in seen:
                    raise RuntimeError(f"{name}: output identity is missing or duplicated")
                seen.add(path)
                if output.get("valid") is not True or output.get("kind") not in ("iso-hdr-heif", "live-photo"):
                    raise RuntimeError(f"{name}: canonical output validation did not pass")
                canonical = canonical_output(output, kind, name)
                canonical_paths.add(canonical)
                work_roots.add(str(Path(canonical).parents[len(Path(path).parts) - 1]))
                if kind == "live-photo":
                    pair_identifiers.add(output["report"]["details"]["value"]["content_identifier"])
            if len(work_roots) != 1 or len(canonical_paths) != len(outputs):
                raise RuntimeError(f"{name}: canonical outputs do not share one owned work directory")
            if members is not None and seen != set(members):
                raise RuntimeError(f"{name}: output contract member roles disagree")
            if name == "batch-3":
                if any(Path(path).parent != Path("batch") or Path(path).suffix != ".heic" for path in seen):
                    raise RuntimeError(f"{name}: batch output contract member roles disagree")
                batch_receipt(validation, inputs, canonical_paths, expected, name)
            if kind == "live-photo":
                if len(pair_identifiers) != 1:
                    raise RuntimeError(f"{name}: canonical Live Photo pair identifiers disagree")
                stills = {Path(path) for path in seen if Path(path).suffix.lower() in (".heic", ".heif")}
                movies = {Path(path) for path in seen if Path(path).suffix == ".mov"}
                if len(stills) != expected or movies != {path.with_suffix(".mov") for path in stills}:
                    raise RuntimeError(f"{name}: Live Photo member identities do not form complete pairs")
        expected_summary = {
            "median_wall_seconds": statistics.median(wall),
            "p95_wall_seconds": statistics.quantiles(wall, n=100, method="inclusive")[94],
            "median_peak_rss_bytes": int(statistics.median(rss)),
            "max_peak_rss_bytes": max(rss),
        }
        for metric, expected_value in expected_summary.items():
            observed = finite_number(case.get(metric), f"{name} {metric}")
            if not math.isclose(observed, expected_value, rel_tol=1e-12, abs_tol=1e-9):
                raise RuntimeError(f"{name}: {metric} disagrees with samples")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--benchmark", type=Path, required=True)
    parser.add_argument("--launches", type=Path, required=True)
    parser.add_argument("--expected-head", required=True)
    parser.add_argument("--historical-replay", action="store_true",
                        help="Read an archived v1 run under its original summary contract; not current CI evidence")
    args = parser.parse_args()

    baseline = load_object(args.baseline, "performance baseline")
    benchmark_report = load_object(args.benchmark,
                                   "historical benchmark report" if args.historical_replay else "current benchmark report",
                                   1 if args.historical_replay else 2)
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
    if args.historical_replay:
        print("historical replay: schema-v1 summaries retain their original budget contract")
    else:
        validated_samples(benchmark_report, current_cases)

    failures: list[str] = []
    print("product performance regression budget:")
    for name, reference_raw in reference_cases.items():
        if not isinstance(name, str) or not isinstance(reference_raw, dict):
            raise RuntimeError("performance baseline case entries are malformed")
        current = current_cases.get(name)
        if current is None:
            failures.append(f"missing benchmark case {name}")
            continue

        case_wall_fraction = finite_number(
            reference_raw.get("wall_fraction", wall_fraction), f"{name} wall_fraction"
        )
        checks = (
            (
                "median_wall_seconds",
                finite_number(reference_raw.get("median_wall_seconds"), f"{name} median reference"),
                finite_number(current.get("median_wall_seconds"), f"{name} median current"),
                case_wall_fraction,
                wall_absolute,
                "s",
            ),
            (
                "p95_wall_seconds",
                finite_number(reference_raw.get("p95_wall_seconds"), f"{name} p95 reference"),
                finite_number(current.get("p95_wall_seconds"), f"{name} p95 current"),
                case_wall_fraction,
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
