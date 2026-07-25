import json
import struct
import tempfile
import unittest
from contextlib import redirect_stderr
from io import StringIO
from pathlib import Path

from xdremux.python import categorize
from xdremux.python import XDRemux


class PhotoCategorizationTests(unittest.TestCase):
    def test_python_cli_uses_categorize_for_command_and_batch_switch(self) -> None:
        parser = XDRemux.build_parser()
        standalone = parser.parse_args([
            "categorize", "--input", "/tmp/a.heic", "--input", "/tmp/photos",
            "--output-dir", "/tmp/output", "--jobs", "2", "--dry-run",
        ])
        self.assertEqual(standalone.command, "categorize")
        self.assertEqual(standalone.input, ["/tmp/a.heic", "/tmp/photos"])
        self.assertEqual(standalone.jobs, 2)
        self.assertTrue(standalone.dry_run)

        batch = parser.parse_args(["batch", "--input-dir", "/tmp/input", "--categorize"])
        self.assertTrue(batch.categorize_output)
        with redirect_stderr(StringIO()), self.assertRaises(SystemExit):
            parser.parse_args(["convert", "--input", "/tmp/input.heic", "--categorize"])

    def test_shared_contract_matrix(self) -> None:
        fixture = Path(__file__).parent / "fixtures" / "oppo_capture_mode_cases.json"
        cases = json.loads(fixture.read_text(encoding="utf-8"))
        for item in cases:
            with self.subTest(item=item["name"]):
                result = categorize.classify_user_comment(item["user_comment"])
                self.assertEqual(result.mode.key if result.mode else None, item["mode"])
                self.assertEqual(result.mode.folder_name if result.mode else None, item["folder"])
                self.assertEqual(result.status, item["status"])

    def test_plan_copies_to_categories_and_root_without_duplicate_reruns(self) -> None:
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
            self.assertEqual(plan[1].destination, output / "plain.jpg")
            self.assertEqual(plan[2].destination.name, "same (2).heic")

            results = categorize.execute_plan(plan, jobs=2)
            self.assertTrue(all(item.disposition == "copied" for item in results))
            repeated = categorize.make_plan([source], output)
            self.assertTrue(all(item.disposition == "duplicate" for item in repeated))

    def test_dry_run_writes_nothing(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "portrait.heic"
            output = root / "output"
            source.write_bytes(b"oplus_18")
            results = categorize.execute_plan(categorize.make_plan([source], output), dry_run=True)
            self.assertEqual(results[0].disposition, "dry-run")
            self.assertFalse(output.exists())

    def test_malformed_comment_is_copied_to_root_and_returns_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "malformed.jpg"
            output = root / "output"
            payload = b"ASCII\0\0\0not-an-oppo-comment"
            header = b"II" + struct.pack("<H", 42) + struct.pack("<I", 8)
            ifd0 = struct.pack("<H", 1) + struct.pack("<HHII", 0x8769, 4, 1, 26) + struct.pack("<I", 0)
            exif = struct.pack("<H", 1) + struct.pack("<HHII", 0x9286, 7, len(payload), 44) + struct.pack("<I", 0)
            source.write_bytes(header + ifd0 + exif + payload)

            result = XDRemux.main([
                "categorize", "--input", str(source), "--output-dir", str(output),
            ])

            self.assertEqual(result, 1)
            self.assertEqual((output / source.name).read_bytes(), source.read_bytes())

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
