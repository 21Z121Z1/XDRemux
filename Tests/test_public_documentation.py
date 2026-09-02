from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[1]

CANONICAL_BILINGUAL_PAIRS = (
    ("README.en.md", "README.md"),
    ("docs/cli.en.md", "docs/cli.md"),
    ("docs/apple-features.en.md", "docs/apple-features.md"),
    ("docs/troubleshooting.en.md", "docs/troubleshooting.md"),
    ("docs/quality/testing.en.md", "docs/quality/testing.md"),
    ("docs/quality/evals.en.md", "docs/quality/evals.md"),
    ("Tests/README.en.md", "Tests/README.md"),
)

SPECIAL_BILINGUAL_PAIRS = (
    ("docs/style-guide.en.md", "docs/style-guide.md"),
)

HISTORICAL_RECORDS = (
    ROOT / "docs" / "research" / "ApplePhotos17_StyleSemantics_2026-02-22.md",
    ROOT / "docs" / "research" / "PhotographicStylesRendering_BestPractices.md",
    ROOT / "docs" / "research" / "PhotographicStyles_Metadata_Key1_ReverseEngineering.md",
    ROOT / "docs" / "research" / "PortraitDepthMapping_Notes_2026-02-20.md",
    ROOT / "docs" / "research" / "ApplePhotos_StyleValidation_2026-02-18.md",
)


class PublicDocumentationTests(unittest.TestCase):
    def test_canonical_bilingual_documents_exist_and_cross_link(self) -> None:
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

    def test_readme_points_to_current_reference_documents(self) -> None:
        english = (ROOT / "README.en.md").read_text(encoding="utf-8")
        chinese = (ROOT / "README.md").read_text(encoding="utf-8")
        for text in (english, chinese):
            self.assertIn("docs/cli", text)
            self.assertIn("docs/apple-features", text)
            self.assertIn("docs/troubleshooting", text)

    def test_readme_does_not_promote_legacy_implementation_names(self) -> None:
        english = (ROOT / "README.en.md").read_text(encoding="utf-8")
        chinese = (ROOT / "README.md").read_text(encoding="utf-8")
        for text in (english, chinese):
            self.assertNotIn("python -m xdremux_py", text)
            self.assertNotIn("swift run xdremux", text)

    def test_research_index_distinguishes_current_guidance_from_historical_records(self) -> None:
        english = (ROOT / "docs" / "research" / "README.en.md").read_text(encoding="utf-8")
        chinese = (ROOT / "docs" / "research" / "README.md").read_text(encoding="utf-8")
        for text in (english, chinese):
            self.assertIn("Historical", text) if text is english else self.assertIn("历史", text)
            self.assertIn("summary", text.lower())

    def test_historical_records_are_not_linked_as_current_public_guidance(self) -> None:
        current_docs = [
            ROOT / "README.en.md",
            ROOT / "README.md",
            ROOT / "docs" / "cli.en.md",
            ROOT / "docs" / "cli.md",
            ROOT / "docs" / "apple-features.en.md",
            ROOT / "docs" / "apple-features.md",
            ROOT / "docs" / "troubleshooting.en.md",
            ROOT / "docs" / "troubleshooting.md",
        ]
        historical_names = {record.name for record in HISTORICAL_RECORDS}
        for document in current_docs:
            text = document.read_text(encoding="utf-8")
            for name in historical_names:
                self.assertNotIn(name, text, f"{document} links historical record {name} as current guidance")

    def test_no_absolute_local_paths_in_canonical_public_documents(self) -> None:
        absolute_path = re.compile(r"/(?:Users|home|private|tmp)/[^\s)`]+")
        for english_relative, chinese_relative in CANONICAL_BILINGUAL_PAIRS:
            for relative in (english_relative, chinese_relative):
                text = (ROOT / relative).read_text(encoding="utf-8")
                self.assertIsNone(
                    absolute_path.search(text),
                    f"canonical public document contains an absolute local path: {relative}",
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

    def test_historical_records_are_kept_with_current_bilingual_summaries(self) -> None:
        for record in HISTORICAL_RECORDS:
            self.assertTrue(record.is_file(), f"missing historical record: {record}")
            stem = record.with_suffix("")
            english_summary = Path(f"{stem}.summary.en.md")
            chinese_summary = Path(f"{stem}.summary.md")
            self.assertTrue(english_summary.is_file(), f"missing historical English summary: {record}")
            self.assertTrue(chinese_summary.is_file(), f"missing historical Chinese summary: {record}")

    def test_style_guide_defines_canonical_language_and_non_compliance_claim(self) -> None:
        english = (ROOT / "docs/style-guide.en.md").read_text(encoding="utf-8")
        self.assertIn("English is the canonical source", english)
        self.assertIn("does not claim formal ASD-STE100 compliance", english)
        for term in ("Motion Photo", "Live Photo", "Gain Map", "still-image-time"):
            self.assertIn(term, english)

    def test_current_cli_docs_define_one_rust_product_and_automatic_motion_photo_support(self) -> None:
        english = (ROOT / "docs/cli.en.md").read_text(encoding="utf-8")
        chinese = (ROOT / "docs/cli.md").read_text(encoding="utf-8")

        self.assertIn("one cross-platform product entry point: the Rust `xdremux` CLI", english)
        self.assertIn("Motion Photos are automatically converted to Live Photos", english)
        self.assertIn("no `--family`", english)
        self.assertIn("no longer define new CLI product semantics", english)
        self.assertNotIn("python -m xdremux_py", english)
        self.assertNotIn("swift run xdremux", english)

        self.assertIn("一个跨平台 Rust CLI", chinese)
        self.assertIn("Motion Photo", chinese)
        self.assertIn("Live Photo", chinese)
        self.assertIn("为什么没有 `--family`", chinese)
        self.assertIn("不再定义新的 CLI 产品语义", chinese)
        self.assertNotIn("python -m xdremux_py", chinese)
        self.assertNotIn("swift run xdremux", chinese)

    def test_current_quality_docs_acknowledge_versioned_motion_fixtures(self) -> None:
        for relative in (
            "docs/quality/testing.en.md",
            "docs/quality/evals.en.md",
            "Tests/README.en.md",
        ):
            text = (ROOT / relative).read_text(encoding="utf-8")
            self.assertIn("fixtures/", text)
            self.assertNotIn("Real samples are not in the repository", text)

    def test_documentation_workflows_reference_present_test_modules(self) -> None:
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
