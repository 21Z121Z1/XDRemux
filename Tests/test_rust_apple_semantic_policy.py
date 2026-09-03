from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[1]
ENGINE = "\n".join(
    path.read_text(encoding="utf-8")
    for path in sorted((ROOT / "crates" / "xdremux-engine" / "src").glob("apple_*.rs"))
)
RUNTIME = (ROOT / "crates" / "xdremux-runtime" / "src" / "apple_styles.rs").read_text(
    encoding="utf-8"
)
RUNTIME_ADAPTER = (
    ROOT / "crates" / "xdremux-runtime" / "src" / "apple_adapter.rs"
).read_text(encoding="utf-8")
ADAPTER_MAIN = (
    ROOT / "Sources" / "XDRemuxAppleAdapter" / "main.swift"
).read_text(encoding="utf-8")
ADAPTER = "\n".join(
    path.read_text(encoding="utf-8")
    for path in sorted((ROOT / "Sources" / "XDRemuxAppleAdapter").glob("*.swift"))
)
ADAPTER_NORMALIZED = " ".join(ADAPTER.split())
APPLE_PROTOCOL_CLIENTS = tuple(
    (ROOT / path).read_text(encoding="utf-8")
    for path in (
        "scripts/check_apple_adapter_handshake.sh",
        "scripts/check_rust_cli_apple_portrait.sh",
        "scripts/check_rust_style_consumer.sh",
        "scripts/build_apple_device_validation.sh",
    )
)


class RustAppleSemanticPolicyTests(unittest.TestCase):
    def test_rust_owns_portrait_and_styles_policy(self) -> None:
        for marker in (
            "build_apple_portrait_rendering_parameters",
            "build_apple_portrait_disparity_payload",
            "apple_style_fit_global_polynomial",
            "apple_style_monotonic_global_tone_curve",
            "resolve_apple_style_scene_type",
            "apple_style_scene_scores_from_vision_observations",
            "FOOD_IDENTIFIERS",
            "SUNSET_IDENTIFIERS",
            "INDOOR_IDENTIFIERS",
            "OUTDOOR_IDENTIFIERS",
        ):
            self.assertIn(marker, ENGINE + RUNTIME)

    def test_adapter_returns_facts_and_primitives_not_product_decisions(self) -> None:
        for forbidden in (
            "recommended_style",
            "fallback_strategy",
            "output_mode",
            "ConversionPlan",
            "ConversionRequest",
            "maximumConfidence",
        ):
            self.assertNotIn(forbidden, ADAPTER)
        for marker in (
            "imageio",
            "Vision",
            "VideoToolbox",
            "capabilities",
            "VisionClassificationObservationFacts",
        ):
            self.assertIn(marker, ADAPTER)

    def test_vision_scene_alias_policy_does_not_live_in_swift(self) -> None:
        for legacy_swift_policy in (
            '["food", "meal", "dish"]',
            '["sunset", "sunrise", "dusk"]',
            '["indoor", "interior", "room"]',
        ):
            self.assertNotIn(legacy_swift_policy, ADAPTER)

    def test_person_segmentation_quality_is_a_fixed_primitive_contract(self) -> None:
        self.assertIn("personSegmentationPrimitiveQuality", ADAPTER)
        self.assertIn(
            "personRequest?.qualityLevel = personSegmentationPrimitiveQuality",
            ADAPTER,
        )
        self.assertNotIn("personRequest?.qualityLevel = .accurate", ADAPTER)
        self.assertNotIn("personRequest?.qualityLevel = .balanced", ADAPTER)
        self.assertNotIn("personRequest?.qualityLevel = .fast", ADAPTER)
        self.assertIn(
            "versioned Rust-owned adapter request must carry that selection",
            ADAPTER_NORMALIZED,
        )

    def test_adapter_schema_version_is_atomic_across_runtime_helper_and_clients(self) -> None:
        rust_match = re.search(
            r"APPLE_ADAPTER_SCHEMA_VERSION:\s*u32\s*=\s*(\d+)\s*;",
            RUNTIME_ADAPTER,
        )
        swift_match = re.search(r"private let schemaVersion\s*=\s*(\d+)", ADAPTER_MAIN)
        self.assertIsNotNone(rust_match)
        self.assertIsNotNone(swift_match)
        rust_version = int(rust_match.group(1))
        swift_version = int(swift_match.group(1))
        self.assertEqual(rust_version, swift_version)
        self.assertGreater(rust_version, 0)

        compact_patterns = (
            f'"schema_version":{rust_version}',
            f'"schema_version": {rust_version}',
        )
        stale_patterns = (
            f'"schema_version":{rust_version - 1}',
            f'"schema_version": {rust_version - 1}',
        )
        for client in APPLE_PROTOCOL_CLIENTS:
            self.assertTrue(
                any(pattern in client for pattern in compact_patterns),
                "Apple protocol client does not declare the runtime/helper schema version",
            )
            if rust_version > 1:
                self.assertFalse(
                    any(pattern in client for pattern in stale_patterns),
                    "Apple protocol client still contains the immediately previous schema version",
                )

    def test_rust_runtime_composes_adapter_at_one_boundary(self) -> None:
        lib = (ROOT / "crates" / "xdremux-runtime" / "src" / "lib.rs").read_text(
            encoding="utf-8"
        )
        self.assertIn("mod apple_adapter;", lib)
        self.assertNotIn("pub mod apple_adapter;", lib)
        self.assertIn("AppleAdapterClient", RUNTIME)

    def test_no_swift_product_sources_survive_outside_adapter(self) -> None:
        source_root = ROOT / "Sources"
        swift_paths = sorted(source_root.rglob("*.swift"))
        self.assertTrue(swift_paths)
        self.assertTrue(all("XDRemuxAppleAdapter" in path.parts for path in swift_paths))


if __name__ == "__main__":
    unittest.main()
