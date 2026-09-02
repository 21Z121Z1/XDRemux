from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]


class ProductOwnershipTests(unittest.TestCase):
    def test_python_tooling_does_not_publish_a_second_cli(self) -> None:
        pyproject = (ROOT / "pyproject.toml").read_text(encoding="utf-8")
        self.assertIn('name = "xdremux-python-tools"', pyproject)
        self.assertNotIn("[project.scripts]", pyproject)
        self.assertNotIn('project.entry-points."pipx.run"', pyproject)
        self.assertNotIn("Environment :: Console", pyproject)
        self.assertNotIn("End Users/Desktop", pyproject)

        for relative in (
            "xdremux_py/__main__.py",
            "xdremux_py/cli.py",
            "xdremux_py/commands.py",
        ):
            self.assertFalse((ROOT / relative).exists(), f"legacy Python product entry point returned: {relative}")

    def test_swift_package_vends_only_apple_platform_artifacts(self) -> None:
        manifest = (ROOT / "Package.swift").read_text(encoding="utf-8")
        self.assertIn('.library(name: "XDRemuxAppleFeatures"', manifest)
        self.assertIn('.executable(name: "xdremux-apple-adapter"', manifest)
        self.assertNotIn('.library(name: "XDRemuxCore"', manifest)
        self.assertNotIn('.executable(name: "xdremux"', manifest)
        self.assertIn('name: "XDRemuxCore"', manifest)
        self.assertIn('name: "XDRemuxCLI"', manifest)
        self.assertIn("Migration-only", manifest)
        self.assertIn("not a user-facing CLI", manifest)

    def test_development_docs_name_rust_as_the_only_product_core(self) -> None:
        english = (ROOT / "docs/development.en.md").read_text(encoding="utf-8")
        chinese = (ROOT / "docs/development.md").read_text(encoding="utf-8")

        self.assertIn("one product core: the Rust workspace", english)
        self.assertIn("The only public CLI", english)
        self.assertIn("It does not install a CLI", english)
        self.assertNotIn("The installed console command is `xdremux-py`", english)
        self.assertNotIn("Use `XDRemuxCore` when you need the standard conversion pipeline", english)

        self.assertIn("只有一个产品核心：Rust workspace", chinese)
        self.assertIn("唯一公开 CLI", chinese)
        self.assertIn("它不再安装 CLI", chinese)
        self.assertNotIn("安装后的命令是 `xdremux-py`", chinese)
        self.assertNotIn("标准转换链路使用 `XDRemuxCore`", chinese)

    def test_canonical_rust_cli_still_exists(self) -> None:
        self.assertTrue((ROOT / "crates/xdremux-cli/Cargo.toml").is_file())
        self.assertTrue((ROOT / "crates/xdremux-runtime/Cargo.toml").is_file())
        self.assertTrue((ROOT / "crates/xdremux-engine/Cargo.toml").is_file())


if __name__ == "__main__":
    unittest.main()
