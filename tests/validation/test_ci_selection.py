"""CI scheduling contracts and real Git change selection, without native tools."""

from __future__ import annotations

import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from scripts.ci.agent_completion_gate import enforce_evidence_policy, load_plan
from scripts.ci.select_checks import RESEARCH, select


ROOT = Path(__file__).resolve().parents[2]


class CheckSelectionTests(unittest.TestCase):
    def selected_jobs(self, paths):
        result = select(paths)
        return {name for name in RESEARCH if result[name] == "true"}

    def test_documentation_changes_use_linux_without_native_experiments(self):
        for path in ("README.md", "docs/cli.en.md", "research/video_styles/README.md",
                     "crates/xdremux-runtime/README.md"):
            with self.subTest(path=path):
                result = select([path])
                self.assertEqual(result["runner"], "ubuntu-latest")
                self.assertEqual(result["plan"], "documentation")
                self.assertEqual(self.selected_jobs([path]), set())
                policy = enforce_evidence_policy(load_plan(ROOT / "scripts/ci/documentation-plan.json"), [path])
                self.assertFalse(policy["production_changed"])
                self.assertFalse(policy["source_changed"])

    def test_classification_change_does_not_launch_unrelated_native_experiments(self):
        path = "crates/xdremux-classification/src/lib.rs"
        self.assertEqual(self.selected_jobs([path]), {"portable-evidence"})
        self.assertEqual(select([path])["runner"], "xcode-27")

    def test_video_constructor_change_selects_its_native_consumer(self):
        self.assertEqual(self.selected_jobs(["research/video_styles/CarrierProbe.swift"]),
                         {"portable-evidence", "video-carrier"})

    def test_pvt_probe_change_does_not_launch_style_experiments(self):
        self.assertEqual(self.selected_jobs(["scripts/apple/pvt_native_import/readiness_control.py"]),
                         {"portable-evidence", "pvt-native-import"})

    def test_styles_resource_change_keeps_all_affected_native_measurements(self):
        jobs = self.selected_jobs(["crates/xdremux-heif/src/styles.rs"])
        self.assertTrue({"styles-version-control", "style-tone-response",
                         "system-heif-hdr-api", "pvt-native-import"}.issubset(jobs))
        self.assertNotIn("video-carrier", jobs)
        self.assertNotIn("coreml-candidate", jobs)
        self.assertNotIn("texture-probe-build", jobs)

    def test_selector_changes_exercise_every_research_consumer(self):
        self.assertEqual(self.selected_jobs(["scripts/ci/select_checks.py"]), set(RESEARCH))

    def test_rust_build_changes_do_not_launch_independently_compiled_probes(self):
        expected = {"portable-evidence", "styles-version-control", "style-tone-response",
                    "system-heif-hdr-api", "pvt-native-import"}
        self.assertEqual(self.selected_jobs(["Cargo.lock"]), expected)
        self.assertEqual(self.selected_jobs(["rust-toolchain.toml"]), expected)
        self.assertEqual(self.selected_jobs(["crates/xdremux-runtime/Cargo.toml"]), expected)

    def test_swift_package_changes_do_not_launch_the_rust_only_pvt_probe(self):
        self.assertEqual(self.selected_jobs(["Package.swift"]),
                         {"portable-evidence", "styles-version-control", "style-tone-response",
                          "system-heif-hdr-api"})

    def test_manual_replay_preserves_every_experiment(self):
        result = select([], manual=True)
        self.assertEqual(result["plan"], "product")
        self.assertEqual(result["runner"], "xcode-27")
        self.assertTrue(all(result[name] == "true" for name in RESEARCH))

    def test_product_plan_preserves_unique_checks_from_both_old_owners(self):
        plan = load_plan(ROOT / "scripts/ci/product-plan.json")
        checks = {check["name"]: check for check in plan["checks"]}
        for name in ("rust-workspace-tests", "apple-capability-adapter-handshake",
                     "rust-driven-apple-portrait-cli", "rust-driven-oppo-native-portrait-cli",
                     "rust-driven-apple-styles-consumer", "rust-native-texture-and-app-switch",
                     "oriented-heif-imageio-readback", "macos-app-models",
                     "rust-component-conformance", "engine-plan-vectors",
                     "rust-motion-photo-real-fixtures"):
            self.assertIn(name, checks)
        workspace_tests = [check for check in plan["checks"]
                           if "cargo test" in " ".join(check["command"])
                           and "--workspace" in " ".join(check["command"])]
        self.assertEqual(len(workspace_tests), 1)

    def test_documentation_plan_needs_only_the_standard_python_runtime(self):
        plan = load_plan(ROOT / "scripts/ci/documentation-plan.json")
        self.assertTrue(all(check["command"][:3] == ["python3", "-m", "unittest"]
                            for check in plan["checks"]))


class GitSelectionCommandTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name)
        self.git("init", "-q")
        self.git("config", "user.name", "CI selection fixture")
        self.git("config", "user.email", "ci-fixture@example.invalid")
        self.write("README.md", "baseline")
        self.write("crates/xdremux-runtime/src/lib.rs", "baseline")
        self.commit()
        self.base = self.git("rev-parse", "HEAD")

    def git(self, *args):
        return subprocess.run(["git", *args], cwd=self.repo, check=True,
                              capture_output=True, text=True).stdout.strip()

    def write(self, name, content):
        path = self.repo / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)

    def commit(self):
        self.git("add", ".")
        self.git("commit", "-qm", "selection fixture")

    def run_selector(self, *, base=None, head=None, event="pull_request"):
        return subprocess.run(
            [sys.executable, "-m", "scripts.ci.select_checks",
             "--base", self.base if base is None else base,
             "--head", self.git("rev-parse", "HEAD") if head is None else head,
             "--event", event],
            cwd=self.repo, env=dict(os.environ, PYTHONPATH=str(ROOT)),
            capture_output=True, text=True, check=False)

    def outputs(self, result):
        self.assertEqual(result.returncode, 0, result.stderr)
        return dict(line.split("=", 1) for line in result.stdout.splitlines())

    def test_cli_selects_docs_from_the_requested_commit(self):
        self.write("docs/new guide.md", "documentation")
        self.commit()
        output = self.outputs(self.run_selector())
        self.assertEqual(output["base"], self.base)
        self.assertEqual(output["plan"], "documentation")

    def test_move_out_of_product_scope_keeps_the_removed_owner_visible(self):
        self.git("mv", "crates/xdremux-runtime/src/lib.rs", "retired.md")
        self.commit()
        output = self.outputs(self.run_selector())
        self.assertEqual(output["product"], "true")
        self.assertEqual(output["pvt-native-import"], "true")

    def test_newline_in_a_source_path_cannot_turn_it_into_documentation(self):
        self.write("crates/new\nREADME.md/file.rs", "source")
        self.commit()
        self.assertEqual(self.outputs(self.run_selector())["product"], "true")

    def test_manual_replay_uses_a_real_parent_and_runs_all_consumers(self):
        self.write("README.md", "second commit")
        self.commit()
        output = self.outputs(self.run_selector(base="", event="workflow_dispatch"))
        self.assertEqual(output["base"], self.base)
        self.assertTrue(all(output[name] == "true" for name in RESEARCH))

    def test_invalid_base_and_mismatched_head_do_not_emit_skip_decisions(self):
        self.write("README.md", "second commit")
        self.commit()
        for args in ({"base": ""}, {"base": "missing-base"}, {"head": self.base}):
            with self.subTest(args=args):
                result = self.run_selector(**args)
                self.assertNotEqual(result.returncode, 0)
                self.assertNotIn("product=", result.stdout)


if __name__ == "__main__":
    unittest.main()
