from pathlib import Path
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "scripts/train_mixed_selfpair.py"

class MixedSelfPairTrainingTests(unittest.TestCase):
    def test_training_entrypoint_declares_explicit_mixed_mode(self):
        source = SCRIPT.read_text()
        self.assertIn("mixed_true_pair_and_single_image_self_pair", source)
        self.assertIn("selfpair_probability", source)
        self.assertIn("consistency_weight", source)
        self.assertIn("splitCounts", source)

    def test_training_report_is_calibration_only(self):
        source = SCRIPT.read_text()
        self.assertIn('"status":"calibration-only"', source)
        self.assertNotIn('split == "heldout"', source)

if __name__ == "__main__":
    unittest.main()
