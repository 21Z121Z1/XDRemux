"""Bounded audit-only kernel probe. Local pixel buffers must not be uploaded."""
from pathlib import Path
import hashlib,json,os,platform,re,resource,statistics,subprocess,time
ROOT=Path(__file__).resolve().parent
REPORT=ROOT/'results.json'

def sha(path):
    h=hashlib.sha256()
    with path.open('rb') as f:
        for chunk in iter(lambda:f.read(1024*1024),b''):h.update(chunk)
    return h.hexdigest()

def byte_equal(left,right):
    if left.stat().st_size!=right.stat().st_size:return False
    with left.open('rb') as a,right.open('rb') as b:
        while True:
            x=a.read(1024*1024);y=b.read(1024*1024)
            if x!=y:return False
            if not x:return True

def command(args,cwd=None):
    result=subprocess.run(args,cwd=cwd,text=True,capture_output=True,check=True)
    return result.stdout.strip()

def main():
    fixture=Path('/workspace/XDRemux-integration/fixtures/proxdr/oppo/find-x9-ultra/uhdr-portrait-01.heic')
    bins={arm:ROOT/f'{arm}-target/release/hdr-validation-probe' for arm in ('baseline','candidate')}
    outputs={arm:ROOT/f'{arm}-public-output.bin' for arm in bins}
    env=os.environ.copy()
    receipts={'scope':'Linux x86_64 pure Rust HDR kernel. Source primary HEIF and private JPEG gain codes; no Apple native or full product claim.',
      'input':{'sha256':sha(ROOT/'public-request.bin'),'bytes':(ROOT/'public-request.bin').stat().st_size,'fixture_sha256':sha(fixture)},
      'host':{'platform':platform.platform(),'cpu':command(['sh','-c','lscpu | head -25']),'load_before':os.getloadavg()},
      'environment':{k:env.get(k) for k in ('RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','CARGO_PROFILE_RELEASE_LTO','CARGO_PROFILE_RELEASE_OPT_LEVEL','LD_PRELOAD')},
      'toolchain':command(['/tmp/xdremux-toolchain/cargo/bin/rustc','-vV']),
      'binaries':{arm:{'path':str(path),'sha256':sha(path),'bytes':path.stat().st_size} for arm,path in bins.items()},
      'samples':[],'complete':False,
      'matrix':{'full_bytes_equal':byte_equal(ROOT/'baseline-matrix.bin',ROOT/'candidate-matrix.bin'),'bytes':(ROOT/'baseline-matrix.bin').stat().st_size,'sha256':sha(ROOT/'baseline-matrix.bin'),'counts':(ROOT/'baseline-matrix.log').read_text().strip()},
      'privacy':'Input/output pixel buffers are local-only. Do not upload public-request.bin, *-public-output.bin, binaries, or rendered media.'}
    def save():REPORT.write_text(json.dumps(receipts,indent=2)+'\n')
    save()
    try:
        for index in range(6):
            order=('baseline','candidate') if index%2==0 else ('candidate','baseline')
            for arm in order:
                args=[str(bins[arm]),'real',str(ROOT/'public-request.bin'),str(outputs[arm])]
                before=resource.getrusage(resource.RUSAGE_CHILDREN);start=time.monotonic()
                proc=subprocess.run(args,text=True,capture_output=True)
                elapsed=time.monotonic()-start;after=resource.getrusage(resource.RUSAGE_CHILDREN)
                log_prefix=ROOT/f'{index:02d}-{arm}'
                log_prefix.with_suffix('.stdout').write_text(proc.stdout);log_prefix.with_suffix('.stderr').write_text(proc.stderr)
                row={'index':index,'warmup':index==0,'arm':arm,'command':args,'returncode':proc.returncode,'process_wall_seconds':elapsed,
                     'process_user_seconds':after.ru_utime-before.ru_utime,'process_system_seconds':after.ru_stime-before.ru_stime}
                receipts['samples'].append(row);save()
                if proc.returncode:raise RuntimeError(f'{arm} sample {index} failed; see owned stdout/stderr')
                match=re.fullmatch(r'kernel_wall_seconds=([0-9.]+) width=3064 height=4080 samples=37503360 gain_width=1532 gain_height=2040 gain_channels=3\n',proc.stdout)
                if not match:raise RuntimeError(f'unexpected measurement receipt: {proc.stdout!r}')
                row['kernel_wall_seconds']=float(match[1]);row['output_bytes']=outputs[arm].stat().st_size;row['output_sha256']=sha(outputs[arm])
                other=outputs['candidate' if arm=='baseline' else 'baseline']
                if other.exists():
                    row['full_bytes_equal_other_arm']=byte_equal(outputs[arm],other)
                    if not row['full_bytes_equal_other_arm']:raise RuntimeError('full public fixture output bytes differ')
                save();print(json.dumps(row),flush=True)
        receipts['summary']={}
        for arm in bins:
            rows=[s for s in receipts['samples'] if s['arm']==arm and not s['warmup']]
            receipts['summary'][arm]={'kernel_median_seconds':statistics.median(s['kernel_wall_seconds'] for s in rows),
              'kernel_min_seconds':min(s['kernel_wall_seconds'] for s in rows),'kernel_max_seconds':max(s['kernel_wall_seconds'] for s in rows),
              'process_user_median_seconds':statistics.median(s['process_user_seconds'] for s in rows),
              'process_system_median_seconds':statistics.median(s['process_system_seconds'] for s in rows)}
        receipts['summary']['candidate_baseline_kernel_ratio']=receipts['summary']['candidate']['kernel_median_seconds']/receipts['summary']['baseline']['kernel_median_seconds']
        receipts['input']['unchanged_after']=receipts['input']['sha256']==sha(ROOT/'public-request.bin') and receipts['input']['fixture_sha256']==sha(fixture)
        if not receipts['input']['unchanged_after']:raise RuntimeError('input changed during measurement')
        receipts['host']['load_after']=os.getloadavg();receipts['complete']=True;save()
        print(json.dumps(receipts['summary'],indent=2),flush=True)
    except BaseException as exc:
        receipts['failure']=f'{type(exc).__name__}: {exc}';save();raise
if __name__=='__main__':main()
