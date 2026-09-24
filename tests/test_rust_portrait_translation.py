from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
GEOMETRY = (
    ROOT / "crates" / "xdremux-engine" / "src" / "apple_portrait_geometry.rs"
).read_text(encoding="utf-8")
PORTRAIT = (
    ROOT / "crates" / "xdremux-runtime" / "src" / "oppo_portrait.rs"
).read_text(encoding="utf-8")


class RustPortraitTranslationTests(unittest.TestCase):
    def test_storage_coordinate_transform_explicitly_covers_exif_one_through_eight(self) -> None:
        self.assertIn("transform_apple_portrait_focus_region", GEOMETRY)
        for orientation in range(1, 9):
            self.assertIn(f"{orientation} =>", GEOMETRY)
        self.assertIn("focus_orientation_transform_matches_all_eight_producer_mappings", GEOMETRY)
        self.assertIn("InvalidOrientation", GEOMETRY)

    def test_portrait_rend_is_derived_inside_the_preflight(self) -> None:
        for marker in (
            "build_apple_portrait_rendering_parameters",
            "private_gain_map_headroom",
            "into_auxiliary_payloads",
            "Apple Portrait REND",
        ):
            self.assertIn(marker, PORTRAIT)
        self.assertNotIn("rendering_parameters:", PORTRAIT)
        self.assertNotIn("REND: Vec", PORTRAIT)

    def test_no_caller_supplied_payload_oracle_remains_in_the_rust_api(self) -> None:
        self.assertIn("pub fn into_auxiliary_payloads(self)", PORTRAIT)
        self.assertNotIn("into_auxiliary_payloads(self,", PORTRAIT)


if __name__ == "__main__":
    unittest.main()
