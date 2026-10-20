"""One-shot Linux filesystem/byte-preservation prototype; no Photos acceptance claim."""
import ctypes
import errno
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import tempfile
import threading
from concurrent.futures import ThreadPoolExecutor

root = Path('/workspace/XDRemux-integration')
audit = Path('/workspace/xdremux-audit')
proof = Path(tempfile.mkdtemp(prefix='pvt-packaging-proof-', dir=audit))
work = Path(tempfile.mkdtemp(prefix='pvt-packaging-work-', dir=audit))
cli = work / 'xdremux-snapshot'
shutil.copyfile(root / 'target/debug/xdremux', cli)
cli.chmod(0o700)
fixture = root / 'fixtures/motion-photo/samsung/heif-ultrahdr-01.heic'
commands = []
results = {}
libc = ctypes.CDLL(None, use_errno=True)
rename = libc.renameat2
rename.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
rename.restype = ctypes.c_int

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def pair_hashes(path):
    return [digest(path), digest(path.with_suffix('.mov'))]

def run(name, *args):
    command = [str(cli), *map(str,args)]
    r = subprocess.run(command, cwd=root, capture_output=True, timeout=20)
    (proof / (name + '.stdout')).write_bytes(r.stdout)
    (proof / (name + '.stderr')).write_bytes(r.stderr)
    commands.append({'name':name,'command':command,'exit':r.returncode})
    return r

def valid(name, image):
    r=run(name,'validate',image,'--json')
    assert r.returncode == 0
    report=json.loads(r.stdout)
    assert report['valid'] is True
    return report

def stage(name, pair):
    directory=Path(tempfile.mkdtemp(prefix=name+'-',dir=work))
    for source in [pair,pair.with_suffix('.mov')]:
        shutil.copyfile(source,directory / source.name)
    data=plistlib.dumps({'PFVideoComplementMetadataVersionKey':'1'},fmt=plistlib.FMT_XML)
    with open(directory/'metadata.plist','xb') as f:
        f.write(data)
        f.flush()
        os.fsync(f.fileno())
    for path in [directory/pair.name,directory/pair.with_suffix('.mov').name]:
        with open(path,'rb') as f:os.fsync(f.fileno())
    return directory

def publish(staging, final):
    ctypes.set_errno(0)
    rc=rename(-100,os.fsencode(staging),-100,os.fsencode(final),1)
    return 0 if rc == 0 else ctypes.get_errno()

def boxes(data,start,end):
    out=[]
    while start < end:
        assert end-start >= 8
        size=int.from_bytes(data[start:start+4],'big')
        header=8
        if size == 1:
            size=int.from_bytes(data[start+8:start+16],'big'); header=16
        elif size == 0:size=end-start
        assert size >= header and start+size <= end
        out.append((data[start+4:start+8],start+header,start+size))
        start+=size
    return out

report={'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root).decode().strip(),
        'binary_sha256':digest(cli),'build_binding':'existing local binary snapshot; exact-HEAD binding not established',
        'fixture_sha256_before':digest(fixture),'commands':commands,'results':results,
        'scope':'Linux renameat2(RENAME_NOREPLACE) + real portable CLI; not a PVT product implementation, original user sample, macOS, Photos, Quick Look, AirDrop or wallpaper acceptance'}
