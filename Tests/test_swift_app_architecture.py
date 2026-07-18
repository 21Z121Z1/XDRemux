from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
TOOLCHAIN = (
    ROOT / "Sources" / "XDRemuxAppleFeatures" / "SemanticScene" / "AppleNativeToolchain.swift"
).read_text()
APP_SOURCES = "\n".join(
    path.read_text()
    for path in sorted((ROOT / "apps" / "macos" / "XDRemuxApp" / "Sources").glob("*.swift"))
)
PACKAGE = (ROOT / "Package.swift").read_text()
PROJECT = (ROOT / "apps" / "macos" / "XDRemuxApp" / "project.yml").read_text()
BUILD_SCRIPT = (ROOT / "scripts" / "build_and_run.sh").read_text()
CI_WORKFLOW = (ROOT / ".github" / "workflows" / "ci.yml").read_text()


class SwiftAppArchitectureTests(unittest.TestCase):
    def test_runtime_compilation_is_absent_from_production_toolchain(self) -> None:
        for forbidden in ("/usr/bin/xcrun", '"swiftc"', '"clang"', "AppleNativeTools", "compile("):
            self.assertNotIn(forbidden, TOOLCHAIN)
        self.assertIn("XDRemuxSemanticHelper", TOOLCHAIN)
        self.assertIn("XDRemuxHEVCEncoderHelper", TOOLCHAIN)
        self.assertIn("XDRemuxStyleValidationHelper", TOOLCHAIN)

    def test_app_calls_shared_modules_without_cli_rpc(self) -> None:
        self.assertNotIn("AppConversionEngine", APP_SOURCES)
        self.assertNotIn("XDRemuxCLI", APP_SOURCES)
        self.assertNotIn("CommandLine.arguments", APP_SOURCES)
        self.assertNotIn("Process()", APP_SOURCES)
        self.assertIn("AppleFeatureConversionEngine.convert", APP_SOURCES)
        self.assertFalse((ROOT / "apps" / "macos" / "XDRemuxApp" / "Sources" / "XDRemuxCore.swift").exists())

    def test_helpers_are_build_products_not_resources(self) -> None:
        self.assertNotIn('.copy("Resources/ApplePlatform")', PACKAGE)
        self.assertIn("XDRemuxSemanticHelper", PACKAGE)
        self.assertIn("XDRemuxHEVCEncoderHelper", PACKAGE)
        self.assertIn("XDRemuxStyleValidationHelper", PACKAGE)
        self.assertNotIn("ApplePlatform", PROJECT)
        self.assertEqual(PROJECT.count("subpath: Contents/Helpers"), 3)

    def test_build_script_has_quiet_and_diagnostic_modes(self) -> None:
        for command in ("run", "build", "debug", "logs", "verify", "clean"):
            self.assertIn(command, BUILD_SCRIPT)
        self.assertIn("-quiet", BUILD_SCRIPT)
        self.assertIn("-resultBundlePath", BUILD_SCRIPT)
        self.assertIn("--verbose", BUILD_SCRIPT)
        self.assertIn('subsystem == \\"$BUNDLE_ID\\"', BUILD_SCRIPT)
        self.assertNotIn("pkill", BUILD_SCRIPT)

    def test_ci_uses_stable_output_summary_and_failure_artifacts(self) -> None:
        self.assertIn("XDREMUX_LANGUAGE: en", CI_WORKFLOW)
        self.assertIn("--format jsonl", CI_WORKFLOW)
        self.assertIn("$GITHUB_STEP_SUMMARY", CI_WORKFLOW)
        self.assertIn("actions/upload-artifact@v7", CI_WORKFLOW)
        self.assertIn("if: failure()", CI_WORKFLOW)
        self.assertNotIn("--verbose", CI_WORKFLOW)

    def test_ci_only_uses_runner_context_after_jobs_are_created(self) -> None:
        workflow_scope = CI_WORKFLOW.split("jobs:", maxsplit=1)[0]
        self.assertNotIn("${{ runner.", workflow_scope)

    def test_ci_uses_the_latest_macos_runner_for_liquid_glass(self) -> None:
        self.assertIn("runs-on: macos-latest", CI_WORKFLOW)
        self.assertNotIn("runs-on: macos-15", CI_WORKFLOW)


if __name__ == "__main__":
    unittest.main()
