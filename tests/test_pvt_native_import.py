"""Portable checks for direct-package admission and isolated probe boundaries.

These checks do not execute or establish Apple native import.
"""
import copy
import contextlib
import io
from pathlib import Path
import tempfile
import unittest

from scripts.apple.pvt_native_import import check_native_import as probe


class PVTNativeImportTests(unittest.TestCase):
    def test_console_retains_native_rejection_and_only_fixed_text_diagnostics(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            (root / "direct-pvt-addmedia.stderr").write_text("native rejection: unsupported directory\n")
            (root / "compile-reader.stderr").write_text("FIRST-ERROR" + "x" * 13000 + "LAST-ERROR")
            (root / "media.jpg").write_bytes(b"never-print-media")
            (root / "unexpected.stderr").write_text("not-an-allowed-diagnostic")
            external = root / "external.txt"
            external.write_text("never-follow-a-link")
            (root / "after.json").symlink_to(external)
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                probe.print_console_receipt(root, {"status": "blocked", "direct_import_exit_code": 2})
            text = output.getvalue()
            self.assertIn('"direct_import_exit_code": 2', text)
            self.assertIn('"status": "blocked"', text)
            self.assertIn("native rejection: unsupported directory\n", text)
            self.assertIn("FIRST-ERROR", text)
            self.assertIn("LAST-ERROR", text)
            self.assertIn("complete text retained in artifact", text)
            for forbidden in ("never-print-media", "not-an-allowed-diagnostic", "never-follow-a-link"):
                self.assertNotIn(forbidden, text)

    def baseline_and_after(self):
        return ({"status": "baseline", "asset_identifiers": ["stock"]},
                {"status": "readback-complete", "asset_identifiers": ["stock", "new"],
                 "asset_local_identifier": "new", "photo_live": True,
                 "resources": [
                     {"is_original_photo": True, "is_original_paired_video": False,
                      "maker_apple_17": "pair-id", "sha256": "still-hash"},
                     {"is_original_photo": False, "is_original_paired_video": True,
                      "content_identifiers": ["pair-id"], "sha256": "video-hash",
                      "still_image_time_samples": [{"value": 30, "timescale": 30, "epoch": 0}]}]})

    def check(self, baseline, after):
        return probe.check_readback(baseline, after, "pair-id",
                                    {"photo": "still-hash", "paired_video": "video-hash"}, 1.)

    def test_stock_assets_are_not_counted_as_new_imports(self):
        baseline, after = self.baseline_and_after()
        facts = self.check(baseline, after)
        self.assertTrue(facts["one_live_photo_asset"])
        self.assertTrue(facts["original_resource_bytes_match"])
        after["resources"].reverse()
        self.assertEqual(self.check(baseline, after), facts)

    def test_missing_extra_removed_or_wrong_asset_cannot_admit_import(self):
        for identifiers, selected in ((["stock"], "stock"),
                                      (["stock", "new", "extra"], "new"),
                                      (["new"], "new"), (["stock", "new"], "stock")):
            with self.subTest(identifiers=identifiers, selected=selected):
                baseline, after = self.baseline_and_after()
                after.update(asset_identifiers=identifiers, asset_local_identifier=selected)
                with self.assertRaises(probe.Blocked):
                    self.check(baseline, after)

    def test_incomplete_native_readback_and_bad_pair_metadata_fail(self):
        mutations = [lambda a: a.update(status="readback-error"),
                     lambda a: a.update(photo_live=False),
                     lambda a: a["resources"].pop(),
                     lambda a: a["resources"].append(copy.deepcopy(a["resources"][0])),
                     lambda a: a["resources"][0].update(maker_apple_17="other"),
                     lambda a: a["resources"][1].update(content_identifiers=["other"]),
                     lambda a: a["resources"][1].update(still_image_time_samples=[]),
                     lambda a: a["resources"][1]["still_image_time_samples"][0].update(value=31),
                     lambda a: a["resources"][1]["still_image_time_samples"][0].update(epoch=1),
                     lambda a: a["resources"][1]["still_image_time_samples"][0].update(timescale=0)]
        for index, mutate in enumerate(mutations):
            with self.subTest(case=index):
                baseline, after = self.baseline_and_after()
                mutate(after)
                with self.assertRaises(probe.Blocked):
                    self.check(baseline, after)

    def test_media_preservation_is_distinct_from_native_admission(self):
        baseline, after = self.baseline_and_after()
        after["resources"][1]["sha256"] = "changed-by-import"
        facts = self.check(baseline, after)
        self.assertTrue(facts["one_live_photo_asset"])
        self.assertFalse(facts["original_resource_bytes_match"])

    def test_failed_import_still_observes_source_and_package_mutation(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            motion = root / "source.jpg"
            motion.write_bytes(b"original-source")
            package = root / "capture.pvt"
            package.mkdir()
            member = package / "image.heic"
            member.write_bytes(b"original-member")
            report = {"status": "blocked", "reason": "native rejection",
                      "synthetic_input_sha256": probe.sha(motion),
                      "package_sha256_before_import": {member.name: probe.sha(member)}}
            probe.observe_media_preservation(report, motion, package)
            self.assertTrue(report["synthetic_input_unchanged"])
            self.assertTrue(report["package_bytes_unchanged"])
            member.write_bytes(b"changed-member")
            motion.write_bytes(b"changed-source")
            probe.observe_media_preservation(report, motion, package)
            self.assertFalse(report["synthetic_input_unchanged"])
            self.assertFalse(report["package_bytes_unchanged"])
            member.unlink()
            package.rmdir()
            probe.observe_media_preservation(report, motion, package)
            self.assertFalse(report["package_bytes_unchanged"])
            self.assertEqual(report["status"], "blocked")
            self.assertEqual(report["reason"], "native rejection")

    def test_synthetic_wrapper_preserves_both_payloads_and_real_offset(self):
        jpeg, movie = b"\xff\xd8synthetic-jpeg\xff\xd9", b"synthetic-mp4"
        wrapped = probe.wrap_motion_photo(jpeg, movie)
        self.assertTrue(wrapped.endswith(jpeg[2:] + movie))
        self.assertIn(f'GCamera:MicroVideoOffset="{len(movie)}"'.encode(), wrapped)
        self.assertIn(b'MicroVideoPresentationTimestampUs="1000000"', wrapped)
        with self.assertRaises(probe.Blocked):
            probe.wrap_motion_photo(b"not-jpeg", movie)

    def test_only_an_installed_compatible_runtime_is_selected(self):
        runtimes = {"runtimes": [{"isAvailable": True, "identifier": "com.apple.iOS-27-0", "version": "27.0"},
                                 {"isAvailable": False, "identifier": "com.apple.iOS-28-0", "version": "28.0"}]}
        types = {"devicetypes": [{"productFamily": "iPhone", "identifier": "phone", "minRuntimeVersion": 27 * 65536}]}
        self.assertEqual(probe.select_device(runtimes, types), ("com.apple.iOS-27-0", "phone"))
        with self.assertRaises(probe.Blocked):
            probe.select_device({"runtimes": []}, types)
        with self.assertRaises(probe.Blocked):
            probe.select_device(runtimes, {"devicetypes": []})


if __name__ == "__main__":
    unittest.main()
