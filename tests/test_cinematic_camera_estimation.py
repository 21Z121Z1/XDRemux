from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = spec_from_file_location("cinematic_camera_estimation", ROOT / "research/cinematic_camera/estimate.py")
ESTIMATOR = module_from_spec(SPEC)
SPEC.loader.exec_module(ESTIMATOR)


class CinematicCameraEstimationTests(unittest.TestCase):
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
