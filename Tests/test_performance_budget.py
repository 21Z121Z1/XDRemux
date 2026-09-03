from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
CHECKER = ROOT / "scripts/check_performance_budget.py"
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
            "apple-portrait": 17,
            "apple-styles": 7,
        },
    }


def benchmark(*, tiny_wall: float = 0.30, large_wall: float = 13.0) -> dict[str, object]:
    return {
        "schema_version": 1,
        "head": HEAD,
        "machine": "arm64",
        "cases": [
            {
                "name": "tiny",
                "median_wall_seconds": tiny_wall,
                "p95_wall_seconds": 0.31,
                "max_peak_rss_bytes": 40 * MIB,
            },
            {
                "name": "large",
                "median_wall_seconds": large_wall,
                "p95_wall_seconds": 15.0,
                "max_peak_rss_bytes": 124 * MIB,
            },
        ],
    }


def launches(*, portrait: int = 17, styles: int = 7, head: str = HEAD) -> dict[str, object]:
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
    ) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory(prefix="xdremux-perf-budget-") as raw:
            work = Path(raw)
            baseline_path = work / "baseline.json"
            benchmark_path = work / "benchmark.json"
            launches_path = work / "launches.json"
            write_json(baseline_path, baseline())
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

    def test_rejects_structural_helper_launch_regression(self) -> None:
        completed = self.run_checker(benchmark(), launches(portrait=18))
        self.assertEqual(completed.returncode, 1)
        self.assertIn("apple-portrait helper launches regressed", completed.stderr)

    def test_rejects_non_exact_head_evidence(self) -> None:
        completed = self.run_checker(benchmark(), launches(head="wrong-head"))
        self.assertEqual(completed.returncode, 2)
        self.assertIn("performance evidence is not exact-head", completed.stderr)


if __name__ == "__main__":
    unittest.main()
