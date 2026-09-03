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
        self.assertIn('.executable(name: "xdremux-apple-adapter"', manifest)
        for legacy in (
            "XDRemuxCore",
            "XDRemuxAppleFeatures",
            "XDRemuxCLI",
            "FormatConformanceOracle",
            "MetadataConformanceOracle",
            "CoreImageRAWDiagnostics",
            "swift-argument-parser",
        ):
            self.assertNotIn(legacy, manifest)

    def test_swift_sources_are_limited_to_the_framework_adapter(self) -> None:
        source_root = ROOT / "Sources"
        swift_sources = sorted(source_root.rglob("*.swift"))
        self.assertTrue(swift_sources)
        self.assertTrue(all("XDRemuxAppleAdapter" in path.parts for path in swift_sources))

    def test_python_package_contains_only_research_tooling(self) -> None:
        package_root = ROOT / "xdremux_py"
        allowed = {"__init__.py", "apple_reverse_key1_training.py"}
        self.assertEqual({path.name for path in package_root.glob("*.py")}, allowed)

    def test_development_docs_name_rust_as_the_only_product_core(self) -> None:
        english = (ROOT / "docs/development.en.md").read_text(encoding="utf-8")
        chinese = (ROOT / "docs/development.md").read_text(encoding="utf-8")

        self.assertIn("one product core: the Rust workspace", english)
        self.assertIn("The only public CLI", english)
        self.assertIn("It does not install a CLI", english)
        self.assertNotIn("The installed console command is `xdremux-py`", english)
        self.assertNotIn("XDRemuxCore", english)

        self.assertIn("只有一个产品核心：Rust workspace", chinese)
        self.assertIn("唯一公开 CLI", chinese)
        self.assertIn("它不再安装 CLI", chinese)
        self.assertNotIn("安装后的命令是 `xdremux-py`", chinese)
        self.assertNotIn("XDRemuxCore", chinese)

    def test_apple_adapter_is_wired_only_at_runtime_composition_root(self) -> None:
        runtime_source = ROOT / "crates/xdremux-runtime/src"
        root = (runtime_source / "lib.rs").read_text(encoding="utf-8")
        self.assertIn("mod apple_adapter;", root)
        self.assertNotIn("pub mod apple_adapter;", root)

        hidden_import = '#[path = "apple_adapter.rs"]'
        for source in sorted(runtime_source.glob("*.rs")):
            if source.name in {"lib.rs", "apple_adapter.rs"}:
                continue
            text = source.read_text(encoding="utf-8")
            self.assertNotIn(
                hidden_import,
                text,
                f"Apple adapter composition must stay visible in runtime/src/lib.rs, not {source.name}",
            )

    def test_canonical_rust_cli_still_exists(self) -> None:
        self.assertTrue((ROOT / "crates/xdremux-cli/Cargo.toml").is_file())
        self.assertTrue((ROOT / "crates/xdremux-runtime/Cargo.toml").is_file())
        self.assertTrue((ROOT / "crates/xdremux-engine/Cargo.toml").is_file())


if __name__ == "__main__":
    unittest.main()
