from pathlib import Path
import json
import math
import os
import sys
import subprocess
import time
import tempfile
import unittest
from unittest.mock import patch

from research.texture_styles import probe


class TextureResearchProbeTests(unittest.TestCase):
    def test_crop_requires_finite_positive_area_within_image(self):
        self.assertEqual(probe.crop_values([.03, .04, .94, .92]), (.03, .04, .94, .92))
        self.assertIsNone(probe.crop_values(None))
        for values in ([0, 0, 1], [0, 0, 0, 1], [-.1, 0, 1, 1], [0, 0, 2, 1],
                       [0, 0, math.nan, 1], [0, 0, 1, math.inf]):
            with self.subTest(values=values), self.assertRaises(probe.ProbeError):
                probe.crop_values(values)

    def test_fresh_output_never_reuses_existing_directory_or_symlink(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            existing = root / "existing"
            existing.mkdir()
            (existing / "evidence").write_text("retain")
            dangling = root / "dangling"
            dangling.symlink_to(root / "absent", target_is_directory=True)
            for path in (existing, dangling):
                with self.subTest(path=path), self.assertRaises(probe.ProbeError):
                    probe.fresh_directory(path)
            self.assertEqual((existing / "evidence").read_text(), "retain")
            self.assertTrue(dangling.is_symlink())

    def test_process_failure_preserves_exact_logs(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result = probe.run_case([sys.executable, "-c", "import sys; print('evidence'); sys.stderr.write('failure'); sys.exit(7)"], root, "runtime", 10)
            self.assertFalse(result["succeeded"])
            self.assertEqual(result["exitCode"], 7)
            self.assertEqual((root / result["logs"]["stdout"]["name"]).read_bytes(), b"evidence\n")
            self.assertEqual((root / result["logs"]["stderr"]["name"]).read_bytes(), b"failure")

    @unittest.skipUnless(os.name == "posix", "native probes use POSIX process groups")
    def test_timeout_is_a_failure_with_preserved_logs(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            real_wait = subprocess.Popen.wait
            first = True
            def expire_after_output(process, timeout=None):
                nonlocal first
                if not first:
                    return real_wait(process, timeout=timeout)
                first = False
                deadline = time.monotonic() + 10
                while (root / "runtime.stdout").read_bytes() != b"before\n":
                    if time.monotonic() >= deadline:
                        process.kill()
                        real_wait(process)
                        self.fail("fixture child failed to produce its readiness marker")
                    time.sleep(.01)
                raise subprocess.TimeoutExpired(process.args, timeout)
            with patch.object(subprocess.Popen, "wait", expire_after_output):
                result = probe.run_case([sys.executable, "-u", "-c", "import time; print('before'); time.sleep(60)"], root, "runtime", 10)
            self.assertTrue(result["timedOut"])
            self.assertFalse(result["succeeded"])
            self.assertNotEqual(result["exitCode"], 0)
            self.assertEqual((root / "runtime.stdout").read_bytes(), b"before\n")

    def test_launch_error_is_not_a_success_or_an_erased_case(self):
        with tempfile.TemporaryDirectory() as temporary:
            result = probe.run_case([str(Path(temporary) / "absent")], Path(temporary), "runtime", 1)
            self.assertFalse(result["succeeded"])
            self.assertIn("launchError", result)
            self.assertIn("sha256", result["logs"]["stderr"])

    def test_invalid_timeout_or_case_cannot_create_log_paths(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for timeout in (0, -1, math.nan, math.inf, 301):
                with self.subTest(timeout=timeout), self.assertRaises(probe.ProbeError):
                    probe.run_case([sys.executable], root, "runtime", timeout)
            with self.assertRaises(probe.ProbeError):
                probe.run_case([sys.executable], root, "../escape", 1)
            self.assertEqual(list(root.iterdir()), [])

    def test_existing_logs_are_never_overwritten(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "runtime.stdout").write_bytes(b"original")
            with self.assertRaises(FileExistsError):
                probe.run_case([sys.executable], root, "runtime", 1)
            self.assertEqual((root / "runtime.stdout").read_bytes(), b"original")

    def test_nonfinite_report_does_not_replace_last_complete_report(self):
        with tempfile.TemporaryDirectory() as temporary:
            target = Path(temporary) / "report.json"
            probe.write_report(target, {"complete": True})
            before = target.read_bytes()
            with self.assertRaises(ValueError):
                probe.write_report(target, {"value": math.nan})
            self.assertEqual(target.read_bytes(), before)
            self.assertFalse(target.with_suffix(".tmp").exists())

    def test_failed_case_does_not_erase_or_skip_remainder_of_matrix(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "run"
            products = {"person": Path("/not-invoked")}
            seen = []
            def run(command, directory, name, timeout):
                seen.append(name)
                return {"name": name, "succeeded": len(seen) != 1, "exitCode": 5 if len(seen) == 1 else 0}
            with patch.object(probe.sys, "platform", "darwin"), patch.object(probe, "build", return_value=products), patch.object(probe, "run_case", side_effect=run):
                self.assertEqual(probe.main(["person", "--output", str(output)]), 2)
            report = json.loads((output / "report.json").read_bytes())
            self.assertEqual(tuple(seen), probe.CASES)
            self.assertFalse(report["succeeded"])
            self.assertFalse(report["productAccepted"])
            self.assertFalse(report["deviceValidated"])
            self.assertEqual(len(report["cases"]), 9)

    def test_invalid_request_does_not_build_or_create_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "run"
            with patch.object(probe, "build") as build:
                self.assertEqual(probe.main(["build", "--output", str(output), "--case", "rect-dict"]), 2)
                build.assert_not_called()
                self.assertFalse(output.exists())

    def test_historical_matrix_retains_types_not_product_claims(self):
        evidence = json.loads((probe.ROOT / "evidence/native-standard-matrix.json").read_bytes())
        maker = evidence["maker84Variants"]["nativeStandardComplete"]
        self.assertIs(type(maker["11"]), bool)
        self.assertIs(type(maker["9"]), float)
        self.assertIs(type(maker["8"]), int)
        self.assertEqual(len(evidence["combinations"]), 4)


if __name__ == "__main__":
    unittest.main()
