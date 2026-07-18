import json
import struct
import unittest

from xdremux.python import container


def build_tail(entries: list[tuple[str, bytes]], tag: bytes = b"jxrs") -> bytes:
    payload = bytearray()
    starts = []
    for name, block in entries:
        starts.append((name, len(payload), len(block)))
        payload.extend(block)
    records = [
        {
            "length": length,
            "name": name,
            "offset": len(payload) - start,
            "version": 1,
        }
        for name, start, length in starts
    ]
    manifest = json.dumps(records, separators=(",", ":"), sort_keys=True).encode("utf-8")
    return bytes(payload) + manifest + b"\x00" + tag + struct.pack("<I", len(manifest) + 9)


def payload_for(tail: bytes, name: str) -> bytes:
    entries, json_start, _ = container.parse_manifest(tail)
    entry = next(item for item in entries if item["name"] == name)
    start = json_start - entry["offset"]
    return tail[start:start + entry["length"]]


class PrivateHDRTailPolicyTests(unittest.TestCase):
    def test_filters_all_private_hdr_families_and_preserves_other_entries(self) -> None:
        source = build_tail([
            ("watermark.logo", b"watermark"),
            ("rear.depth", b"depth"),
            ("local.uhdr.gainmap.data", b"uhdr-data"),
            ("local.uhdr.gainmap.info", b"uhdr-info"),
            ("local.hdr.meta.data", b"local-hdr"),
            ("src.local.hdr.linear.mask", b"source-hdr"),
            ("hdr.transform.data", b"hdr-transform"),
            ("vendor.unknown", b"unknown"),
        ], tag=b"wtmk")

        filtered = container.filter_private_hdr_tail(source)
        entries, _, _ = container.parse_manifest(filtered)
        names = [entry["name"] for entry in entries]

        self.assertEqual(names, ["watermark.logo", "rear.depth", "vendor.unknown"])
        self.assertEqual(payload_for(filtered, "watermark.logo"), b"watermark")
        self.assertEqual(payload_for(filtered, "rear.depth"), b"depth")
        self.assertEqual(payload_for(filtered, "vendor.unknown"), b"unknown")
        self.assertEqual(filtered[-8:-4], b"wtmk")

    def test_leaves_non_hdr_tail_byte_identical(self) -> None:
        source = build_tail([
            ("watermark.logo", b"watermark"),
            ("rear.depth", b"depth"),
        ])
        self.assertEqual(container.filter_private_hdr_tail(source), source)

    def test_rejects_unparseable_tail_instead_of_leaking_private_hdr(self) -> None:
        with self.assertRaisesRegex(ValueError, "unable to parse"):
            container.filter_private_hdr_tail(b"opaque-private-tail")


if __name__ == "__main__":
    unittest.main()
