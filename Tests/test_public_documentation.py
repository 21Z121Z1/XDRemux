from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[1]

CANONICAL_BILINGUAL_PAIRS = (
    ("README.en.md", "README.md"),
    ("docs/README.en.md", "docs/README.md"),
    ("docs/cli.en.md", "docs/cli.md"),
    ("docs/apple-features.en.md", "docs/apple-features.md"),
    ("docs/development.en.md", "docs/development.md"),
    ("docs/supported-devices.en.md", "docs/supported-devices.md"),
    ("docs/quality/testing.en.md", "docs/quality/testing.md"),
    ("docs/quality/evals.en.md", "docs/quality/evals.md"),
    ("docs/quality/logging.en.md", "docs/quality/logging.md"),
    ("Tests/README.en.md", "Tests/README.md"),
)

SPECIAL_BILINGUAL_PAIRS = (
    ("docs/style-guide.en.md", "docs/style-guide.md"),
)


class PublicDocumentationTests(unittest.TestCase):
    def test_current_bilingual_documents_exist_and_cross_link(self) -> None:
        for english_relative, chinese_relative in CANONICAL_BILINGUAL_PAIRS:
            english = ROOT / english_relative
            chinese = ROOT / chinese_relative
            self.assertTrue(english.is_file(), f"missing English document: {english_relative}")
            self.assertTrue(chinese.is_file(), f"missing Chinese document: {chinese_relative}")
            self.assertIn(
                f"({Path(chinese_relative).name})",
                english.read_text(encoding="utf-8"),
            )
            self.assertIn(
                f"({Path(english_relative).name})",
                chinese.read_text(encoding="utf-8"),
            )

    def test_readme_is_a_single_rust_product_entry_point(self) -> None:
        english = (ROOT / "README.en.md").read_text(encoding="utf-8")
        chinese = (ROOT / "README.md").read_text(encoding="utf-8")
        for text in (english, chinese):
            self.assertIn("xdremux convert", text)
            self.assertIn("xdremux batch", text)
            self.assertIn("xdremux categorize", text)
            self.assertIn("docs/cli", text)
            self.assertIn("docs/apple-features", text)
            self.assertIn("docs/development", text)
            self.assertIn("docs/quality/testing", text)
            self.assertNotIn("swift run xdremux", text)
            self.assertNotIn("xdremux-py", text)
            self.assertNotIn("python3 -m xdremux_py", text)

    def test_current_cli_docs_define_one_rust_product_and_automatic_source_handling(self) -> None:
        english = (ROOT / "docs/cli.en.md").read_text(encoding="utf-8")
        chinese = (ROOT / "docs/cli.md").read_text(encoding="utf-8")

        self.assertIn("one cross-platform product entry point: the Rust `xdremux` CLI", english)
        self.assertIn("detected automatically", english)
        self.assertIn("Motion Photos are automatically converted to Live Photos", english)
        self.assertIn("--oppo-compatible", english)
        self.assertIn("no longer define new CLI product semantics", english)
        self.assertNotIn("python -m xdremux_py", english)
        self.assertNotIn("swift run xdremux", english)
        self.assertNotIn("Why there is no `--family`", english)

        self.assertIn("一个跨平台 Rust CLI", chinese)
        self.assertIn("自动识别", chinese)
        self.assertIn("Motion Photo", chinese)
        self.assertIn("Live Photo", chinese)
        self.assertIn("--oppo-compatible", chinese)
        self.assertIn("不再定义新的 CLI 产品语义", chinese)
        self.assertNotIn("python -m xdremux_py", chinese)
        self.assertNotIn("swift run xdremux", chinese)
        self.assertNotIn("为什么没有 `--family`", chinese)

    def test_documentation_index_links_current_reference_documents(self) -> None:
        english = (ROOT / "docs/README.en.md").read_text(encoding="utf-8")
        chinese = (ROOT / "docs/README.md").read_text(encoding="utf-8")
        for text in (english, chinese):
            for relative in (
                "cli",
                "apple-features",
                "supported-devices",
                "development",
                "quality/testing",
                "quality/evals",
                "quality/logging",
            ):
                self.assertIn(relative, text)

    def test_no_absolute_local_paths_in_current_public_documents(self) -> None:
        absolute_path = re.compile(r"/(?:Users|home|private|tmp)/[^\s)`]+")
        for english_relative, chinese_relative in CANONICAL_BILINGUAL_PAIRS:
            for relative in (english_relative, chinese_relative):
                text = (ROOT / relative).read_text(encoding="utf-8")
                self.assertIsNone(
                    absolute_path.search(text),
                    f"current public document contains an absolute local path: {relative}",
                )

    def test_special_bilingual_documents_exist_and_cross_link(self) -> None:
        for english_relative, chinese_relative in SPECIAL_BILINGUAL_PAIRS:
            english = ROOT / english_relative
            chinese = ROOT / chinese_relative
            self.assertTrue(english.is_file(), f"missing English document: {english_relative}")
            self.assertTrue(chinese.is_file(), f"missing Chinese document: {chinese_relative}")
            self.assertIn(
                f"({Path(chinese_relative).name})",
                english.read_text(encoding="utf-8"),
            )
            self.assertIn(
                f"({Path(english_relative).name})",
                chinese.read_text(encoding="utf-8"),
            )

    def test_style_guide_defines_canonical_language_and_non_compliance_claim(self) -> None:
        english = (ROOT / "docs/style-guide.en.md").read_text(encoding="utf-8")
        self.assertIn("English is the canonical source", english)
        self.assertIn("does not claim formal ASD-STE100 compliance", english)
        for term in ("Motion Photo", "Live Photo", "Gain Map", "still-image-time"):
            self.assertIn(term, english)

    def test_current_quality_docs_acknowledge_versioned_motion_fixtures(self) -> None:
        for relative in (
            "docs/quality/testing.en.md",
            "docs/quality/evals.en.md",
            "Tests/README.en.md",
        ):
            text = (ROOT / relative).read_text(encoding="utf-8")
            self.assertIn("fixtures/", text)
            self.assertNotIn("Real samples are not in the repository", text)

    def test_documentation_workflows_reference_present_test_module(self) -> None:
        workflows = (
            ROOT / ".github" / "workflows" / "docs.yml",
            ROOT / ".github" / "workflows" / "policy.yml",
        )
        for workflow in workflows:
            text = workflow.read_text(encoding="utf-8")
            self.assertIn("python3 -m unittest Tests.test_public_documentation", text)
        self.assertTrue((ROOT / "Tests" / "test_public_documentation.py").is_file())


if __name__ == "__main__":
    unittest.main()
