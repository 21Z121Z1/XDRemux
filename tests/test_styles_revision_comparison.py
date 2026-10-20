"""Portable behavioral checks for the same-runner Styles diagnostic owner."""
from pathlib import Path
import json
import os
import io
from contextlib import nullcontext, redirect_stderr, redirect_stdout
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

from scripts.performance import compare_apple_styles_revisions as comparison


class StylesRevisionComparisonTests(unittest.TestCase):
    def test_interleaving_keeps_one_warmup_and_five_samples_per_arm(self):
        order = comparison.sample_order()
        self.assertEqual(order[:2], [(True, 0, "baseline"), (True, 0, "candidate")])
        self.assertEqual(len(order), 12)
        for iteration in range(5):
            pair = order[2 + 2 * iteration:4 + 2 * iteration]
            self.assertEqual({row[2] for row in pair}, {"baseline", "candidate"})
            self.assertTrue(all(not warmup and index == iteration for warmup, index, _ in pair))
            self.assertEqual(pair[0][2], "baseline" if iteration % 2 == 0 else "candidate")

    def test_time_receipts_require_cpu_and_rss(self):
        facts = comparison.time_facts("  1.50 real  2.00 user  0.25 sys\n 1048576  maximum resident set size\n")
        self.assertEqual(facts, {"time_real_seconds": 1.5, "user_seconds": 2.0,
                                 "sys_seconds": 0.25, "peak_rss_bytes": 1048576})
        for broken in ("", "1.5 real 2 user .25 sys\n"):
            with self.assertRaisesRegex(RuntimeError, "receipt"):
                comparison.time_facts(broken)

    def test_validation_runs_after_all_timed_samples_and_preserves_outputs(self):
        events = []
        arms = {name: {"cli": {"path": f"/{name}/cli"}, "adapter": {"path": f"/{name}/adapter"}}
                for name in ("baseline", "candidate")}

        def measure(arm, fixture, output, logs):
            events.append(("convert", arm["cli"]["path"]))
            output.write_bytes(b"real conversion stand-in")
            return {"wall_seconds": 2, "user_seconds": 1, "sys_seconds": 0.1,
                    "peak_rss_bytes": 100, "output_sha256": comparison.digest(output)}

        def validate(command, cwd, env, log, timeout):
            self.assertEqual(len([e for e in events if e[0] == "convert"]), 12)
            self.assertEqual(env["XDREMUX_APPLE_ADAPTER"], command[0].replace("cli", "adapter"))
            self.assertEqual(command[1], "validate")
            self.assertEqual(command[-1], "--apple-styles")
            events.append(("validate", command[0]))

        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            directory, owned = root / "logs", root / "owned"
            directory.mkdir(); owned.mkdir()
            report = {"samples": []}
            with mock.patch.object(comparison, "measure", side_effect=measure), \
                 mock.patch.object(comparison, "run_logged", side_effect=validate):
                comparison.collect(arms, root / "fixture.heic", owned, directory, report)
            self.assertEqual(len(events), 24)
            self.assertTrue(all(row["status"] == "validated" for row in report["samples"]))
            self.assertEqual(comparison.summarize(report)["candidate_over_baseline_median"], 1)

    def test_failed_validation_leaves_failure_receipt_instead_of_success_summary(self):
        arms = {name: {"cli": {"path": name}, "adapter": {"path": name}}
                for name in ("baseline", "candidate")}
        def measure(arm, fixture, output, logs):
            output.write_bytes(b"output")
            return {"wall_seconds": 1, "output_sha256": comparison.digest(output)}
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            directory, owned = root / "logs", root / "owned"
            directory.mkdir(); owned.mkdir()
            report = {"samples": []}
            with mock.patch.object(comparison, "measure", side_effect=measure), \
                 mock.patch.object(comparison, "run_logged", side_effect=RuntimeError("native rejection")):
                with self.assertRaisesRegex(RuntimeError, "native rejection"):
                    comparison.collect(arms, root / "fixture", owned, directory, report)
            saved = json.loads((directory / "report.json").read_text())
            self.assertEqual(saved["samples"][0]["status"], "validation_failed")
            with self.assertRaisesRegex(RuntimeError, "validated samples"):
                comparison.summarize(saved)

    def test_build_failure_removes_only_the_owned_reference_worktree(self):
        commands = []
        def logged(command, cwd, env, log, timeout):
            commands.append(command)
            if command[0] == "cargo":
                raise RuntimeError("build failed")
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            owned, logs = root / "owned", root / "logs"
            owned.mkdir(); logs.mkdir()
            report = {}
            with mock.patch.object(comparison, "run_logged", side_effect=logged), \
                 mock.patch.object(comparison, "git", return_value=""), \
                 mock.patch.object(comparison.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, "", "")) as remove:
                with self.assertRaisesRegex(RuntimeError, "build failed"):
                    with comparison.reference_build(root, "a" * 40, owned, logs, report):
                        self.fail("a failed build cannot yield binaries")
            self.assertEqual(remove.call_args.args[0], ["git", "worktree", "remove", "--force", str(owned / "reference-source")])
            self.assertEqual(report["owned_worktree_cleanup"]["returncode"], 0)

    @unittest.skipUnless(os.name == "posix", "process-group termination requires POSIX")
    def test_timeout_reaps_the_owned_process(self):
        with tempfile.TemporaryFile() as log:
            with self.assertRaises(subprocess.TimeoutExpired):
                comparison.run_process([sys.executable, "-c", "import time; time.sleep(30)"],
                                       Path.cwd(), os.environ.copy(), log, log, 0.02)

    @unittest.skipUnless(os.name == "posix", "real diagnostic process/worktree lifecycle requires POSIX")
    def test_real_same_repository_worktree_is_cleaned_after_build_failure(self):
        real_logged = comparison.run_logged
        def fail_build(command, cwd, env, log, timeout):
            if command[0] == "cargo":
                raise RuntimeError("deliberate build failure before native compilation")
            real_logged(command, cwd, env, log, timeout)
        with tempfile.TemporaryDirectory(prefix="xdremux-comparison-test-") as raw:
            owned = Path(raw)
            logs = owned / "logs"
            logs.mkdir()
            report = {}
            with mock.patch.object(comparison, "run_logged", side_effect=fail_build):
                with self.assertRaisesRegex(RuntimeError, "deliberate build failure"):
                    with comparison.reference_build(comparison.ROOT, comparison.commit(comparison.ROOT, "HEAD"), owned, logs, report):
                        self.fail("the deliberately failed build must not yield")
            self.assertFalse((owned / "reference-source").exists())
            registry = comparison.git(comparison.ROOT, "worktree", "list", "--porcelain")
            self.assertNotIn(f"worktree {owned / 'reference-source'}", registry.splitlines())
            self.assertIn(f"worktree {comparison.ROOT}", registry.splitlines())
            self.assertEqual(report["owned_worktree_cleanup"]["returncode"], 0)

    def test_same_sha_is_explicitly_non_attributable_and_diagnostic_only(self):
        sha = "a" * 40
        arm = {"sha": sha, "tracked_status": "", "fixture_sha256": "fixture",
               "cli": {"path": "/cli"}, "adapter": {"path": "/adapter"}}
        def collect(arms, fixture, owned, directory, report):
            report["samples"] = [
                {"arm": name, "warmup": warmup, "status": "validated", "wall_seconds": 1,
                 "user_seconds": 0.5, "sys_seconds": 0.1, "peak_rss_bytes": 10}
                for warmup, _, name in comparison.sample_order()
            ]
        with tempfile.TemporaryDirectory() as raw:
            output = Path(raw) / "report"
            arguments = ["compare", "--cli", "/cli", "--apple-adapter", "/adapter",
                         "--baseline-ref", sha, "--expected-head", sha, "--output-dir", str(output)]
            with mock.patch.object(sys, "argv", arguments), \
                 mock.patch.object(comparison.platform, "system", return_value="Darwin"), \
                 mock.patch.object(comparison, "commit", return_value=sha), \
                 mock.patch.object(comparison, "git", return_value=""), \
                 mock.patch.object(comparison, "checked_fixture", return_value="fixture"), \
                 mock.patch.object(comparison, "binary", return_value={"path": "/binary"}), \
                 mock.patch.object(comparison, "reference_build", return_value=nullcontext(arm)), \
                 mock.patch.object(comparison, "collect", side_effect=collect), \
                 mock.patch.object(comparison.subprocess, "check_output", return_value="toolchain"), \
                 redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
                self.assertEqual(comparison.main(), 0)
            report = json.loads((output / "report.json").read_text())
            self.assertFalse(report["distinct_revisions"])
            self.assertTrue(report["diagnostic_only"])
            self.assertTrue(report["absolute_budget_unchanged"])
            self.assertTrue(any("same revision" in limit for limit in report["comparison_limits"]))

    def test_workflow_keeps_absolute_failure_and_records_the_actual_base(self):
        workflow = (comparison.ROOT / ".github/workflows/performance.yml").read_text()
        step = workflow.split("- name: Compare exact Styles revisions on the same runner", 1)[1].split("- name:", 1)[0]
        self.assertIn("if: ${{ !cancelled() }}", step)
        self.assertIn("github.event.pull_request.base.sha", step)
        self.assertIn("refs/remotes/origin/main", step)
        self.assertNotIn("continue-on-error", step)
        self.assertNotIn("|| true", step)
        self.assertLess(workflow.index("- name: Enforce stable-runner performance regression budget"),
                        workflow.index("- name: Compare exact Styles revisions on the same runner"))


if __name__ == "__main__":
    unittest.main()
