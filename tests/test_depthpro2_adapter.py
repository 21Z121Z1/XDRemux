import hashlib
import io
import json
import os
import stat
import subprocess
import sys
import tempfile
import unittest
from importlib.util import find_spec, module_from_spec, spec_from_file_location
from pathlib import Path
from unittest.mock import Mock, patch

from model_adapters.depthpro2 import pytorch_depth
from scripts.distribution import install_depthpro2

ROOT = Path(__file__).resolve().parents[1]
SPEC = spec_from_file_location("depthpro2_estimate", ROOT / "model_adapters/depthpro2/estimate.py")
ESTIMATOR = module_from_spec(SPEC)
SPEC.loader.exec_module(ESTIMATOR)


class DepthPro2AdapterTests(unittest.TestCase):
    def test_pytorch_help_is_available_without_model_weights(self) -> None:
        result = subprocess.run(
            [sys.executable, str(ROOT / "model_adapters/depthpro2/pytorch_depth.py"), "--help"],
            capture_output=True,
            text=True,
            timeout=10,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("--model", result.stdout)
        self.assertIn("--native", result.stdout)

    def test_pytorch_installer_checks_license_and_digest_before_publication(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            weights_dir = root / "pytorch"
            receipt = weights_dir / install_depthpro2.LICENSE_RECEIPT_NAME
            filename = "depthpro2-s.safetensors"
            valid = b"good"
            spec = (filename, "https://example.org/s", len(valid), hashlib.sha256(valid).hexdigest())
            with (
                patch.dict(os.environ, {"XDREMUX_ACCEPT_APPLE_AMLR_LICENSE": ""}),
                patch.dict(install_depthpro2.PYTORCH_MODELS, {"s": spec}),
                patch.object(install_depthpro2.urllib.request, "urlopen") as fetch,
            ):
                with self.assertRaisesRegex(ValueError, "accept-apple-amlr-license"):
                    install_depthpro2.download_pytorch_models(root, ["s"], accept_license=False)
                fetch.assert_not_called()

                fetch.side_effect = [io.BytesIO(b"license"), io.BytesIO(b"bad!")]
                with self.assertRaisesRegex(ValueError, "SHA-256"):
                    install_depthpro2.download_pytorch_models(root, ["s"], accept_license=True)
                self.assertFalse((weights_dir / filename).exists())
                self.assertFalse(receipt.exists())

                fetch.side_effect = [io.BytesIO(valid)]
                self.assertEqual(
                    install_depthpro2.download_pytorch_models(root, ["s"], accept_license=True),
                    ["depthpro2-s"],
                )
            self.assertEqual((weights_dir / filename).read_bytes(), valid)
            self.assertEqual(json.loads(receipt.read_text())["modelIDs"], ["depthpro2-s"])
            self.assertFalse((root / receipt.name).exists())

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

    @unittest.skipUnless(find_spec("numpy") and find_spec("PIL"), "requires NumPy and Pillow")
    def test_pytorch_portrait_sidecar_returns_stored_pixel_alignment(self) -> None:
        import numpy as np
        from PIL import Image, ImageOps

        stored = np.arange(1, 16, dtype=np.float32).reshape(3, 5)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for orientation in range(1, 9):
                with self.subTest(orientation=orientation):
                    image = Image.fromarray(stored)
                    image.getexif()[274] = orientation
                    display = np.asarray(ImageOps.exif_transpose(image))
                    estimator = Mock(model_id="depthpro2-s", weights_sha256="a" * 64)
                    estimator.estimate.return_value = (display, 35.0)
                    output = root / f"orientation-{orientation}.hdis"
                    response = pytorch_depth.portrait_disparity(
                        estimator,
                        {
                            "schema_version": 2,
                            "operation": "depthpro2-disparity",
                            "input_path": str(root / "image.heic"),
                            "output_path": str(output),
                            "depthpro2_disparity": {
                                "target_width": 5,
                                "target_height": 3,
                                "orientation": orientation,
                            },
                        },
                    )["depthpro2_disparity"]
                    np.testing.assert_array_equal(np.fromfile(output, dtype="<f2").reshape(3, 5), stored)
                    self.assertEqual(int.from_bytes(b"hdis", "big"), response["pixel_format"])
                    self.assertEqual((response["width"], response["height"]), (5, 3))
                    self.assertEqual(response["p50_disparity"], 8.0)
                    self.assertEqual(response["focal_length_35mm"], 35.0)

    @unittest.skipUnless(
        find_spec("torch") and find_spec("numpy") and find_spec("PIL"),
        "requires PyTorch, NumPy and Pillow",
    )
    def test_pytorch_half_map_dimensions_survive_capping_and_exif_rotation(self) -> None:
        import numpy as np
        import torch
        from PIL import Image

        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "odd.png"
            exif = Image.Exif()
            exif[274] = 6
            Image.new("RGB", (101, 75)).save(source, exif=exif)
            estimator = pytorch_depth.DepthEstimator.__new__(pytorch_depth.DepthEstimator)
            estimator.device = torch.device("cpu")
            estimator.mean = torch.zeros((1, 3, 1, 1))
            estimator.std = torch.ones((1, 3, 1, 1))
            estimator.memory_format = torch.channels_last
            estimator.model = Mock()
            estimator.model.predict_inverse_depth.side_effect = lambda x: torch.full(
                (1, *x.shape[-2:]), 0.5
            )
            for native, cap, expected_input in (
                (False, 0, (51, 38)),
                (False, 32, (32, 24)),
                (True, 0, (101, 75)),
            ):
                with self.subTest(native=native, cap=cap):
                    estimator.native = native
                    estimator.max_side = cap
                    disparity, _ = estimator.estimate(source, focal_px=75.0)
                    actual_input = estimator.model.predict_inverse_depth.call_args.args[0]
                    self.assertEqual(tuple(actual_input.shape[-2:]), expected_input)
                    self.assertEqual(disparity.shape, (51, 38))
                    np.testing.assert_allclose(disparity, 0.5, rtol=1e-6)


if __name__ == "__main__":
    unittest.main()
