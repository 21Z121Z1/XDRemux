from copy import deepcopy
from pathlib import Path
import runpy
import unittest


compare_runs = runpy.run_path(str(Path(__file__).resolve().parents[1] / "scripts/compare_cinematic_alignment.py"))["compare_runs"]


def report(error, ordinary_hash):
    packet = {"type": "video", "codec": "hevc", "packets": 90,
              "packet_bytes_timestamps_and_trim_identical": True}
    frames = []
    for pts in [0.75, 1.5, 2.25]:
        for aperture in [2, 16]:
            frame = {"pts": pts, "f_number": aperture, "decoded_color_identical": True,
                     "render_difference": {"rmse": error, "psnr_db": "40.0", "components": 100},
                     "native_focus_detection": {}, "generated_focus_detection": {}}
            if pts == 0.75 and aperture == 2:
                frame["native_repeat_difference"] = {"identical": True}
            if aperture == 16:
                frame["effect_comparison"] = {"native_effect_rmse": 0.1, "effect_error_rmse": error,
                    "relative_effect_error": error / 0.1, "generated_to_native_effect_strength": 1,
                    "effect_cosine_similarity": 0.99}
                frame["native_focus_region_effect_comparison"] = {"effect_error_rmse": error}
            frames.append(frame)
    return {"runtime": "27.0.1", "head": "test-commit", "assets": [{
        "source": "/samples/example.MOV", "source_sha256": "unchanged-reference",
        "ordinary_sha256": ordinary_hash, "source_unchanged": True, "ordinary_unchanged": True,
        "stripped_compressed_media": [deepcopy(packet)], "compressed_media": [deepcopy(packet)],
        "strip": {"capability": "none", "cinematic_data_removed": True},
        "headless_comparison": {"renderer": "public CNRenderingSession", "quality": "export",
            "texture_format": "RGBA16Float", "input_pixel_format": "P010", "comparison_space": "linear RGB",
            "source_transform": "identity", "native_rendering_version": 5, "frames": frames}}]}


class CinematicAlignmentReportTests(unittest.TestCase):
    def test_identical_media_in_different_mov_containers_is_comparable(self):
        result = compare_runs(report(0.1, "layout-A"), report(0.025, "layout-B"))
        self.assertAlmostEqual(result["assets"][0]["f2_rmse_reduction"], 0.75)
        self.assertAlmostEqual(result["pooled_effect_error_reduction"], 0.75)
        self.assertTrue(result["assets"][0]["ordinary_media_packets_identical_to_reference"])

    def test_unproven_or_changed_media_is_rejected(self):
        original = report(0.1, "layout-A")
        for key in ["stripped_compressed_media", "compressed_media"]:
            changed = report(0.025, "layout-B")
            changed["assets"][0][key][0]["packet_bytes_timestamps_and_trim_identical"] = False
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                compare_runs(original, changed)
        changed = report(0.025, "layout-B")
        changed["assets"][0]["source_sha256"] = "another-reference"
        with self.assertRaisesRegex(RuntimeError, "reference input differs"):
            compare_runs(original, changed)

    def test_changed_native_render_is_not_an_alignment_improvement(self):
        changed = report(0.025, "layout-B")
        changed["assets"][0]["headless_comparison"]["frames"][1]["effect_comparison"]["native_effect_rmse"] = 0.2
        with self.assertRaisesRegex(RuntimeError, "native reference effect changed"):
            compare_runs(report(0.1, "layout-A"), changed)
