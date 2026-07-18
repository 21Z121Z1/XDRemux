from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
STYLES_SWIFT = (
    ROOT
    / "Sources"
    / "XDRemuxAppleFeatures"
    / "PhotographicStyles"
    / "ApplePhotographicStylesPipeline.swift"
).read_text()
HDR_SWIFT = (ROOT / "Sources" / "XDRemuxCore" / "HDR" / "HDRPipeline.swift").read_text()


class SwiftArtifactLifecycleTests(unittest.TestCase):
    def test_styles_evidence_is_temporary_without_debug_root(self) -> None:
        function = STYLES_SWIFT.split("private static func augmentPhotographicStyles", 1)[1].split(
            "private static func validatePhotographicStylesOutput", 1
        )[0]
        self.assertIn("let persistEvidence = debugRootURL != nil", function)
        self.assertIn("FileManager.default.temporaryDirectory", function)
        self.assertIn("defer {", function)
        self.assertIn("if !persistEvidence", function)
        self.assertIn("try? FileManager.default.removeItem(at: evidenceContainer)", function)
        self.assertNotIn('appendingPathExtension("xdremux")', function)
        self.assertIn("if persistEvidence {", function)
        self.assertIn('evidenceContainer.appendingPathComponent("latest.json")', function)

    def test_hybrid_intermediate_cleanup_covers_thrown_errors(self) -> None:
        function = HDR_SWIFT.split("enum ISOHDRWriter", 1)[1].split(
            "static func writeWithPreserveReencode", 1
        )[0]
        creation = function.index('let intermediateURL = outputURL.appendingPathExtension("intermediate")')
        cleanup = function.index("defer { try? FileManager.default.removeItem(at: intermediateURL) }")
        phase_two = function.index("try writeWithPreserveReencode")
        self.assertLess(creation, cleanup)
        self.assertLess(cleanup, phase_two)


if __name__ == "__main__":
    unittest.main()
