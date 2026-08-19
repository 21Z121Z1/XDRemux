import unittest
from pathlib import Path

import numpy as np

from xdremux_py.apple_reverse_key1_training import (
    GRID_LONG,
    GRID_SHORT,
    INPUT_CHANNELS,
    INPUT_SIZE,
    KEY1_BYTE_LENGTH,
    build_model,
    decode_key1,
    encode_key1,
    identity_key1,
    input_features,
    split_for_session,
)


class ReverseKey1TrainingTests(unittest.TestCase):
    def test_identity_template_has_native_quadratic_diagonal(self) -> None:
        identity = identity_key1()
        self.assertEqual(identity.shape, (12, 12, 8, 10, 3))
        self.assertEqual(int(np.count_nonzero(identity)), 12 * 12 * 8 * 3)
        self.assertTrue(np.all(identity[:, :, :, 1, 0] == 1))
        self.assertTrue(np.all(identity[:, :, :, 2, 1] == 1))
        self.assertTrue(np.all(identity[:, :, :, 3, 2] == 1))

    def test_key1_round_trip_preserves_landscape_and_portrait_layouts(self) -> None:
        rng = np.random.default_rng(42)
        values = rng.standard_normal(KEY1_BYTE_LENGTH // 2).astype("<f2")
        payload = values.tobytes()
        for width, height, grid in (
            (4032, 3024, (GRID_LONG, GRID_SHORT)),
            (3024, 4032, (GRID_SHORT, GRID_LONG)),
        ):
            decoded, mask, grid_width, grid_height = decode_key1(
                payload, display_width=width, display_height=height
            )
            self.assertEqual((grid_width, grid_height), grid)
            self.assertEqual(int(mask.sum()), GRID_LONG * GRID_SHORT)
            self.assertEqual(
                encode_key1(
                    decoded,
                    width_slots=grid_width,
                    height_slots=grid_height,
                ),
                payload,
            )

    def test_input_features_keep_pair_delta_and_ycbcr_delta_separate(self) -> None:
        images = np.zeros((2, 3, INPUT_SIZE, INPUT_SIZE), dtype=np.uint8)
        images[0, 0] = 255
        features = input_features(images)
        self.assertEqual(features.shape, (INPUT_CHANNELS, INPUT_SIZE, INPUT_SIZE))
        self.assertTrue(np.allclose(features[0], 1.0))
        self.assertTrue(np.allclose(features[6], 1.0))
        self.assertTrue(np.allclose(features[9], 0.2126))

    def test_session_split_is_stable(self) -> None:
        values = [split_for_session(f"session-{index}") for index in range(100)]
        self.assertEqual(values, [split_for_session(f"session-{index}") for index in range(100)])
        self.assertEqual(set(values), {"train", "calibration", "heldout"})

    def test_model_is_identity_centered_and_structured(self) -> None:
        try:
            import torch
        except ImportError:
            self.skipTest("PyTorch training extra is unavailable")
        torch.manual_seed(7)
        model = build_model(np.ones((8, 10, 3), dtype=np.float32))
        value = torch.zeros((1, INPUT_CHANNELS, INPUT_SIZE, INPUT_SIZE))
        output = model(value)
        self.assertEqual(tuple(output.shape), (1, 12, 12, 8, 10, 3))
        self.assertTrue(
            torch.equal(output, torch.from_numpy(identity_key1()).unsqueeze(0))
        )
        self.assertLess(sum(parameter.numel() for parameter in model.parameters()), 2_000_000)


if __name__ == "__main__":
    unittest.main()
