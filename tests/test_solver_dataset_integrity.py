"""Synthetic dataset tests; none of these labels are native capture evidence."""
from __future__ import annotations
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import numpy as np
from research.oppo_styles import training
from xdremux_py.apple_reverse_key1_training import SAMPLE_SCHEMA, ReverseKey1Error, canonical_json_bytes
from tests import test_universal_photographic_style_training as model_fixtures


def digest(path:Path)->str:return hashlib.sha256(path.read_bytes()).hexdigest()

def write_manifest(root:Path)->Path:
    records=[]
    for i,split in enumerate(('train','calibration','heldout')):
        source=hashlib.sha256(f'synthetic source {i}'.encode()).hexdigest()
        sample=root/f'sample-{i}.npz'
        np.savez(sample,schema=np.array([SAMPLE_SCHEMA]),source_sha256=np.array([source]),
                 images=np.zeros((2,3,256,256),np.uint8),key1=np.zeros((12,12,8,10,3),np.float32),
                 mask=np.ones((12,12),np.bool_))
        records.append({'index':i,'sourceSHA256':source,'samplePath':str(sample),
                        'sampleSHA256':digest(sample),'captureSession':f'synthetic-{i}','split':split})
    labels=root/'labels.npz'
    np.savez(labels,gtc=np.zeros((3,516),np.uint8),light_maps=np.zeros((3,2,32,32),np.float16),
             scalars=np.zeros((3,6),np.float32),scalar_mask=np.zeros((3,6),np.bool_),
             metadata=np.zeros((3,16),np.float32),metadata_mask=np.zeros((3,16),np.bool_))
    header={'schema':training.DATASET_SCHEMA,'recordsSHA256':hashlib.sha256(canonical_json_bytes(records)).hexdigest(),
            'sampleCount':3,'labelsSHA256':digest(labels)}
    path=root/'manifest.json';path.write_text(json.dumps({'header':header,'samples':records}))
    return path


class SolverDatasetIntegrityTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.root=Path(self.temp.name)
        self.path=write_manifest(self.root)
    def tearDown(self):self.temp.cleanup()

    def mutate(self, change):
        value=json.loads(self.path.read_text());change(value)
        value['header']['recordsSHA256']=hashlib.sha256(canonical_json_bytes(value['samples'])).hexdigest()
        self.path.write_text(json.dumps(value))

    def test_v2_pins_labels_and_each_cached_sample(self):
        header,records=training.load_universal_manifest(self.path)
        self.assertEqual(header['sampleCount'],3)
        self.assertEqual({r['split'] for r in records},{'train','calibration','heldout'})
        (self.root/'labels.npz').write_bytes(b'changed labels')
        with self.assertRaisesRegex(ReverseKey1Error,'identity'):training.load_universal_manifest(self.path)

    def test_cached_pixels_cannot_change_under_a_reused_source_hash(self):
        sample=self.root/'sample-0.npz'
        with np.load(sample) as archive:arrays={k:archive[k] for k in archive.files}
        arrays['images'][0,0,0,0]=255;np.savez(sample,**arrays)
        with self.assertRaisesRegex(ReverseKey1Error,'identity'):training.load_universal_manifest(self.path)

    def test_rehashed_manifest_still_rejects_wrong_index_split_and_session_leakage(self):
        original=self.path.read_bytes()
        changes=[lambda v:v['samples'][0].update(index=1),lambda v:v['samples'][0].update(split='validation'),
                 lambda v:v['samples'][1].update(captureSession='synthetic-0'),
                 lambda v:v['samples'][0].update(index=False),
                 lambda v:v['samples'][1].update(sourceSHA256=v['samples'][0]['sourceSHA256'])]
        for change in changes:
            with self.subTest(change=change):
                self.path.write_bytes(original);self.mutate(change)
                with self.assertRaises(ReverseKey1Error):training.load_universal_manifest(self.path)

    def test_label_shape_and_missingness_are_not_asserted_only_by_hash(self):
        labels=self.root/'labels.npz'
        with np.load(labels) as archive:arrays={k:archive[k] for k in archive.files}
        arrays['scalar_mask']=np.full((3,6),2,np.int8);np.savez(labels,**arrays)
        self.mutate(lambda v:v['header'].update(labelsSHA256=digest(labels)))
        with self.assertRaisesRegex(ReverseKey1Error,'binary'):training.load_universal_manifest(self.path)

    def test_integer_dark_rgb_is_not_mistaken_for_normalized_float(self):
        result=training.primary_image_features(np.ones((3,256,256),np.uint8))
        np.testing.assert_array_equal(result[:3],np.full((3,256,256),1/255,np.float32))
        with self.assertRaises(ReverseKey1Error):training.primary_image_features(np.full((3,256,256),1.5,np.float32))

    def test_missing_scalar_labels_do_not_optimize_an_unobserved_task(self):
        import torch
        _,records=training.load_universal_manifest(self.path)
        data=training._UniversalDataset(self.path,records[:1])
        batch=next(iter(torch.utils.data.DataLoader(data,batch_size=1)))
        model=training.build_universal_model(model_fixtures.UniversalPhotographicStyleTrainingTests._statistics()).eval()
        output=training._model_forward(model,batch)
        total,details=training._losses(torch,model,output,batch)
        self.assertTrue(torch.isfinite(total));self.assertEqual(details['scalars'],0)
        total.backward()
        self.assertEqual(float(model.task_log_variances.grad[3]),0)
        with torch.no_grad():model.task_log_variances[3]=3
        after,_=training._losses(torch,model,output,batch)
        self.assertEqual(float(total.detach()),float(after.detach()))

    def test_rmse_is_pixel_squared_error_and_missing_metrics_remain_null(self):
        import torch
        _,records=training.load_universal_manifest(self.path)
        batch=next(iter(torch.utils.data.DataLoader(training._UniversalDataset(self.path,records[:1]),batch_size=1)))
        model=training.build_universal_model(model_fixtures.UniversalPhotographicStyleTrainingTests._statistics()).eval()
        output=training._model_forward(model,batch)
        proxy=torch.zeros((1,3,64,64));proxy[:,:,:,32:]=1
        with patch.object(training,'_model_forward',return_value=output),patch.object(training,'_consumer_quadratic_proxy',return_value=proxy):
            report=training._evaluate(torch,model,[batch],'cpu')
        self.assertAlmostEqual(report['consumerProxyMAE'],.5)
        self.assertAlmostEqual(report['consumerProxyRMSE8'],255*np.sqrt(.5))
        self.assertIsNone(report['scalarsNormalizedMAE']);self.assertEqual(report['scalarObservations'],0)
        self.assertIsNone(report['uncertaintyErrorCorrelation'])
        json.dumps(report,allow_nan=False)

    def test_warm_start_rejects_partial_learned_state(self):
        model=training.build_universal_model(model_fixtures.UniversalPhotographicStyleTrainingTests._statistics())
        state=model.state_dict();del state[next(k for k in state if k not in training._STATISTIC_STATE_KEYS)]
        with self.assertRaisesRegex(ReverseKey1Error,'missing'):
            training._warm_start_model_state(model,state,source_architecture='base',target_architecture='base')

if __name__=='__main__':unittest.main()
