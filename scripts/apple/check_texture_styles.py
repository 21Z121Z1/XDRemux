#!/usr/bin/env python3
"""Drive the actual Styles switch, native Texture producer, and HDR output."""
from __future__ import annotations
import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))
from scripts.diagnostics.inspect_oppo_heif import boxes, inspect, payload_for_item

URI = b'tag:apple.com,2026:photo:metadata:texture_styles\0'

def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def run(args, log, env=None, success=True):
    with Path(log).open('wb') as stream:
        process = subprocess.run([str(a) for a in args], cwd=ROOT, stdout=stream,
                                 stderr=subprocess.STDOUT, env=env, timeout=1200)
    if (process.returncode == 0) != success:
        raise AssertionError(f'{args[0]} exit {process.returncode}; see {log}')


def texture(path):
    raw = Path(path).read_bytes()
    top = list(boxes(raw, 0, len(raw)))
    meta = next(b for b in top if b['type'] == 'meta')
    children = list(boxes(raw, meta['payload_start']+4, meta['end']))
    iinf = next(b for b in children if b['type'] == 'iinf')
    pos = iinf['payload_start']
    infos = list(boxes(raw, pos+4+(4 if raw[pos] else 2), iinf['end']))
    ids=[]
    for info in infos:
        pos=info['payload_start']; version=raw[pos]; width=4 if version==3 else 2
        item=int.from_bytes(raw[pos+4:pos+4+width], 'big')
        start=pos+4+width+6
        end=raw.index(b'\0',start,info['end'])+1
        if raw[end:info['end']]==URI: ids.append(item)
    report=inspect(Path(path))
    if not ids: return None, report
    assert len(ids)==1
    item=next(i for i in report['items'] if i['id']==ids[0])
    idat=next((b for b in children if b['type']=='idat'),None)
    return plistlib.loads(payload_for_item(raw,item['location'],idat)),report


def readback(adapter,path,output):
    request={'schema_version':2,'operation':'texture-readback','input_path':str(path),'output_path':str(output)}
    result=subprocess.run([str(adapter)],input=(json.dumps(request)+'\n').encode(),capture_output=True,timeout=180)
    assert result.returncode==0,result.stderr.decode(errors='replace')[-3000:]
    data=json.loads(output.read_text())
    assert not data.get('exception'),data.get('exception')
    return data


def check(path, baseline, adapter, directory):
    data,graph=texture(path); plain,original=texture(baseline)
    assert plain is None and data is not None,'same Styles switch did not add Texture'
    assert data['TextureStylePeopleDataVersion']==3
    assert data['Preset']=='Standard'
    expected=Counter(i['payload_sha256'] for i in original['items'] if i['type']=='hvc1')
    actual=Counter(i['payload_sha256'] for i in graph['items'] if i['type']=='hvc1')
    assert not expected-actual,'primary or Gain Map HEVC payload changed'
    facts=readback(adapter,path,directory/'readback.json')
    assert facts['ImageIO_decoded'] and facts['texture_properties_present'] and facts['semantic_properties_present']
    people=data['TextureStylePostProcessedPeopleData']
    assert facts['texture_person_count']==len(people)==len(facts['native_person_data_readback'])
    assert len({p['instanceMaskReferenceKey'] for p in people})==len(people)
    for person in people:
        assert person['imageStats']['Mattify']
        for key in ['facePitch','faceYaw','faceRoll']: assert abs(person[key])<=360
    assert any(p.get('details',{}).get('aux_type','').endswith(':semanticpersoninstances') for p in graph['properties']) or not people
    return {'output':str(path.relative_to(directory.parent)), 'bytes':path.stat().st_size,
            'sha256':digest(path),'people':len(people),'texture_version':3,
            'primary_and_gain_map_hevc_preserved':True,
            'decoded_dimensions':[facts['decoded_width'],facts['decoded_height']],
            'native_groups':[sorted(p['imageStats']) for p in people]}


