from __future__ import annotations
import dataclasses
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile
from research.oppo_styles import inference
from Tests import test_solver_evidence_boundaries as cases
from xdremux_py.apple_reverse_key1_training import ReverseKey1Error


class SolverCandidateArchiveTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.root=Path(self.temp.name)
        self.source=self.root/'source.png';self.source.write_bytes(b'synthetic input identity only')
        self.image=dataclasses.replace(cases.SolverEvidenceBoundaryTests.image(),path=self.source,
            source_sha256=hashlib.sha256(self.source.read_bytes()).hexdigest())
        self.resources=inference.candidate_state_resources(self.image,cases.SolverEvidenceBoundaryTests.prediction())
        self.output=self.root/'candidate.zip'
    def tearDown(self):self.temp.cleanup()
    def publish(self):
        return inference.publish_candidate_archive(self.output,self.image,self.resources,
            {'_checkpointSHA256':'1'*64,'architecture':'synthetic'},device='cpu',model_seconds=0)

    def test_complete_archive_has_bound_payloads_without_native_claims(self):
        before=self.source.read_bytes();receipt=self.publish()
        with zipfile.ZipFile(self.output) as archive:
            self.assertEqual(set(archive.namelist()),{'key1.bin','key3-gtc.bin','c.bin','d.bin','report.json'})
            self.assertEqual(json.loads(archive.read('report.json')),receipt)
            for record in receipt['resources'].values():
                data=archive.read(record['path']);self.assertEqual(len(data),record['bytes'])
                self.assertEqual(hashlib.sha256(data).hexdigest(),record['sha256'])
        self.assertFalse(receipt['nativeParseValidated']);self.assertFalse(receipt['photosEditingValidated'])
        self.assertEqual(self.source.read_bytes(),before)
        old=self.output.read_bytes()
        with self.assertRaises(FileExistsError):self.publish()
        self.assertEqual(self.output.read_bytes(),old)

    def test_staging_failure_does_not_publish_partial_output(self):
        with patch.object(zipfile.ZipFile,'writestr',side_effect=OSError('synthetic write failure')):
            with self.assertRaises(OSError):self.publish()
        self.assertFalse(self.output.exists());self.assertEqual(list(self.root.glob('.style-candidate-*')),[])

    def test_racing_destination_and_changed_source_are_preserved(self):
        def competing_writer(source,destination):
            Path(destination).write_bytes(b'concurrent owner')
            raise FileExistsError(destination)
        with patch.object(inference.os,'link',side_effect=competing_writer):
            with self.assertRaises(FileExistsError):self.publish()
        self.assertEqual(self.output.read_bytes(),b'concurrent owner')
        self.assertEqual(list(self.root.glob('.style-candidate-*')),[])
        self.output.unlink();self.source.write_bytes(b'changed input')
        with self.assertRaises(ReverseKey1Error):self.publish()
        self.assertFalse(self.output.exists())

if __name__=='__main__':unittest.main()
