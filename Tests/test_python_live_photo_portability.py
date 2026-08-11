from __future__ import annotations

import ast
import struct
import unittest
from pathlib import Path

from xdremux_py.ultrahdr_iso import decode_iso21496_payload, parse_iso21496_jpeg_metadata


class PythonLivePhotoPortabilityTests(unittest.TestCase):
    def test_live_photo_runtime_does_not_import_apple_platform_modules(self):
        root = Path(__file__).resolve().parents[1] / "xdremux_py"
        runtime_files = [
            "motion_photo.py",
            "motion_video.py",
            "live_photo.py",
            "live_photo_mov.py",
            "live_photo_still.py",
            "live_photo_still_portable.py",
            "ultrahdr_iso.py",
        ]
        forbidden_roots = {
            "AVFoundation", "CoreMedia", "CoreGraphics", "ImageIO", "Photos",
            "Quartz", "Foundation", "objc", "PyObjCTools",
        }
        violations: list[str] = []
        for filename in runtime_files:
            tree = ast.parse((root / filename).read_text(encoding="utf-8"), filename=filename)
            for node in ast.walk(tree):
                names: list[str] = []
                if isinstance(node, ast.Import):
                    names.extend(alias.name for alias in node.names)
                elif isinstance(node, ast.ImportFrom) and node.module:
                    names.append(node.module)
                for name in names:
                    if name.split(".", 1)[0] in forbidden_roots:
                        violations.append(f"{filename}: {name}")
        self.assertEqual(violations, [])

    def test_decodes_libultrahdr_common_denominator_iso_metadata(self):
        # ISO 21496 fraction serialization used by AOSP libultrahdr:
        # minVersion, writerVersion, flags(common denominator + base colourspace), denominator,
        # base/alternate headroom, then per-channel min/max/gamma/baseOffset/alternateOffset.
        payload = (
            struct.pack(">HHB", 0, 0, 0x48)
            + struct.pack(">III", 1000, 0, 2500)
            + struct.pack(">iiIii", -500, 3000, 1000, 16, 32)
        )
        metadata = decode_iso21496_payload(payload)
        self.assertEqual(metadata["gainMapMin"], [-0.5, -0.5, -0.5])
        self.assertEqual(metadata["gainMapMax"], [3.0, 3.0, 3.0])
        self.assertEqual(metadata["gamma"], [1.0, 1.0, 1.0])
        self.assertEqual(metadata["offsetSdr"], [0.016, 0.016, 0.016])
        self.assertEqual(metadata["offsetHdr"], [0.032, 0.032, 0.032])
        self.assertEqual(metadata["hdrCapacityMin"], 0.0)
        self.assertEqual(metadata["hdrCapacityMax"], 2.5)
        self.assertTrue(metadata["useBaseColorSpace"])

    def test_finds_iso_metadata_in_jpeg_app2(self):
        payload = (
            struct.pack(">HHB", 0, 0, 0x48)
            + struct.pack(">III", 100, 0, 200)
            + struct.pack(">iiIii", 0, 200, 100, 0, 0)
        )
        namespace = b"urn:iso:std:iso:ts:21496:-1\0"
        app2_data = namespace + payload
        app2 = b"\xff\xe2" + struct.pack(">H", len(app2_data) + 2) + app2_data
        jpeg = b"\xff\xd8" + app2 + b"\xff\xd9"
        metadata = parse_iso21496_jpeg_metadata(jpeg)
        self.assertIsNotNone(metadata)
        assert metadata is not None
        self.assertEqual(metadata["gainMapMax"], [2.0, 2.0, 2.0])
        self.assertEqual(metadata["hdrCapacityMax"], 2.0)


if __name__ == "__main__":
    unittest.main()
