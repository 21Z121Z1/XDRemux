from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
SWIFT = "\n".join(
    path.read_text()
    for path in sorted((ROOT / "Sources" / "XDRemuxAppleFeatures").rglob("*.swift"))
)


class SwiftAppleSemanticPolicyTests(unittest.TestCase):
    def test_native_semantic_write_profiles_are_role_aware(self) -> None:
        self.assertIn('case styleSkyOnly = "style_sky_only"', SWIFT)
        self.assertIn('case styleHuman = "style_human"', SWIFT)
        self.assertIn('case portraitAndStyles = "portrait_and_styles"', SWIFT)
        self.assertIn("roles: [.sky]", SWIFT)
        self.assertIn("roles: [.person, .skin, .sky]", SWIFT)
        self.assertIn("roles: [.person, .skin, .hair, .teeth, .glasses]", SWIFT)
        self.assertIn("roles: Set(AppleSemanticRole.allCases)", SWIFT)

    def test_styles_scaffold_does_not_force_six_portrait_roles(self) -> None:
        self.assertIn("profile: AppleSemanticWriteProfile", SWIFT)
        self.assertIn("dictionaries.count == profile.roles.count", SWIFT)
        self.assertIn("semanticImageIDs.count == profile.roles.count", SWIFT)
        self.assertNotIn("semantic scaffold must expose exactly PEM", SWIFT)
        self.assertIn("styles-only semantics must be sky-only or PEM+skin+sky", SWIFT)

    def test_sparse_masks_are_not_dropped_by_a_percentage_threshold(self) -> None:
        semantic_matte = SWIFT.split("struct AppleSemanticMatte", 1)[1].split(
            "enum AppleSemanticRole", 1
        )[0]
        self.assertIn("thresholdPixelCount() >= 16", semantic_matte)
        self.assertIn("statistics.maximum >= 128", semantic_matte)
        self.assertNotIn("coverage <", semantic_matte)
        self.assertNotIn("coverage <=", semantic_matte)

    def test_oppo_person_and_hair_are_constrained_priors(self) -> None:
        self.assertIn("edgeGuidedOPPOPrior", SWIFT)
        self.assertIn("OPPO subject prior did not overlap the Vision person topology", SWIFT)
        self.assertIn("OPPO hair support gated by fused person matte", SWIFT)
        self.assertIn(
            "Vision-only; OPPO person/hair planes are not reused for unrelated semantics",
            SWIFT,
        )
        self.assertIn("facialHairPolicy", SWIFT)

    def test_person_mask_validity_hint_tracks_real_content(self) -> None:
        self.assertIn(
            "let personMasksValidHint = semantics.hasCrediblePerson ? 1.0 : -1.0",
            SWIFT,
        )
        self.assertNotIn(
            "let personMasksValidHint = semantics.hasCrediblePerson ? 1.0 : 0.0",
            SWIFT,
        )
        self.assertIn('"PersonMasksValidHint": personMasksValidHint', SWIFT)

    def test_combined_portrait_and_styles_reuse_one_vision_analysis(self) -> None:
        styles_source = (
            ROOT
            / "Sources"
            / "XDRemuxAppleFeatures"
            / "PhotographicStyles"
            / "ApplePhotographicStylesPipeline.swift"
        ).read_text()
        portrait_source = (
            ROOT
            / "Sources"
            / "XDRemuxAppleFeatures"
            / "Portrait"
            / "PortraitConversionPipeline.swift"
        ).read_text()

        self.assertIn("semanticOutputDirectory: sharedSemanticDirectory", styles_source)
        self.assertIn("portraitSemanticAnalysis = outcome.semanticAnalysis", styles_source)
        self.assertIn("analysis = portraitSemanticAnalysis", styles_source)
        self.assertIn('semanticAnalysisSource = "portrait_shared"', styles_source)
        self.assertIn("skipped duplicate Styles analysis", styles_source)
        self.assertIn('"timingsSeconds": [', styles_source)
        self.assertIn("Vision semantic request batch profile=%@ requests=%d masks=%d", SWIFT)
        self.assertIn("semanticAnalysis: mattes.semanticAnalysis", portrait_source)
        self.assertEqual(styles_source.count("AppleSemanticSceneAnalyzer.analyze("), 1)

    def test_semantic_helper_does_not_run_unused_face_detection(self) -> None:
        helper_source = (
            ROOT
            / "Sources"
            / "XDRemuxSemanticHelper"
            / "main.swift"
        ).read_text()

        self.assertNotIn("VNDetectFaceRectanglesRequest", helper_source)
        self.assertNotIn('("human_attribute_facial_hair", "facial_hair")', helper_source)
        self.assertIn('case "--roles":', helper_source)
        self.assertIn('case "--raw-only":', helper_source)
        self.assertIn("try handler.perform(requests)", helper_source)
        self.assertIn('"request_count": requests.count', helper_source)
        self.assertIn('selectedRoles.contains("glasses")', helper_source)
        self.assertIn('selectedRoles.contains("sky")', helper_source)
        self.assertIn('"status": "not_requested"', helper_source)
        self.assertIn('"reason": "no production consumer"', helper_source)

    def test_semantic_helper_uses_prebuilt_executable(self) -> None:
        toolchain = (
            ROOT
            / "Sources"
            / "XDRemuxAppleFeatures"
            / "SemanticScene"
            / "AppleNativeToolchain.swift"
        ).read_text()

        self.assertIn('try executable(named: "XDRemuxSemanticHelper")', toolchain)
        self.assertIn('ProcessInfo.processInfo.environment["XDREMUX_HELPER_DIRECTORY"]', toolchain)
        self.assertIn('Bundle.main.bundleURL.pathExtension == "app"', toolchain)
        self.assertNotIn('arguments: ["swiftc"', toolchain)

    def test_normal_app_workflow_uses_release_except_lldb(self) -> None:
        script = (ROOT / "scripts" / "build_and_run.sh").read_text()
        self.assertIn('CONFIGURATION="Release"', script)
        self.assertIn('if [[ "$MODE" == "debug" ]]; then\n  CONFIGURATION="Debug"', script)


if __name__ == "__main__":
    unittest.main()
