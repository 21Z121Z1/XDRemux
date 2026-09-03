from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
APP_SOURCE_ROOT = ROOT / "apps" / "macos" / "XDRemuxApp" / "Sources"
APP_SOURCES = {
    path.name: path.read_text(encoding="utf-8")
    for path in sorted(APP_SOURCE_ROOT.glob("*.swift"))
}
BRIDGE = APP_SOURCES["RustAppConversionBridge.swift"]
PROJECT = (ROOT / "apps" / "macos" / "XDRemuxApp" / "project.yml").read_text(
    encoding="utf-8"
)
CI_WORKFLOW = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")


class SwiftAppArchitectureTests(unittest.TestCase):
    def test_app_uses_rust_cli_without_swift_conversion_engines(self) -> None:
        combined = "\n".join(APP_SOURCES.values())
        for forbidden in (
            "import XDRemuxCLI",
            "ConversionEngine.convert(request)",
            "AppleFeatureConversionEngine.convert(request)",
            "import XDRemuxCore",
            "import XDRemuxAppleFeatures",
        ):
            self.assertNotIn(forbidden, combined)
        self.assertIn("Process()", APP_SOURCES["RustCLIClient.swift"])
        self.assertIn("RustCLIClient.convert", BRIDGE)
        self.assertIn("RustCLIClient.isValidOutput", BRIDGE)

    def test_categorization_ui_uses_rust_cli_receipts(self) -> None:
        model = APP_SOURCES["PhotoCategorizationViewModel.swift"]
        view = APP_SOURCES["PhotoCategorizationView.swift"]
        queue = APP_SOURCES["XDRemuxViewModel.swift"]
        for source in (model, view, queue):
            self.assertNotIn("import XDRemuxCore", source)
            self.assertNotIn("import XDRemuxAppleFeatures", source)
        self.assertIn("RustCLIClient.categorize", model)
        self.assertIn("RustCLIClient.classify", queue)

    def test_app_and_model_tools_have_no_swift_product_dependencies(self) -> None:
        self.assertNotIn("product: XDRemuxCore", PROJECT)
        self.assertNotIn("product: XDRemuxAppleFeatures", PROJECT)
        self.assertIn("xdremux-apple-adapter", PROJECT)
        self.assertIn("XDRemuxAppModelTests:", PROJECT)
        self.assertIn("XDRemuxAppConversionSmoke:", PROJECT)
        self.assertIn("PhotoCategorizationView.swift", PROJECT)

    def test_app_build_bundles_the_rust_product_and_adapter(self) -> None:
        self.assertIn("Embed Rust product and Apple adapter", PROJECT)
        self.assertIn("cargo build --locked -p xdremux-cli", PROJECT)
        self.assertIn("--product xdremux-apple-adapter", PROJECT)
        self.assertIn("${TARGET_BUILD_DIR}/${CONTENTS_FOLDER_PATH}/Helpers", PROJECT)
        self.assertIn("cp \"${rust_binary}\" \"${helper_directory}/xdremux\"", PROJECT)
        self.assertIn("cp \"${adapter_binary}\" \"${helper_directory}/xdremux-apple-adapter\"", PROJECT)

    def test_ci_runs_architecture_and_app_model_checks(self) -> None:
        self.assertIn("python3 -m unittest Tests.test_app_architecture", CI_WORKFLOW)
        self.assertIn("-scheme XDRemuxApp", CI_WORKFLOW)
        self.assertIn("-scheme XDRemuxAppModelTests", CI_WORKFLOW)
        self.assertIn("CODE_SIGNING_ALLOWED=NO", CI_WORKFLOW)
        self.assertIn("XDRemuxAppModelTests\"", CI_WORKFLOW)
        self.assertIn("actions/upload-artifact@v7", CI_WORKFLOW)


if __name__ == "__main__":
    unittest.main()
