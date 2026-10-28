import copy
import unittest

import numpy as np

from research.styles_fidelity.blind import choose_candidate, score
from research.styles_fidelity.texture import area_rgba, numeric_leaves, pixel_person


class JointCalibrationTests(unittest.TestCase):
    def test_selection_uses_training_error_and_keeps_default_on_exact_tie(self):
        default = {'passed': True, 'training_rmse': 0.1, 'holdout_rmse': 0.8, 'index': 0}
        other = {'passed': True, 'training_rmse': 0.2, 'holdout_rmse': 0.0, 'index': 1}
        self.assertIs(choose_candidate([default, other]), default)
        self.assertIs(choose_candidate([default, {**other, 'training_rmse': 0.1}]), default)
        self.assertEqual(choose_candidate([default, {**other, 'training_rmse': 0.05}])['index'], other['index'])

    def test_failed_and_nonfinite_trials_cannot_win(self):
        with self.assertRaises(ValueError):
            choose_candidate([{'passed': False, 'training_rmse': 0},
                              {'passed': True, 'training_rmse': float('nan')}])

    def test_native_error_metrics_measure_actual_values_and_reject_geometry_drift(self):
        reference = np.zeros((2, 2, 3), dtype='f4')
        actual = reference.copy()
        actual[0, 0, 0] = 1
        result = score(reference, actual)
        self.assertAlmostEqual(result['rmse'], np.sqrt(1 / 12))
        self.assertAlmostEqual(result['mae'], 1 / 12)
        self.assertEqual(result['maximum_absolute'], 1)
        with self.assertRaises(ValueError):
            score(reference, actual[:1])

    def test_hdr_area_reduction_preserves_values_above_sdr_white(self):
        image = np.array([[[2, 4, 6, 1], [4, 6, 8, 1]],
                          [[6, 8, 10, 1], [8, 10, 12, 1]]], dtype='f4')
        np.testing.assert_array_equal(area_rgba(image, 1, 1), [[[5, 7, 9, 1]]])
        constant = np.full((7, 11, 4), 3.125, dtype='f4')
        np.testing.assert_array_equal(area_rgba(constant, 3, 2), np.full((2, 3, 4), 3.125))

    def test_final_person_mapping_uses_working_dimensions_without_modifying_stored_data(self):
        rect = {'x': .1, 'y': .2, 'width': .25, 'height': .5}
        person = {key: rect.copy() for key in ('faceROI', 'faceSkinROI', 'instanceROI')}
        person.update({'faceLandmarks': [{'point': {'x': .5, 'y': .25}}],
                       'imageStats': {'Mattify': {'Mean': .75}}})
        original = copy.deepcopy(person)
        pixels = pixel_person(person, 100, 200)
        self.assertEqual(pixels['faceROI'], {'x': 10, 'y': 40, 'width': 25, 'height': 100})
        self.assertEqual(pixels['faceLandmarks'][0]['point'], {'x': 50, 'y': 50})
        self.assertEqual(person, original)
        self.assertEqual(pixels['imageStats'], person['imageStats'])

    def test_statistics_comparison_does_not_treat_missing_fields_as_zero(self):
        self.assertEqual(numeric_leaves({'a': 2, 'flag': True, 'v': [1.5]}),
                         {'/a': 2, '/v/0': 1.5})
        with self.assertRaises(ValueError):
            numeric_leaves({'bad': float('inf')})


if __name__ == '__main__':
    unittest.main()
