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
        self.assertIn("private static func completeIdentityStyleData", SWIFT)
        self.assertIn("let identityIndices = Set([3, 7, 11])", SWIFT)
        self.assertIn("let tileCount = 12 * 9 * 8", SWIFT)
        self.assertIn(EXPECTED_IDENTITY_SHA256, SWIFT)
        self.assertNotIn("learnIdentityStyleData", SWIFT)

    def test_unrecovered_scene_and_face_fields_use_neutral_fallbacks(self) -> None:
        style_function = SWIFT.split("private static func makeStylePropertyList", 1)[1].split(
            "private static func validateWithSemanticStyleProperties", 1
        )[0]
        self.assertIn("let sceneType = 0", style_function)
        self.assertIn("let faceBoost = 1.0", style_function)
        self.assertNotIn("peopleRatio >= 0.01 ? 2 : 0", style_function)
        self.assertNotIn("sqrt(globalP50 / personP50)", style_function)
        self.assertIn('"sceneTypeFallback"', style_function)
        self.assertIn('"faceExposureBoostFallback"', style_function)

    def test_linear_light_map_is_not_scaled_by_empirical_base_gain(self) -> None:
        style_function = SWIFT.split("private static func makeStylePropertyList", 1)[1].split(
            "private static func validateWithSemanticStyleProperties", 1
        )[0]
        linear_map = style_function.split("let linearLightMap", 1)[1].split(
            "guard toneLightMap.count", 1
        )[0]
        self.assertIn("valueScale: 1", linear_map)
        self.assertNotIn("valueScale: Float(baseGain)", linear_map)
        self.assertIn('"linearBaseGainApplied": false', style_function)

    def test_gain_map_is_decoded_as_unmanaged_parameter_samples(self) -> None:
        raster_function = SWIFT.split("private static func linearSceneRaster", 1)[1].split(
            "private static func writeRGBPNG", 1
        )[0]
        self.assertIn(".auxiliaryHDRGainMap: true", raster_function)
        self.assertIn(".colorSpace: NSNull()", raster_function)
        self.assertIn("image: gain", raster_function)
        self.assertIn("colorSpace: nil", raster_function)
        self.assertNotIn("CGColorSpace.linearSRGB", raster_function)
        self.assertIn('"domain": "raw-normalized-parameter-code-value"', SWIFT)
        self.assertIn('"colorManagementApplied": false', SWIFT)


if __name__ == "__main__":
    unittest.main()
