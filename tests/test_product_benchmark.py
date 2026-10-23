from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import textwrap
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "product_benchmark_under_test", ROOT / "scripts/performance/benchmark_rust_product.py"
)
BENCHMARK = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = BENCHMARK
SPEC.loader.exec_module(BENCHMARK)


class ProductBenchmarkTests(unittest.TestCase):
    def test_workflow_summary_keeps_partial_failure_observable(self):
        workflow = (ROOT / ".github/workflows/performance.yml").read_text()
        block = workflow.split("if [[ -s artifacts/performance/product-benchmark.json ]]; then", 1)[1]
        code = textwrap.dedent(block.split("\n", 2)[2].split("\n          PY", 1)[0])
        report = {"status": "failed", "errors": [{"error": "command exited 7"}], "cases": [
            {"name": "motion-heif", "status": "failed", "samples": [{"status": "failed"}],
             "errors": [{"error": "command exited 7"}]},
        ]}
        with tempfile.TemporaryDirectory() as raw:
            work = Path(raw)
            path = work / "artifacts/performance/product-benchmark.json"
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps(report))
            result = subprocess.run([sys.executable, "-c", code], cwd=work,
                                    text=True, capture_output=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("failed", result.stdout)
        self.assertIn("command exited 7", result.stdout)
        self.assertIn("0/1", result.stdout)

    def test_nonempty_invalid_output_fails_canonical_validation(self):
        with tempfile.TemporaryDirectory() as raw:
            work = Path(raw)
            source = work / "source.heic"
            source.write_bytes(b"input")
            (work / "output.heic").write_bytes(b"invalid but nonempty")
            case = BENCHMARK.Case("hdr", (source,), ("convert",), ("output.heic",))
            rejected = subprocess.CompletedProcess([], 1, json.dumps({
                "schema_version": 1, "command": "validate", "valid": False,
            }), "invalid media")
            with mock.patch.object(BENCHMARK.subprocess, "run", return_value=rejected) as run:
                with self.assertRaisesRegex(RuntimeError, "canonical validation failed"):
                    BENCHMARK.validate_outputs(case, Path("xdremux"), work, "", {})
            self.assertEqual(run.call_args.args[0][1:], ["validate", str((work / "output.heic").resolve()), "--json"])

    def test_partial_batch_receipt_cannot_be_counted_as_three_conversions(self):
        with tempfile.TemporaryDirectory() as raw:
            work = Path(raw)
            (work / "batch").mkdir()
            (work / "batch/output.heic").write_bytes(b"nonempty")
            case = BENCHMARK.Case("batch-3", tuple(work / f"input-{n}.heic" for n in range(3)),
                                  ("batch", "--json"), ("batch",))
            receipt = {"command": "batch", "processed": 3, "succeeded": 2,
                       "failed": 0, "skipped_existing": 1, "successes": [], "failures": []}
            with self.assertRaisesRegex(RuntimeError, "does not match expected work"):
                BENCHMARK.validate_outputs(case, Path("xdremux"), work, json.dumps(receipt), {})

    def test_failed_sample_remains_observable_before_scratch_cleanup(self):
        with tempfile.TemporaryDirectory() as raw:
            source = Path(raw) / "source.heic"
            source.write_bytes(b"source bytes")
            case = BENCHMARK.Case("hdr", (source,), ("convert",), ("output.heic",))
            observations = []
            def observe(value):
                observations.append(json.loads(json.dumps(value)))
            with mock.patch.object(BENCHMARK, "run_timed", side_effect=RuntimeError("command exited 1")):
                with self.assertRaisesRegex(RuntimeError, "command exited 1"):
                    BENCHMARK.measure_case(case, Path("xdremux"), {}, 0, 5, observe)
            final = observations[-1]
            self.assertEqual(final["status"], "failed")
            self.assertEqual(final["samples"][0]["status"], "failed")
            self.assertEqual(final["errors"], [{"run": 0, "warmup": False, "error": "command exited 1"}])
            self.assertNotIn("median_wall_seconds", final)
            self.assertEqual(source.read_bytes(), b"source bytes")

    def test_validation_observes_only_a_finished_timed_conversion(self):
        with tempfile.TemporaryDirectory() as raw:
            source = Path(raw) / "source.heic"
            source.write_bytes(b"source")
            case = BENCHMARK.Case("hdr", (source,), ("convert",), ("output.heic",))
            events = []
            def timed(command, env):
                events.append("timed")
                return 0.25, 1024, "", ""
            def validated(*args):
                events.append("validated")
                return {"completed_work": 1}
            with mock.patch.object(BENCHMARK, "run_timed", side_effect=timed), \
                 mock.patch.object(BENCHMARK, "validate_outputs", side_effect=validated):
                result = BENCHMARK.measure_case(case, Path("xdremux"), {}, 1, 5)
            self.assertEqual(events, ["timed", "validated"] * 6)
            self.assertEqual(result["status"], "complete")
            self.assertEqual(len(result["samples"]), 5)
            self.assertEqual(result["warmup_completed"], 1)


if __name__ == "__main__":
    unittest.main()
