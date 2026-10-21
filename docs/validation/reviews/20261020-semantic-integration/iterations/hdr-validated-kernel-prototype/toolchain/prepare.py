from pathlib import Path
import concurrent.futures,hashlib,json,os,platform,shutil,subprocess,tarfile,time,tomllib,urllib.request,urllib.error
ROOT=Path('/workspace/xdremux-audit/hdr-kernel-toolchain')
manifest=tomllib.loads((ROOT/'channel-rust-stable.toml').read_text())
assert platform.system()=='Linux' and platform.machine()=='x86_64'
target='x86_64-unknown-linux-gnu';prefix=ROOT/'install';prefix.mkdir(exist_ok=False)
report={'manifest_sha256':hashlib.sha256((ROOT/'channel-rust-stable.toml').read_bytes()).hexdigest(),'target':target,'prefix':str(prefix),'status':'starting','components':[],'installer_scripts_executed':False}
def fetch(name):
 spec=manifest['pkg'][name]['target'][target];assert spec['available']
 url=spec['xz_url'];assert url.startswith('https://static.rust-lang.org/dist/')
 r={'component':name,'version':manifest['pkg'][name]['version'],'url':url,'expected_sha256':spec['xz_hash'],'status':'starting'};start=time.monotonic()
 report['components'].append(r)
 try:
  file=ROOT/(name+'.tar.xz')
  with urllib.request.urlopen(url,timeout=45) as response,file.open('xb') as stream:
   r['http_status']=response.status
   while chunk:=response.read(1024*1024):
    stream.write(chunk)
    if stream.tell()>256*1024*1024:raise ValueError('component archive exceeds bound')
  with file.open('rb') as stream:actual=hashlib.file_digest(stream,'sha256').hexdigest()
  assert actual==spec['xz_hash'],'official manifest hash mismatch'
  r.update(actual_sha256=actual,archive_bytes=file.stat().st_size)
  unpack=ROOT/(name+'-unpacked');unpack.mkdir(exist_ok=False)
  with tarfile.open(file) as archive:archive.extractall(unpack,filter='data')
  folders=list(unpack.iterdir());assert len(folders)==1
  component=folders[0]/(name if name!='rust-std' else 'rust-std-'+target)
  for subdir in ('bin','lib'):
   source=component/subdir
   if source.exists():shutil.copytree(source,prefix/subdir,dirs_exist_ok=True,symlinks=True)
  r['status']='verified-and-locally-materialized'
 except urllib.error.HTTPError as e:
  r.update(status='failed-no-retry',error_type=type(e).__name__,http_status=e.code,reason=str(e.reason));raise
 except urllib.error.URLError as e:
  r.update(status='failed-no-retry',error_type=type(e).__name__,reason_type=type(e.reason).__name__,reason=str(e.reason));raise
 except BaseException as e:
  r.update(status='failed-no-retry',error_type=type(e).__name__,reason=str(e));raise
 finally:r['duration_seconds']=time.monotonic()-start
try:
 # Archive requests are independent; component copies are disjoint Rust paths.
 with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
  list(pool.map(fetch,('rustc','rust-std','cargo')))
 env={**os.environ,'PATH':str(prefix/'bin')+os.pathsep+os.environ['PATH']}
 report['tool_versions']={name:subprocess.check_output([str(prefix/'bin'/name),'--version','--verbose'],env=env,text=True,timeout=10) for name in ('rustc','cargo')}
 report['binary_sha256']={name:hashlib.sha256((prefix/'bin'/name).read_bytes()).hexdigest() for name in ('rustc','cargo')}
 report['status']='ready'
except BaseException as e:
 report.update(status='failed',error_type=type(e).__name__,reason=str(e))
finally:(ROOT/'preparation-receipt.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
if report['status']!='ready':raise SystemExit(2)
