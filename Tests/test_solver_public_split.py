"""Public/synthetic supervision is not native supervision, including after fitting."""
from __future__ import annotations
import hashlib
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import numpy as np
from PIL import Image
from research.oppo_styles import public_pretraining as public
from xdremux_py.apple_reverse_key1_training import ReverseKey1Error


def records(root:Path,count:int=5)->list[dict]:
    rows=[]
    for i in range(count):
        path=root/f'image-{i}.npy';np.save(path,np.full((3,256,256),i*30+30,np.uint8))
        rows.append({'title':f'synthetic-{i}','license':'CC0','category':'synthetic-test-only',
            'imagePath':str(path),'tensorSHA256':hashlib.sha256(path.read_bytes()).hexdigest(),
            'downloadSHA256':hashlib.sha256(f'synthetic source {i}'.encode()).hexdigest(),
            'sourceURL':f'https://test.invalid/synthetic-{i}'})
    return rows


class PublicSplitTests(unittest.TestCase):
    def setUp(self):self.temp=tempfile.TemporaryDirectory();self.root=Path(self.temp.name)
    def tearDown(self):self.temp.cleanup()

    def test_split_is_source_disjoint_order_independent_and_rejects_aliases(self):
        rows=records(self.root)
        split=public.split_public_records(rows,1)
        self.assertEqual(split,public.split_public_records(list(reversed(rows)),1))
        self.assertEqual(sorted(len(v) for v in split.values()),[1,1,3])
        self.assertEqual(len({r['downloadSHA256'] for values in split.values() for r in values}),5)
        for field in ('tensorSHA256','downloadSHA256','sourceURL'):
            bad=[dict(row) for row in rows];bad[1][field]=bad[0][field]
            with self.subTest(field=field),self.assertRaisesRegex(ReverseKey1Error,'duplicate'):
                public.split_public_records(bad,1)

    def test_public_pinned_download_is_verified_before_any_tensor_is_written(self):
        payload=io.BytesIO();Image.new('RGB',(16,16)).save(payload,format='PNG');raw=payload.getvalue()
        class Response(io.BytesIO):
            def geturl(self):return 'https://raw.githubusercontent.com/project/repo/v1/image.png'
        row={'downloadURL':'https://raw.githubusercontent.com/project/repo/v1/image.png',
             'title':'pinned-test','sourceGitBlobSHA1':'0'*40}
        path=self.root/'candidate.npy'
        with patch.object(public.urllib.request,'urlopen',return_value=Response(raw)):
            with self.assertRaisesRegex(ReverseKey1Error,'blob mismatch'):public._download_sample(row,path)
        self.assertFalse(path.exists())
        row['sourceGitBlobSHA1']=hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()
        with patch.object(public.urllib.request,'urlopen',return_value=Response(raw)):
            receipt=public._download_sample(row,path)
        self.assertEqual(receipt['downloadSHA256'],hashlib.sha256(raw).hexdigest())
        original=path.read_bytes()
        with patch.object(public.urllib.request,'urlopen',return_value=Response(raw)):
            with self.assertRaises(FileExistsError):public._download_sample(row,path)
        self.assertEqual(path.read_bytes(),original)

    def test_download_cannot_use_local_file_or_leave_public_host(self):
        for url in ('file:///tmp/x','http://raw.githubusercontent.com/x','https://example.test/x'):
            with self.subTest(url=url),patch.object(public.urllib.request,'urlopen') as opener:
                with self.assertRaises(ReverseKey1Error):public._download_sample({'downloadURL':url},self.root/'x')
                opener.assert_not_called()

    def test_corpus_receipt_is_no_clobber_and_hashes_the_decoded_tensor_bytes(self):
        path=self.root/'report.json';public._write_new_json(path,{'first':True})
        original=path.read_bytes()
        with self.assertRaises(FileExistsError):public._write_new_json(path,{'first':False})
        self.assertEqual(path.read_bytes(),original)
        self.assertEqual(list(self.root.glob('.public-corpus-*')),[])
        row=records(self.root,1)[0]
        image=public._verified_public_tensor(row);self.assertEqual(image.shape,(3,256,256))
        Path(row['imagePath']).write_bytes(b'changed')
        with self.assertRaisesRegex(ReverseKey1Error,'identity'):public._verified_public_tensor(row)

    def test_actual_fitting_uses_only_calibration_before_final_heldout(self):
        import torch
        rows=records(self.root,3);manifest=self.root/'manifest.json'
        manifest.write_text(json.dumps({'schema':public.PUBLIC_CORPUS_SCHEMA,'samples':rows}))
        split=public.split_public_records(rows,17)
        by_level={int(np.load(row['imagePath'])[0,0,0]):row['downloadSHA256'] for row in rows}
        visits=[]
        class ToyModel(torch.nn.Module):
            def __init__(self):super().__init__();self.bias=torch.nn.Parameter(torch.tensor(0.001))
            def forward(self,primary,*unused):
                visits.append((self.training,[by_level[round(float(v)*255)] for v in primary[:,0,0,0]]))
                count=len(primary)
                return {'key1':self.bias.expand(count,12,12,8,10,3),
                        'unstyled':self.bias.expand(count,3,64,64)}
        def identity(image,rng):
            clean=image.astype(np.float32)/255
            return clean,clean,np.zeros((12,12,8,10,3),np.float32)
        with patch.object(public,'build_universal_model',return_value=ToyModel()),patch.object(public,'synthetic_affine_pair',side_effect=identity):
            report=public.pretrain_public_synthetic_style(public.PublicPretrainingConfig(
                manifest=manifest,output=self.root/'fit',epochs=2,transforms_per_image=1,seed=17))
        calibration={r['downloadSHA256'] for r in split['calibration']}
        heldout={r['downloadSHA256'] for r in split['heldout']}
        evaluations=[set(ids) for train,ids in visits if not train]
        self.assertEqual(evaluations[:-1],[calibration]*3) # baseline and two epoch selections
        self.assertEqual(evaluations[-1],heldout)
        self.assertEqual(sum(bool(set(ids) & heldout) for train,ids in visits),1)
        self.assertTrue(all('heldout' not in row for row in report['history']))
        self.assertEqual(report['splitSourceSHA256'],{n:[r['downloadSHA256'] for r in v] for n,v in split.items()})
        json.dumps(report,allow_nan=False)
        checkpoint=torch.load(self.root/'fit/synthetic-pretrained.pt',weights_only=True)
        self.assertTrue(checkpoint['syntheticPretrainingOnly'])

if __name__=='__main__':unittest.main()
