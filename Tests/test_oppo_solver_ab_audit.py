import json
import tempfile
import unittest
from pathlib import Path

from scripts.evaluate_oppo_solver_ab import summarize_scene


class OppoSolverAuditTests(unittest.TestCase):
    def test_missing_full_solver_is_not_backfilled_from_bounded(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            scene = Path(root) / "scene"
            (scene / "seeded-neutral").mkdir(parents=True)
            (scene / "seeded-neutral" / "solver-result.json").write_text(
                json.dumps(
                    {
                        "bestMetrics": {"rmse8": 7.0},
                        "identityMetrics": {"rmse8": 8.0},
                        "nativeResponseValidated": False,
                        "renderRequestCount": 7,
                        "timing": {"totalSeconds": 60.0},
                    }
                ),
                encoding="utf-8",
            )
            result = summarize_scene(scene)
            self.assertIsNone(result["paths"]["fullSolver"])
            self.assertEqual(result["paths"]["boundedOneStepResidual"]["rmse8"], 7.0)
            self.assertIsNone(result["solver"]["fullSeconds"])
            self.assertIsNone(result["solver"]["nativeResponseValidated"])
            self.assertIsNone(result["solver"]["renderRequestCount"])
            self.assertFalse(result["locked"])
            self.assertIsNone(result["paths"]["universalProposal"])

    def test_full_solver_metrics_and_provenance_are_retained(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            scene = Path(root) / "scene"
            (scene / "seeded-neutral").mkdir(parents=True)
            (scene / "baseline-solver").mkdir(parents=True)
            for directory, payload in (
                (
                    "seeded-neutral",
                    {"bestMetrics": {"rmse8": 7.0}, "timing": {"totalSeconds": 60.0}},
                ),
                (
                    "baseline-solver",
                    {
                        "bestMetrics": {"rmse8": 6.2},
                        "identityMetrics": {"rmse8": 8.0},
                        "nativeResponseValidated": True,
                        "renderRequestCount": 11,
                        "timing": {"totalSeconds": 120.0},
                    },
                ),
            ):
                (scene / directory / "solver-result.json").write_text(
                    json.dumps(payload), encoding="utf-8"
                )
            result = summarize_scene(scene)
            self.assertEqual(result["paths"]["fullSolver"]["rmse8"], 6.2)
            self.assertEqual(result["solver"]["fullSeconds"], 120.0)
            self.assertTrue(result["solver"]["nativeResponseValidated"])
            self.assertEqual(result["solver"]["renderRequestCount"], 11)


if __name__ == "__main__":
    unittest.main()
