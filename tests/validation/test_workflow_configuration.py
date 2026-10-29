#!/usr/bin/env python3
"""Regression tests for the repository's GitHub Actions configuration."""

from __future__ import annotations

from pathlib import Path
import fnmatch
import re
import unittest


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
WORKFLOW_ROOT = REPOSITORY_ROOT / ".github" / "workflows"
CONCURRENCY = (
    "concurrency:\n"
    "  group: ${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}\n"
    "  cancel-in-progress: true"
)


class WorkflowConfigurationTests(unittest.TestCase):
    @classmethod
    def uploaded(cls, workflow: str, path: str) -> bool:
        for step in re.split(r"(?m)^      - ", cls.workflow(workflow)):
            if "uses: actions/upload-artifact@" not in step:
                continue
            match = re.search(r"(?m)^          path: (.+)$", step)
            if match is None:
                continue
            if match[1] == "|":
                patterns = re.findall(r"(?m)^            (\S.*)$", step[match.end():])
            else:
                patterns = [match[1]]
            for pattern in patterns:
                if path.startswith(pattern.rstrip("/") + "/") or fnmatch.fnmatchcase(path, pattern):
                    return True
        return False

    def test_product_uploads_keep_receipts_without_media_or_pixel_dictionaries(self):
        for path in ("artifacts/product-integration/rust-tests.log",
                     "artifacts/product-integration/texture/result.json",
                     "artifacts/product-integration/orientation/expected.json",
                     "artifacts/styles-acceptance/linear-thumbnail-roundtrip.json"):
            with self.subTest(path=path):
                self.assertTrue(self.uploaded("ci.yml", path))
        for path in ("artifacts/product-integration/texture/case/styles-texture.heic",
                     "artifacts/product-integration/texture/native/context/working.png",
                     "artifacts/product-integration/texture/native/context/face.l8",
                     "artifacts/product-integration/texture/native/context/stored-person-pixels.json",
                     "artifacts/styles-acceptance/photo.heic"):
            with self.subTest(path=path):
                self.assertFalse(self.uploaded("ci.yml", path), "media must stay on the runner")

    def test_research_uploads_keep_scores_without_photos_or_render_buffers(self):
        for path in ("${{ runner.temp }}/styles-version-native.json",
                     "artifacts/tone-response/report/tone-response.json",
                     "artifacts/tone-response/asymmetry/asymmetry.json",
                     "artifacts/system-heif-hdr-measurement/tone-shape.json",
                     "artifacts/system-heif-hdr-measurement/tone/consumer-run.json",
                     "artifacts/system-heif-hdr-measurement/tone/diagnostics/consumer.stderr.log"):
            with self.subTest(path=path):
                self.assertTrue(self.uploaded("research.yml", path))
        for path in ("${{ runner.temp }}/styles-photos-import/styles-v15.heic",
                     "artifacts/tone-response/generated/portrait.heic",
                     "artifacts/tone-response/report/comparisons/portrait.png",
                     "artifacts/tone-response/xdremux/photo/tone-0/renderer-0/_outputImageTexture.bin",
                     "artifacts/tone-response/xdremux/photo/pixels.json",
                     "artifacts/system-heif-hdr-measurement/lhdr-v1-01.system.heic",
                     "artifacts/system-heif-hdr-measurement/tone/photo/pixels.f32",
                     "artifacts/system-heif-hdr-measurement/tone/photo/tone-0/renderer-0/trace.json",
                     "artifacts/system-heif-hdr-measurement/tone/photo/tone-0/facts.json",
                     "artifacts/system-heif-hdr-measurement/tone/consumer-compat/heif_dump.py"):
            with self.subTest(path=path):
                self.assertFalse(self.uploaded("research.yml", path), "only scored reports may leave the runner")

    def test_pvt_native_import_uses_an_owned_simulator_and_keeps_only_receipts(self):
        workflow = self.workflow("research.yml")
        job = workflow.split("  pvt-native-import:\n", 1)[1]
        self.assertIn("runs-on: xcode-27", job)
        self.assertIn("EXPECTED_HEAD: ${{ github.event.pull_request.head.sha || github.sha }}", job)
        steps = re.split(r"(?m)^      - ", job)[1:]
        checkouts = [step for step in steps if step.startswith("uses: actions/checkout@")]
        self.assertEqual(len(checkouts), 1, "the current product and probe share one clean source checkout")
        self.assertIn("ref: ${{ env.EXPECTED_HEAD }}", checkouts[0])
        self.assertIn("persist-credentials: false", checkouts[0])
        self.assertNotIn("path:", checkouts[0])
        execution = next(step for step in steps if "readiness_control.py" in step)
        self.assertIn('python3 "$GITHUB_WORKSPACE/scripts/apple/pvt_native_import/readiness_control.py"', execution)
        self.assertIn('--repo "$GITHUB_WORKSPACE"', execution)
        self.assertIn('--expected-head "$EXPECTED_HEAD"', execution)
        self.assertIn('--required-runtime com.apple.CoreSimulator.SimRuntime.iOS-27-0', execution)
        self.assertIn('--required-device-type com.apple.CoreSimulator.SimDeviceType.iPhone-11', execution)
        self.assertIn('CARGO_TARGET_DIR: ${{ github.workspace }}/target', job)
        self.assertNotIn("--expected-product-head", execution)
        self.assertNotIn("--expected-harness-head", execution)
        self.assertNotRegex(job, r"\b[0-9a-f]{40}\b", "the current-source experiment must not freeze a historical pin")
        self.assertIn("needs: changes", job, "only check selection precedes the single control/product job")
        self.assertIn("github.event.repository.private == false", job)
        self.assertIn("github.event.repository.visibility == 'public'", job)
        self.assertIn("--execute-owned-simulator", job)
        self.assertNotIn("continue-on-error", job)
        self.assertNotIn("|| true", job)
        self.assertNotRegex(job, r"runs-on: .*-(?:large|xlarge)")
        prefix = "${{ runner.temp }}/pvt-native-import/pvt-native-owned/"
        for name in ("result.json", "baseline.json", "after.json", "share.json", "share-transfer.json", "test-share-ui.stderr"):
            self.assertTrue(self.uploaded("research.yml", prefix + name))
        for name in ("synthetic-motion.jpg", "synthetic.mp4", "capture.pvt/image.heic",
                     "readback/resource-0.bin", "PVTReadback.app/Info.plist", "share-results.xcresult/Info.plist"):
            self.assertFalse(self.uploaded("research.yml", prefix + name))
        control_prefix = "${{ runner.temp }}/pvt-readiness-control/control-owned/"
        self.assertTrue(self.uploaded("research.yml", control_prefix + "result.json"))
        self.assertTrue(self.uploaded("research.yml", control_prefix + "control-listapps.stdout"))
        self.assertFalse(self.uploaded("research.yml", control_prefix + "capture.pvt/image.heic"))

    @staticmethod
    def workflow(name: str) -> str:
        return (WORKFLOW_ROOT / name).read_text(encoding="utf-8")

    @staticmethod
    def event_block(workflow: str, event: str) -> str:
        match = re.search(rf"(?m)^  {re.escape(event)}:\n", workflow)
        if match is None:
            raise AssertionError(f"workflow is missing on.{event}")
        remaining = workflow[match.end() :]
        next_top_level = re.search(r"(?m)^  [A-Za-z0-9_-]+:", remaining)
        return remaining[: next_top_level.start() if next_top_level else None]

    def test_completion_gate_has_the_required_exact_head_check_context(self) -> None:
        workflow = self.workflow("ci.yml")
        self.assertRegex(workflow, r"(?m)^  exact-head:\n    name: exact-head$")
        self.assertIn("EXPECTED_HEAD: ${{ github.event.pull_request.head.sha || github.sha }}", workflow)
        self.assertIn("ref: ${{ env.EXPECTED_HEAD }}", workflow)
        self.assertIn('--base "$XDREMUX_GATE_BASE"', workflow)
        self.assertIn("github.event.pull_request.base.sha || github.event.before", workflow)
        self.assertIn("XDREMUX_GATE_BASE: ${{ needs.changes.outputs.base }}", workflow)
        job = workflow.split("  exact-head:\n", 1)[1]
        self.assertIn("if: ${{ always() }}", job)
        self.assertIn('test "$SELECTION_RESULT" = success', job)

    def test_completion_gate_pr_trigger_is_always_on_without_paths(self) -> None:
        workflow = self.workflow("ci.yml")
        pull_request = self.event_block(workflow, "pull_request")
        self.assertEqual(pull_request.strip(), "")

    def test_required_workflow_pushes_are_main_only(self) -> None:
        workflows = (
            "ci.yml",
            "performance.yml",
            "rust-cli-core.yml",
            "rust-proxdr-real-fixtures.yml",
        )
        for name in workflows:
            with self.subTest(workflow=name):
                push = self.event_block(self.workflow(name), "push")
                self.assertRegex(
                    push,
                    r"(?m)(?:^    branches:\s*\[main\]$|^    branches:\n(?:      - main\n)+)",
                )

    def test_push_path_filters_remain_present(self) -> None:
        for name in ("performance.yml", "rust-cli-core.yml", "rust-proxdr-real-fixtures.yml"):
            with self.subTest(workflow=name):
                push = self.event_block(self.workflow(name), "push")
                self.assertRegex(push, r"(?m)^    paths(?:-ignore)?:")

    def test_workflow_set_stays_minimal(self) -> None:
        expected = {
            "ci.yml",
            "performance.yml",
            "rust-cli-core.yml",
            "rust-proxdr-real-fixtures.yml",
            # One reusable research-evidence workflow, not a product gate or
            # another per-hypothesis writer. Historical one-shot workflows stay out.
            "research.yml",
        }
        actual = {path.name for path in WORKFLOW_ROOT.glob("*.yml")}
        self.assertEqual(actual, expected,
                         "Use ci.yml for product regressions or research.yml for experiments; "
                         "a new workflow needs a distinct maintained owner.")

    def test_research_evidence_is_read_only_exact_head_and_separate(self) -> None:
        workflow = self.workflow("research.yml")
        self.assertIn("permissions:\n  contents: read", workflow)
        self.assertIn("ref: ${{ github.event.pull_request.head.sha || github.sha }}", workflow)
        self.assertIn(CONCURRENCY, workflow)
        self.assertIn("    branches: [main]", self.event_block(workflow, "push"))
        self.assertIn("python3 -m scripts.ci.select_checks", workflow)
        for forbidden in ("contents: write", "secrets.", "pull_request_target", "git push",
                          "agent_completion_gate.py"):
            self.assertNotIn(forbidden, workflow)
        self.assertIn("research.video_styles.smoke", workflow)

    def test_manual_evidence_runs_are_read_only_and_accept_no_inputs(self) -> None:
        # Exact-main acceptance sometimes needs a workflow whose path filter
        # did not fire. Manual replay may not accept alternate code or secrets.
        for name in ("research.yml", "rust-cli-core.yml", "rust-proxdr-real-fixtures.yml"):
            with self.subTest(workflow=name):
                workflow = self.workflow(name)
                self.assertRegex(workflow, r"(?m)^  workflow_dispatch:\s*\n\npermissions:")
                self.assertIn("permissions:\n  contents: read", workflow)
                for forbidden in ("inputs:", "contents: write", "secrets.", "pull_request_target"):
                    self.assertNotIn(forbidden, workflow)

    def test_portability_and_fixtures_test_the_requested_exact_head(self) -> None:
        for name in ("rust-cli-core.yml", "rust-proxdr-real-fixtures.yml"):
            with self.subTest(workflow=name):
                workflow = self.workflow(name)
                self.assertIn("ref: ${{ github.event.pull_request.head.sha || github.sha }}", workflow)
                self.assertIn("persist-credentials: false", workflow)
                self.assertIn("EXPECTED_HEAD: ${{ github.event.pull_request.head.sha || github.sha }}", workflow)
                self.assertIn('test "$(git rev-parse HEAD)" = "$EXPECTED_HEAD"', workflow)

    def test_native_research_failure_artifacts_are_not_a_success_waiver(self) -> None:
        workflow = self.workflow("research.yml")
        self.assertIn("- uses: actions/upload-artifact@v7\n        if: ${{ always() }}", workflow)
        self.assertIn('> "$RUNNER_TEMP/video-style-evidence.json"', workflow)
        self.assertNotIn("continue-on-error", workflow)
        self.assertNotIn("|| true", workflow)

    def test_workflows_with_new_concurrency_policy_have_the_shared_group(self) -> None:
        workflows = (
            "rust-cli-core.yml",
            "rust-proxdr-real-fixtures.yml",
        )
        for name in workflows:
            with self.subTest(workflow=name):
                self.assertIn(CONCURRENCY, self.workflow(name))

    def test_rust_cli_workflow_is_portability_only(self) -> None:
        workflow = self.workflow("rust-cli-core.yml")
        self.assertIn("name: Rust CLI portability", workflow)
        self.assertIn("os: [ubuntu-latest, macos-latest, windows-latest]", workflow)
        self.assertNotIn("agent_completion_gate.py", workflow)
        self.assertNotIn("cargo clippy", workflow)
        self.assertNotIn("cargo fmt", workflow)

    def test_macos_packages_share_portability_owner_and_require_relocated_runtime(self) -> None:
        workflow = self.workflow("rust-cli-core.yml")
        package_job = workflow.split("  macos-package:\n", 1)[1]
        # Full product packages target Apple silicon/macOS 27; the portable
        # Rust core keeps the three-OS job above.
        self.assertIn("runs-on: xcode-27", package_job)
        self.assertNotIn("matrix", package_job)
        self.assertNotIn("x86_64", package_job)
        self.assertNotIn("macos-26-intel", package_job)
        self.assertIn('MACOSX_DEPLOYMENT_TARGET: "27.0"', package_job)
        self.assertIn("--target aarch64-apple-darwin", package_job)
        self.assertNotRegex(package_job, r"runs-on: .*-(?:large|xlarge)")
        self.assertIn("swift build -c release --product xdremux-apple-adapter", package_job)
        self.assertNotIn("platforms/apple", package_job)
        self.assertIn("scripts/distribution/build_native.py", package_job)
        self.assertIn("scripts/distribution/macos.py archive", package_job)
        self.assertIn("scripts/distribution/smoke_macos.py", package_job)
        self.assertIn('--fixtures "$GITHUB_WORKSPACE/fixtures"', package_job)
        self.assertLess(package_job.index("scripts/distribution/smoke_macos.py"),
                        package_job.index("actions/upload-artifact@"))
        self.assertNotIn("continue-on-error", package_job)
        self.assertNotIn("|| true", package_job)
        for event in ("push", "pull_request"):
            self.assertIn('"scripts/distribution/**"', self.event_block(workflow, event))
            self.assertIn('"Package.swift"', self.event_block(workflow, event))
            self.assertIn('"Sources/**"', self.event_block(workflow, event))

    def test_performance_workflow_does_not_duplicate_completion(self) -> None:
        workflow = self.workflow("performance.yml")
        self.assertIn("scripts/performance/benchmark_rust_product.py", workflow)
        self.assertIn("scripts/performance/check_performance_budget.py", workflow)
        self.assertIn("tests.test_product_benchmark", workflow)
        self.assertNotIn("--historical-replay", workflow,
                         "current CI cannot use archived summary-only reports")
        self.assertNotIn("scripts/ci/agent_completion_gate.py", workflow)
        self.assertNotIn("scripts/validation/check_rust_cli_smoke.sh", workflow)
        self.assertNotIn("XDRemuxAppModelTests", workflow)

    def test_style_tone_sweep_is_read_only_and_pins_the_published_harness(self) -> None:
        workflow = self.workflow("research.yml")
        self.assertIn("permissions:\n  contents: read", workflow)
        self.assertIn("runs-on: xcode-27", workflow)
        self.assertIn("ref: 37dd551a949047ed14580efa0c0a2556922791b5", workflow)
        self.assertIn("ref: ${{ github.event.pull_request.head.sha || github.sha }}", workflow)
        self.assertIn("retention-days: 1", workflow)
        self.assertIn("cancel-in-progress: true", workflow)
        self.assertRegex(workflow, r"(?m)^  workflow_dispatch:\s*\n\npermissions:")
        for forbidden in (
            "contents: write",
            "secrets.",
            "pull_request_target",
            "git push",
            "inputs:",
            "agent_completion_gate.py",
        ):
            self.assertNotIn(forbidden, workflow)

    def test_system_heif_hdr_api_probe_is_read_only(self) -> None:
        workflow = self.workflow("research.yml")
        self.assertIn("permissions:\n  contents: read", workflow)
        self.assertIn("runs-on: xcode-27", workflow)
        self.assertIn("ref: ${{ github.event.pull_request.head.sha || github.sha }}", workflow)
        self.assertIn("retention-days: 1", workflow)
        self.assertIn("cancel-in-progress: true", workflow)
        self.assertIn("scripts/apple/probe_system_heif_hdr_api.sh", workflow)
        self.assertRegex(workflow, r"(?m)^  workflow_dispatch:\s*\n\npermissions:")
        for forbidden in (
            "contents: write",
            "secrets.",
            "pull_request_target",
            "git push",
            "inputs:",
            "agent_completion_gate.py",
        ):
            self.assertNotIn(forbidden, workflow)

    def test_oriented_imageio_readback_uses_the_product_exact_head(self) -> None:
        workflow = self.workflow("ci.yml")
        self.assertIn("ref: ${{ env.EXPECTED_HEAD }}", workflow)
        self.assertIn("XDREMUX_ORIENTATION_OUTPUT:", workflow)
        plan = (REPOSITORY_ROOT / "scripts/ci/product-plan.json").read_text()
        self.assertIn("scripts/apple/read_oriented_heif_imageio.swift", plan)
        self.assertIn("artifacts/product-integration/orientation/expected.json", plan)

    def test_completion_gate_owns_repository_policy_regressions(self) -> None:
        workflow = self.workflow("ci.yml")
        self.assertIn('scripts/ci/$PLAN-plan.json', workflow)
        plan = (REPOSITORY_ROOT / "scripts/ci/product-plan.json").read_text()
        self.assertIn("tests.validation.test_agent_completion_gate", plan)
        self.assertIn("tests.validation.test_workflow_configuration", plan)
        self.assertIn("scripts/validation/check_engine_plan_vectors.sh", plan)

    def test_product_checks_have_one_execution_owner(self):
        self.assertFalse((WORKFLOW_ROOT / "completion-gate.yml").exists())
        self.assertIn("agent_completion_gate.py run", self.workflow("ci.yml"))
        self.assertNotIn("cargo test", self.workflow("ci.yml"))

    def test_research_jobs_require_relevant_changes(self):
        workflow = self.workflow("research.yml")
        for job in ("video-carrier", "coreml-candidate", "texture-probe-build",
                    "styles-version-control", "style-tone-response", "system-heif-hdr-api",
                    "pvt-native-import"):
            block = re.split(r"(?m)^  [a-z][a-z0-9-]*:", workflow.split("  " + job + ":\n", 1)[1], 1)[0]
            self.assertIn("needs.changes.outputs.", block, job)

    def test_real_fixture_job_does_not_repeat_portable_unit_suite(self):
        workflow = self.workflow("rust-proxdr-real-fixtures.yml")
        self.assertNotIn("cargo test", workflow)
        self.assertIn("sha256sum -c SHA256SUMS", workflow)
        self.assertIn("Inspect every ProXDR fixture through Rust", workflow)
        self.assertIn("Convert representative real fixtures through Rust CLI", workflow)

    def test_broad_performance_diagnostics_require_manual_replay(self):
        workflow = self.workflow("performance.yml")
        for name in ("Measure isolated portable Gain Map codec primitives",
                     "Measure Apple VideoToolbox primitive separately",
                     "Compare exact Styles revisions on the same runner"):
            block = workflow.split("- name: " + name, 1)[1].split("- name:", 1)[0]
            self.assertIn("github.event_name == 'workflow_dispatch'", block)
        for name in ("Measure representative end-to-end product workloads",
                     "Characterize Apple adapter process lifecycle",
                     "Enforce stable-runner performance regression budget"):
            block = workflow.split("- name: " + name, 1)[1].split("- name:", 1)[0]
            self.assertNotIn("workflow_dispatch", block)


if __name__ == "__main__":
    unittest.main()
