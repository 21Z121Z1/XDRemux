#!/usr/bin/env python3
"""Regression tests for the repository's GitHub Actions configuration."""

from __future__ import annotations

from pathlib import Path
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
        workflow = self.workflow("completion-gate.yml")
        self.assertRegex(workflow, r"(?m)^  exact-head:\n    name: exact-head$")
        self.assertIn(
            "XDREMUX_GATE_SHA: ${{ github.event.pull_request.head.sha || github.sha }}",
            workflow,
        )
        self.assertIn(
            "XDREMUX_GATE_BASE: ${{ github.event_name == 'pull_request' && format('origin/{0}', github.event.pull_request.base.ref) || github.event.before || 'main' }}",
            workflow,
        )
        self.assertIn("ref: ${{ env.XDREMUX_GATE_SHA }}", workflow)
        self.assertIn('--base "$XDREMUX_GATE_BASE"', workflow)
        self.assertNotIn('--base "origin/$XDREMUX_GATE_BASE"', workflow)

    def test_completion_gate_pr_trigger_is_always_on_without_paths(self) -> None:
        workflow = self.workflow("completion-gate.yml")
        pull_request = self.event_block(workflow, "pull_request")
        self.assertEqual(pull_request.strip(), "")

    def test_required_workflow_pushes_are_main_only(self) -> None:
        workflows = (
            "ci.yml",
            "completion-gate.yml",
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
        path_filtered = (
            "ci.yml",
            "performance.yml",
            "rust-cli-core.yml",
            "rust-proxdr-real-fixtures.yml",
        )
        for name in path_filtered:
            with self.subTest(workflow=name):
                push = self.event_block(self.workflow(name), "push")
                self.assertRegex(push, r"(?m)^    paths(?:-ignore)?:")

    def test_workflow_set_stays_minimal(self) -> None:
        expected = {
            "ci.yml",
            "completion-gate.yml",
            "performance.yml",
            "rust-cli-core.yml",
            "rust-proxdr-real-fixtures.yml",
        }
        actual = {path.name for path in WORKFLOW_ROOT.glob("*.yml")}
        self.assertEqual(actual, expected)

    def test_workflows_with_new_concurrency_policy_have_the_shared_group(self) -> None:
        workflows = (
            "completion-gate.yml",
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

    def test_performance_workflow_does_not_duplicate_completion(self) -> None:
        workflow = self.workflow("performance.yml")
        self.assertIn("scripts/benchmark_rust_product.py", workflow)
        self.assertIn("scripts/check_performance_budget.py", workflow)
        self.assertNotIn("scripts/agent_completion_gate.py", workflow)
        self.assertNotIn("scripts/check_rust_cli_smoke.sh", workflow)
        self.assertNotIn("XDRemuxAppModelTests", workflow)

    def test_completion_gate_owns_repository_policy_regressions(self) -> None:
        completion_gate = self.workflow("completion-gate.yml")
        self.assertIn("Tests.validation.test_agent_completion_gate", completion_gate)
        self.assertIn("Tests.validation.test_workflow_configuration", completion_gate)
        self.assertIn("scripts/check_engine_plan_vectors.sh", completion_gate)


if __name__ == "__main__":
    unittest.main()
