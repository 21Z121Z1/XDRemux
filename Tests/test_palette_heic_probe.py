import importlib.util
import json
import struct
import unittest
from pathlib import Path

from xdremux_py import container


SCRIPT_PATH = Path(__file__).resolve().parents[1] / "scripts" / "build_palette_heic_probe.py"
SPEC = importlib.util.spec_from_file_location("build_palette_heic_probe", SCRIPT_PATH)
assert SPEC is not None and SPEC.loader is not None
probe = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(probe)


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


class PaletteHEICProbeTests(unittest.TestCase):
    def test_selects_only_palette_context_entries(self) -> None:
        source = build_tail(
            [
                ("watermark.logo", b"watermark"),
                ("master.mode.preset.info", b"unrelated-preset"),
                ("basictone.info", b"basic-info"),
                ("basictone.lmtlut.table", b"lut"),
                ("basictone.vig.table", b"vig"),
                ("hdr.transform.data", b"hdr"),
                ("filter.info", b"filter"),
            ],
            tag=b"wtmk",
        )
        entries, tag = probe.extract_manifested_payloads(source)
        selected = probe.select_palette_entries(entries)
        rebuilt = probe.build_tail(selected, tag)
        names = [entry["name"] for entry, _ in selected]

        self.assertEqual(
            names,
            [
                "basictone.info",
                "basictone.lmtlut.table",
                "basictone.vig.table",
                "hdr.transform.data",
            ],
        )
        self.assertEqual(payload_for(rebuilt, "basictone.info"), b"basic-info")
        self.assertEqual(payload_for(rebuilt, "basictone.lmtlut.table"), b"lut")
        self.assertEqual(payload_for(rebuilt, "basictone.vig.table"), b"vig")
        self.assertEqual(payload_for(rebuilt, "hdr.transform.data"), b"hdr")
        self.assertEqual(rebuilt[-8:-4], b"wtmk")

    def test_rejects_unparseable_source(self) -> None:
        with self.assertRaisesRegex(ValueError, "parseable OPPO extension manifest"):
            probe.extract_manifested_payloads(b"not-a-container-tail")


if __name__ == "__main__":
    unittest.main()
