import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest

import numpy as np

from research.styles_fidelity.blind import choose_candidate, compare_reverse, score
from research.styles_fidelity.texture import area_rgba, compare_statistics, numeric_leaves, pixel_person, run, validate_working_input


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

    def test_missing_native_reverse_is_failure_even_when_forward_values_are_available(self):
        image = np.full((2, 2, 3), .5, dtype='f4')
        self.assertFalse(compare_reverse(None, image)['passed'])
        self.assertFalse(compare_reverse(image, None)['passed'])
        self.assertNotIn('metrics', compare_reverse(None, image))
        self.assertEqual(score(image, image)['rmse'], 0)
        self.assertTrue(compare_reverse(image, image)['passed'])

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

    def test_statistics_use_native_effect_identity_instead_of_class_suffix(self):
        person = {'imageStats': {'UnderEyeBrightening': {'mean': 0.25},
                                 'SkinSmoothingStandalone': {'mean': 0.5}}}
        for group, expected in [('UnderEyeBrightening', 0.5), ('SkinSmoothingStandalone', 0.25)]:
            result = compare_statistics(person, {'native_effect_name': group,
                'native_output_person_stats': {'mean': 0.75}})
            self.assertEqual(result['statistics_compared_fields'], 1)
            self.assertEqual(result['statistics_maximum_absolute_difference'], expected)
            self.assertEqual(result['statistics_group'], group)

    def test_native_failure_retains_report_reason_without_exporting_person_geometry(self):
        with tempfile.TemporaryDirectory() as root:
            report = Path(root) / 'native.json'
            report.write_text(json.dumps({'stage': 'setup',
                'exception': {'name': 'NativeInput', 'reason': 'missing required effect input'},
                'person_before_stats': {'faceLandmarks': ['private geometry']}}))
            with self.assertRaises(RuntimeError) as failure:
                run([sys.executable, '-c', 'raise SystemExit(1)'], diagnostic=report)
            self.assertIn('exited 1', str(failure.exception))
            self.assertIn('missing required effect input', str(failure.exception))
            self.assertNotIn('faceLandmarks', str(failure.exception))
            self.assertNotIn('private geometry', str(failure.exception))

    def test_each_hdr_effect_must_receive_the_actual_submitted_hdr_pixels(self):
        expected = 'a' * 64
        pixels = {'sha256': expected, 'nonfinite': 0, 'maximum': 2.5}
        self.assertEqual(validate_working_input({'image_pixels': pixels}, expected, True), pixels)
        with self.assertRaises(ValueError):
            validate_working_input({'image_pixels': {**pixels, 'sha256': 'b' * 64}}, expected, True)
        with self.assertRaises(ValueError):
            validate_working_input({'image_pixels': {**pixels, 'maximum': .9}}, expected, True)


if __name__ == '__main__':
    unittest.main()
