from pathlib import Path
import hashlib,json,os,sys
ROOT=Path('/workspace/XDRemux-hdr-validated-prototype');OUT=Path('/workspace/xdremux-audit/hdr-kernel-tests')
D=Path('/workspace/XDRemux-review-evidence/docs/validation/reviews/20261020-semantic-integration/iterations/e1072a9-clone-object-independence')
sys.dont_write_bytecode=True;sys.path.insert(0,str(D))
from bounded_stage_lifecycle import Commands
out=OUT/'final';out.mkdir(exist_ok=False)
report={'expected_head':'baa763e4cea14ee79953912b50c90d877ab9d352','expected_tree':'c0f0b24f3cd77336049da50a499a0867f46153fa','scope':'Real HDR crate source through equivalent dependency-free manifest; debug and release Rust tests, not workspace/native/performance acceptance','status':'running'}
c=Commands(out,report)
try:
 assert c.git(ROOT,'rev-parse','HEAD')==report['expected_head']
 assert c.git(ROOT,'rev-parse','HEAD^{tree}')==report['expected_tree']
 assert not c.git(ROOT,'status','--porcelain=v1','--untracked-files=all')
 files=c.git(ROOT,'diff-tree','--no-commit-id','--name-only','-r','HEAD').splitlines()
 report['changed_files']=[{'path':f,'sha256':hashlib.sha256((ROOT/f).read_bytes()).hexdigest()} for f in files]
 env={**os.environ,'PATH':'/workspace/xdremux-audit/hdr-kernel-toolchain/install/bin:'+os.environ['PATH'],'CARGO_HOME':str(OUT/'cargo-home'),'CARGO_TARGET_DIR':str(OUT/'isolated-target'),'CARGO_INCREMENTAL':'0'}
 report['environment']={k:env.get(k) for k in ('PATH','CARGO_HOME','CARGO_TARGET_DIR','CARGO_INCREMENTAL','RUSTC','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS')}
 c.capture(['/workspace/xdremux-audit/hdr-kernel-toolchain/install/bin/rustc','--version','--verbose'],ROOT)
 c.capture(['/workspace/xdremux-audit/hdr-kernel-toolchain/install/bin/cargo','--version','--verbose'],ROOT)
 report['profiles']={}
 for profile,extra in [('debug',[]),('release',['--release'])]:
  argv=['cargo','test','--locked','--offline','--manifest-path',str(OUT/'isolated/Cargo.toml'),*extra]
  with (out/(profile+'.log')).open('w') as stream:code=c.process(argv,ROOT,env,stream,-2,120)
  text=(out/(profile+'.log')).read_text()
  assert code==0 and '17 passed; 0 failed' in text and '6 passed; 0 failed' in text
  report['profiles'][profile]={'exit_code':code,'unit_tests':17,'differential_integration_tests':6,'doc_tests':0,'log_sha256':hashlib.sha256(text.encode()).hexdigest()}
 c.capture(['/workspace/xdremux-audit/hdr-kernel-toolchain/install/bin/rustfmt','--check','--edition','2021','crates/xdremux-hdr/src/scene_linear.rs','crates/xdremux-hdr/tests/scene_linear_equivalence.rs'],ROOT)
 c.git(ROOT,'diff','--check');assert not c.git(ROOT,'status','--porcelain=v1','--untracked-files=all')
 facts=json.loads((ROOT/'crates/xdremux-hdr/tests/oracles/provenance.json').read_text())
 raw=c.capture(['git','show',facts['source_commit']+':'+facts['source_path']],ROOT).encode()
 frozen=(ROOT/'crates/xdremux-hdr/tests/oracles'/facts['oracle_file']).read_bytes()
 assert hashlib.sha256(raw).hexdigest()==facts['source_sha256']
 assert raw[:facts['prefix_bytes']]==frozen and hashlib.sha256(frozen).hexdigest()==facts['oracle_sha256']
 report.update(status='passed',frozen_oracle_exact=True,scalar_matrix_cases=1600,image_matrix_cases=720,native_runs=0,performance_claim=False)
except BaseException as error:
 report.update(status='failed-or-uncertain',error_type=type(error).__name__,error=str(error));raise
finally:c.checkpoint()
print(json.dumps({k:v for k,v in report.items() if k not in ('commands','environment','changed_files')},indent=2))
