import json
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class NativeConsumerCalibrationSummaryTests(unittest.TestCase):
    def test_missing_response_is_not_backfilled(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            out = Path(temp)
            manifest = out / "manifest.json"
            manifest.write_text(json.dumps({
                "schema": "test",
                "samples": [{
                    "split": "heldout",
                    "model": "iPhone 17",
                    "session": "s",
                    "sourceSHA256": "a" * 64,
                    "sourcePath": "/missing/source.HEIC",
                }],
            }))
            report = out / "report.json"
            markdown = out / "report.md"
            subprocess.run([
                "python3", "scripts/summarize_native_consumer_calibration.py",
                "--manifest", str(manifest),
                "--output-json", str(report),
                "--output-markdown", str(markdown),
            ], cwd=ROOT, check=True)
            value = json.loads(report.read_text())
            row = value["heldout"]["rows"][0]
            self.assertIsNone(row["response"]["aggregateRGBRMSE"])
            self.assertEqual(row["failure"], "candidate_native_conversion_missing")
            self.assertFalse(value["promotion"]["promoted"])


if __name__ == "__main__":
    unittest.main()
