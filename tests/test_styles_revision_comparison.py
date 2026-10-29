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
    def test_normal_measure_disables_inherited_profiling_and_uses_real_adapter_directly(self):
        arm = {"cli": {"path": "/candidate/cli"}, "adapter": {"path": "/candidate/adapter"}}
        def run(command, cwd, env, stdout, stderr, timeout):
            self.assertEqual(command[:3], [str(comparison.TIME), "-l", "/candidate/cli"])
            self.assertEqual(env["XDREMUX_APPLE_ADAPTER"], "/candidate/adapter")
            self.assertNotIn("XDREMUX_STYLES_PROFILE", env)
            Path(command[command.index("--output") + 1]).write_bytes(b"output")
            stderr.write("1.00 real 0.75 user 0.10 sys\n100 maximum resident set size\n")
            return 0
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            with mock.patch.dict(os.environ, {"XDREMUX_STYLES_PROFILE": "1"}), \
                 mock.patch.object(comparison, "run_process", side_effect=run):
                result = comparison.measure(arm, root / "fixture", root / "output.heic", root)
            self.assertEqual(result["user_seconds"], 0.75)
            self.assertNotIn("stages", result)

    def test_failed_profile_conversion_retains_completed_spans_and_cpu_receipt(self):
        arm = {"cli": {"path": "/cli"}, "adapter": {"path": "/adapter"}}
        def run(command, cwd, env, stdout, stderr, timeout):
            self.assertEqual(env["XDREMUX_STYLES_PROFILE"], "1")
            stderr.write("xdremux_styles_profile stage=source_analysis seconds=0.010000000\n"
                         "native failed\n1.00 real 0.75 user 0.10 sys\n100 maximum resident set size\n")
            return 1
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            report = {"summary": {"sentinel": 1}}
            with mock.patch.object(comparison, "run_process", side_effect=run), \
                 mock.patch.object(comparison, "run_logged") as validate, redirect_stdout(io.StringIO()):
                with self.assertRaisesRegex(RuntimeError, "conversion exited 1"):
                    comparison.profile_candidate(arm, root / "fixture", root, root, report)
                validate.assert_not_called()
            saved = json.loads((root / "report.json").read_text())
            self.assertEqual(saved["candidate_profile"]["status"], "failed")
            self.assertEqual(saved["candidate_profile"]["partial_stages"],
                             [{"stage": "source_analysis", "wall_seconds": 0.01}])
            self.assertEqual(saved["candidate_profile"]["user_seconds"], 0.75)
            self.assertEqual(saved["summary"], {"sentinel": 1})

    def test_profile_receipt_requires_complete_unique_finite_stages(self):
        labels = ["source_analysis", "base_conversion", "semantic_production", "resource_prepare",
                  "scene_hdr_reconstruction", "scene_linear_area_reduction", "scene_derivation",
                  "thumbnail_encode", "key1_fit", "style_metadata",
                  "native_style_properties", "style_assembly", "texture_generation",
                  "final_validation", "publication"]
        receipt = "native diagnostic\n" + "".join(
            f"xdremux_styles_profile stage={label} seconds=0.010000000\n" for label in labels)
        stages = comparison.profile_facts(receipt)
        self.assertEqual([stage["stage"] for stage in stages], labels)
        self.assertTrue(all(stage["wall_seconds"] == 0.01 for stage in stages))
        for broken in ("", receipt.rsplit("xdremux_styles_profile", 1)[0],
                       receipt + "xdremux_styles_profile stage=publication seconds=1\n",
                       receipt.replace("seconds=0.010000000", "seconds=nan", 1),
                       receipt.replace("stage=key1_fit", "stage=unknown", 1)):
            with self.subTest(receipt=broken[-100:]):
                with self.assertRaisesRegex(RuntimeError, "profile receipt"):
                    comparison.profile_facts(broken)

    def test_successful_conversion_without_profile_receipt_is_rejected(self):
        arm = {"cli": {"path": "/cli"}, "adapter": {"path": "/adapter"}}
        def run(command, cwd, env, stdout, stderr, timeout):
            self.assertEqual(env["XDREMUX_STYLES_PROFILE"], "1")
            Path(command[command.index("--output") + 1]).write_bytes(b"output")
            stderr.write("1.00 real 0.75 user 0.10 sys\n100 maximum resident set size\n")
            return 0
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            with mock.patch.object(comparison, "run_process", side_effect=run):
                with self.assertRaisesRegex(RuntimeError, "profile receipt"):
                    comparison.measure(arm, root / "fixture", root / "output.heic", root, profile=True)

    def test_profile_is_candidate_only_outside_the_interleaved_summary(self):
        arm = {"cli": {"path": "/candidate/cli"}, "adapter": {"path": "/candidate/adapter"}}
        events = []
        def measure(actual_arm, fixture, output, logs, *, profile=False):
            self.assertIs(actual_arm, arm)
            self.assertTrue(profile)
            output.write_bytes(b"profile output")
            events.append("profile")
            return {"wall_seconds": 2, "user_seconds": 1.5, "sys_seconds": 0.2,
                    "stages": [{"stage": "key1_fit", "wall_seconds": 0.5}],
                    "output_sha256": comparison.digest(output)}
        def validate(command, cwd, env, log, timeout):
            self.assertEqual(events, ["profile"])
            self.assertEqual(command, ["/candidate/cli", "validate", command[2], "--apple-styles"])
            self.assertNotIn("XDREMUX_STYLES_PROFILE", env)
            events.append("validate")
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            owned, directory = root / "owned", root / "logs"
            owned.mkdir(); directory.mkdir()
            report = {"samples": [{"status": "validated"}], "summary": {"sentinel": 1}}
            with mock.patch.object(comparison, "measure", side_effect=measure), \
                 mock.patch.object(comparison, "run_logged", side_effect=validate), \
                 mock.patch.dict(os.environ, {"XDREMUX_STYLES_PROFILE": "1"}):
                comparison.profile_candidate(arm, root / "fixture", owned, directory, report)
            self.assertEqual(events, ["profile", "validate"])
            self.assertEqual(report["samples"], [{"status": "validated"}])
            self.assertEqual(report["summary"], {"sentinel": 1})
            self.assertEqual(report["candidate_profile"]["status"], "validated")
            self.assertTrue(report["candidate_profile"]["outside_budget_and_comparison_samples"])

    def test_failed_profile_validation_preserves_the_original_summary(self):
        arm = {"cli": {"path": "/cli"}, "adapter": {"path": "/adapter"}}
        def measure(arm, fixture, output, logs, *, profile=False):
            output.write_bytes(b"profile output")
            return {"output_sha256": comparison.digest(output)}
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            report = {"summary": {"sentinel": 1}}
            with mock.patch.object(comparison, "measure", side_effect=measure), \
                 mock.patch.object(comparison, "run_logged", side_effect=RuntimeError("native profile rejection")), \
                 redirect_stdout(io.StringIO()):
                with self.assertRaisesRegex(RuntimeError, "native profile rejection"):
                    comparison.profile_candidate(arm, root / "fixture", root, root, report)
            saved = json.loads((root / "report.json").read_text())
            self.assertEqual(saved["candidate_profile"]["status"], "validation_failed")
            self.assertEqual(saved["summary"], {"sentinel": 1})

    def test_profile_validation_cannot_change_the_measured_output(self):
        arm = {"cli": {"path": "/cli"}, "adapter": {"path": "/adapter"}}
        def measure(arm, fixture, output, logs, *, profile=False):
            output.write_bytes(b"original")
            return {"output_sha256": comparison.digest(output)}
        def validate(command, cwd, env, log, timeout):
            Path(command[2]).write_bytes(b"changed by validation")
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            report = {}
            with mock.patch.object(comparison, "measure", side_effect=measure), \
                 mock.patch.object(comparison, "run_logged", side_effect=validate), \
                 redirect_stdout(io.StringIO()):
                with self.assertRaisesRegex(RuntimeError, "changed the measured output"):
                    comparison.profile_candidate(arm, root / "fixture", root, root, report)
            self.assertEqual(report["candidate_profile"]["status"], "validation_failed")

    def test_reference_build_uses_the_workflow_release_scope_and_records_features(self):
        commands = []
        def logged(command, cwd, env, log, timeout):
            commands.append(command)
            log.write_text("feature graph or build output\n")
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            report = {}
            with mock.patch.object(comparison, "run_logged", side_effect=logged), \
                 mock.patch.object(comparison, "git", return_value=""), \
                 mock.patch.object(comparison, "commit", return_value="a" * 40), \
                 mock.patch.object(comparison, "binary", return_value={"path": "/binary"}), \
                 mock.patch.object(comparison, "checked_fixture", return_value="fixture"), \
                 mock.patch.object(comparison.subprocess, "check_output", return_value="/swift/bin"), \
                 mock.patch.object(comparison.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, "", "")):
                with comparison.reference_build(root, "a" * 40, root, root, report) as arm:
                    self.assertIn(["cargo", "build", "--locked", "--workspace", "--all-targets", "--release"], commands)
                    self.assertIn(["cargo", "tree", "--locked", "--workspace", "--edges", "features"], commands)
                    self.assertEqual(arm["feature_graph"]["sha256"], comparison.digest(root / "reference-cargo-features.log"))
                    self.assertEqual(arm["build_commands"][0], commands[1])

    def test_feature_graph_identity_distinguishes_features_from_worktree_paths(self):
        def logged(command, cwd, env, log, timeout):
            log.write_text(f"crate v0.1.0 ({cwd}/crates/crate)\nfeature: enabled\n")
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            with mock.patch.object(comparison, "run_logged", side_effect=logged):
                left = comparison.feature_graph(root / "baseline", {}, root, "baseline")
                right = comparison.feature_graph(root / "candidate", {}, root, "candidate")
            self.assertNotEqual(left["sha256"], right["sha256"])
            self.assertEqual(left["source_path_normalized_sha256"], right["source_path_normalized_sha256"])

    @unittest.skipUnless(hasattr(os, "symlink"), "worktree path alias test requires symlinks")
    def test_feature_graph_identity_handles_real_symlink_cwd_and_keeps_feature_changes(self):
        real_logged = comparison.run_logged
        feature = "enabled"
        def logged(command, cwd, env, log, timeout):
            # A real child resolves cwd through the filesystem, as Cargo does.
            program = ("import os; print('crate (' + os.getcwd() + '/crates/crate)'); "
                       "print('requested (' + os.environ['REQUESTED_SOURCE'] + '/crates/crate)'); "
                       "print('feature: ' + os.environ['TEST_FEATURE'])")
            real_logged([sys.executable, "-c", program], cwd,
                        {**os.environ, "REQUESTED_SOURCE": str(cwd), "TEST_FEATURE": feature}, log, timeout)
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            source = root / "source"
            source.mkdir()
            alias = source / "nested-alias"
            try:
                alias.symlink_to(source, target_is_directory=True)
            except OSError as error:
                self.skipTest(f"cannot create the test directory symlink: {error}")
            with mock.patch.object(comparison, "run_logged", side_effect=logged):
                canonical = comparison.feature_graph(source, {}, root, "canonical")
                through_alias = comparison.feature_graph(alias, {}, root, "alias")
                feature = "different"
                changed = comparison.feature_graph(alias, {}, root, "changed")
            self.assertNotEqual(canonical["sha256"], through_alias["sha256"])
            self.assertEqual(canonical["source_path_normalized_sha256"], through_alias["source_path_normalized_sha256"])
            self.assertNotEqual(through_alias["source_path_normalized_sha256"], changed["source_path_normalized_sha256"])

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
            return {"wall_seconds": 2, "time_real_seconds": 2, "user_seconds": 1, "sys_seconds": 0.1,
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
                 mock.patch.object(comparison, "run_logged", side_effect=validate), \
                 redirect_stdout(io.StringIO()):
                comparison.collect(arms, root / "fixture.heic", owned, directory, report)
            self.assertEqual(len(events), 24)
            self.assertTrue(all(row["status"] == "validated" for row in report["samples"]))
            self.assertEqual(comparison.summarize(report)["candidate_over_baseline_median"], 1)

    def test_failed_validation_leaves_failure_receipt_instead_of_success_summary(self):
        arms = {name: {"cli": {"path": name}, "adapter": {"path": name}}
                for name in ("baseline", "candidate")}
        def measure(arm, fixture, output, logs):
            output.write_bytes(b"output")
            return {"wall_seconds": 1, "time_real_seconds": 1, "user_seconds": 0.5, "sys_seconds": 0.1,
                    "peak_rss_bytes": 100, "output_sha256": comparison.digest(output)}
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            directory, owned = root / "logs", root / "owned"
            directory.mkdir(); owned.mkdir()
            report = {"samples": []}
            with mock.patch.object(comparison, "measure", side_effect=measure), \
                 mock.patch.object(comparison, "run_logged", side_effect=RuntimeError("native rejection")), \
                 redirect_stdout(io.StringIO()):
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
        def profile(arm, fixture, owned, directory, report):
            self.assertEqual(len(report["samples"]), 12)
            self.assertTrue(all(row["status"] == "validated" for row in report["samples"]))
            self.assertIn("candidate_over_baseline_median", report["summary"])
            report["candidate_profile"] = {"status": "validated"}
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
                 mock.patch.object(comparison, "feature_graph", return_value={"sha256": "features"}), \
                 mock.patch.object(comparison, "profile_candidate", side_effect=profile), \
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
        self.assertIn("if: ${{ !cancelled() && github.event_name == 'workflow_dispatch' }}", step)
        self.assertIn("github.event.pull_request.base.sha", step)
        self.assertIn("refs/remotes/origin/main", step)
        self.assertNotIn("continue-on-error", step)
        self.assertNotIn("|| true", step)
        self.assertLess(workflow.index("- name: Enforce stable-runner performance regression budget"),
                        workflow.index("- name: Compare exact Styles revisions on the same runner"))


if __name__ == "__main__":
    unittest.main()
