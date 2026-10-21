from pathlib import Path
import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest

ROOT=Path('/workspace/XDRemux-integration')
FILE=Path('/workspace/xdremux-audit/e107-stage-attribution-prototype.py')
spec=importlib.util.spec_from_file_location('stage_prototype',FILE)
p=importlib.util.module_from_spec(spec);spec.loader.exec_module(p)
BASE=subprocess.check_output(['git','-C',str(ROOT),'show',p.BASELINE+':'+str(p.OWNER)],text=True)
CAND=(ROOT/p.OWNER).read_text()

class PrototypeTests(unittest.TestCase):
    def test_real_fixed_baseline_only_adds_timers_at_all_thirteen_boundaries(self):
        modified,proof=p.insert_timers(BASE,CAND)
        self.assertTrue(proof['removed_insertions_restore_exact_original'])
        self.assertEqual(modified.count('    profile_stage(&mut profile,'),13)
        self.assertEqual(tuple(stage for stage,_,_ in p.ANCHORS),p.COMMON)
        self.assertNotIn('scene_hdr_reconstruction',modified)

    def test_wrong_or_already_instrumented_source_fails_before_a_build(self):
        modified,_=p.insert_timers(BASE,CAND)
        for broken in (modified,BASE.replace('let scene_raster = downsample_raster','let scene_raster = unknown'),BASE+BASE):
            with self.subTest(case=broken[-20:]),self.assertRaises(ValueError):p.insert_timers(broken,CAND)

    def test_native_receipts_require_each_arms_real_stage_sequence(self):
        def receipt(labels):return ''.join(f'xdremux_styles_profile stage={s} seconds=0.125\n' for s in labels)
        baseline=receipt(p.COMMON)
        candidate=receipt(p.COMMON[:4]+p.DETAIL+p.COMMON[4:])
        self.assertEqual(len(p.spans(baseline,'baseline')),13)
        self.assertEqual(len(p.spans(candidate,'candidate')),15)
        for text,arm in [(baseline,'candidate'),(candidate,'baseline'),(baseline+baseline,'baseline'),('', 'baseline'),(baseline.replace('0.125','nan',1),'baseline')]:
            with self.subTest(arm=arm),self.assertRaises(ValueError):p.spans(text,arm)

    def test_common_summary_excludes_nested_details_and_requires_all_readbacks(self):
        rows=[]
        for i in range(5):
            for arm in ('baseline','candidate'):
                rows.append(dict(arm=arm,warmup=False,status='validated',stages=[dict(stage=s,wall_seconds=1 if arm=='baseline' else 2) for s in p.COMMON]+[dict(stage=s,wall_seconds=999) for s in p.DETAIL]))
        result=p.summary(rows)
        self.assertEqual(tuple(result),p.COMMON)
        self.assertTrue(all(r['candidate_minus_baseline_seconds']==1 for r in result.values()))
        rows[-1]['status']='converted'
        with self.assertRaisesRegex(ValueError,'independently validated'):p.summary(rows)
        with self.assertRaises(ValueError):p.summary(rows[:-2])

    def test_real_linux_entry_blocks_before_loading_or_building_native_owners(self):
        with tempfile.TemporaryDirectory() as raw:
            out=Path(raw)/'output'
            r=subprocess.run([sys.executable,str(FILE),'--source',str(ROOT),'--cli','/missing-cli','--apple-adapter','/missing-adapter','--cli-sha256','0'*64,'--adapter-sha256','0'*64,'--output-dir',str(out)],capture_output=True,text=True)
            self.assertEqual(r.returncode,2)
            report=json.loads((out/'report.json').read_text())
            self.assertEqual(report['status'],'blocked')
            self.assertIn('zero builds or native conversions',report['error'])
            self.assertEqual(report['samples'],[])

if __name__=='__main__':unittest.main()
