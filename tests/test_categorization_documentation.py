import unittest
from pathlib import Path


class CategorizationDocumentationTests(unittest.TestCase):
    def test_bilingual_readmes_use_the_canonical_command_names(self) -> None:
        root = Path(__file__).resolve().parents[1]
        for name in ("README.md", "README.en.md"):
            with self.subTest(name=name):
                text = (root / name).read_text(encoding="utf-8")
                self.assertIn("xdremux categorize", text)
                self.assertIn("--categorize", text)
                self.assertNotIn("--categorize-output", text)
                self.assertNotIn("--organize-by-mode", text)
                self.assertNotIn("xdremux classify", text)
                self.assertNotIn("python3 -m xdremux_py", text)
                self.assertNotIn("swift run xdremux", text)

    def test_rust_cli_exposes_only_categorize_naming(self) -> None:
        root = Path(__file__).resolve().parents[1]
        text = (root / "crates/xdremux-cli/src/lib.rs").read_text(encoding="utf-8")
        self.assertIn("Categorize", text)
        self.assertIn("categorize", text)
        self.assertNotIn("--categorize-output", text)
        self.assertNotIn("--organize-by-mode", text)


if __name__ == "__main__":
    unittest.main()
