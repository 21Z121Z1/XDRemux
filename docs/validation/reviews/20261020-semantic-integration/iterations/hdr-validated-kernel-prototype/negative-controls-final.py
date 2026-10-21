from pathlib import Path
import difflib,hashlib,json,os,shutil,sys
ROOT=Path('/workspace/XDRemux-hdr-validated-prototype')
OUT=Path('/workspace/xdremux-audit/hdr-kernel-tests')
D=Path('/workspace/XDRemux-review-evidence/docs/validation/reviews/20261020-semantic-integration/iterations/e1072a9-clone-object-independence')
sys.dont_write_bytecode=True;sys.path.insert(0,str(D))
from bounded_stage_lifecycle import Commands
original=(ROOT/'crates/xdremux-hdr/src/scene_linear.rs').read_text()
mutations=[
 ('gamma', 'code.powf(1.0 / channel.gamma)', 'code.powf(channel.gamma)', 'scalar_boundary_values_match_e107_bits_or_errors'),
 ('public-order', '    validate_gain_sample(sdr_linear, gain_code)?;\n    validate_channel(channel)?;', '    validate_channel(channel)?;\n    validate_gain_sample(sdr_linear, gain_code)?;', 'public_scalar_preserves_validation_order_and_exact_errors')]
for name,old,new,test in mutations:
 assert original.count(old)==1
 out=OUT/('negative-control-final-'+name);out.mkdir(exist_ok=False)
 shutil.copytree(ROOT/'crates/xdremux-hdr/src',out/'src')
 changed=original.replace(old,new,1);(out/'src/scene_linear.rs').write_text(changed)
 (out/'mutation.patch').write_text(''.join(difflib.unified_diff(original.splitlines(True),changed.splitlines(True),fromfile='candidate/scene_linear.rs',tofile='owned-negative-control/scene_linear.rs')))
 (out/'Cargo.toml').write_text('[package]\nname = "xdremux-hdr-negative-control-'+name+'"\nversion = "0.1.0"\nedition = "2021"\n\n[workspace]\n\n[lints.rust]\nunsafe_code = "forbid"\n\n[lib]\nname = "xdremux_hdr"\npath = "src/lib.rs"\n\n[[test]]\nname = "scene_linear_equivalence"\npath = '+json.dumps(str(ROOT/'crates/xdremux-hdr/tests/scene_linear_equivalence.rs'))+'\n')
 report={'status':'starting','purpose':'Intentional wrong kernel or public validation order must be rejected by independent frozen e107 tests','live_source_modified':False,'original_source_sha256':hashlib.sha256(original.encode()).hexdigest(),'mutant_source_sha256':hashlib.sha256(changed.encode()).hexdigest()}
 c=Commands(out,report)
 env={**os.environ,'PATH':'/workspace/xdremux-audit/hdr-kernel-toolchain/install/bin:'+os.environ['PATH'],'CARGO_HOME':str(OUT/'cargo-home'),'CARGO_TARGET_DIR':str(out/'target'),'CARGO_INCREMENTAL':'0'}
 with (out/'cargo-test.log').open('w') as f:code=c.process(['cargo','test','--offline','--manifest-path',str(out/'Cargo.toml'),'--test','scene_linear_equivalence',test,'--','--exact'],ROOT,env,f,-2,120)
 report.update(exit_code=code,status='intentional-mutation-rejected' if code==101 and 'test result: FAILED. 0 passed; 1 failed;' in (out/'cargo-test.log').read_text() else 'unexpected-outcome');c.checkpoint()
 print(json.dumps({k:v for k,v in report.items() if k!='commands'},indent=2));assert report['status']=='intentional-mutation-rejected'
assert (ROOT/'crates/xdremux-hdr/src/scene_linear.rs').read_text()==original
