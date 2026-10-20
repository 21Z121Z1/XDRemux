"""Verify the Styles version-only research control changes exactly one byte."""
import hashlib
import plistlib
import struct
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from research.oppo_styles.tools.probe_style_version import (
    change_only_version, patch_heic, root_version_offset, style_metadata_payload,
)


class StyleVersionProbeTests(unittest.TestCase):
    def test_only_version_changes(self):
        raw = plistlib.dumps({"0": 15, "1": bytes(range(256)) * 8,
                              "4": 6.0, "nested": {"15": 14},
                              "c": b"example"}, fmt=plistlib.FMT_BINARY)
        edited = change_only_version(raw)
        self.assertEqual(raw[root_version_offset(raw)], 15)
        self.assertEqual(edited[root_version_offset(raw)], 16)
        self.assertEqual(sum(a != b for a, b in zip(raw, edited)), 1)
        self.assertEqual(plistlib.loads(edited),
                         {**plistlib.loads(raw), "0": 16})
        with self.assertRaises(ValueError):
            change_only_version(edited)

    def test_reverse_v16_to_v15_changes_only_one_byte(self):
        raw = plistlib.dumps({"0": 16, "1": bytes(range(256)) * 8,
                              "nested": {"other": 8}}, fmt=plistlib.FMT_BINARY)
        edited = change_only_version(raw, target_version=15)
        self.assertEqual(raw[root_version_offset(raw, expected_version=16)], 16)
        self.assertEqual(edited[root_version_offset(raw, expected_version=16)], 15)
        self.assertEqual(len(edited), len(raw))
        self.assertEqual(sum(a != b for a, b in zip(raw, edited)), 1)
        self.assertEqual(plistlib.loads(edited),
                         {**plistlib.loads(raw), "0": 15})
        with self.assertRaises(ValueError):
            change_only_version(edited, target_version=15)

    def test_rejects_shared_version_object_instead_of_mutating_other_fields(self):
        raw = plistlib.dumps({"0": 15, "nested": {"same_integer": 15}},
                              fmt=plistlib.FMT_BINARY)
        with self.assertRaises(ValueError):
            change_only_version(raw)

    def test_heif_scope_and_refusal(self):
        raw = plistlib.dumps({"0": 15, "1": b"xyz"}, fmt=plistlib.FMT_BINARY)
        source = b"header" + struct.pack(">I4s", 8 + len(raw), b"idat") + raw + b"trailer"
        report = {
            "items": [{"type": "uri ", "name": "styleMetadata",
                       "location": {"extents": [{"offset": 0, "length": len(raw)}]}}],
            "meta_children": [{"type": "idat", "start": 6}],
        }
        changed, receipt = patch_heic(source, report)
        self.assertEqual(sum(a != b for a, b in zip(source, changed)), 1)
        self.assertEqual(receipt["source_sha256"], hashlib.sha256(source).hexdigest())
        self.assertEqual(receipt["candidate_sha256"], hashlib.sha256(changed).hexdigest())
        with self.assertRaises(ValueError):
            patch_heic(source, {"items": [], "meta_children": []})

    def test_file_backed_styles_keep_existing_texture_bytes(self):
        raw = plistlib.dumps({"0": 15, "1": b"xyz"}, fmt=plistlib.FMT_BINARY)
        texture = b"tag:apple.com,2026:photo:metadata:texture_styles\0bplist00actual"
        source = b"header" + raw + texture
        report = {"items": [{"type": "uri ", "name": "styleMetadata",
                  "location": {"construction_method": 0, "data_reference": 0,
                               "extents": [{"offset": 6, "length": len(raw)}]}}],
                  "meta_children": []}
        changed, receipt = patch_heic(source, report)
        self.assertEqual(changed[-len(texture):], texture)
        self.assertTrue(receipt["other_bytes_identical"])
        self.assertEqual(sum(a != b for a, b in zip(source, changed)), 1)

    def test_idat_extent_uses_real_header_and_cannot_escape_its_box(self):
        raw = plistlib.dumps({"0": 15}, fmt=plistlib.FMT_BINARY)
        for header in (struct.pack(">I4s", len(raw) + 8, b"idat"),
                       struct.pack(">I4sQ", 1, b"idat", len(raw) + 16)):
            source = b"prefix" + header + raw + b"unrelated-tail"
            report = {"items": [{"type": "uri ", "name": "styleMetadata",
                      "location": {"construction_method": 1,
                                   "extents": [{"offset": 0, "length": len(raw)}]}}],
                      "meta_children": [{"type": "idat", "start": 6}]}
            self.assertEqual(style_metadata_payload(source, report), raw)
            report["items"][0]["location"]["extents"][0]["length"] += 1
            with self.assertRaises(ValueError):
                style_metadata_payload(source, report)

    def test_research_cli_entrypoints_preserve_the_single_byte_contract(self):
        raw = plistlib.dumps({"0": 15, "1": b"actual-existing-style"}, fmt=plistlib.FMT_BINARY)
        repository = Path(__file__).resolve().parents[1]
        for entry in (["-m", "research.oppo_styles.tools.probe_style_version"],
                      [str(repository / "research/oppo_styles/tools/probe_style_version.py")]):
            with self.subTest(entry=entry), tempfile.TemporaryDirectory(prefix="xdremux-version-cli-") as tmp:
                root = Path(tmp)
                source = root / "input.heic"
                source.write_bytes(raw)
                inspection = root / "inspect.json"
                inspection.write_text(json.dumps({"items": [{"type": "uri ", "name": "styleMetadata",
                    "location": {"construction_method": 0,
                                 "extents": [{"offset": 0, "length": len(raw)}]}}], "meta_children": []}))
                command = [sys.executable, *entry,
                           "--input", str(source), "--inspect", str(inspection),
                           "--output", str(root / "output.heic"), "--report", str(root / "report.json")]
                result = subprocess.run(command, cwd=repository,
                                        capture_output=True, text=True, check=False)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(source.read_bytes(), raw)
                edited = (root / "output.heic").read_bytes()
                self.assertEqual(sum(a != b for a, b in zip(raw, edited)), 1)
                self.assertEqual(plistlib.loads(edited), {"0": 16, "1": b"actual-existing-style"})


if __name__ == "__main__":
    unittest.main()
