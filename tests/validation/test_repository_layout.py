"""Portable checks for directory ownership and runnable repository entrypoints."""

from __future__ import annotations

import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest
from urllib.parse import unquote, urlsplit


ROOT = Path(__file__).resolve().parents[2]
SCRIPT_GROUPS = {"apple", "ci", "diagnostics", "distribution", "performance", "validation"}
HARNESSES = (
    "scripts/validation/verify_batch_categorize_idempotence.sh",
    "scripts/validation/verify_error_messages.sh",
    "scripts/validation/verify_validate_only_harness.sh",
)


def tracked_paths() -> set[str]:
    result = subprocess.run(
        ["git", "ls-files", "-z"], cwd=ROOT, capture_output=True, check=True
    )
    return {os.fsdecode(path) for path in result.stdout.split(b"\0") if path}


class RepositoryLayoutTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.paths = tracked_paths()

    def test_apple_adapter_has_one_platform_owner(self) -> None:
        self.assertNotIn("Package.swift", self.paths)
        self.assertFalse(any(path.startswith("Sources/") for path in self.paths))
        self.assertIn("platforms/apple/Package.swift", self.paths)
        self.assertIn("platforms/apple/Sources/XDRemuxAppleAdapter/main.swift", self.paths)
        self.assertIn("apps/macos/XDRemuxApp/project.yml", self.paths)

    def test_apple_build_references_use_explicit_package_boundary(self) -> None:
        old_source = re.compile(r"(?<![\w/])Sources/" + r"XDRemuxAppleAdapter/")
        for path in sorted(self.paths):
            if not path.endswith((".sh", ".yml", ".md", ".pbxproj")) or path.startswith("docs/history/"):
                continue
            content = (ROOT / path).read_text(encoding="utf-8")
            with self.subTest(path=path):
                self.assertIsNone(old_source.search(content))
                for command in re.findall(r"swift build[^\n\\]*", content):
                    self.assertIn("--package-path", command)

    def test_paths_do_not_collide_on_case_insensitive_filesystems(self) -> None:
        folded: dict[str, str] = {}
        for path in sorted(self.paths):
            self.assertNotIn(path.casefold(), folded, f"case collision: {path}")
            folded[path.casefold()] = path

    def test_support_code_has_one_documented_owner(self) -> None:
        for path in sorted(self.paths):
            with self.subTest(path=path):
                self.assertFalse(path.startswith(("Tests/", "Models/", "docs/xdremux/")))
                self.assertFalse(path.startswith("tests/validation/") and path.endswith(".sh"))
                parts = Path(path).parts
                if parts[0] == "scripts" and path.endswith((".sh", ".py")):
                    if path != "scripts/__init__.py":
                        self.assertGreaterEqual(len(parts), 3)
                        self.assertIn(parts[1], SCRIPT_GROUPS)

    def test_workflow_and_script_references_use_exact_tracked_case(self) -> None:
        reference = re.compile(
            r"(?<![\w/])((?:scripts|tests|research)/[\w./-]+\.(?:sh|py|swift|tsv))\b"
        )
        for path in sorted(self.paths):
            if not (path.endswith((".sh", ".yml")) and
                    path.startswith(("scripts/", "apps/", ".github/workflows/"))):
                continue
            for target in reference.findall((ROOT / path).read_text(encoding="utf-8")):
                with self.subTest(source=path, target=target):
                    self.assertIn(target, self.paths, "missing or incorrectly cased entrypoint")

    def test_current_commands_do_not_use_pre_migration_test_paths(self) -> None:
        stale = re.compile(r"\bdiscover\b[^\n]*\s-s\s+Tests\b|(?<![\w/])Tests/fixtures/")
        for path in sorted(self.paths):
            if not path.endswith((".md", ".sh", ".yml")) or path.startswith("docs/history/"):
                continue
            with self.subTest(path=path):
                self.assertIsNone(stale.search((ROOT / path).read_text(encoding="utf-8")))

    def test_active_document_links_resolve_with_exact_case(self) -> None:
        link = re.compile(r"(?<!!)\[[^\]]*\]\(([^)\s]+)\)")
        for path in sorted(self.paths):
            if not path.endswith(".md") or path == "docs/history/iso-conformance-audit-20260511.md":
                continue
            for target in link.findall((ROOT / path).read_text(encoding="utf-8")):
                url = urlsplit(target.strip("<>"))
                if url.scheme or url.netloc or not url.path:
                    continue
                resolved = (ROOT / path).parent.joinpath(unquote(url.path)).resolve()
                with self.subTest(source=path, target=target):
                    self.assertTrue(resolved.is_relative_to(ROOT), "link leaves repository")
                    relative = resolved.relative_to(ROOT).as_posix()
                    self.assertTrue(
                        relative in self.paths or relative == "." or
                        any(item.startswith(relative.rstrip("/") + "/") for item in self.paths),
                        "missing or incorrectly cased documentation target",
                    )

    def test_new_research_tools_and_public_fixtures_are_not_ignored(self) -> None:
        paths = (
            "research/oppo_styles/tools/new_tool.py",
            "fixtures/new.heic",
            "fixtures/motion-photo/new.heic",
        )
        result = subprocess.run(
            ["git", "check-ignore", "--no-index", "--stdin"], cwd=ROOT,
            input="\n".join(paths) + "\n", text=True, capture_output=True, check=False,
        )
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual(result.stdout, "")

    def test_local_scratch_and_non_fixture_media_remain_ignored(self) -> None:
        paths = ("tools/scratch.py", "private/new.heic")
        result = subprocess.run(
            ["git", "check-ignore", "--no-index", "--stdin"], cwd=ROOT,
            input="\n".join(paths) + "\n", text=True, capture_output=True, check=True,
        )
        self.assertEqual(set(result.stdout.splitlines()), set(paths))

    @unittest.skipUnless(shutil.which("bash"), "bash is required for shell entrypoints")
    def test_shell_entrypoints_parse(self) -> None:
        for path in sorted(self.paths):
            if path.endswith(".sh"):
                with self.subTest(path=path):
                    result = subprocess.run(
                        ["bash", "-n", str(ROOT / path)], capture_output=True, text=True,
                        timeout=10, check=False,
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)

    @unittest.skipUnless(shutil.which("bash"), "bash is required for shell entrypoints")
    def test_validation_builds_resolve_the_manifest_outside_repository_cwd(self) -> None:
        # A failing test double stops before conversion. This proves path routing,
        # not Rust conversion or native-framework behavior.
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            cargo = work / "cargo"
            cargo.write_text('#!/bin/sh\nprintf "%s\\n" "$PWD" "$@" > "$LAYOUT_CALL_LOG"\nexit 37\n')
            cargo.chmod(0o755)
            (work / "sample.heic").write_bytes(b"path-routing input only")
            log = work / "call.log"
            env = dict(os.environ, PATH=f"{work}{os.pathsep}{os.environ.get('PATH', '')}",
                       XDREMUX_CLI=str(work / "not-built"), LAYOUT_CALL_LOG=str(log))
            for path in HARNESSES:
                with self.subTest(path=path):
                    result = subprocess.run(
                        ["bash", str(ROOT / path), "sample.heic"], cwd=work, env=env,
                        capture_output=True, text=True, timeout=10, check=False,
                    )
                    self.assertEqual(result.returncode, 37, result.stdout + result.stderr)
                    arguments = log.read_text().splitlines()
                    self.assertEqual(Path(arguments[0]).resolve(), work.resolve())
                    index = arguments.index("--manifest-path")
                    self.assertEqual(Path(arguments[index + 1]), ROOT / "Cargo.toml")

    @unittest.skipUnless(shutil.which("bash"), "bash is required for shell entrypoints")
    def test_app_model_launcher_resolves_project_root_after_relocation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            xcodebuild = work / "xcodebuild"
            xcodebuild.write_text('#!/bin/sh\nprintf "%s\\n" "$PWD" "$@" > "$LAYOUT_CALL_LOG"\nexit 37\n')
            xcodebuild.chmod(0o755)
            log = work / "call.log"
            env = dict(os.environ, PATH=f"{work}{os.pathsep}{os.environ.get('PATH', '')}",
                       XDREMUX_APP_MODEL_DERIVED_DATA=str(work / "DerivedData"),
                       LAYOUT_CALL_LOG=str(log))
            result = subprocess.run(
                ["bash", str(ROOT / "apps/macos/XDRemuxApp/scripts/verify_model_tests.sh")],
                cwd=work, env=env, capture_output=True, text=True, timeout=10, check=False,
            )
            self.assertEqual(result.returncode, 37, result.stdout + result.stderr)
            arguments = log.read_text().splitlines()
            self.assertEqual(Path(arguments[0]).resolve(), ROOT)
            index = arguments.index("-project")
            self.assertIn(arguments[index + 1] + "/project.pbxproj", self.paths)


if __name__ == "__main__":
    unittest.main()
