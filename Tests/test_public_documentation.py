from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[1]

# Current normative documents. Historical evidence records are intentionally not
# rewritten or required to have a translated body.
BILINGUAL_STEMS = (
    "README",
    "docs/README",
    "docs/style-guide",
    "docs/cli",
    "docs/apple-features",
    "docs/development",
    "docs/supported-devices",
    "docs/quality/testing",
    "docs/quality/evals",
    "docs/quality/logging",
    "docs/validation/README",
    "docs/xdremux/README",
    "Tests/README",
    "fixtures/README",
    "Models/ReverseKey1Ensemble.model-card",
    "docs/validation/encoding-quality-pareto-20260718.summary",
    "docs/validation/vendor-live-photo-geometry.summary",
    "docs/xdremux/iso-conformance-audit-20260511.summary",
)

# AGENTS.md is a tool-discovered fixed filename, so English remains at the
# conventional path and Chinese is published as a sidecar.
SPECIAL_BILINGUAL_PAIRS = (
    ("AGENTS.md", "AGENTS.zh-CN.md"),
)

PUBLIC_DOCUMENTS = tuple(
    path
    for stem in BILINGUAL_STEMS
    for path in (ROOT / f"{stem}.md", ROOT / f"{stem}.en.md")
) + tuple(
    ROOT / path
    for pair in SPECIAL_BILINGUAL_PAIRS
    for path in pair
)

HISTORICAL_RECORDS = (
    ROOT / "docs/validation/encoding-quality-pareto-20260718.md",
    ROOT / "docs/validation/vendor-live-photo-geometry.md",
    ROOT / "docs/xdremux/iso-conformance-audit-20260511.md",
)

MARKDOWN_LINK = re.compile(r"\[[^\]]+\]\(([^)]+)\)")


class PublicDocumentationTests(unittest.TestCase):
    def test_bilingual_readmes_publish_matching_categorize_workflows(self) -> None:
        required = (
            "swift run xdremux categorize",
            "python3 -m xdremux_py categorize",
            "--categorize",
        )
        forbidden = ("--categorize-output", "--organize-by-mode", "xdremux classify")
        for readme in (ROOT / "README.md", ROOT / "README.en.md"):
            text = readme.read_text(encoding="utf-8")
            for value in required:
                self.assertIn(value, text, f"{value} missing from {readme.name}")
            for value in forbidden:
                self.assertNotIn(value, text, f"legacy naming in {readme.name}: {value}")

    def test_local_links_in_public_documents_resolve_inside_the_repository(self) -> None:
        for document in PUBLIC_DOCUMENTS:
            self.assertTrue(document.is_file(), f"missing public document: {document}")
            for match in MARKDOWN_LINK.finditer(document.read_text(encoding="utf-8")):
                target = match.group(1).strip().strip("<>")
                if target.startswith(("https://", "http://", "mailto:", "#")):
                    continue
                target = target.split("#", maxsplit=1)[0]
                if not target:
                    continue
                resolved = (document.parent / target).resolve()
                try:
                    resolved.relative_to(ROOT)
                except ValueError:
                    self.fail(f"{document}: link escapes repository: {target}")
                self.assertTrue(resolved.exists(), f"{document}: missing link target {target}")

    def test_standard_bilingual_documents_exist_and_cross_link(self) -> None:
        for stem in BILINGUAL_STEMS:
            chinese = ROOT / f"{stem}.md"
            english = ROOT / f"{stem}.en.md"
            self.assertTrue(chinese.is_file(), f"missing Chinese document: {stem}.md")
            self.assertTrue(english.is_file(), f"missing English document: {stem}.en.md")

            name = Path(stem).name
            self.assertIn(
                f"({name}.en.md)",
                chinese.read_text(encoding="utf-8"),
                f"{stem}.md does not link to its English version",
            )
            self.assertIn(
                f"({name}.md)",
                english.read_text(encoding="utf-8"),
                f"{stem}.en.md does not link to its Chinese version",
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

    def test_current_cli_docs_describe_python_motion_photo_support(self) -> None:
        english = (ROOT / "docs/cli.en.md").read_text(encoding="utf-8")
        chinese = (ROOT / "docs/cli.md").read_text(encoding="utf-8")
        self.assertIn("Motion Photo to Live Photo conversion", english)
        self.assertNotIn("does HDR conversion only", english)
        self.assertIn("Motion Photo", chinese)
        self.assertIn("Live Photo", chinese)

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
