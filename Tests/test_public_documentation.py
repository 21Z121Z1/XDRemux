from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[1]
ROOT_READMES = (ROOT / "README.md", ROOT / "README.en.md")
PUBLIC_DOCUMENTS = ROOT_READMES + tuple(
    ROOT / relative
    for relative in (
        "docs/README.md",
        "docs/README.en.md",
        "docs/cli.md",
        "docs/cli.en.md",
        "docs/apple-features.md",
        "docs/apple-features.en.md",
        "docs/development.md",
        "docs/development.en.md",
        "docs/supported-devices.md",
        "docs/supported-devices.en.md",
        "docs/quality/testing.md",
        "docs/quality/testing.en.md",
        "docs/quality/evals.md",
        "docs/quality/evals.en.md",
        "docs/quality/logging.md",
        "docs/quality/logging.en.md",
        "docs/validation/README.md",
        "docs/xdremux/README.md",
        "docs/xdremux/README.en.md",
        "Tests/README.md",
        "xdremux/README.md",
    )
)
# Documents published in both languages. Each pair must exist and cross-link,
# so a new document cannot ship in one language only.
BILINGUAL_STEMS = (
    "README",
    "docs/README",
    "docs/cli",
    "docs/apple-features",
    "docs/development",
    "docs/supported-devices",
    "docs/quality/testing",
    "docs/quality/evals",
    "docs/quality/logging",
    "docs/xdremux/README",
)
MARKDOWN_LINK = re.compile(r"\[[^\]]+\]\(([^)]+)\)")


class PublicDocumentationTests(unittest.TestCase):
    def test_bilingual_readmes_publish_matching_categorize_workflows(self) -> None:
        required = (
            "swift run xdremux categorize",
            "python3 xdremux/python/XDRemux.py categorize",
            "--categorize",
        )
        forbidden = ("--categorize-output", "--organize-by-mode", "xdremux classify")
        for readme in ROOT_READMES:
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

    def test_bilingual_documents_exist_in_both_languages_and_cross_link(self) -> None:
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
