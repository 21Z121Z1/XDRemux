import unittest
import numpy as np
from research.texture_styles.sparse import materialize_sparse_shift_scale


class TextureSparseLoweringTests(unittest.TestCase):
    def test_nonzero_offset_never_materializes_masked_out_entries(self):
        mask = np.array([[1, 0], [0, 1]], dtype=np.uint8)
        nonzero = np.array([4, 7], dtype=np.int8)
        scale = np.array([[.5]], dtype=np.float32)
        offset = np.array([[2]], dtype=np.int8)
        actual = materialize_sparse_shift_scale(mask, nonzero, scale, offset)
        np.testing.assert_array_equal(actual, [[1, 0], [0, 2.5]])
        # The historical r10 identity omitted this final mask. Keep the
        # counterexample; otherwise this incorrect lowering looks plausible.
        dense_quantized = np.zeros(mask.shape, dtype=np.int8)
        dense_quantized[mask != 0] = nonzero
        invalid_old_identity = (dense_quantized.astype(np.float32) - offset) * scale
        self.assertFalse(np.array_equal(actual, invalid_old_identity))
        self.assertEqual(invalid_old_identity[0, 1], -1)

    def test_block_coordinates_and_c_order_match_scalar_definition(self):
        rng = np.random.default_rng(0x20190)
        for dtype in (np.float16, np.float32):
            for shape, scales in (((8, 12), (2, 3)), ((4, 6, 8), (1, 3, 2)), ((16, 16), (4, 1))):
                mask = (rng.random(shape) > .68).astype(np.uint8)
                data = rng.integers(-8, 8, size=np.count_nonzero(mask), dtype=np.int8)
                scale = (rng.random(scales) * 1.75 + .01).astype(dtype)
                offset = rng.integers(-3, 4, size=scales, dtype=np.int8)
                actual = materialize_sparse_shift_scale(mask, data, scale, offset)
                expected = np.zeros(shape, dtype=dtype)
                cursor = 0
                for index in np.ndindex(shape):
                    if mask[index]:
                        block = tuple(i // (d // s) for i, d, s in zip(index, shape, scales))
                        expected[index] = (dtype(data[cursor]) - offset[block]) * scale[block]
                        cursor += 1
                with self.subTest(dtype=dtype, shape=shape):
                    np.testing.assert_array_equal(actual, expected)

    def test_all_zero_mask_and_optional_offset(self):
        mask = np.zeros((2, 3), dtype=np.uint8)
        result = materialize_sparse_shift_scale(mask, np.array([], np.int8), np.ones((1, 1), np.float16))
        np.testing.assert_array_equal(result, np.zeros((2, 3), np.float16))

    def test_invalid_shapes_types_counts_values_and_overflow_are_explicit(self):
        valid = [np.array([[1, 0]], np.uint8), np.array([2], np.int8), np.ones((1, 1), np.float16), None]
        variants = [
            (0, np.array([[2, 0]], np.uint8)), (0, np.array([[1., 0.]])),
            (1, np.array([], np.int8)), (1, np.array([2], np.int64)),
            (2, np.ones((2, 1), np.float32)), (2, np.ones((1,), np.float32)),
            (2, np.array([[np.nan]], np.float32)), (3, np.ones((1, 2), np.int8)),
            (3, np.ones((1, 1), np.uint8)), (3, np.array([[np.inf]], np.float32)),
        ]
        for index, value in variants:
            args = valid.copy(); args[index] = value
            with self.subTest(index=index, value=value), self.assertRaises(ValueError):
                materialize_sparse_shift_scale(*args)
        with self.assertRaisesRegex(ValueError, "overflowed"):
            materialize_sparse_shift_scale(valid[0], np.array([65504], np.float16), np.array([[2]], np.float16))


if __name__ == "__main__":
    unittest.main()
