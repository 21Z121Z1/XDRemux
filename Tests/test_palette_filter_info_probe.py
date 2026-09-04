import importlib.util
import json
import struct
import sys
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "build_palette_filter_info_probe.py"
SPEC = importlib.util.spec_from_file_location("probe", SCRIPT)
assert SPEC and SPEC.loader
probe = importlib.util.module_from_spec(SPEC)
sys.modules["probe"] = probe
SPEC.loader.exec_module(probe)


def fake_container(entries):
    prefix = b"fake-heif-prefix"
    payload = bytearray()
    starts = []
    for name, block in entries:
        starts.append((name, len(payload), len(block)))
        payload.extend(block)
    records = [
        {"name": name, "length": length, "offset": len(payload) - start, "version": 1}
        for name, start, length in starts
    ]
    manifest = json.dumps(records, separators=(",", ":")).encode()
    return prefix + payload + manifest + b"\x00jxrs" + struct.pack("<I", len(manifest) + 9)


class ProbeTests(unittest.TestCase):
    def test_filter_info_v51_layout(self):
        info = probe.FilterPhotoInfoV51(filter_type="palette-default", capture_mode="common")
        raw = info.encode()
        self.assertEqual(len(raw), 220)
        self.assertEqual(raw[28:44], b"palette-default\x00")
        self.assertEqual(raw[156:163], b"common\x00")
        self.assertEqual(struct.unpack_from("<i", raw, 140)[0], 1)
        self.assertEqual(struct.unpack_from("<i", raw, 144)[0], 1)
        self.assertEqual(probe.FilterPhotoInfoV51.decode(raw).capture_mode, "common")

    def test_preserves_existing_manifested_payloads(self):
        source = fake_container([
            ("capture.mode", b"common\x00"),
            ("hdr.transform.data", b"hdr-data"),
            ("local.uhdr.gainmap.info", b"gain-info"),
        ])
        raw = probe.FilterPhotoInfoV51().encode()
        output, report = probe.rebuild_with_filter_info(source, raw)
        _, payloads, _, _ = probe.manifested_entries(output)
        self.assertEqual(payloads["capture.mode"], b"common\x00")
        self.assertEqual(payloads["hdr.transform.data"], b"hdr-data")
        self.assertEqual(payloads["local.uhdr.gainmap.info"], b"gain-info")
        self.assertEqual(payloads["filter.info"], raw)
        self.assertTrue(all(report["non_filter_entries_preserved"].values()))


if __name__ == "__main__":
    unittest.main()
