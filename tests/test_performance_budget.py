from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/performance/check_performance_budget.py"
MIB = 1024 * 1024
HEAD = "0123456789abcdef"


def write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value), encoding="utf-8")


def baseline() -> dict[str, object]:
    return {
        "schema_version": 1,
        "tolerances": {
            "wall_fraction": 0.35,
            "wall_absolute_seconds": 0.25,
            "rss_fraction": 0.25,
            "rss_absolute_mib": 32.0,
        },
        "cases": {
            "tiny": {
                "median_wall_seconds": 0.10,
                "p95_wall_seconds": 0.12,
                "max_peak_rss_mib": 16.0,
            },
            "large": {
                "median_wall_seconds": 10.0,
                "p95_wall_seconds": 12.0,
                "max_peak_rss_mib": 100.0,
            },
        },
        "apple_adapter_process_launches": {
            "apple-portrait": 1,
            "apple-styles": 1,
        },
    }


def measured_case(name: str, median: float, p95: float, rss_mib: int) -> dict[str, object]:
    samples = []
    for index, wall in enumerate([median] * 4 + [median + (p95 - median) / 0.8]):
        samples.append({
            "index": index,
            "status": "validated",
            "process_return_code": 0,
            "wall_seconds": wall,
            "peak_rss_bytes": rss_mib * MIB,
            "validation": {
                "expected_work": 1, "completed_work": 1, "failed_work": 0, "skipped_work": 0,
                "outputs": [{"relative_path": "output.heic", "bytes": 100,
                             "sha256": "b" * 64, "kind": "iso-hdr-heif", "valid": True}],
            },
        })
    return {
        "name": name, "status": "complete", "expected_work": 1,
        "iterations": 5, "warmup": 1, "warmup_completed": 1, "errors": [],
        "inputs": [{"path": "source.heic", "bytes": 1000, "sha256": "a" * 64}],
        "samples": samples, "median_wall_seconds": median, "p95_wall_seconds": p95,
        "median_peak_rss_bytes": rss_mib * MIB, "max_peak_rss_bytes": rss_mib * MIB,
    }


def benchmark(*, tiny_wall: float = 0.30, large_wall: float = 13.0,
              large_p95: float | None = None) -> dict[str, object]:
    return {
        "schema_version": 2,
        "head": HEAD,
        "machine": "arm64",
        "status": "complete", "errors": [], "iterations": 5, "warmup": 1,
        "cli_identity": {"bytes": 100, "sha256": "c" * 64},
        "cargo_lock_identity": {"bytes": 100, "sha256": "e" * 64},
        "apple_adapter": None, "adapter_identity": None,
        "execution": {"id": "769b7995-2658-4b77-8783-fbb230145fd5", "host": "test-host",
                      "logical_cpus": 4, "run_id": None, "run_attempt": None,
                      "job": None, "runner": None},
        "source": {"head": HEAD, "tree": "d" * 40, "tracked_tree_clean": True},
        "cases": [
            measured_case("tiny", tiny_wall, max(0.31, tiny_wall + 0.01), 40),
            measured_case("large", large_wall, large_p95 or max(15.0, large_wall + 2), 124),
        ],
    }


def launches(*, portrait: int = 1, styles: int = 1, head: str = HEAD) -> dict[str, object]:
    return {
        "schema_version": 1,
        "head": head,
        "metric": "apple_adapter_process_launches_per_conversion",
        "cases": {
            "apple-portrait": portrait,
            "apple-styles": styles,
        },
    }


