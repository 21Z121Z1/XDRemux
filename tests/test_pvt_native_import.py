"""Portable checks for direct-package admission and isolated probe boundaries.

These checks do not execute or establish Apple native import.
"""
import copy
import ast
import contextlib
import io
import inspect
import json
import os
import plistlib
from pathlib import Path
import tempfile
import sys
import time
import unittest

from scripts.apple.pvt_native_import import check_native_import as probe


class PVTNativeImportTests(unittest.TestCase):
    def test_cold_sdk_and_reader_compile_before_creating_or_booting_a_simulator(self):
        calls = {node.args[0].value: node for node in ast.walk(ast.parse(inspect.getsource(probe.main)))
                 if isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id == 'run'
                 and node.args and isinstance(node.args[0], ast.Constant)}
        self.assertLess(calls['compile-sdk-canary'].lineno, calls['create-owned-simulator'].lineno)
        self.assertLess(calls['compile-reader'].lineno, calls['create-owned-simulator'].lineno)
        for name, budget in [('compile-sdk-canary', 240), ('compile-reader', 120)]:
            self.assertEqual(next(k.value.value for k in calls[name].keywords if k.arg == 'timeout'), budget)

    @unittest.skipUnless(os.name == 'posix', 'native compile uses POSIX process ownership')
    def test_cold_compile_timeout_keeps_owned_process_stage_and_stops_its_group(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            result = probe.run_owned_process([sys.executable, '-c', 'import time; time.sleep(30)'],
                                            root, root / 'compile.stdout', root / 'compile.stderr', timeout=.3,
                                            diagnostic_path=root / 'compile.processes.json')
            self.assertEqual(result['outcome'], 'uncertain-after-timeout')
            self.assertTrue(result['owned_process_group_stopped'])
            snapshots = json.loads((root / 'compile.processes.json').read_text())
            self.assertGreaterEqual(len(snapshots), 1)
            self.assertTrue(any(row['pid'] == result['owned_process_group']
                                for snapshot in snapshots for row in snapshot.get('processes', [])))
            self.assertTrue(all(row['pgid'] == result['owned_process_group']
                                for snapshot in snapshots for row in snapshot.get('processes', [])))

    @unittest.skipUnless(os.name == 'posix', 'owned simulator runner uses POSIX process groups')
    def test_missing_native_activity_stops_only_its_owned_ui_process_without_waiting_for_timeout(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            started = time.monotonic()
            result = probe.run_share_ui(
                [sys.executable, '-u', '-c',
                 'import time; print("PVT_SHARE_UI_OBSERVATION: phase=share-control save_candidates=1"); '
                 'print("PVT_SHARE_UI_OBSERVATION: phase=share-url save_candidates=0"); '
                 'print("PVT_SHARE_UI_BLOCKED: no Photos save action", flush=True); time.sleep(30)'],
                root, root / 'ui.stdout', root / 'ui.stderr', timeout=5)
            self.assertLess(time.monotonic() - started, 3)
            self.assertEqual(result['outcome'], 'native-save-activity-unavailable')
            self.assertIn('PVT_SHARE_UI_BLOCKED:', (root / 'ui.stdout').read_text())
            self.assertIsNotNone(result['exit_code'])
            self.assertIs(result['owned_process_group_stopped'], True)
            self.assertEqual(result['ui_observations'], [{'phase': 'share-control', 'save_candidates': 1},
                                                        {'phase': 'share-url', 'save_candidates': 0}])

    @unittest.skipUnless(os.name == 'posix', 'owned simulator runner uses POSIX process groups')
    def test_ui_timeout_and_normal_exit_remain_distinct_from_native_activity_absence(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            result = probe.run_share_ui([sys.executable, '-c', 'import time; time.sleep(30)'],
                                        root, root / 'timeout.stdout', root / 'timeout.stderr', timeout=.2)
            self.assertEqual(result['outcome'], 'uncertain-after-timeout')
            result = probe.run_share_ui([sys.executable, '-c', 'print("native test finished")'],
                                        root, root / 'exit.stdout', root / 'exit.stderr', timeout=5)
            self.assertEqual(result['exit_code'], 0)
            self.assertNotIn('outcome', result)

    def test_installed_app_declares_the_scene_configuration_required_by_current_sdk(self):
        bundle = "org.xdremux.audit.PVTReadback.u" + "a" * 32
        info = plistlib.loads(plistlib.dumps(probe.reader_info(bundle)))
        self.assertEqual(info["CFBundleIdentifier"], bundle)
        self.assertEqual(info["CFBundleExecutable"], "PVTReadback")
        scenes = info["UIApplicationSceneManifest"]
        self.assertIs(scenes["UIApplicationSupportsMultipleScenes"], False)
        self.assertEqual(scenes["UISceneConfigurations"]["UIWindowSceneSessionRoleApplication"],
                         [{"UISceneConfigurationName": "PVT audit"}])

    def test_console_retains_native_rejection_and_only_fixed_text_diagnostics(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            (root / "test-share-ui.stderr").write_text("native rejection: unsupported directory\n")
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

    def test_share_completion_requires_the_whole_current_package_file_promise(self):
        shared = {"status": "completed", "activity_completed": True, "is_save_to_camera_roll": True, "share_phase": "share"}
        package = Path("/owned/container/Documents/capture.pvt")
        hashes = {"capture.heic": "still", "capture.mov": "movie", "metadata.plist": "metadata"}
        transfer = {"type_identifier": "com.apple.live-photo", "representation": "file", "share_phase": "share",
                    "is_in_place": False, "whole_package_url": str(package), "member_sha256": hashes}
        probe.check_share_receipt(shared, transfer, package, hashes)
        for mutation in ({}, {"type_identifier": "public.jpeg"}, {"representation": "data"},
                         {"is_in_place": True}, {"whole_package_url": str(package / "capture.heic")},
                         {"member_sha256": {"capture.heic": "still"}}):
            with self.subTest(mutation=mutation):
                changed = {} if not mutation else {**transfer, **mutation}
                with self.assertRaises(probe.Blocked):
                    probe.check_share_receipt(shared, changed, package, hashes)
        for changed in ({**shared, "activity_completed": False}, {**shared, "is_save_to_camera_roll": False}):
            with self.assertRaises(probe.Blocked):
                probe.check_share_receipt(changed, transfer, package, hashes)
        # URL input has no provider-load callback: it is an offered whole URL,
        # and still requires the same independent single-asset readback.
        url_transfer = {**transfer, "representation": "whole-package-url", "type_identifier": "public.directory", "share_phase": "share-url"}
        url_shared = {**shared, "share_phase": "share-url"}
        probe.check_share_receipt(url_shared, url_transfer, package, hashes)
        for mutation in ({"whole_package_url": str(package / 'capture.mov')},
                         {"member_sha256": {"capture.mov": "movie"}},
                         {"representation": "resource-list"}, {"is_in_place": True}, {"share_phase": "share"}):
            with self.assertRaises(probe.Blocked):
                probe.check_share_receipt(url_shared, {**url_transfer, **mutation}, package, hashes)
        with self.assertRaises(probe.Blocked):
            probe.check_share_receipt(shared, url_transfer, package, hashes)
        # A completed share callback remains distinct from library admission.
        baseline, after = self.baseline_and_after()
        after["photo_live"] = False
        with self.assertRaises(probe.Blocked):
            self.check(baseline, after)

    def test_ui_project_is_bound_only_to_the_created_probe_bundle(self):
        source = Path(probe.__file__).parent
        with tempfile.TemporaryDirectory() as raw:
            work = Path(raw)
            bundle = "org.xdremux.audit.PVTReadback.u" + "a" * 32
            project = probe.prepare_share_tests(source, work, bundle)
            code = (project.parent / "PVTShareUITests.swift").read_text()
            self.assertIn('XCUIApplication(bundleIdentifier: "' + bundle + '")', code)
            self.assertNotIn("@@BUNDLE_ID@@", code)
            self.assertTrue((project / "project.pbxproj").is_file())
            self.assertTrue((project / "xcshareddata/xcschemes/PVTShareUITests.xcscheme").is_file())
        with tempfile.TemporaryDirectory() as raw:
            with self.assertRaises(probe.Blocked):
                probe.prepare_share_tests(source, Path(raw), "com.apple.mobileslideshow")

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
            probe.select_device({"runtimes": [{"isAvailable": True, "identifier": "com.apple.iOS-16-0", "version": "16.0"}]}, types)
        with self.assertRaises(probe.Blocked):
            probe.select_device(runtimes, {"devicetypes": []})


if __name__ == "__main__":
    unittest.main()
