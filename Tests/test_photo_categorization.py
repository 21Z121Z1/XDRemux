import json
import struct
import tempfile
import unittest
from pathlib import Path

from xdremux_py import categorize


class PhotoCategorizationTests(unittest.TestCase):
    def test_shared_contract_matrix(self) -> None:
        fixture = Path(__file__).parent / "fixtures" / "oppo_capture_mode_cases.json"
        cases = json.loads(fixture.read_text(encoding="utf-8"))
        for item in cases:
            with self.subTest(item=item["name"]):
                result = categorize.classify_user_comment(item["user_comment"])
                self.assertEqual(result.mode.key if result.mode else None, item["mode"])
                self.assertEqual(result.mode.folder_name if result.mode else None, item["folder"])
                self.assertEqual(result.status, item["status"])

    def test_plan_projects_asset_type_and_mode_without_duplicate_reruns(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            nested = source / "nested"
            output = source / "categorized"
            nested.mkdir(parents=True)
            (source / "same.heic").write_bytes(b"head-oplus_18-tail")
            (nested / "same.heic").write_bytes(b"different-oplus_18-tail")
            (source / "plain.jpg").write_bytes(b"no comment")

            plan = categorize.make_plan([source], output)
            self.assertEqual(len(plan), 3)
            self.assertEqual(plan[0].destination.parent.name, "人像")
            self.assertEqual(plan[1].destination, output / "静态照片" / "未分类" / "plain.jpg")
            self.assertEqual(plan[2].destination.name, "same (2).heic")

            results = categorize.execute_plan(plan, jobs=2)
            self.assertTrue(all(item.disposition == "copied" for item in results))
            repeated = categorize.make_plan([source], output)
            self.assertTrue(all(item.disposition == "duplicate" for item in repeated))

    def test_validated_live_photo_pair_moves_as_one_asset(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            output = root / "output"
            source.mkdir()
            image = source / "pair.heic"
            video = source / "pair.mov"
            image.write_bytes(b"oplus_18")
            video.write_bytes(b"paired-video")
            occupied = output / "实况照片" / "人像"
            occupied.mkdir(parents=True)
            (occupied / "pair.heic").write_bytes(b"foreign-image")

            paired = categorize.make_plan(
                [source],
                output,
                live_photo_pair_validator=lambda candidate_image, candidate_video: (
                    candidate_image == image and candidate_video == video
                ),
            )
            self.assertEqual(len(paired), 2)
            self.assertTrue(all(item.classification.asset_type is categorize.AssetType.LIVE_PHOTO for item in paired))
            self.assertEqual({item.destination.name for item in paired}, {"pair (2).heic", "pair (2).mov"})
            self.assertTrue(all("实况照片/人像" in item.destination.as_posix() for item in paired))

            rejected = categorize.make_plan(
                [source],
                output,
                live_photo_pair_validator=lambda _image, _video: False,
            )
            self.assertEqual(len(rejected), 1)
            self.assertEqual(rejected[0].source, image)
            self.assertIs(rejected[0].classification.asset_type, categorize.AssetType.STATIC_PHOTO)

    def test_dry_run_writes_nothing(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "portrait.heic"
            output = root / "output"
            source.write_bytes(b"oplus_18")
            results = categorize.execute_plan(categorize.make_plan([source], output), dry_run=True)
            self.assertEqual(results[0].disposition, "dry-run")
            self.assertFalse(output.exists())

    def test_malformed_comment_is_copied_to_unclassified_and_remains_failed_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "malformed.jpg"
            output = root / "output"
            payload = b"ASCII\0\0\0not-an-oppo-comment"
            header = b"II" + struct.pack("<H", 42) + struct.pack("<I", 8)
            ifd0 = struct.pack("<H", 1) + struct.pack("<HHII", 0x8769, 4, 1, 26) + struct.pack("<I", 0)
            exif = struct.pack("<H", 1) + struct.pack("<HHII", 0x9286, 7, len(payload), 44) + struct.pack("<I", 0)
            source.write_bytes(header + ifd0 + exif + payload)

            plan = categorize.make_plan([source], output)
            self.assertEqual(len(plan), 1)
            self.assertEqual(plan[0].classification.status, "malformed-user-comment")
            self.assertEqual(plan[0].destination, output / "静态照片" / "未分类" / source.name)

            result = categorize.execute_plan(plan)[0]
            self.assertEqual(result.disposition, "copied")
            self.assertEqual(result.classification.status, "malformed-user-comment")
            self.assertEqual(result.destination.read_bytes(), source.read_bytes())

    def test_reads_tiff_user_comment_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "comment.jpg"
            payload = b"ASCII\0\0\0Oplus_4096"
            header = b"II" + struct.pack("<H", 42) + struct.pack("<I", 8)
            ifd0 = struct.pack("<H", 1) + struct.pack("<HHII", 0x8769, 4, 1, 26) + struct.pack("<I", 0)
            exif = struct.pack("<H", 1) + struct.pack("<HHII", 0x9286, 7, len(payload), 44) + struct.pack("<I", 0)
            source.write_bytes(header + ifd0 + exif + payload)

            result = categorize.classify_path(source)
            self.assertEqual(result.mode, categorize.CaptureMode.ENHANCED_TEXT)
            self.assertEqual(result.status, "categorized")


if __name__ == "__main__":
    unittest.main()
