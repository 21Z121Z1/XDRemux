import inspect
import json
import unittest

import numpy as np

from research.oppo_styles.calibration import ChannelBias, apply_channel_bias, fit_channel_bias


class FrozenStyleCalibrationTests(unittest.TestCase):
    def setUp(self):
        self.prediction = np.zeros((2, 2, 10, 3))
        self.target = np.full_like(self.prediction, 4.0)
        self.mask = np.asarray([[True, False], [True, False]])
        self.scales = np.ones((10, 3))

    def test_fit_is_masked_ridge_not_an_averaged_grid_or_per_pixel_parameter(self):
        self.target[:, 1] = 900.0
        fitted = fit_channel_bias(self.prediction, self.target, self.mask, self.scales, ridge=2)
        np.testing.assert_array_equal(fitted.coefficients, np.full(30, 2.0))
        self.assertEqual(len(fitted.coefficients), 30)

    def test_heldout_application_uses_stored_coefficients_not_new_labels(self):
        fitted = fit_channel_bias(self.prediction, self.target, self.mask, self.scales, ridge=2)
        restored = ChannelBias.from_dict(json.loads(json.dumps(fitted.to_dict())))
        heldout = np.full((3, 4, 10, 3), 11.0)
        result = apply_channel_bias(heldout, self.scales, restored)
        np.testing.assert_array_equal(result, np.full_like(heldout, 13.0))
        np.testing.assert_array_equal(heldout, np.full_like(heldout, 11.0))
        self.assertNotIn('target', inspect.signature(apply_channel_bias).parameters)
        self.assertNotIn('mask', inspect.signature(apply_channel_bias).parameters)
        self.target[:] = -500.0
        np.testing.assert_array_equal(apply_channel_bias(heldout, self.scales, restored), result)

    def test_coefficients_survive_space_and_plane_broadcast_without_refitting(self):
        frozen = ChannelBias(tuple(float(x) for x in range(30)), 10.0)
        scales = np.full((10, 3), 2.0)
        result = apply_channel_bias(np.zeros((1, 12, 9, 8, 10, 3)), scales, frozen)
        np.testing.assert_array_equal(result[0, 11, 8, 7], np.arange(30).reshape(10, 3) * 2)

    def test_invalid_labels_masks_scales_and_regularization_fail(self):
        for changed in ('target_shape', 'mask_type', 'empty_mask', 'zero_scale', 'nan_target', 'nan_ridge'):
            with self.subTest(changed=changed):
                target, mask, scales, ridge = self.target.copy(), self.mask.copy(), self.scales.copy(), 10.0
                if changed == 'target_shape': target = target[:1]
                if changed == 'mask_type': mask = mask.astype(float)
                if changed == 'empty_mask': mask[:] = False
                if changed == 'zero_scale': scales[0, 0] = 0
                if changed == 'nan_target': target[0, 0, 0, 0] = np.nan
                if changed == 'nan_ridge': ridge = np.nan
                with self.assertRaises(ValueError):
                    fit_channel_bias(self.prediction, target, mask, scales, ridge=ridge)

    def test_frozen_record_is_strict_and_finite(self):
        record = ChannelBias((0.0,) * 30, 10.0).to_dict()
        for broken in ({**record, 'extra': 1}, {**record, 'schema': True},
                       {**record, 'coefficients': [0.0]}, {**record, 'ridge': False},
                       {**record, 'coefficients': [float('inf')] * 30}):
            with self.subTest(record=broken), self.assertRaises(ValueError):
                ChannelBias.from_dict(broken)

    def test_overflow_cannot_publish_finite_looking_coefficients(self):
        target = np.full_like(self.target, 1e308)
        with self.assertRaises(ValueError):
            fit_channel_bias(-target, target, self.mask, self.scales)
        with self.assertRaises(ValueError):
            apply_channel_bias(target, np.full((10, 3), 1e308), ChannelBias((1e308,) * 30, 1))


if __name__ == '__main__':
    unittest.main()