# This transparent observer forwards real requests and responses. It copies
# reports while caller-owned staging still exists. It never supplies SPI data.
WRAPPER = r'''#!/usr/bin/env python3
import json,os,shutil,subprocess,sys
from pathlib import Path
adapter=os.environ['XDREMUX_REAL_TEXTURE_ADAPTER']
evidence=Path(os.environ['XDREMUX_TEXTURE_OBSERVATIONS'])
child=subprocess.Popen([adapter,'--persistent-json-lines'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=sys.stderr)
try:
 for line in sys.stdin.buffer:
  request=json.loads(line); operation=request['operation']
  if os.environ.get('XDREMUX_TEXTURE_INJECT_FAILURE') and operation=='texture-statistics':
   sys.stderr.write('injected native statistics failure for publication regression\n');sys.exit(73)
  child.stdin.write(line);child.stdin.flush();response=child.stdout.readline()
  if not response: sys.exit(child.wait())
  if operation=='texture-readback':
   root=Path(request['output_path']).parent
   if (root/'carrier-production.json').is_file():
    target=evidence/root.parent.name
    shutil.copytree(root,target)
  sys.stdout.buffer.write(response);sys.stdout.buffer.flush()
finally:
 if child.poll() is None:
  child.stdin.close()
  try:child.wait(timeout=5)
  except subprocess.TimeoutExpired:child.kill();child.wait()
'''


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path)
    parser.add_argument('--all-fixtures',action='store_true')
    parser.add_argument('--skip-build',action='store_true')
    args=parser.parse_args()
    if sys.platform!='darwin':parser.error('requires macOS with Texture SPI')
    out=(args.output or Path(tempfile.mkdtemp(prefix='xdremux-texture-'))).resolve()
    out.mkdir(parents=True,exist_ok=True)
    summary={'schema':'xdremux-product-texture-v1','head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
             'cases':[],'passed':False,'photos_device_result':'new HDR product outputs require device import; earlier SDR research activation was user-reported'}
    try:
        if not args.skip_build:
            run(['swift','build','--product','xdremux-apple-adapter'],out/'swift-build.log')
            run(['cargo','build','--locked','-p','xdremux-cli'],out/'rust-build.log')
        adapter=Path(subprocess.check_output(['swift','build','--show-bin-path'],cwd=ROOT,text=True).strip())/'xdremux-apple-adapter'
        cli=ROOT/'target/debug/xdremux'
        observer=out/'observe-adapter.py';observer.write_text(WRAPPER);observer.chmod(0o755)
        observations=out/'native';observations.mkdir(exist_ok=True)
        env={**os.environ,'XDREMUX_APPLE_ADAPTER':str(observer),'XDREMUX_REAL_TEXTURE_ADAPTER':str(adapter),'XDREMUX_TEXTURE_OBSERVATIONS':str(observations)}
        fixtures=sorted((ROOT/'fixtures/proxdr').rglob('*.heic')) if args.all_fixtures else [ROOT/'fixtures/proxdr/oppo'/p for p in [
            'find-x9-ultra/uhdr-portrait-02.heic','find-x9-ultra/uhdr-hr-01.heic','find-x7-ultra/lhdr-v2-01.heic']]
        manifest={line.split(None,1)[1].strip():line.split()[0] for line in (ROOT/'fixtures/SHA256SUMS').read_text().splitlines()}
        for index,fixture in enumerate(fixtures):
            assert digest(fixture)==manifest[str(fixture.relative_to(ROOT/'fixtures'))]
            case=out/(fixture.parent.name+'__'+fixture.stem);case.mkdir()
            baseline=case/'standard-hdr.heic';styled=case/'styles-texture.heic'
            run([cli,'convert','--input',fixture,'--output',baseline],case/'baseline.log',env)
            run([cli,'convert','--input',fixture,'--output',styled,'--apple-styles'],case/'convert.log',env)
            run([cli,'validate',styled],case/'validate.log')
            facts=check(styled,baseline,adapter,case);facts['fixture']=str(fixture.relative_to(ROOT));summary['cases'].append(facts)
            if index==0:
                batch=case/'batch';batch.mkdir()
                run([cli,'batch','--input',fixture,'--output-dir',batch,'--apple-styles','--jobs','2','--json'],case/'batch.log',env)
                paths=list(batch.glob('*.heic'));assert len(paths)==1
                facts=check(paths[0],baseline,adapter,case);facts['entry_point']='batch';summary['cases'].append(facts)
                sentinel=case/'existing.heic';sentinel.write_bytes(b'previous published output')
                failing={**env,'XDREMUX_TEXTURE_INJECT_FAILURE':'1'}
                run([cli,'convert','--input',fixture,'--output',sentinel,'--apple-styles'],case/'expected-failure.log',failing,success=False)
                assert sentinel.read_bytes()==b'previous published output','native failure damaged published output'
                summary['atomic_failure_preserves_existing_output']=True
        summary['native_transaction_count']=len(list(observations.glob('*/final-native-readback.json')))
        assert summary['native_transaction_count']==len(summary['cases'])
        summary['passed']=True
    finally:
        (out/'result.json').write_text(json.dumps(summary,indent=2)+'\n')
    print(json.dumps(summary,indent=2))


if __name__=='__main__':main()
