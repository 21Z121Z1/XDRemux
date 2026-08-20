import importlib.util
from pathlib import Path
import unittest
import numpy as np

SCRIPT = Path(__file__).resolve().parents[1] / "scripts/evaluate_selfpair_lowdim_adapter.py"
spec = importlib.util.spec_from_file_location("selfpair_lowdim", SCRIPT)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(module)

class SelfPairLowDimAdapterTests(unittest.TestCase):
    def setUp(self):
        shape = (2, 12, 12, 8, 10, 3)
        identity = np.zeros(shape[1:], np.float32)
        identity[:, :, :, 1, 0] = 1; identity[:, :, :, 2, 1] = 1; identity[:, :, :, 3, 2] = 1
        self.d = {"identity": identity, "v3": np.broadcast_to(identity, shape).copy(), "v4": np.broadcast_to(identity, shape).copy(), "target": np.broadcast_to(identity, shape).copy(), "scales": np.ones(shape[1:], np.float32), "mask": np.ones((2, 12, 12), bool), "shuffleV3": np.broadcast_to(identity, shape).copy(), "shuffleV4": np.broadcast_to(identity, shape).copy(), "sessions": np.asarray(["a", "b"])}

    def test_affine_is_identity_centered(self):
        self.d["v4"][:, :, :, :, 4, 0] = .2
        result = module._candidate("global_residual_affine", self.d, alpha=1, gain=.5, bias=.1)
        self.assertAlmostEqual(float(result[0, 0, 0, 0, 4, 0]), .2)

    def test_selection_rule_has_shuffle_guard_and_one_percent_gate(self):
        self.assertIn("heldout is final-only", module.RULE["selection"])
        self.assertEqual(.01, module.RULE["promotion"]["minimumRelativeImprovement"])

if __name__ == "__main__":
    unittest.main()
