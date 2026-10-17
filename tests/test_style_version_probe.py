"""Verify the Styles version-only research control changes exactly one byte."""
import hashlib
import plistlib
import unittest

from research.oppo_styles.tools.probe_style_version import (
    change_only_version, patch_heic, root_version_offset,
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
        source = b"header" + b"12345678" + raw + b"trailer"
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


if __name__ == "__main__":
    unittest.main()
