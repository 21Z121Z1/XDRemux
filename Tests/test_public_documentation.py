from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[1]
ROOT_READMES = (ROOT / "README.md", ROOT / "README.en.md")
PUBLIC_DOCUMENTS = ROOT_READMES + tuple(
    ROOT / relative
    for relative in (
        "docs/cli.md",
        "docs/cli.en.md",
        "docs/apple-features.md",
        "docs/apple-features.en.md",
        "docs/development.md",
        "docs/development.en.md",
        "docs/supported-devices.md",
        "docs/supported-devices.en.md",
        "xdremux/README.md",
    )
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
