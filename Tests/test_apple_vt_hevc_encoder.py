#!/usr/bin/env python3
"""Regression coverage for the direct VideoToolbox HEVC RExt 4:4:4 path."""

from __future__ import annotations

import json
import platform
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ENCODER = (
    ROOT
    / ".build"
    / "debug"
    / "XDRemuxHEVCEncoderHelper"
)


@unittest.skipUnless(platform.system() == "Darwin", "VideoToolbox is macOS-only")
class AppleVT444EncoderTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        subprocess.run(
            ["swift", "build", "--product", "XDRemuxHEVCEncoderHelper"],
            cwd=ROOT,
            check=True,
            capture_output=True,
        )

    def test_rgb4448_emits_hevc_range_extensions_profile(self) -> None:
        with tempfile.TemporaryDirectory(prefix="xdremux-vt444-") as temporary:
            directory = Path(temporary)
            source_ppm = directory / "source.ppm"
            source_png = directory / "source.png"
            low_quality_annex_b = directory / "gain-low.hevc"
            low_quality_hvcc = directory / "gain-low.hvcc"
            annex_b = directory / "gain.hevc"
            hvcc = directory / "gain.hvcc"

            width = 64
            height = 64
            pixels = bytearray()
            for y in range(height):
                for x in range(width):
                    pixels.extend((x * 255 // (width - 1), y * 255 // (height - 1), (x ^ y) * 4))
            source_ppm.write_bytes(f"P6\n{width} {height}\n255\n".encode("ascii") + pixels)

            subprocess.run(
                ["sips", "-s", "format", "png", str(source_ppm), "--out", str(source_png)],
                check=True,
                capture_output=True,
            )
            subprocess.run(
                [
                    str(ENCODER),
                    str(source_png),
                    str(low_quality_annex_b),
                    "0.45",
                    "rgb4448",
                    str(low_quality_hvcc),
                ],
                check=True,
                capture_output=True,
            )
            result = subprocess.run(
                [str(ENCODER), str(source_png), str(annex_b), "0.9", "rgb4448", str(hvcc)],
                check=True,
                capture_output=True,
                text=True,
            )
            protocol = json.loads(result.stdout)
            self.assertEqual(protocol["schema"], "xdremux-hevc-encoder-helper-v1")
            self.assertEqual(protocol["event"], "completed")
            self.assertEqual(protocol["mode"], "rgb4448")
            self.assertEqual(result.stderr, "")

            configuration = hvcc.read_bytes()
            self.assertGreater(len(configuration), 23)
            self.assertEqual(configuration[0], 1, "unexpected hvcC configuration version")
            self.assertEqual(configuration[1] & 0x1F, 4, "encoder did not emit HEVC RExt profile_idc=4")
            self.assertGreater(
                annex_b.stat().st_size,
                low_quality_annex_b.stat().st_size,
                "quality was ignored when mode and hvcC arguments were also present",
            )

    def test_rgb4448tile_emits_rext_tile_sample(self) -> None:
        with tempfile.TemporaryDirectory(prefix="xdremux-vttile444-") as temporary:
            directory = Path(temporary)
            source_ppm = directory / "source.ppm"
            source_png = directory / "source.png"
            annex_b = directory / "tile.hevc"
            hvcc = directory / "tile.hvcc"

            width = 512
            height = 512
            pixels = bytearray()
            for y in range(height):
                for x in range(width):
                    pixels.extend((x % 256, y % 256, (x ^ y) % 256))
            source_ppm.write_bytes(f"P6\n{width} {height}\n255\n".encode("ascii") + pixels)

            subprocess.run(
                ["sips", "-s", "format", "png", str(source_ppm), "--out", str(source_png)],
                check=True,
                capture_output=True,
            )
            result = subprocess.run(
                [str(ENCODER), str(source_png), str(annex_b), "0.9", "rgb4448tile", str(hvcc)],
                check=True,
                capture_output=True,
                text=True,
            )
            protocol = json.loads(result.stdout)
            self.assertEqual(protocol["schema"], "xdremux-hevc-encoder-helper-v1")
            self.assertEqual(protocol["mode"], "rgb4448tile")

            configuration = hvcc.read_bytes()
            self.assertGreater(len(configuration), 23)
            self.assertEqual(configuration[0], 1, "unexpected tile hvcC configuration version")
            self.assertEqual(configuration[1] & 0x1F, 4, "tile encoder did not emit RExt profile")
            self.assertGreater(annex_b.stat().st_size, 0)

    def test_tile_modes_pad_partial_geometry_without_partial_tile_abi(self) -> None:
        with tempfile.TemporaryDirectory(prefix="xdremux-vttile-partial-") as temporary:
            directory = Path(temporary)
            source_pgm = directory / "source.pgm"
            source_png = directory / "source.png"
            annex_b = directory / "tile.hevc"
            hvcc = directory / "tile.hvcc"

            width = 513
            height = 515
            pixels = bytes((x + y) % 256 for y in range(height) for x in range(width))
            source_pgm.write_bytes(f"P5\n{width} {height}\n255\n".encode("ascii") + pixels)

            subprocess.run(
                ["sips", "-s", "format", "png", str(source_pgm), "--out", str(source_png)],
                check=True,
                capture_output=True,
            )
            subprocess.run(
                [str(ENCODER), str(source_png), str(annex_b), "0.9", "mono8tile", str(hvcc)],
                check=True,
                capture_output=True,
            )

            configuration = hvcc.read_bytes()
            self.assertGreater(len(configuration), 23)
            self.assertEqual(configuration[1] & 0x1F, 4, "monochrome tile encoder did not emit RExt")
            self.assertEqual(configuration[16] & 0x03, 0, "monochrome tile hvcC has chroma")

            data = annex_b.read_bytes()
            starts = [index for index in range(len(data) - 4) if data[index : index + 4] == b"\0\0\0\1"]
            idr_count = sum(1 for index in starts if ((data[index + 4] >> 1) & 0x3F) in (19, 20))
            self.assertEqual(idr_count, 4, "513x515 input was not padded to a 2x2 full-tile grid")


if __name__ == "__main__":
    unittest.main()
