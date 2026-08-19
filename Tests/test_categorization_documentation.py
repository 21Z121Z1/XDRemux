import unittest
from pathlib import Path


class CategorizationDocumentationTests(unittest.TestCase):
    def test_bilingual_readmes_use_the_same_public_commands(self) -> None:
        root = Path(__file__).resolve().parents[1]
        for name in ("README.md", "README.en.md"):
            with self.subTest(name=name):
                text = (root / name).read_text(encoding="utf-8")
                self.assertIn("swift run xdremux categorize", text)
                self.assertIn("python3 -m xdremux_py categorize", text)
                self.assertIn("--categorize", text)
                self.assertNotIn("--categorize-output", text)
                self.assertNotIn("--organize-by-mode", text)
                self.assertNotIn("xdremux classify", text)

    def test_cli_usage_exposes_only_categorize_naming(self) -> None:
        root = Path(__file__).resolve().parents[1]
        paths = (
            root / "Sources/XDRemuxCLI/Commands/XDRemuxCommand.swift",
            root / "xdremux_py/cli.py",
        )
        for path in paths:
            with self.subTest(path=path.name):
                text = path.read_text(encoding="utf-8")
                self.assertIn("categorize", text)
                self.assertNotIn("--categorize-output", text)
                self.assertNotIn("--organize-by-mode", text)


if __name__ == "__main__":
    unittest.main()
