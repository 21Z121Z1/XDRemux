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
        # Scratch evidence is discarded on success and kept on failure, where it
        # is the only record of what the pipeline produced.
        self.assertIn("} else if succeeded {", function)
        self.assertIn("try? FileManager.default.removeItem(at: evidenceContainer)", function)
        self.assertIn("succeeded = true", function)
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

    def test_styles_final_validation_reuses_accepted_artifacts(self) -> None:
        self.assertIn("private struct StylesValidationResult", STYLES_SWIFT)
        self.assertIn("prevalidatedStylePropertyList: stylePayload.stylePropertyList", STYLES_SWIFT)
        self.assertIn("if prevalidatedStylePropertyList != styleData", STYLES_SWIFT)
        self.assertIn("let outputData = validation.outputData", STYLES_SWIFT)
        self.assertIn("let contaminationReport = validation.contaminationReport", STYLES_SWIFT)
        validation = STYLES_SWIFT.split(
            "private static func validatePhotographicStylesOutput", 1
        )[1].split("private static func donorContaminationScan", 1)[0]
        self.assertIn("donorContaminationScan", validation)
        self.assertIn("guard contamination.matches.isEmpty", validation)

    def test_styles_validation_skips_legacy_uri_when_prevalidated_payload_exists(self) -> None:
        validation = STYLES_SWIFT.split(
            "private static func validatePhotographicStylesOutput", 1
        )[1].split("private static func donorContaminationScan", 1)[0]
        # Native files can retain an older style URI item before the generated
        # item. The validator must select the exact payload already accepted by
        # the producer instead of blindly taking the first URI item.
        self.assertIn("let styleURIItems = items.filter", validation)
        self.assertIn("let styleCandidates = styleURIItems.compactMap", validation)
        self.assertIn("prevalidatedStylePropertyList == nil || $0.1 == prevalidatedStylePropertyList", validation)
        self.assertIn("styleCandidates.first(where:", validation)

    def test_styles_scene_bundle_retains_one_shared_hdr_rgb_plane(self) -> None:
        scene_bundle = STYLES_SWIFT.split(
            "private struct PhotoDerivedStyleSceneBundle", 1
        )[1].split("private struct StyleSceneResourcePayload", 1)[0]
        scene_builder = STYLES_SWIFT.split(
            "private static func photoDerivedStyleSceneBundle", 1
        )[1].split("private static func", 1)[0]

        self.assertIn("let hdrLinearP3RGB: [Float]", scene_bundle)
        self.assertIn("let logGainRGB: [Float]", scene_bundle)
        self.assertEqual(scene_builder.count("var hdrRGB ="), 1)
        self.assertIn("hdrLinearP3RGB: hdrRGB", scene_builder)
        self.assertIn('"resourceDecodeCount": 1', STYLES_SWIFT)


if __name__ == "__main__":
    unittest.main()
