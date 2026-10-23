from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
import hashlib
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from scripts.distribution import install_geocalib

ROOT = Path(__file__).resolve().parents[1]
SPEC = spec_from_file_location("cinematic_camera_estimation", ROOT / "model_adapters/geocalib/estimate.py")
ESTIMATOR = module_from_spec(SPEC)
SPEC.loader.exec_module(ESTIMATOR)


class CinematicCameraEstimationTests(unittest.TestCase):
    def test_installed_helper_runs_with_quoted_paths_and_refuses_replacement(self):
        with tempfile.TemporaryDirectory(prefix="model's environment ") as temporary:
            root = Path(temporary)
            weights = root / "model's weights.tar"
            weights.write_bytes(b"test weights")
            python = root / "environment python"
            python.symlink_to(sys.executable)
            with patch.object(install_geocalib, "WEIGHTS_SHA256", hashlib.sha256(weights.read_bytes()).hexdigest()):
                helper = install_geocalib.install(python, weights, root)
                result = subprocess.run([str(helper), "--help"], capture_output=True, text=True, timeout=10)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn("--input", result.stdout)
                before = helper.read_bytes()
                with self.assertRaises(FileExistsError):
                    install_geocalib.install(python, weights, root)
                self.assertEqual(helper.read_bytes(), before)
            self.assertEqual(list(root.glob(".geocalib-install-*")), [])

    def test_installer_rejects_unverified_weights_without_publishing_helper(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            weights = root / "weights.tar"
            weights.write_bytes(b"wrong weights")
            with self.assertRaisesRegex(ValueError, "SHA-256"):
                install_geocalib.install(Path(sys.executable), weights, root)
            self.assertFalse((root / "xdremux-geocalib").exists())

    def test_shared_uncertainty_uses_focal_covariance_and_original_pixel_units(self):
        covariance = [[0.0] * 7 for _ in range(7)]
        covariance[2][2] = 0.0004
        covariance[6][6] = 1600.0
        self.assertEqual(ESTIMATOR.shared_focal_uncertainty(covariance, 3, 0.5), 40.0)
        self.assertEqual(ESTIMATOR.shared_focal_uncertainty(covariance, 3, 0.25), 80.0)

    def test_invalid_covariance_does_not_produce_a_confident_fallback(self):
        for value in [-1.0, float("nan"), float("inf")]:
            covariance = [[0.0] * 7 for _ in range(7)]
            covariance[6][6] = value
            with self.assertRaises(ValueError):
                ESTIMATOR.shared_focal_uncertainty(covariance, 3, 0.5)
        with self.assertRaises(ValueError):
            ESTIMATOR.shared_focal_uncertainty([[1.0]], 3, 0.5)
