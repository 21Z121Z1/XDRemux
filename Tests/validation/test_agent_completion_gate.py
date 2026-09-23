#!/usr/bin/env python3
"""Regression tests for scripts/agent_completion_gate.py."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
GATE = REPOSITORY_ROOT / "scripts" / "agent_completion_gate.py"


class CompletionGateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.repo = Path(self.temporary.name)
        self.git("init", "-q")
        self.git("config", "user.name", "Completion Gate Test")
        self.git("config", "user.email", "gate@example.invalid")
        (self.repo / "README.md").write_text("base\n", encoding="utf-8")
        self.commit_all("base")
        self.base = self.git("rev-parse", "HEAD").stdout.strip()

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def git(self, *arguments: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["git", *arguments],
            cwd=self.repo,
            text=True,
            capture_output=True,
            check=True,
        )

    def commit_all(self, message: str) -> None:
        self.git("add", "--all")
        self.git("commit", "-q", "-m", message)

    def write_plan(self, checks: list[dict[str, object]]) -> Path:
        path = self.repo / "plan.json"
        path.write_text(
            json.dumps({"schema_version": 1, "scope": "gate regression", "checks": checks}),
            encoding="utf-8",
        )
        return path

    def run_gate(self, plan: Path, receipt: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(GATE),
                "run",
                "--base",
                self.base,
                "--plan",
                str(plan),
                "--receipt",
                str(receipt),
            ],
            cwd=self.repo,
            text=True,
            capture_output=True,
            check=False,
        )

    @staticmethod
    def passing_check(name: str, kind: str) -> dict[str, object]:
        return {
            "name": name,
            "kind": kind,
            "command": [sys.executable, "-c", "print('ok')"],
            "timeout_seconds": 30,
        }

    def add_tracked_file(self, name: str, contents: str = "// source\n") -> None:
        path = self.repo / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(contents, encoding="utf-8")
        self.commit_all("add " + repr(name))

    def complete_plan(self) -> Path:
        return self.write_plan([
            self.passing_check("regression", "regression"),
            self.passing_check("functional", "functional"),
        ])

    def test_rust_product_requires_regression_and_functional_evidence(self) -> None:
        self.add_tracked_file("crates/xdremux-runtime/src/lib.rs")
        receipt = self.repo / "receipt.json"
        result = self.run_gate(self.write_plan([self.passing_check("static", "static")]), receipt)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("regression", result.stderr)
        result = self.run_gate(self.write_plan([self.passing_check("regression", "regression")]), receipt)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("functional, integration, or device", result.stderr)
        result = self.run_gate(self.complete_plan(), receipt)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(json.loads(receipt.read_text())["policy"],
                         {"production_changed": True, "source_changed": True})

    def test_build_inputs_require_regression_and_functional_evidence(self) -> None:
        for name in ("Cargo.toml", "Cargo.lock", "Package.swift", "Package.resolved",
                     "rust-toolchain", "rust-toolchain.toml", "build.rs", ".cargo/config.toml",
                     "crates/xdremux-runtime/Cargo.toml",
                     "apps/macos/XDRemuxApp/XDRemuxApp.xcodeproj/project.pbxproj"):
            with self.subTest(path=name):
                self.base = self.git("rev-parse", "HEAD").stdout.strip()
                self.add_tracked_file(name)
                receipt = self.repo / "receipt.json"
                result = self.run_gate(self.write_plan([self.passing_check("static", "static")]), receipt)
                self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                self.assertIn("regression", result.stderr)
                result = self.run_gate(self.write_plan([self.passing_check("regression", "regression")]), receipt)
                self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                self.assertIn("functional, integration, or device", result.stderr)

    def test_research_rust_is_source_not_product(self) -> None:
        self.add_tracked_file("research/probe.rs")
        receipt = self.repo / "receipt.json"
        result = self.run_gate(self.write_plan([self.passing_check("static", "static")]), receipt)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        result = self.run_gate(self.write_plan([self.passing_check("regression", "regression")]), receipt)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(json.loads(receipt.read_text())["policy"],
                         {"production_changed": False, "source_changed": True})

    def test_deleted_rust_source_requires_functional_evidence(self) -> None:
        name = "crates/xdremux-runtime/src/lib.rs"
        self.add_tracked_file(name)
        self.base = self.git("rev-parse", "HEAD").stdout.strip()
        (self.repo / name).unlink()
        self.commit_all("remove product owner")
        result = self.run_gate(self.write_plan([self.passing_check("regression", "regression")]),
                               self.repo / "receipt.json")
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("functional, integration, or device", result.stderr)

    def test_moves_account_for_both_ownership_endpoints(self) -> None:
        for index, (before, after) in enumerate((("crates/from.rs", "research/to.rs"),
                                                ("research/from.rs", "crates/to.rs"))):
            with self.subTest(before=before, after=after):
                self.add_tracked_file(before, f"// distinct file {index}\n")
                self.base = self.git("rev-parse", "HEAD").stdout.strip()
                (self.repo / after).parent.mkdir(parents=True, exist_ok=True)
                self.git("mv", before, after)
                self.commit_all("move source")
                # Rename detection must not hide a removed product owner.
                self.git("config", "diff.renames", "true")
                receipt = self.repo / "receipt.json"
                result = self.run_gate(self.write_plan([self.passing_check("regression", "regression")]), receipt)
                self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                result = self.run_gate(self.complete_plan(), receipt)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(set(json.loads(receipt.read_text())["changed_files"]), {before, after})

    def test_changed_paths_are_nul_delimited_not_display_quoted(self) -> None:
        names = ['crates/space and "quote".rs', "crates/new\nline.rs", "crates/tab\tname.rs",
                 "research/中文.rs", " leading-space.py"]
        if os.name == "posix":
            names.append(os.fsdecode(b"crates/non-utf8-\xff.rs"))
        for name in names:
            self.add_tracked_file(name)
        receipt = self.repo / "receipt.json"
        result = self.run_gate(self.complete_plan(), receipt)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(set(json.loads(receipt.read_text())["changed_files"]), set(names))
        result = self.run_gate(self.write_plan([self.passing_check("static", "static")]), receipt)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)

    def test_dirty_status_preserves_leading_columns_and_embedded_newlines(self) -> None:
        name = 'a\n"quoted".txt'
        self.add_tracked_file(name, "before\n")
        (self.repo / name).write_text("dirty\n")
        (self.repo / "README.md").write_text("also dirty\n")
        receipt = self.repo / "receipt.json"
        result = self.run_gate(self.write_plan([self.passing_check("static", "static")]), receipt)
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        record = json.loads(receipt.read_text())
        self.assertEqual(set(record["initial_tracked_status"]), {" M " + name, " M README.md"})
        self.assertEqual(record["initial_tracked_status"], record["final_tracked_status"])
        self.assertFalse(record["builtins"]["initial_tracked_tree_clean"])
        self.assertFalse(record["builtins"]["final_tracked_tree_clean"])

    def test_docs_change_passes_and_receipt_verifies(self) -> None:
        (self.repo / "README.md").write_text("updated\n", encoding="utf-8")
        self.commit_all("docs")
        receipt = self.repo / "receipt.json"
        result = self.run_gate(
            self.write_plan([self.passing_check("docs-static", "static")]),
            receipt,
        )
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        verify = subprocess.run(
            [sys.executable, str(GATE), "verify", str(receipt)],
            cwd=self.repo,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(verify.returncode, 0, verify.stderr + verify.stdout)

    def test_source_change_requires_regression_check(self) -> None:
        source = self.repo / "scripts" / "tool.py"
        source.parent.mkdir()
        source.write_text("print('tool')\n", encoding="utf-8")
        self.commit_all("source")
        result = self.run_gate(
            self.write_plan([self.passing_check("static-only", "static")]),
            self.repo / "receipt.json",
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("regression check", result.stderr)

    def test_production_change_requires_functional_evidence(self) -> None:
        source = self.repo / "xdremux" / "tool.py"
        source.parent.mkdir()
        source.write_text("print('tool')\n", encoding="utf-8")
        self.commit_all("production")
        result = self.run_gate(
            self.write_plan([self.passing_check("regression", "regression")]),
            self.repo / "receipt.json",
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("functional, integration, or device", result.stderr)

    def test_production_change_passes_with_regression_and_functional_checks(self) -> None:
        source = self.repo / "xdremux" / "tool.py"
        source.parent.mkdir()
        source.write_text("print('tool')\n", encoding="utf-8")
        self.commit_all("production")
        checks = [
            self.passing_check("regression", "regression"),
            self.passing_check("real-sample", "functional"),
        ]
        result = self.run_gate(self.write_plan(checks), self.repo / "receipt.json")
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)

    def test_failed_check_writes_failed_receipt(self) -> None:
        (self.repo / "README.md").write_text("updated\n", encoding="utf-8")
        self.commit_all("docs")
        plan = self.write_plan(
            [{
                "name": "failure",
                "kind": "static",
                "command": [sys.executable, "-c", "raise SystemExit(7)"],
                "timeout_seconds": 30,
            }]
        )
        receipt = self.repo / "receipt.json"
        result = self.run_gate(plan, receipt)
        self.assertEqual(result.returncode, 1)
        self.assertFalse(json.loads(receipt.read_text(encoding="utf-8"))["passed"])

    def test_new_commit_invalidates_existing_receipt(self) -> None:
        (self.repo / "README.md").write_text("updated\n", encoding="utf-8")
        self.commit_all("docs")
        receipt = self.repo / "receipt.json"
        result = self.run_gate(
            self.write_plan([self.passing_check("docs-static", "static")]),
            receipt,
        )
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        (self.repo / "README.md").write_text("changed again\n", encoding="utf-8")
        self.commit_all("later")
        verify = subprocess.run(
            [sys.executable, str(GATE), "verify", str(receipt)],
            cwd=self.repo,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(verify.returncode, 1)
        self.assertIn("stale", verify.stdout)


if __name__ == "__main__":
    unittest.main()
