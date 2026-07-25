from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
APP_SOURCE_ROOT = ROOT / "apps" / "macos" / "XDRemuxApp" / "Sources"
APP_SOURCES = {
    path.name: path.read_text(encoding="utf-8")
    for path in sorted(APP_SOURCE_ROOT.glob("*.swift"))
}
BRIDGE = APP_SOURCES["XDRemuxCore.swift"]
PROJECT = (ROOT / "apps" / "macos" / "XDRemuxApp" / "project.yml").read_text(
    encoding="utf-8"
)
CI_WORKFLOW = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")


class SwiftAppArchitectureTests(unittest.TestCase):
    def test_app_uses_shared_modules_without_cli_subprocess_rpc(self) -> None:
        combined = "\n".join(APP_SOURCES.values())
        for forbidden in ("import XDRemuxCLI", "CommandLine.arguments", "Process()"):
            self.assertNotIn(forbidden, combined)
        self.assertIn("import XDRemuxCore", BRIDGE)
        self.assertIn("import XDRemuxAppleFeatures", BRIDGE)
        self.assertIn("ConversionEngine.convert(request)", BRIDGE)
        self.assertIn("AppleFeatureConversionEngine.convert(request)", BRIDGE)

    def test_categorization_ui_uses_the_shared_core_engine(self) -> None:
        model = APP_SOURCES["PhotoCategorizationViewModel.swift"]
        view = APP_SOURCES["PhotoCategorizationView.swift"]
        queue = APP_SOURCES["XDRemuxViewModel.swift"]
        for source in (model, view, queue):
            self.assertIn("import XDRemuxCore", source)
        self.assertIn("PhotoCategorizationEngine.makePlan", model)
        self.assertIn("PhotoCategorizationEngine.execute", model)
        self.assertIn("PhotoCategorizationEngine.classify", queue)

    def test_app_and_model_tools_depend_on_shared_package_products(self) -> None:
        self.assertIn("product: XDRemuxCore", PROJECT)
        self.assertIn("product: XDRemuxAppleFeatures", PROJECT)
        self.assertIn("XDRemuxAppModelTests:", PROJECT)
        self.assertIn("XDRemuxAppConversionSmoke:", PROJECT)
        self.assertIn("PhotoCategorizationView.swift", PROJECT)

    def test_ci_runs_architecture_and_app_model_checks(self) -> None:
        self.assertIn("python3 -m unittest Tests.test_swift_app_architecture", CI_WORKFLOW)
        self.assertIn("-scheme XDRemuxAppModelTests", CI_WORKFLOW)
        self.assertIn("XDRemuxAppModelTests\"", CI_WORKFLOW)
        self.assertIn("actions/upload-artifact@v7", CI_WORKFLOW)


if __name__ == "__main__":
    unittest.main()
