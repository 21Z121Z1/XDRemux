import plistlib
from pathlib import Path
import tempfile
import unittest

from research.styles_fidelity.tone_attribution import substitute


class ToneAttributionTests(unittest.TestCase):
    def test_linear_components_can_be_substituted_independently(self):
        original = {'4': 4., 'h': .75, 'i': {'Gain': 3., 'OriginalRangeMax': .5}}
        generated = {'4': 6., 'h': 2., 'i': {'Gain': 8., 'OriginalRangeMax': .25},
                     '1': b'generated coefficients', '3': b'generated curve'}
        for group, key in [('thumbnail_pixels', None), ('baseline_exposure', '4'),
                           ('base_gain', 'h'), ('thumbnail_encoding', 'i')]:
            with self.subTest(group=group), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                for name, metadata in [('base', generated), ('reference', original)]:
                    (root / name).mkdir()
                    (root / name / 'styles.plist').write_bytes(plistlib.dumps(metadata))
                    (root / name / 'linear.heic').write_bytes(name.encode())
                substitute(root / 'base', root / 'reference', root / 'mixed', (group,))
                expected = dict(generated)
                if key is not None:
                    expected[key] = original[key]
                self.assertEqual(plistlib.loads((root / 'mixed/styles.plist').read_bytes()), expected)
                self.assertEqual((root / 'mixed/linear.heic').read_bytes(),
                                 b'reference' if key is None else b'base')

    def test_substitution_changes_only_the_selected_evaluator_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            base, reference, output = [root / name for name in ('base', 'reference', 'mixed')]
            base.mkdir()
            reference.mkdir()
            generated = {'1': b'generated coefficients', '3': b'generated curve', '4': 6.0,
                         'h': 2.0, 'i': {'Gain': 8.0}, '6': {'mean': .25}}
            original = {'1': b'withheld coefficients', '3': b'withheld curve', '4': 4.0,
                        'h': .75, 'i': {'Gain': 3.0}, '6': {'mean': .5}}
            for folder, metadata, pixels in [(base, generated, b'generated thumbnail'),
                                              (reference, original, b'reference thumbnail')]:
                (folder / 'styles.plist').write_bytes(plistlib.dumps(metadata))
                (folder / 'linear.heic').write_bytes(pixels)
            before = {str(p): p.read_bytes() for folder in (base, reference) for p in folder.iterdir()}
            substitute(base, reference, output, ('linear_exposure',))
            expected = {**generated, '4': 4.0, 'h': .75, 'i': {'Gain': 3.0}}
            self.assertEqual(plistlib.loads((output / 'styles.plist').read_bytes()), expected)
            self.assertEqual((output / 'linear.heic').read_bytes(), b'reference thumbnail')
            self.assertEqual({str(p): p.read_bytes() for folder in (base, reference)
                              for p in folder.iterdir()}, before)

    def test_absent_reference_mask_does_not_retain_generated_mask(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ('base', 'reference'):
                (root / name).mkdir()
                (root / name / 'styles.plist').write_bytes(plistlib.dumps({'1': b'coefficients'}))
            (root / 'base/inputSkinMatteImage.heic').write_bytes(b'generated skin')
            substitute(root / 'base', root / 'reference', root / 'mixed', ('masks',))
            self.assertFalse((root / 'mixed/inputSkinMatteImage.heic').exists())
            self.assertTrue((root / 'base/inputSkinMatteImage.heic').exists())