try:
    pair=work/'source-pair/capture.heic'
    r=run('convert','convert','--input',fixture,'--output',pair)
    assert r.returncode == 0
    baseline=valid('source-pair-valid',pair)
    before=pair_hashes(pair)
    staging=stage('package',pair)
    final=work/'capture.pvt'
    assert publish(staging,final) == 0
    assert final.is_dir() and not staging.exists()
    assert sorted(p.name for p in final.iterdir()) == ['capture.heic','capture.mov','metadata.plist']
    metadata=plistlib.loads((final/'metadata.plist').read_bytes())
    assert metadata == {'PFVideoComplementMetadataVersionKey':'1'} and type(metadata['PFVideoComplementMetadataVersionKey']) is str
    assert pair_hashes(final/'capture.heic') == before
    packaged=valid('package-member-valid',final/'capture.heic')
    report_result=run('current-cli-package-path','validate',final,'--json')
    assert report_result.returncode != 0
    unwrapped=work/'unwrapped'; unwrapped.mkdir()
    for member in ['capture.heic','capture.mov']:
        shutil.copyfile(final/member,unwrapped/member)
    valid('unwrapped-pair-valid',unwrapped/'capture.heic')
    assert pair_hashes(unwrapped/'capture.heic') == before == pair_hashes(pair)
    movie=(final/'capture.mov').read_bytes()
    moov=next(b for b in boxes(movie,0,len(movie)) if b[0] == b'moov')
    direct_meta=[b for b in boxes(movie,moov[1],moov[2]) if b[0] == b'meta']
    assert any(b'com.apple.quicktime.content.identifier' in movie[b[1]:b[2]] for b in direct_meta)
    results['package_roundtrip']={'directory':True,'plist_version_type':'str','plist_version':'1',
        'all_pair_bytes_identical':True,'source_pair_hashes':before,
        'package_members_validate':True,'unwrapped_pair_validates':True,
        'direct_moov_meta_content_identifier':True,
        'current_cli_accepts_package_directory':False,
        'native_acceptance':'not executed'}

    for kind in ['empty-directory','nonempty-directory','regular-file','dangling-symlink']:
        candidate=work/(kind+'.pvt')
        if kind == 'empty-directory':candidate.mkdir()
        elif kind == 'nonempty-directory':
            candidate.mkdir(); (candidate/'user-bytes').write_bytes(b'KEEP ORIGINAL')
        elif kind == 'regular-file':candidate.write_bytes(b'KEEP ORIGINAL')
        else:candidate.symlink_to(work/'absent-target')
        inode=candidate.lstat().st_ino
        pending=stage(kind,pair)
        error=publish(pending,candidate)
        assert error == errno.EEXIST and pending.is_dir() and candidate.lstat().st_ino == inode
        assert pair_hashes(pending/'capture.heic') == before
        if kind == 'regular-file':assert candidate.read_bytes() == b'KEEP ORIGINAL'
        if kind == 'nonempty-directory':assert (candidate/'user-bytes').read_bytes() == b'KEEP ORIGINAL'
        if kind == 'dangling-symlink':assert candidate.is_symlink() and os.readlink(candidate) == str(work/'absent-target')
        results[kind]={'errno':error,'existing_inode_preserved':True,'staged_pair_preserved':True}

    candidates=[stage('concurrent-a',pair),stage('concurrent-b',pair)]
    inodes=[p.stat().st_ino for p in candidates]
    destination=work/'concurrent.pvt'
    barrier=threading.Barrier(2)
    def worker(item):
        barrier.wait()
        return publish(item,destination)
    with ThreadPoolExecutor(max_workers=2) as pool:codes=list(pool.map(worker,candidates))
    assert sorted(codes) == [0,errno.EEXIST]
    winner=codes.index(0); loser=1-winner
    assert destination.stat().st_ino == inodes[winner] and candidates[loser].is_dir()
    assert pair_hashes(destination/'capture.heic') == before
    valid('concurrent-pair-valid',destination/'capture.heic')
    results['same-output-concurrent']={'errno_by_writer':codes,'winner_inode_preserved':True,
                                       'loser_staging_preserved':True,'complete_winning_pair_valid':True}

    candidates=[stage('different-a',pair),stage('different-b',pair)]
    destinations=[work/'alpha.pvt',work/'beta.pvt']
    with ThreadPoolExecutor(max_workers=2) as pool:
        codes=list(pool.map(lambda x:publish(*x),zip(candidates,destinations)))
    assert codes == [0,0]
    assert all(pair_hashes(p/'capture.heic') == before for p in destinations)
    results['different-output-concurrent']={'errno_by_writer':codes,'both_complete_pair_bytes_preserved':True}
finally:
    report['fixture_sha256_after']=digest(fixture)
    report['binary_sha256_after']=digest(cli)
    report['scratch_files_retained']=False
    shutil.rmtree(work)
    report['scratch_removed']=not work.exists()
    (proof/'manifest.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'proof':str(proof),'results':results},indent=2))