class PerformanceBudgetTests(unittest.TestCase):
    def run_checker(
        self,
        benchmark_value: dict[str, object],
        launch_value: dict[str, object],
        baseline_value: dict[str, object] | None = None,
        *, historical: bool = False,
    ) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory(prefix="xdremux-perf-budget-") as raw:
            work = Path(raw)
            baseline_path = work / "baseline.json"
            benchmark_path = work / "benchmark.json"
            launches_path = work / "launches.json"
            write_json(baseline_path, baseline() if baseline_value is None else baseline_value)
            write_json(benchmark_path, benchmark_value)
            write_json(launches_path, launch_value)
            return subprocess.run(
                [
                    sys.executable,
                    str(CHECKER),
                    "--baseline",
                    str(baseline_path),
                    "--benchmark",
                    str(benchmark_path),
                    "--launches",
                    str(launches_path),
                    "--expected-head",
                    HEAD,
                    *(["--historical-replay"] if historical else []),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
                check=False,
            )

    def test_accepts_metrics_inside_relative_and_absolute_headroom(self) -> None:
        completed = self.run_checker(benchmark(), launches())
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertIn("performance regression gate passed", completed.stdout)

    def test_rejects_wall_time_regression_beyond_budget(self) -> None:
        completed = self.run_checker(benchmark(large_wall=14.0), launches())
        self.assertEqual(completed.returncode, 1)
        self.assertIn("large median_wall_seconds regressed", completed.stderr)

    def test_case_wall_budget_accepts_the_expanded_workload(self) -> None:
        reference = baseline()
        reference["cases"]["large"]["wall_fraction"] = 1.0
        completed = self.run_checker(benchmark(large_wall=18.0), launches(), reference)
        self.assertEqual(completed.returncode, 0, completed.stderr)

    def test_case_wall_budget_keeps_other_workloads_bounded(self) -> None:
        reference = baseline()
        reference["cases"]["large"]["wall_fraction"] = 1.0
        completed = self.run_checker(benchmark(tiny_wall=0.40), launches(), reference)
        self.assertEqual(completed.returncode, 1)
        self.assertIn("tiny median_wall_seconds regressed", completed.stderr)

    def test_case_wall_budget_keeps_median_and_p95_bounded(self) -> None:
        reference = baseline()
        reference["cases"]["large"]["wall_fraction"] = 1.0
        current = benchmark(large_wall=21.0, large_p95=25.0)
        completed = self.run_checker(current, launches(), reference)
        self.assertEqual(completed.returncode, 1)
        self.assertIn("large median_wall_seconds regressed", completed.stderr)
        self.assertIn("large p95_wall_seconds regressed", completed.stderr)

    def test_case_wall_budget_keeps_rss_and_helper_launch_limits(self) -> None:
        reference = baseline()
        reference["cases"]["large"]["wall_fraction"] = 1.0
        current = benchmark()
        current["cases"][1]["max_peak_rss_bytes"] = 140 * MIB
        for sample in current["cases"][1]["samples"]:
            sample["peak_rss_bytes"] = 140 * MIB
        current["cases"][1]["median_peak_rss_bytes"] = 140 * MIB
        completed = self.run_checker(current, launches(styles=2), reference)
        self.assertEqual(completed.returncode, 1)
        self.assertIn("large max_peak_rss_mib regressed", completed.stderr)
        self.assertIn("apple-styles helper launches regressed", completed.stderr)

    def test_rejects_invalid_case_wall_budget(self) -> None:
        for invalid in (True, -1, "1.0", float("inf")):
            with self.subTest(invalid=invalid):
                reference = baseline()
                reference["cases"]["large"]["wall_fraction"] = invalid
                completed = self.run_checker(benchmark(), launches(), reference)
                self.assertEqual(completed.returncode, 2)
                self.assertIn("large wall_fraction", completed.stderr)

    def test_rejects_structural_helper_launch_regression(self) -> None:
        completed = self.run_checker(benchmark(), launches(portrait=2))
        self.assertEqual(completed.returncode, 1)
        self.assertIn("apple-portrait helper launches regressed", completed.stderr)

    def test_rejects_non_exact_head_evidence(self) -> None:
        completed = self.run_checker(benchmark(), launches(head="wrong-head"))
        self.assertEqual(completed.returncode, 2)
        self.assertIn("performance evidence is not exact-head", completed.stderr)

    def test_current_entry_rejects_summary_only_schema_downgrade(self) -> None:
        current = benchmark()
        current["schema_version"] = 1
        for case in current["cases"]:
            case.pop("samples")
        result = self.run_checker(current, launches())
        self.assertEqual(result.returncode, 2)
        self.assertIn("current benchmark report requires schema_version 2", result.stderr)

    def test_historical_replay_preserves_original_summary_budget_verdicts(self) -> None:
        for wall, expected in [(13.0, 0), (14.0, 1)]:
            current = benchmark(large_wall=wall)
            current["schema_version"] = 1
            for case in current["cases"]:
                case.pop("samples")
            result = self.run_checker(current, launches(), historical=True)
            self.assertEqual(result.returncode, expected, result.stderr)
            self.assertIn("historical replay", result.stdout)

    def test_rejects_missing_observations_even_when_summaries_pass(self) -> None:
        current = benchmark()
        current["cases"][0]["samples"] = []
        result = self.run_checker(current, launches())
        self.assertEqual(result.returncode, 2)
        self.assertIn("sample count", result.stderr)

    def test_rejects_fewer_than_five_completed_measurements(self) -> None:
        current = benchmark()
        current["iterations"] = 4
        current["cases"][0]["iterations"] = 4
        current["cases"][0]["samples"].pop()
        result = self.run_checker(current, launches())
        self.assertEqual(result.returncode, 2)

    def test_rejects_summary_that_disagrees_with_observed_work(self) -> None:
        for field, value in [("median_wall_seconds", 0.01), ("p95_wall_seconds", 0.01),
                             ("max_peak_rss_bytes", 1)]:
            with self.subTest(field=field):
                current = benchmark()
                current["cases"][0][field] = value
                result = self.run_checker(current, launches())
                self.assertEqual(result.returncode, 2)
                self.assertIn("disagrees with samples", result.stderr)

    def test_rejects_unfinished_failed_or_unvalidated_work(self) -> None:
        for field, value in [("status", "running"), ("process_return_code", 1)]:
            with self.subTest(field=field):
                current = benchmark()
                current["cases"][0]["samples"][0][field] = value
                result = self.run_checker(current, launches())
                self.assertEqual(result.returncode, 2)
        for field, value in [("completed_work", 0), ("failed_work", 1), ("skipped_work", 1)]:
            with self.subTest(field=field):
                current = benchmark()
                current["cases"][0]["samples"][0]["validation"][field] = value
                result = self.run_checker(current, launches())
                self.assertEqual(result.returncode, 2)

    def test_rejects_failed_or_missing_output_validation(self) -> None:
        current = benchmark()
        current["cases"][0]["samples"][0]["validation"]["outputs"][0]["valid"] = False
        result = self.run_checker(current, launches())
        self.assertEqual(result.returncode, 2)
        current = benchmark()
        current["cases"][0]["samples"][0]["validation"]["outputs"] = []
        self.assertEqual(self.run_checker(current, launches()).returncode, 2)

    def test_rejects_partial_run_with_retained_failure(self) -> None:
        current = benchmark()
        current["status"] = "failed"
        current["errors"] = [{"case": "large", "error": "validation failed"}]
        result = self.run_checker(current, launches())
        self.assertEqual(result.returncode, 2)

    def test_rejects_missing_dependency_and_execution_identity(self) -> None:
        for field in ("cargo_lock_identity", "adapter_identity", "execution"):
            with self.subTest(field=field):
                current = benchmark()
                current.pop(field)
                self.assertEqual(self.run_checker(current, launches()).returncode, 2)
        for field, invalid in (("id", ""), ("host", ""), ("logical_cpus", 0),
                               ("run_attempt", 1)):
            with self.subTest(field=field):
                current = benchmark()
                current["execution"][field] = invalid
                self.assertEqual(self.run_checker(current, launches()).returncode, 2)
        current = benchmark()
        current["cases"][0]["apple"] = True
        self.assertEqual(self.run_checker(current, launches()).returncode, 2)

    def test_live_photo_requires_both_member_identities(self) -> None:
        current = benchmark()
        for sample in current["cases"][0]["samples"]:
            output = sample["validation"]["outputs"][0]
            output.update(relative_path="live.heic", kind="live-photo")
            sample["validation"]["outputs"].append({**output, "relative_path": "live.mov"})
        self.assertEqual(self.run_checker(current, launches()).returncode, 0)
        for sample in current["cases"][0]["samples"]:
            sample["validation"]["outputs"].pop()
        self.assertEqual(self.run_checker(current, launches()).returncode, 2)


if __name__ == "__main__":
    unittest.main()
