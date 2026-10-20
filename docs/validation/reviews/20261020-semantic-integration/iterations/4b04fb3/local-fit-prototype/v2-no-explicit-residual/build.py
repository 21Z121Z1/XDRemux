from pathlib import Path
import hashlib
import json
import os
import platform
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parent
TOOLCHAIN = Path('/tmp/xdremux-toolchain/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin')
TARGET = Path('/tmp/xdremux-style-fit-accumulation-v2-target')

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

env = {**os.environ, 'PATH': str(TOOLCHAIN)+os.pathsep+os.environ['PATH'],
       'RUSTUP_HOME': '/tmp/xdremux-toolchain/rustup', 'CARGO_HOME': '/tmp/xdremux-toolchain/cargo',
       'PKG_CONFIG_PATH': '/tmp/xdremux-toolchain/heif-dev/usr/lib/x86_64-linux-gnu/pkgconfig'}
receipt = {'host': platform.uname()._asdict(), 'commands': [], 'arms': {},
           'environment': {key: env.get(key) for key in ('RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','CARGO_BUILD_TARGET','PKG_CONFIG_PATH')},
           'limits': 'Linux public-primary kernel with synthetic recovery target; no native/E2E claim.'}
for command in ([str(TOOLCHAIN/'rustc'),'--version','--verbose'],[str(TOOLCHAIN/'cargo'),'--version'],
                ['pkg-config','--modversion','libheif'],['lscpu']):
    receipt['commands'].append({'command':command,'stdout':subprocess.check_output(command,env=env,text=True)})
repo_packages = tomllib.loads((ROOT/'baseline-source/Cargo.lock').read_text())['package']
repo_ids = {(p['name'],p['version'],p.get('source'),p.get('checksum')) for p in repo_packages}
for arm in ('baseline','candidate'):
    source = ROOT/(arm+'-source')
    probe = ROOT/(arm+'-probe')
    target = TARGET/arm
    command = [str(TOOLCHAIN/'cargo'),'build','--locked','--offline','--release','--manifest-path',str(probe/'Cargo.toml')]
    print('building',arm,flush=True)
    with (ROOT/(arm+'-build.log')).open('w') as log:
        result = subprocess.run(command,cwd=probe,env={**env,'CARGO_TARGET_DIR':str(target)},stdout=log,stderr=subprocess.STDOUT)
    result.check_returncode()
    binary = target/'release/xdremux-style-fit-probe'
    packages = tomllib.loads((probe/'Cargo.lock').read_text())['package']
    ids = {(p['name'],p['version'],p.get('source'),p.get('checksum')) for p in packages if p['name']!='xdremux-style-fit-probe'}
    assert ids <= repo_ids
    receipt['arms'][arm] = {'command':command,'target':str(target),'binary':str(binary),'binary_sha256':digest(binary),
        'manifest_sha256':digest(probe/'Cargo.toml'),'lock_sha256':digest(probe/'Cargo.lock'),
        'probe_source_sha256':digest(probe/'src/main.rs'),'repo_lock_sha256':digest(source/'Cargo.lock'),
        'engine_source_sha256':digest(source/'crates/xdremux-engine/src/apple_photographic_styles.rs'),
        'all_dependency_identities_in_repository_lock':True,
        'head':subprocess.check_output(['git','-C',str(source),'rev-parse','HEAD'],text=True).strip(),
        'status':subprocess.check_output(['git','-C',str(source),'status','--porcelain=v1'],text=True)}
    print('built',arm,receipt['arms'][arm]['binary_sha256'],flush=True)
    (ROOT/'build-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
assert receipt['arms']['baseline']['probe_source_sha256']==receipt['arms']['candidate']['probe_source_sha256']
assert receipt['arms']['baseline']['lock_sha256']==receipt['arms']['candidate']['lock_sha256']
patch = subprocess.check_output(['git','-C',str(ROOT/'candidate-source'),'diff','--','crates/xdremux-engine/src/apple_photographic_styles.rs'])
(ROOT/'candidate.patch').write_bytes(patch)
receipt['candidate_patch_sha256'] = digest(ROOT/'candidate.patch')
(ROOT/'build-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
