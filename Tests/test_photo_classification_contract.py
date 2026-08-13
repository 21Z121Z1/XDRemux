import json
import tempfile
import unittest
from pathlib import Path

from xdremux_py import categorize


class PhotoClassificationContractTests(unittest.TestCase):
    def test_canonical_golden_contract(self) -> None:
        fixture = Path(__file__).parent / "fixtures" / "photo_classification_cases.json"
        cases = json.loads(fixture.read_text(encoding="utf-8"))
        asset_types = {item.key: item for item in categorize.AssetType}
        for item in cases:
            with self.subTest(item=item["name"]):
                classification = categorize.classify_user_comment(item["user_comment"]).with_asset_type(
                    asset_types[item["asset_type"]]
                )
                self.assertEqual(categorize.classification_contract(classification), {
                    key: value for key, value in item.items() if key not in {"name", "user_comment"}
                })

    def test_multi_bit_flags_are_lossless_but_projection_is_stable(self) -> None:
        classification = categorize.classify_user_comment("oplus_18")
        self.assertEqual(
            classification.capture_modes,
            frozenset({categorize.CaptureMode.PORTRAIT, categorize.CaptureMode.BEAUTY}),
        )
        self.assertEqual(classification.mode, categorize.CaptureMode.PORTRAIT)
        self.assertNotIn("capture.normal", classification.tags)
        self.assertIn("capture.portrait", classification.tags)
        self.assertIn("capture.beauty", classification.tags)

    def test_known_unmapped_and_unknown_flags_are_distinct(self) -> None:
        known = categorize.classify_user_comment("oplus_262144")
        self.assertEqual(known.known_unmapped_flags, 262144)
        self.assertEqual(known.unknown_flags, 0)
        self.assertEqual(known.metadata_status, "ok")
        self.assertEqual(known.mode, categorize.CaptureMode.NORMAL)

        unknown = categorize.classify_user_comment("oplus_17179869184")
        self.assertEqual(unknown.known_unmapped_flags, 0)
        self.assertEqual(unknown.unknown_flags, 17179869184)
        self.assertEqual(unknown.metadata_status, "ok")
        self.assertIsNone(unknown.mode)
        self.assertEqual(unknown.status, "unknown-flags")  # legacy compatibility view only

    def test_folder_projection_separates_asset_type_from_capture_tags(self) -> None:
        portrait = categorize.classify_user_comment("oplus_18")
        self.assertEqual(
            categorize.FolderProjection.relative_directory(portrait),
            Path("静态照片") / "人像",
        )
        live = portrait.with_asset_type(categorize.AssetType.LIVE_PHOTO)
        self.assertEqual(
            categorize.FolderProjection.relative_directory(live),
            Path("实况照片") / "人像",
        )
        self.assertEqual(categorize.CLASSIFICATION_LAYOUT_VERSION, "asset-type-v1")

    def test_photo_asset_keeps_live_photo_resources_together(self) -> None:
        asset = categorize.PhotoAsset.live_photo(Path("IMG.heic"), Path("IMG.mov"), "asset-id")
        self.assertEqual(asset.asset_type, categorize.AssetType.LIVE_PHOTO)
        self.assertEqual(asset.primary_image, Path("IMG.heic"))
        self.assertEqual(
            [resource.role for resource in asset.resources],
            [categorize.ResourceRole.PRIMARY_IMAGE, categorize.ResourceRole.PAIRED_VIDEO],
        )

    def test_capabilities_require_complete_manifest_entry_names(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            complete = root / "complete.heic"
            complete.write_bytes(
                b'oplus_18 {"name":"local.uhdr.gainmap.data"} '
                b'{"name":"rear.depth"} {"name":"rear.depth.config"}'
            )
            classification = categorize.classify_path(complete, categorize.AssetType.STATIC_PHOTO)
            self.assertEqual(
                classification.capabilities,
                frozenset({
                    categorize.PhotoCapability.PROXDR,
                    categorize.PhotoCapability.GAIN_MAP,
                    categorize.PhotoCapability.HDR,
                    categorize.PhotoCapability.DEPTH,
                }),
            )

            config_only = root / "config-only.heic"
            config_only.write_bytes(b'oplus_18 {"name":"rear.depth.config"}')
            config_classification = categorize.classify_path(
                config_only, categorize.AssetType.STATIC_PHOTO
            )
            self.assertNotIn(categorize.PhotoCapability.DEPTH, config_classification.capabilities)


if __name__ == "__main__":
    unittest.main()
