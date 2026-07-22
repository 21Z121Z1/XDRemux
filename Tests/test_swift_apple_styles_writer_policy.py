import hashlib
import struct
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
SWIFT = (
    ROOT
    / "Sources"
    / "XDRemuxAppleFeatures"
    / "PhotographicStyles"
    / "ApplePhotographicStylesPipeline.swift"
).read_text()
STYLE_DATA_SWIFT = (
    ROOT
    / "Sources"
    / "XDRemuxAppleFeatures"
    / "PhotographicStyles"
    / "AppleStyleDataProducer.swift"
).read_text()
NATIVE_SCENE_HELPER = (
    ROOT
    / "Sources"
    / "XDRemuxStyleScenePayloadHelper"
    / "main.m"
).read_text()
EXPECTED_IDENTITY_SHA256 = (
    "43e0ae73508cc10684d4be708fa1d19f3b55b8de15cb8e3544ef16300db91dbe"
)


def expected_complete_identity() -> bytes:
    block = b"".join(
        struct.pack("<e", 1.0 if index in (3, 7, 11) else 0.0)
        for index in range(30)
    )
    return block * (12 * 9 * 8)


class SwiftAppleStylesWriterPolicyTests(unittest.TestCase):
    def test_verified_complete_identity_layout_is_the_default(self) -> None:
        identity = expected_complete_identity()
        self.assertEqual(len(identity), 51_840)
        self.assertEqual(hashlib.sha256(identity).hexdigest(), EXPECTED_IDENTITY_SHA256)
        self.assertIn("static func completeIdentity()", STYLE_DATA_SWIFT)
        self.assertIn("static let identityIndices = Set([3, 7, 11])", STYLE_DATA_SWIFT)
        self.assertIn("static let tileCount = 12 * 9 * 8", STYLE_DATA_SWIFT)
        self.assertIn(EXPECTED_IDENTITY_SHA256, STYLE_DATA_SWIFT)
        self.assertNotIn("learnIdentityStyleData", STYLE_DATA_SWIFT)

    def test_scene_and_face_fields_are_source_derived_and_audited(self) -> None:
        style_function = SWIFT.split("private static func makeStylePropertyList", 1)[1].split(
            "private static func validateWithSemanticStyleProperties", 1
        )[0]
        self.assertIn("sceneClassification.sceneType", style_function)
        self.assertIn("photoDerivedFaceExposureBoost", style_function)
        self.assertNotIn("let sceneType = 0", style_function)
        self.assertNotIn("let faceBoost = 1.0", style_function)
        self.assertNotIn("peopleRatio >= 0.01 ? 2 : 0", style_function)
        self.assertIn('"sceneClassification"', style_function)
        self.assertIn('"faceExposureBoost"', style_function)
        classifier = SWIFT.split("private static func photoDerivedSceneClassification", 1)[1].split(
            "private static func photoDerivedFaceExposureBoost", 1
        )[0]
        self.assertIn('selectedClass = "native-default"', classifier)
        self.assertIn('"nativeDefaultApplied": nativeDefaultApplied', classifier)
        self.assertIn('"sceneDependentFallback": false', classifier)
        self.assertIn('"cameraProducerExact": false', classifier)
        self.assertNotIn("84-photo", classifier)

    def test_linear_thumbnail_h_i_and_key4_are_jointly_photo_derived(self) -> None:
        style_function = SWIFT.split("private static func makeStylePropertyList", 1)[1].split(
            "private static func validateWithSemanticStyleProperties", 1
        )[0]
        bundle_function = SWIFT.split("private static func photoDerivedStyleSceneBundle", 1)[1].split(
            "private static func writeRGBPNG", 1
        )[0]
        self.assertIn("let codedLinear = hdrRGB", bundle_function)
        self.assertIn("/ baselineExposure", bundle_function)
        self.assertIn("let rendererLinear = (codedLinear / encodingGain)", bundle_function)
        self.assertIn("codedLinear / encodingGain", bundle_function)
        self.assertIn("normalizationGain: (baselineExposure * encodingGain)", bundle_function)
        self.assertIn("inverse scale", bundle_function)
        self.assertIn("XDREMUX_RESEARCH_STYLES_LINEAR_INPUT_SCALE", bundle_function)
        self.assertIn("XDREMUX_RESEARCH_STYLES_GTC_IDENTITY_BLEND", bundle_function)
        self.assertIn('"researchOverrideActive": bundle.researchLinearInputScale != 1', SWIFT)
        self.assertIn("appleEncodeLinear(codedLinear)", bundle_function)
        self.assertIn("abs(encodingGain - 4 * baseGain)", style_function)
        self.assertIn('"4": baselineExposure', style_function)
        self.assertIn('"h": baseGain', style_function)
        self.assertIn('"Gain": encodingGain', style_function)
        self.assertIn("linearBaseGainPerGainMapStop * gainMapMaximumStops", SWIFT)
        self.assertIn(
            "linearBaseGainPerHighlightCompression * highlightCompressionRatio",
            SWIFT,
        )
        self.assertNotIn("h = 0.5", style_function)
        self.assertNotIn('"Gain": 2.0', style_function)

    def test_gtc_is_source_derived_inside_the_native_curve_family(self) -> None:
        bundle_function = SWIFT.split("private static func photoDerivedStyleSceneBundle", 1)[1].split(
            "private static func writeRGBPNG", 1
        )[0]
        self.assertIn("inputLuminance: codedLinearLuminance", bundle_function)
        self.assertIn("outputLuminance: baseLuminance", bundle_function)
        self.assertIn("applyGlobalToneCurve(\n            codedLinearLuminance", bundle_function)
        self.assertNotIn("inputLuminance: baseLuminance", bundle_function)
        gtc_function = SWIFT.split("private static func monotonicGlobalToneCurve", 1)[1].split(
            "private static func applyGlobalToneCurve", 1
        )[0]
        self.assertIn("let sourceFeature = sourceShape[8]", gtc_function)
        self.assertIn("clampedSourceFeature", gtc_function)
        self.assertIn("Leave-one-out curve RMSE", gtc_function)
        self.assertNotIn("protocolIdentityGTC()", bundle_function)

    def test_default_local_scene_producer_is_native_and_has_no_silent_fallback(self) -> None:
        dispatcher = SWIFT.split("private static func photoDerivedLocalScenePayload", 1)[1].split(
            "private struct PhotoDerivedSceneClassification", 1
        )[0]
        self.assertIn('?? "native-cmimaging"', dispatcher)
        self.assertIn('case "native-cmimaging", "native":', dispatcher)
        self.assertIn('case "source-derived-behavior-equivalent-v1", "behavior-equivalent", "cpu":', dispatcher)
        self.assertIn("Failure is", dispatcher)
        self.assertIn("no silent CPU fallback", dispatcher)
        self.assertIn("unknown XDREMUX_STYLES_SCENE_PRODUCER mode", dispatcher)
        native_producer = SWIFT.split("private static func nativePhotoDerivedScenePayload", 1)[1].split(
            "private static func photoDerivedLocalScenePayload", 1
        )[0]
        self.assertIn('"mode": "native-cmimaging-final-heic-proxy-v1"', native_producer)
        self.assertIn('"nativeProducerExact": false', native_producer)
        self.assertIn('"consumerExactForProvidedLinearInput": true', native_producer)
        self.assertIn('"captureTimePreLTMInputAvailable": false', native_producer)
        self.assertIn('"behaviorEquivalentLinearInputValidated": false', native_producer)
        self.assertIn('"fallbackKind": "missing-capture-time-pre-ltm-input"', native_producer)

    def test_cli_help_does_not_describe_the_current_scene_proxy_as_production_ready(self) -> None:
        cli = (
            ROOT
            / "Sources"
            / "XDRemuxCLI"
            / "Resources"
            / "en.lproj"
            / "Localizable.strings"
        ).read_text()
        self.assertIn("The current final-HEIC scene path is a research candidate", cli)
        self.assertIn("manifest remains production-ineligible", cli)

    def test_c_and_d_are_positive_paired_maps_not_signed_serialized_residuals(self) -> None:
        producer = SWIFT.split(
            "private static func behaviorEquivalentPhotoDerivedScenePayload", 1
        )[1].split("private static func nativePhotoDerivedScenePayload", 1)[0]
        self.assertIn("bundle.baseLuminance", producer)
        self.assertIn("bundle.gtcMappedLuminance", producer)
        self.assertIn('"negativeSerializedSamplesAllowed": false', producer)
        self.assertIn('"signedLocalRelation"', producer)
        self.assertIn("outputMinimum: toneLightMapMinimum", producer)
        self.assertIn("outputMaximum: linearLightMapMaximum", producer)
        light_map = SWIFT.split("package static func lightMap", 1)[1].split(
            "private static func styleStatistics", 1
        )[0]
        self.assertIn("sum += Double(value)", light_map)
        self.assertNotIn("min(max(luma[y * width + x], 0), 1)", light_map)

    def test_key1_admission_cannot_claim_full_production_eligibility(self) -> None:
        self.assertIn("var key1IncrementEligible: Bool", STYLE_DATA_SWIFT)
        self.assertIn("var productionEligible: Bool { false }", STYLE_DATA_SWIFT)
        self.assertIn('"key1IncrementEligible": key1IncrementEligible', STYLE_DATA_SWIFT)
        self.assertIn('"fullSceneResponseGatePassed": false', SWIFT)
        self.assertIn('"counterexampleGatePassed": false', SWIFT)
        self.assertIn('"photosAcceptancePassed": false', SWIFT)
        self.assertIn('"captureLinearInputGatePassed": captureLinearInputGatePassed', SWIFT)
        self.assertIn('"productionEligible": false', SWIFT)
        self.assertIn(
            'sceneClassificationManifest["sceneDependentFallback"] as? Bool == false',
            SWIFT,
        )
        self.assertIn('styleDataManifest["fallbackKind"] is NSNull', SWIFT)
        self.assertIn("XDREMUX_RESEARCH_STYLES_SEMANTIC_GRAPH_MODE", SWIFT)
        self.assertIn("&& researchSemanticOverride == nil", SWIFT)
        self.assertIn('styleDeltaManifest["fixedProtocolConstant"] as? Bool == true', SWIFT)
        self.assertIn('localSceneManifest["fallbackKind"] is NSNull', SWIFT)
        self.assertIn("&& captureLinearInputGatePassed", SWIFT)
        self.assertIn(
            '"eligibilityFormula": "neutral && captureLinearInput && fullScene && key1Increment && counterexample && noSceneDependentFallback && structural && photosAcceptance"',
            SWIFT,
        )

    def test_native_helpers_have_bounded_fail_closed_execution(self) -> None:
        constrained = (
            ROOT
            / "Sources"
            / "XDRemuxAppleFeatures"
            / "PhotographicStyles"
            / "ConstrainedPolynomialStyleDataProducer.swift"
        ).read_text()
        self.assertIn('arguments: ["--render-style-batch", planURL.path],', constrained)
        self.assertIn("timeout: 180", constrained)
        self.assertIn("guard !process.timedOut", constrained)
        self.assertIn("timeout: 120", SWIFT)
        self.assertIn("timeout: 30", SWIFT)

    def test_native_scene_scalar_formulas_are_pinned_to_producer_fields(self) -> None:
        self.assertIn('metadata[@"LTMDigitalGain"]', NATIVE_SCENE_HELPER)
        self.assertIn("kFigCaptureStreamMetadata_HRGainDownRatio", NATIVE_SCENE_HELPER)
        self.assertIn("kFigCaptureSampleBufferMetadata_LTMRelativeBrightness", NATIVE_SCENE_HELPER)
        self.assertIn("computeLinearImageEncodingGainWithMetadata:", NATIVE_SCENE_HELPER)
        self.assertIn("computeLinearImageExposureWithMetadata:outputBaseGain:outputBaselineExposure:", NATIVE_SCENE_HELPER)
        self.assertIn('"nativeFormula": "4 * LTMDigitalGain"', SWIFT)
        self.assertIn(
            '"nativeEncodingFormula": "(HRGainDownRatio / 4096) / LTMRelativeBrightness"',
            SWIFT,
        )
        self.assertIn("4 * baseGain", SWIFT)

    def test_default_neutral_style_delta_reuses_a_verified_protocol_resource(self) -> None:
        self.assertIn("neutralStyleDeltaAnnexBBase64", SWIFT)
        self.assertIn("neutralStyleDeltaProtocolResourceHashes", SWIFT)
        self.assertIn("bundled-verified-protocol-constant", SWIFT)
        self.assertIn("XDREMUX_RESEARCH_STYLES_DELTA_RGB_CODES", SWIFT)
        self.assertIn('"fixedProtocolConstant": !researchStyleDeltaOverride', SWIFT)
        self.assertIn(
            "14b04fcde02476f24f83a893d245b4d06728954e8ad004f416b6e3a956eba216",
            SWIFT,
        )
        self.assertIn("runtime-videotoolbox-custom-quality", SWIFT)

    def test_every_style_data_producer_persists_one_canonical_final_metadata_file(self) -> None:
        augment = SWIFT.split("private static func augmentPhotographicStyles", 1)[1].split(
            "private struct StylesValidationResult", 1
        )[0]
        branch_end = augment.split(
            "let stylePayloadSeconds = CFAbsoluteTimeGetCurrent() - stylePayloadStartedAt",
            1,
        )[0]
        self.assertIn(
            'to: styleDirectory.appendingPathComponent("style-metadata.bplist")',
            branch_end,
        )
        self.assertIn("try stylePayload.stylePropertyList.write", branch_end)

    def test_gain_map_is_decoded_as_unmanaged_parameter_samples(self) -> None:
        raster_function = SWIFT.split("private static func photoDerivedStyleSceneBundle", 1)[1].split(
            "private static func writeRGBPNG", 1
        )[0]
        self.assertIn(".auxiliaryHDRGainMap: true", raster_function)
        self.assertIn(".colorSpace: NSNull()", raster_function)
        self.assertIn("image: gain", raster_function)
        self.assertIn("colorSpace: nil", raster_function)
        self.assertNotIn("CGColorSpace.linearSRGB", raster_function)
        self.assertIn('"domain": "raw-normalized-parameter-code-value"', SWIFT)
        self.assertIn('"colorManagementApplied": false', SWIFT)

    def test_each_statistics_distribution_sorts_pixels_once(self) -> None:
        percentile_section = SWIFT.split("private static func percentile", 1)[1].split(
            "private static func maskValue", 1
        )[0]
        self.assertIn("let sorted = values.lazy.filter", percentile_section)
        self.assertIn("percentile(sorted, $0)", percentile_section)
        self.assertNotIn("percentile(finite, $0)", percentile_section)
        self.assertEqual(percentile_section.count(".sorted()"), 1)


if __name__ == "__main__":
    unittest.main()
