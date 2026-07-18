from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[1]
ROOT_READMES = [ROOT / "README.md", ROOT / "README.en.md"]
PUBLIC_DOCUMENTS = ROOT_READMES + [
    ROOT / "docs" / "README.md",
    ROOT / "docs" / "README.en.md",
    ROOT / "docs" / "cli.md",
    ROOT / "docs" / "cli.en.md",
    ROOT / "docs" / "apple-features.md",
    ROOT / "docs" / "apple-features.en.md",
    ROOT / "docs" / "development.md",
    ROOT / "docs" / "development.en.md",
    ROOT / "docs" / "supported-devices.md",
    ROOT / "docs" / "supported-devices.en.md",
    ROOT / "docs" / "xdremux" / "README.md",
    ROOT / "docs" / "xdremux" / "README.en.md",
    ROOT / "xdremux" / "swift-cli" / "README.md",
]
MARKDOWN_LINK = re.compile(r"\[[^\]]+\]\(([^)]+)\)")


class PublicDocumentationTests(unittest.TestCase):
    def test_root_readmes_remain_product_focused(self) -> None:
        forbidden = (
            "XDRemuxCore",
            "XDRemuxAppleFeatures",
            "xdremux-dev",
            "XDRemuxSemanticHelper",
            "rear.depth.config",
            "CalFocusDepthEngine",
            "branchEvidence",
            "ScreenCaptureKit",
            "controlled_corpus_fit",
        )
        for readme in ROOT_READMES:
            text = readme.read_text(encoding="utf-8")
            self.assertLessEqual(len(text.splitlines()), 150, readme)
            for term in forbidden:
                self.assertNotIn(term, text, f"{term} leaked into {readme.name}")

    def test_root_readmes_link_to_split_documentation(self) -> None:
        chinese = ROOT_READMES[0].read_text(encoding="utf-8")
        english = ROOT_READMES[1].read_text(encoding="utf-8")
        for path in (
            "docs/cli.md",
            "docs/apple-features.md",
            "docs/development.md",
            "docs/supported-devices.md",
        ):
            self.assertIn(path, chinese)
        for path in (
            "docs/cli.en.md",
            "docs/apple-features.en.md",
            "docs/development.en.md",
            "docs/supported-devices.en.md",
        ):
            self.assertIn(path, english)

    def test_cli_reference_covers_every_public_option(self) -> None:
        options = (
            "--input",
            "--output",
            "--input-dir",
            "--output-dir",
            "--glob",
            "--jobs",
            "--overwrite",
            "--discard-portrait-data",
            "--oppo-compatible",
            "--apple-photographic-styles",
            "--apple-portrait",
            "--quiet",
            "--verbose",
            "--debug",
            "--format",
            "--language",
        )
        for name in ("cli.md", "cli.en.md"):
            text = (ROOT / "docs" / name).read_text(encoding="utf-8")
            for option in options:
                self.assertIn(option, text, f"{option} missing from {name}")

    def test_apple_product_docs_do_not_contain_research_logs(self) -> None:
        forbidden = (
            "CalFocusDepthEngine",
            "branchEvidence",
            "ControlLogicForXHLRB",
            "0x0190",
            "ScreenCaptureKit",
            "controlled_corpus_fit",
        )
        for name in ("apple-features.md", "apple-features.en.md"):
            text = (ROOT / "docs" / name).read_text(encoding="utf-8")
            for term in forbidden:
                self.assertNotIn(term, text, f"{term} leaked into {name}")

    def test_legacy_cli_readme_is_only_a_compatibility_pointer(self) -> None:
        path = ROOT / "xdremux" / "swift-cli" / "README.md"
        text = path.read_text(encoding="utf-8")
        self.assertLessEqual(len(text.splitlines()), 40)
        self.assertIn("../../docs/cli.en.md", text)
        self.assertNotIn("## Public commands and options", text)
        self.assertNotIn("## Apple portrait conversion", text)

    def test_local_links_in_public_documents_resolve(self) -> None:
        for document in PUBLIC_DOCUMENTS:
            text = document.read_text(encoding="utf-8")
            for match in MARKDOWN_LINK.finditer(text):
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


if __name__ == "__main__":
    unittest.main()
