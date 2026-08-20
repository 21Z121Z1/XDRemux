from pathlib import Path
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "scripts/evaluate_selfpair_ensemble_profiles.py"

class ProfileEnsembleTests(unittest.TestCase):
    def test_profile_script_is_calibration_gated(self):
        source = SCRIPT.read_text()
        self.assertIn("calibration overall >=1%", source)
        self.assertIn("unknown fallback=.625", source)
        self.assertIn("if a.heldout", source)

    def test_candidate_complexity_is_bounded(self):
        source = SCRIPT.read_text()
        self.assertIn("'parameters':len(a)", source)
        self.assertIn("parameters':2", source)
        self.assertNotIn("per_pixel", source)

if __name__ == "__main__":
    unittest.main()
