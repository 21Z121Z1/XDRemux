import json
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class NativeAlphaGridSummaryTests(unittest.TestCase):
    def test_selection_rule_is_calibration_only_and_missing_is_not_backfilled(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            d = Path(temp)
            manifest = d / "manifest.json"
            manifest.write_text(json.dumps({"samples": [{
                "split": "calibration", "model": "iPhone 16", "sourceSHA256": "a" * 64,
            }]}))
            output = d / "report.json"
            subprocess.run([
                "python3", "scripts/summarize_native_alpha_grid.py",
                "--manifest", str(manifest), "--output", str(output),
            ], cwd=ROOT, check=True)
            value = json.loads(output.read_text())
            self.assertFalse(value["selectionRule"]["heldoutUsedForSelection"])
            self.assertEqual(value["chosenAlpha"], 0.625)
            self.assertIsNone(value["byAlpha"]["0.0"]["aggregateRGBRMSE"])


if __name__ == "__main__":
    unittest.main()
