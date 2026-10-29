import plistlib
from pathlib import Path
import tempfile
import unittest

from research.styles_fidelity.tone_attribution import substitute


class ToneAttributionTests(unittest.TestCase):
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
