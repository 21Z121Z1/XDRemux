"""Failed research runs must not publish or reuse successful-run evidence."""
from __future__ import annotations

import contextlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import numpy as np
import torch

from research.oppo_styles import public_pretraining as public, training
from tests.test_solver_public_split import records
from xdremux_py.apple_reverse_key1_training import ReverseKey1Error


class TrainingFailureTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.manifest = self.root / 'manifest.json'
        self.manifest.write_text('{}')
        self.output = self.root / 'run'

    @contextlib.contextmanager
    def native_run(self, *, loss_kind='finite', metric=1.0):
        class ToyModel(torch.nn.Module):
            def __init__(self):
                super().__init__()
                self.weight = torch.nn.Parameter(torch.tensor(0.1))

        class NonfiniteBackward(torch.autograd.Function):
            @staticmethod
            def forward(ctx, value):
                return value.clone()

            @staticmethod
            def backward(ctx, gradient):
                return gradient * float('nan')

        model = ToyModel()
        rows = [{'split': split, 'index': i} for i, split in enumerate(('train', 'calibration', 'heldout'))]
        header = {'sourceCorpusSHA256': '0' * 64, 'inputContract': 'synthetic-test',
                  'outputContract': 'synthetic-test'}
        metrics = {'key1NormalizedMAE': metric, 'gtcNormalizedMAE': 0.0, 'lightMapsNormalizedMAE': 0.0}

        def losses(*args, **kwargs):
            value = model.weight.square()
            if loss_kind == 'nan':
                value = value * float('nan')
            if loss_kind == 'nan-gradient':
                value = NonfiniteBackward.apply(value)
            return value, {'synthetic': float(value.detach())}

        with contextlib.ExitStack() as stack:
            stack.enter_context(patch.object(training, 'load_universal_manifest', return_value=(header, rows)))
            stack.enter_context(patch.object(training, 'universal_state_statistics', return_value={}))
            stack.enter_context(patch.object(training, 'build_universal_model', return_value=model))
            stack.enter_context(patch.object(training, '_UniversalDataset', return_value=[(torch.zeros(1),)]))
            stack.enter_context(patch.object(training, '_model_forward', return_value={}))
            stack.enter_context(patch.object(training, '_losses', side_effect=losses))
            evaluate = stack.enter_context(patch.object(training, '_evaluate', return_value=metrics))
            yield evaluate

    def native_config(self):
        return training.UniversalTrainingConfig(self.manifest, self.output, epochs=1, batch_size=1, device='cpu')

    def assert_no_checkpoint(self):
        self.assertEqual(list(self.output.glob('*.pt')), [])
        self.assertFalse((self.output / 'report.json').exists())

    def test_existing_native_run_is_never_reused_or_overwritten(self):
        self.output.mkdir()
        previous = self.output / 'best.pt'
        previous.write_bytes(b'previous successful run must survive')
        with self.native_run(), self.assertRaises(FileExistsError):
            training.train_universal_model(self.native_config())
        self.assertEqual(previous.read_bytes(), b'previous successful run must survive')

    def test_nonfinite_loss_stops_before_selection_or_checkpoint(self):
        with self.native_run(loss_kind='nan') as evaluate:
            with self.assertRaisesRegex(ReverseKey1Error, 'non-finite'):
                training.train_universal_model(self.native_config())
            evaluate.assert_not_called()
        self.assert_no_checkpoint()

    def test_finite_loss_with_nonfinite_gradient_stops_before_selection(self):
        with self.native_run(loss_kind='nan-gradient') as evaluate:
            with self.assertRaisesRegex(RuntimeError, 'non-finite'):
                training.train_universal_model(self.native_config())
            evaluate.assert_not_called()
        self.assert_no_checkpoint()

    def test_nonfinite_calibration_cannot_reuse_an_old_best_checkpoint(self):
        with self.native_run(metric=float('nan')) as evaluate:
            with self.assertRaisesRegex(ReverseKey1Error, 'non-finite'):
                training.train_universal_model(self.native_config())
            self.assertEqual(evaluate.call_count, 1)
        self.assert_no_checkpoint()

    @contextlib.contextmanager
    def public_run(self, *, nonfinite=False, mutate_manifest=False):
        source_rows = records(self.root, 3)
        self.manifest.write_text(json.dumps({'schema': public.PUBLIC_CORPUS_SCHEMA, 'samples': source_rows}))
        manifest = self.manifest
        visits = []

        class ToyModel(torch.nn.Module):
            def __init__(self):
                super().__init__()
                self.weight = torch.nn.Parameter(torch.tensor(0.01))

            def forward(self, primary, *unused):
                visits.append(self.training)
                if mutate_manifest:
                    manifest.write_text('{"changed": true}')
                key = self.weight * (float('nan') if nonfinite else 1)
                return {'key1': key.expand(len(primary), 12, 12, 8, 10, 3),
                        'unstyled': self.weight.expand(len(primary), 3, 64, 64)}

        with patch.object(public, 'build_universal_model', return_value=ToyModel()):
            yield visits

    def public_config(self):
        return public.PublicPretrainingConfig(self.manifest, self.output, epochs=1, batch_size=1, transforms_per_image=1)

    def test_nonfinite_public_baseline_never_reaches_training_or_heldout(self):
        with self.public_run(nonfinite=True) as visits:
            with self.assertRaisesRegex(ReverseKey1Error, 'non-finite'):
                public.pretrain_public_synthetic_style(self.public_config())
            self.assertEqual(visits, [False])
        self.assert_no_checkpoint()

    def test_existing_public_output_is_not_overwritten(self):
        self.output.mkdir()
        previous = self.output / 'synthetic-pretrained.pt'
        previous.write_bytes(b'previous public evidence')
        with self.public_run(), self.assertRaises(FileExistsError):
            public.pretrain_public_synthetic_style(self.public_config())
        self.assertEqual(previous.read_bytes(), b'previous public evidence')

    def test_changed_public_manifest_cannot_be_attached_to_trained_weights(self):
        with self.public_run(mutate_manifest=True):
            with self.assertRaisesRegex(ReverseKey1Error, 'manifest.*changed'):
                public.pretrain_public_synthetic_style(self.public_config())
        self.assert_no_checkpoint()

    def test_failed_checkpoint_staging_preserves_the_last_complete_checkpoint(self):
        self.output.mkdir()
        target = self.output / 'best.pt'
        target.write_bytes(b'complete previous epoch')

        def partial_save(checkpoint, raw):
            raw.write(b'partial')
            raise OSError('synthetic interrupted save')

        with patch.object(torch, 'save', side_effect=partial_save):
            with self.assertRaisesRegex(OSError, 'interrupted save'):
                training._atomic_checkpoint(torch, target, {'synthetic': True})
        self.assertEqual(target.read_bytes(), b'complete previous epoch')
        self.assertEqual(list(self.output.glob('.checkpoint-*')), [])

    def test_output_symlink_never_redirects_a_run_into_existing_evidence(self):
        target = self.root / 'other'
        target.mkdir()
        self.output.symlink_to(target, target_is_directory=True)
        with self.native_run(), self.assertRaises(FileExistsError):
            training.train_universal_model(self.native_config())
        self.assertEqual(list(target.iterdir()), [])


if __name__ == '__main__':
    unittest.main()
