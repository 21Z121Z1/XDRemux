import base64
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

import numpy as np

from xdremux_py.apple_reverse_key1_training import ReverseKey1Error
from xdremux_py.universal_photographic_style_training import (
    GAIN_MAP_CHANNELS,
    INPUT_SIZE,
    LINEAR_CHANNELS,
    METADATA_FIELDS,
    MODALITY_FIELDS,
    OPTIONAL_MODALITIES_SCHEMA,
    PRIMARY_CHANNELS,
    STYLE_SCALAR_FIELDS,
    build_universal_model,
    decode_style_binary,
    gain_map_features,
    metadata_vector,
    primary_image_features,
    _load_optional_modalities_manifest,
    _warm_start_model_state,
    _consumer_quadratic_proxy,
)
from xdremux_py.universal_photographic_style import (
    UniversalImageInput,
    native_state_resources,
)


class UniversalPhotographicStyleTrainingTests(unittest.TestCase):
    @staticmethod
    def _statistics() -> dict[str, np.ndarray]:
        return {
            "metadataCenter": np.zeros(len(METADATA_FIELDS), dtype=np.float32),
            "metadataScale": np.ones(len(METADATA_FIELDS), dtype=np.float32),
            "metadataActive": np.ones(len(METADATA_FIELDS), dtype=np.float32),
            "key1Scale": np.ones((8, 10, 3), dtype=np.float32),
            "gtcCenter": np.zeros(516, dtype=np.float32),
            "gtcScale": np.ones(516, dtype=np.float32),
            "lightCenter": np.zeros((2, 32, 32), dtype=np.float32),
            "lightScale": np.ones(2, dtype=np.float32),
            "scalarCenter": np.zeros(len(STYLE_SCALAR_FIELDS), dtype=np.float32),
            "scalarScale": np.ones(len(STYLE_SCALAR_FIELDS), dtype=np.float32),
            "scalarLow": np.full(len(STYLE_SCALAR_FIELDS), -10.0, dtype=np.float32),
            "scalarHigh": np.full(len(STYLE_SCALAR_FIELDS), 10.0, dtype=np.float32),
        }

    def test_published_coreml_package_has_recorded_file_identities(self) -> None:
        root = (
            Path(__file__).resolve().parents[1]
            / "Models"
            / "UniversalPhotographicStyleStateNet.mlpackage"
        )
        expected = {
            "Manifest.json": "c31a42263e9e23a378edc04371340794a6e27d9425d9755e9892460d217cd7be",
            "Data/com.apple.CoreML/model.mlmodel": "84c51a998dc293165ec202215bce8d0412e46776d9a5e4b28aa87cdc13798d4c",
            "Data/com.apple.CoreML/weights/weight.bin": "7acd3ed2478aa28e90870140d5c1aaeaa9d929acbe869490c48419495fe7107a",
        }
        for relative, digest in expected.items():
            with self.subTest(relative=relative):
                actual = hashlib.sha256((root / relative).read_bytes()).hexdigest()
                self.assertEqual(actual, digest)

    def test_primary_features_are_finite_and_single_image_only(self) -> None:
        image = np.zeros((3, 256, 256), dtype=np.uint8)
        image[0] = 255
        features = primary_image_features(image)
        self.assertEqual(features.shape, (PRIMARY_CHANNELS, 256, 256))
        self.assertTrue(np.isfinite(features).all())
        self.assertGreater(float(features[0].mean()), float(features[1].mean()))

    def test_metadata_has_explicit_missing_value_mask(self) -> None:
        values, mask = metadata_vector(
            {"displayWidth": 4032, "displayHeight": 3024, "Orientation": 6},
            {"ExposureTime": 0.01, "ISO": 64},
        )
        self.assertEqual(values.shape, (len(METADATA_FIELDS),))
        self.assertEqual(mask.shape, values.shape)
        self.assertEqual(mask[METADATA_FIELDS.index("focal_length_mm")], 0)
        self.assertEqual(mask[METADATA_FIELDS.index("has_gain_map")], 1)
        self.assertEqual(values[METADATA_FIELDS.index("has_gain_map")], 0)

    def test_gain_map_features_use_linear_gain_ratio_contract(self) -> None:
        neutral = gain_map_features(np.ones((INPUT_SIZE, INPUT_SIZE), dtype=np.float32))
        self.assertEqual(neutral.shape, (GAIN_MAP_CHANNELS, INPUT_SIZE, INPUT_SIZE))
        self.assertTrue(np.array_equal(neutral, np.zeros_like(neutral)))
        with self.assertRaises(ReverseKey1Error):
            gain_map_features(np.zeros((INPUT_SIZE, INPUT_SIZE), dtype=np.float32))

    def test_optional_modality_manifest_is_content_addressed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            np.save(
                root / "linear.npy",
                np.full((3, INPUT_SIZE, INPUT_SIZE), 0.5, dtype=np.float32),
            )
            np.save(
                root / "gain.npy",
                np.ones((INPUT_SIZE, INPUT_SIZE), dtype=np.float32),
            )
            manifest = root / "modalities.json"
            manifest.write_text(
                json.dumps(
                    {
                        "schema": OPTIONAL_MODALITIES_SCHEMA,
                        "samples": [
                            {
                                "sourceSHA256": "a" * 64,
                                "linearRGBPath": "linear.npy",
                                "gainMapPath": "gain.npy",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            loaded = _load_optional_modalities_manifest(manifest)["a" * 64]
            self.assertEqual(len(loaded["linearRGBSHA256"]), 64)
            self.assertEqual(len(loaded["gainMapSHA256"]), 64)
            self.assertEqual(Path(loaded["linearRGBPath"]), (root / "linear.npy").resolve())

    def test_binary_labels_are_length_checked(self) -> None:
        encoded = "base64:" + base64.b64encode(b"abcd").decode("ascii")
        self.assertEqual(decode_style_binary(encoded, 4, "test"), b"abcd")
        with self.assertRaises(ReverseKey1Error):
            decode_style_binary(encoded, 3, "test")

    def test_model_forward_has_complete_finite_state_contract(self) -> None:
        try:
            import torch
        except ImportError:
            self.skipTest("PyTorch is unavailable")
        statistics = self._statistics()
        expected = {
            "key1": (1, 12, 12, 8, 10, 3),
            "key1LogVariance": (1, 8, 10, 3),
            "gtc": (1, 516),
            "lightMaps": (1, 2, 32, 32),
            "scalars": (1, len(STYLE_SCALAR_FIELDS)),
            "unstyled": (1, 3, 64, 64),
        }
        for architecture in ("base", "multiscale_large", "multimodal_large"):
            with self.subTest(architecture=architecture):
                model = build_universal_model(
                    statistics, architecture=architecture
                ).eval()
                with torch.no_grad():
                    output = model(
                        torch.zeros((1, PRIMARY_CHANNELS, 256, 256)),
                        torch.zeros((1, len(METADATA_FIELDS))),
                        torch.zeros((1, len(METADATA_FIELDS))),
                    )
                self.assertEqual(
                    {name: tuple(value.shape) for name, value in output.items()},
                    expected,
                )
                self.assertTrue(
                    all(bool(torch.isfinite(value).all()) for value in output.values())
                )

    def test_multimodal_zero_mask_is_identical_to_missing_sidecars(self) -> None:
        try:
            import torch
        except ImportError:
            self.skipTest("PyTorch is unavailable")
        model = build_universal_model(
            self._statistics(), architecture="multimodal_large"
        ).eval()
        primary = torch.rand((1, PRIMARY_CHANNELS, INPUT_SIZE, INPUT_SIZE))
        metadata = torch.zeros((1, len(METADATA_FIELDS)))
        metadata_mask = torch.zeros_like(metadata)
        with torch.no_grad():
            missing = model(primary, metadata, metadata_mask)
            masked = model(
                primary,
                metadata,
                metadata_mask,
                torch.rand((1, LINEAR_CHANNELS, INPUT_SIZE, INPUT_SIZE)),
                torch.rand((1, GAIN_MAP_CHANNELS, INPUT_SIZE, INPUT_SIZE)),
                torch.zeros((1, len(MODALITY_FIELDS))),
            )
        for name in missing:
            self.assertTrue(torch.equal(missing[name], masked[name]), name)

    def test_warm_start_migrates_stem_but_keeps_target_statistics(self) -> None:
        try:
            import torch
        except ImportError:
            self.skipTest("PyTorch is unavailable")
        source_statistics = self._statistics()
        source_statistics["key1Scale"] = np.full((8, 10, 3), 7.0, dtype=np.float32)
        source = build_universal_model(
            source_statistics, architecture="multiscale_large"
        )
        target = build_universal_model(
            self._statistics(), architecture="multimodal_large"
        )
        _warm_start_model_state(
            target,
            source.state_dict(),
            source_architecture="multiscale_large",
            target_architecture="multimodal_large",
        )
        self.assertTrue(torch.equal(target.key1_scale, torch.ones_like(target.key1_scale)))
        source_stem = source.encoder_stages[0][0].weight
        target_stem = target.encoder_stages[0][0].weight
        self.assertTrue(torch.equal(target_stem[:, :PRIMARY_CHANNELS], source_stem))
        self.assertTrue(
            torch.equal(
                target_stem[:, PRIMARY_CHANNELS:],
                torch.zeros_like(target_stem[:, PRIMARY_CHANNELS:]),
            )
        )

    def test_native_state_resources_have_native_byte_lengths(self) -> None:
        image = UniversalImageInput(
            path=Path("sample.jpg"),
            primary=np.zeros((PRIMARY_CHANNELS, 256, 256), dtype=np.float32),
            metadata=np.zeros(len(METADATA_FIELDS), dtype=np.float32),
            metadata_mask=np.zeros(len(METADATA_FIELDS), dtype=np.float32),
            display_width=4032,
            display_height=3024,
            has_raw=False,
            has_gain_map=False,
            source_sha256="0" * 64,
        )
        prediction = {
            "key1": np.zeros((12, 12, 8, 10, 3), dtype=np.float32),
            "key1LogVariance": np.zeros((8, 10, 3), dtype=np.float32),
            "gtc": np.zeros(516, dtype=np.float32),
            "lightMaps": np.zeros((2, 32, 32), dtype=np.float32),
            "scalars": np.zeros(len(STYLE_SCALAR_FIELDS), dtype=np.float32),
        }
        resources = native_state_resources(image, prediction)
        self.assertEqual(len(resources["key1"]), 51_840)
        self.assertEqual(len(resources["gtc"]), 516)
        self.assertEqual(len(resources["c"]), 2_048)
        self.assertEqual(len(resources["d"]), 2_048)
        self.assertEqual(resources["uncertainty"], 1.0)

    def test_consumer_quadratic_proxy_identity_is_neutral(self) -> None:
        try:
            import torch
        except ImportError:
            self.skipTest("PyTorch is unavailable")
        primary = torch.rand((2, PRIMARY_CHANNELS, 256, 256))
        identity = torch.zeros((2, 12, 12, 8, 10, 3))
        identity[..., 1, 0] = 1.0
        identity[..., 2, 1] = 1.0
        identity[..., 3, 2] = 1.0
        actual = _consumer_quadratic_proxy(torch, identity, primary)
        expected = primary[:, :3, ::4, ::4]
        self.assertTrue(torch.allclose(actual, expected, atol=1e-6))


if __name__ == "__main__":
    unittest.main()
