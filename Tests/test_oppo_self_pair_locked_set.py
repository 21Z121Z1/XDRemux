import json
import subprocess
import tempfile
import unittest
from pathlib import Path


class OppoSelfPairLockedSetTests(unittest.TestCase):
    def test_freeze_selects_smallest_hash_per_model_and_excludes_history(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first = root / "first.heic"
            second = root / "second.heic"
            excluded = root / "IMG20260502134950.heic"
            for path in (first, second, excluded):
                path.write_bytes(path.name.encode())
            inventory = root / "inventory.json"
            inventory.write_text(json.dumps({"rows": [
                {"path": str(first), "sha256": "b" * 64, "model": "X", "suffix": ".heic"},
                {"path": str(second), "sha256": "a" * 64, "model": "X", "suffix": ".heic"},
                {"path": str(excluded), "sha256": "0" * 64, "model": "Y", "suffix": ".heic"},
            ]}))
            history = root / "history.json"
            history.write_text(json.dumps({"comparison": [{"scene": "IMG20260502134950"}]}))
            output = root / "out"
            subprocess.run([
                "python3", "scripts/freeze_oppo_self_pair_locked_set.py",
                "--inventory", str(inventory), "--historical-audit", str(history),
                "--output", str(output), "--head", "test-head",
            ], check=True)
            manifest = json.loads((output / "manifest.json").read_text())
            self.assertEqual(manifest["sampleCount"], 1)
            self.assertEqual(manifest["samples"][0]["sha256"], "a" * 64)
            self.assertEqual(manifest["historicalBoundary"]["sceneNames"], ["IMG20260502134950"])


if __name__ == "__main__":
    unittest.main()
