import json
import subprocess
import tempfile
import unittest
from pathlib import Path


class FreezeNativeConsumerManifestTests(unittest.TestCase):
    def test_fixed_split_and_smallest_hash_selection(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            rows = []
            for split, suffix in (("calibration", "a"), ("heldout", "b")):
                for model_index, model in enumerate(("iPhone 16", "iPhone 16 Pro", "iPhone 17", "iPhone 17 Pro")):
                    token = f"{suffix}{model_index}"
                    path = root / f"{split}-{model_index}.heic"
                    path.write_bytes(token.encode())
                    rows.append({"split": split, "Model": model, "captureSession": f"{split}-{model}",
                             "sourcePath": str(path), "sourceSHA256": suffix * 64,
                             "samplePath": f"samples/{token}.npz", "relativePath": path.name,
                             "displayWidth": 4032, "displayHeight": 3024})
            dataset = root / "dataset.json"
            dataset.write_text(json.dumps({"samples": rows}))
            output = root / "out"
            subprocess.run(["python3", "scripts/freeze_native_consumer_manifest.py",
                            "--dataset", str(dataset), "--output", str(output), "--head", "test"], check=True)
            value = json.loads((output / "manifest.json").read_text())
            self.assertEqual(value["sampleCount"], 8)
            self.assertEqual({row["split"] for row in value["samples"]}, {"calibration", "heldout"})


if __name__ == "__main__":
    unittest.main()
