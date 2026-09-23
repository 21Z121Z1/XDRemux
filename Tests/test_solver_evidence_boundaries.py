"""Observable regression contracts for research inputs and finite candidate bytes."""
from __future__ import annotations
import dataclasses
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import numpy as np
from research.oppo_styles import inference, training, public_pretraining
from xdremux_py.apple_reverse_key1_training import ReverseKey1Error, canonical_json_bytes


class SolverEvidenceBoundaryTests(unittest.TestCase):
    @staticmethod
    def image() -> inference.UniversalImageInput:
        return inference.UniversalImageInput(
            path=Path('synthetic.jpg'), primary=np.zeros((9,256,256),np.float32),
            metadata=np.zeros(16,np.float32), metadata_mask=np.zeros(16,np.float32),
            display_width=4032, display_height=3024, has_raw=False, has_gain_map=False,
            source_sha256='0'*64)

    @staticmethod
    def prediction() -> dict[str,np.ndarray]:
        return {'key1':np.zeros((12,12,8,10,3),np.float32),
                'key1LogVariance':np.zeros((8,10,3),np.float32),
                'gtc':np.zeros(516,np.float32), 'lightMaps':np.zeros((2,32,32),np.float32),
                'scalars':np.zeros(6,np.float32)}

    def test_container_hints_are_not_decoded_optional_modalities(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp)/'preview.dng'
            path.write_bytes(b'synthetic preview carrier hdrgm')
            with patch.object(inference,'_image_and_size',return_value=(np.zeros((3,256,256),np.uint8),4032,3024,False)), patch.object(inference,'_metadata_tags',return_value={}):
                image=inference.load_universal_image(path)
            self.assertFalse(image.has_raw)
            self.assertFalse(image.has_gain_map)
            self.assertEqual(image.metadata[training.METADATA_FIELDS.index('has_raw')],0)
            self.assertEqual(image.metadata[training.METADATA_FIELDS.index('has_gain_map')],0)
            self.assertIsNone(image.linear_rgb_features)
            self.assertIsNone(image.gain_map_features)

    def test_decoded_sidecars_set_the_corresponding_modality(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);path=root/'image.png';path.write_bytes(b'synthetic')
            linear=root/'linear.npy'; gain=root/'gain.npy'
            np.save(linear,np.full((3,256,256),0.5,np.float32))
            np.save(gain,np.ones((256,256),np.float32))
            with patch.object(inference,'_image_and_size',return_value=(np.zeros((3,256,256),np.uint8),4032,3024,False)), patch.object(inference,'_metadata_tags',return_value={}):
                image=inference.load_universal_image(path,linear_rgb_sidecar=linear,gain_map_sidecar=gain)
            self.assertTrue(image.has_raw);self.assertTrue(image.has_gain_map)
            self.assertEqual(image.metadata[training.METADATA_FIELDS.index('has_raw')],1)
            self.assertEqual(image.metadata[training.METADATA_FIELDS.index('has_gain_map')],1)

    def test_finite_float32_cannot_overflow_candidate_float16(self) -> None:
        values=self.prediction(); values['lightMaps'][0,0,0]=1e20
        with self.assertRaises(ReverseKey1Error):
            inference.candidate_state_resources(self.image(),values)

    def test_uncertainty_requires_full_shape_and_finite_payload(self) -> None:
        for invalid in (np.zeros(1,np.float32),np.full((8,10,3),np.nan,np.float32),np.full((8,10,3),1000,np.float32)):
            with self.subTest(shape=invalid.shape):
                values=self.prediction();values['key1LogVariance']=invalid
                with self.assertRaises(ReverseKey1Error):inference.candidate_state_resources(self.image(),values)

    def test_resource_geometry_must_be_observed_positive_integers(self) -> None:
        for value in (0,-1,False,4.5):
            with self.subTest(width=value), self.assertRaises(ReverseKey1Error):
                inference.candidate_state_resources(dataclasses.replace(self.image(),display_width=value),self.prediction())

    def test_primary_and_synthetic_inputs_reject_nonfinite_or_ambiguous_range(self) -> None:
        for value in (np.nan,np.inf,-1.0,256.0):
            image=np.full((3,256,256),value,np.float32)
            for builder in (training.primary_image_features,lambda x:public_pretraining.synthetic_affine_pair(x,np.random.default_rng(3))):
                with self.subTest(value=value,builder=builder),self.assertRaises(ReverseKey1Error):builder(image)

    def test_boolean_exif_measurement_is_missing_not_one(self) -> None:
        values,mask=training.metadata_vector({'displayWidth':True,'displayHeight':3024},{'ISO':True})
        self.assertEqual(mask[training.METADATA_FIELDS.index('log2_display_width')],0)
        self.assertEqual(mask[training.METADATA_FIELDS.index('log2_iso')],0)

    def test_optional_identity_rejects_non_hex_aliases(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);np.save(root/'linear.npy',np.zeros((3,256,256),np.float32))
            for sha in ('+'+'a'*63,' '+'a'*63,'A'*64):
                value={'schema':training.OPTIONAL_MODALITIES_SCHEMA,'samples':[{'sourceSHA256':sha,'linearRGBPath':'linear.npy'}]}
                path=root/'manifest.json';path.write_text(json.dumps(value))
                with self.subTest(sha=sha),self.assertRaises(ReverseKey1Error):training._load_optional_modalities_manifest(path)

    def test_source_change_during_read_cannot_gain_final_hash_provenance(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp)/'source.png';path.write_bytes(b'initial source')
            def change(*args):path.write_bytes(b'different source');return {}
            with patch.object(inference,'_image_and_size',return_value=(np.zeros((3,256,256),np.uint8),4032,3024,False)), patch.object(inference,'_metadata_tags',side_effect=change):
                with self.assertRaises(ReverseKey1Error):inference.load_universal_image(path)

if __name__=='__main__':unittest.main()
