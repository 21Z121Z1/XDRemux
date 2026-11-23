from importlib.util import module_from_spec, spec_from_file_location
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from scripts.distribution import install_depthpro2

ROOT = Path(__file__).resolve().parents[1]
SPEC = spec_from_file_location("depthpro2_estimate", ROOT / "model_adapters/depthpro2/estimate.py")
ESTIMATOR = module_from_spec(SPEC)
SPEC.loader.exec_module(ESTIMATOR)


class DepthPro2AdapterTests(unittest.TestCase):
    def test_installer_requires_explicit_license_and_verifies_weights(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            models_dir = root / "models"
            with patch.dict(os.environ, {"XDREMUX_ACCEPT_APPLE_AMLR_LICENSE": ""}):
                with self.assertRaisesRegex(ValueError, "accept-apple-amlr-license"):
                    install_depthpro2.download_models(
                        models_dir,
                        ["s"],
                        accept_license=False,
                    )

            pkg_dir = models_dir / "depthpro2-s_w4.mlpackage"
            weight = pkg_dir / "Data" / "com.apple.CoreML" / "weights" / "weight.bin"
            weight.parent.mkdir(parents=True, exist_ok=True)
            (pkg_dir / "Manifest.json").write_text("{}", encoding="utf-8")
            weight.write_bytes(b"corrupted")
            (models_dir / "LICENSE").write_text("license", encoding="utf-8")
            (models_dir / "README.md").write_text("readme", encoding="utf-8")

            with self.assertRaisesRegex(ValueError, "SHA-256"):
                install_depthpro2.download_models(
                    models_dir,
                    ["s"],
                    accept_license=True,
                )
            self.assertFalse((models_dir / install_depthpro2.LICENSE_RECEIPT_NAME).exists())

            valid_bytes = b"valid-depthpro2-s-weights"
            weight.write_bytes(valid_bytes)
            digest = hashlib.sha256(valid_bytes).hexdigest()
            patched = dict(install_depthpro2.MODEL_CATALOG)
            patched["s"] = dict(patched["s"], weight_sha256=digest)
            with patch.dict(install_depthpro2.MODEL_CATALOG, patched):
                installed = install_depthpro2.download_models(
                    models_dir,
                    ["s"],
                    accept_license=True,
                )
            self.assertEqual(installed, ["depthpro2-s_w4"])
            self.assertTrue((models_dir / install_depthpro2.LICENSE_RECEIPT_NAME).is_file())

    def test_installed_launcher_quotes_paths_and_refuses_replacement(self) -> None:
        with tempfile.TemporaryDirectory(prefix="model's environment ") as temporary:
            root = Path(temporary)
            models_dir = root / "models' dir"
            models_dir.mkdir()
            fake_adapter = root / "xdremux-apple-adapter"
            fake_adapter.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
            fake_adapter.chmod(0o700)

            launcher = install_depthpro2.install_launcher(
                root,
                models_dir,
                python_path=Path(sys.executable),
                adapter_path=fake_adapter,
            )
            help_res = subprocess.run(
                [str(launcher), "--help"],
                capture_output=True,
                text=True,
                timeout=10,
            )
            self.assertEqual(help_res.returncode, 0, help_res.stderr)
            self.assertIn("--input", help_res.stdout)
            before = launcher.read_bytes()
            with self.assertRaises(FileExistsError):
                install_depthpro2.install_launcher(
                    root,
                    models_dir,
                    python_path=Path(sys.executable),
                    adapter_path=fake_adapter,
                )
            self.assertEqual(launcher.read_bytes(), before)
            self.assertEqual(list(root.glob(".depthpro2-install-*")), [])

    def test_estimator_emits_schema_v1_camera_json_from_adapter(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            video = root / "clip.mov"
            video.write_bytes(b"synthetic video bytes")
            adapter = root / "fake-adapter.py"
            adapter.write_text(
                f"""#!{sys.executable}
import json, sys
req = json.loads(sys.stdin.read())
assert req["operation"] == "depthpro2-camera-estimate"
print(json.dumps({{
    "schema_version": 2,
    "depthpro2_camera": {{
        "estimator_name": "depthpro2-fov",
        "revision": "{ESTIMATOR.REPO_REVISION}",
        "weights_sha256": "{'a' * 64}",
        "equivalent_focal_length_35mm": 35.0,
        "horizontal_fov_degrees": 54.0,
        "vertical_fov_degrees": 32.0,
        "samples": [
            {{"time_seconds": t, "image_size": [1920, 1080], "focal_length_pixels": [1780.0, 1780.0], "principal_point_pixels": [960.0, 540.0], "focal_uncertainty_pixels": 89.0}}
            for t in (0.1, 0.5, 0.9)
        ],
        "shared_intrinsics": {{"time_seconds": 0.0, "image_size": [1920, 1080], "focal_length_pixels": [1780.0, 1780.0], "principal_point_pixels": [960.0, 540.0], "focal_uncertainty_pixels": 89.0}}
    }}
}}))
""",
                encoding="utf-8",
            )
            adapter.chmod(stat.S_IRUSR | stat.S_IWUSR | stat.S_IXUSR)
            payload = ESTIMATOR.estimate(video, 3, adapter=adapter)
            self.assertEqual(payload["schema_version"], 1)
            self.assertEqual(len(payload["samples"]), 3)
            self.assertEqual(payload["estimator"]["name"], "depthpro2-fov")
            self.assertEqual(
                payload["source_sha256"],
                hashlib.sha256(video.read_bytes()).hexdigest(),
            )


if __name__ == "__main__":
    unittest.main()
