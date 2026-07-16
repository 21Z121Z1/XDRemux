from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
SWIFT = (ROOT / "xdremux" / "swift-cli" / "XDRemux.swift").read_text()


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
        semantic_matte = SWIFT.split("private struct AppleSemanticMatte", 1)[1].split(
            "private enum AppleSemanticRole", 1
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
            "let personMasksValidHint = semantics.hasCrediblePerson ? 1.0 : 0.0",
            SWIFT,
        )
        self.assertIn('"PersonMasksValidHint": personMasksValidHint', SWIFT)


if __name__ == "__main__":
    unittest.main()
