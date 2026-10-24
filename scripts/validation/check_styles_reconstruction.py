#!/usr/bin/env python3
"""Source-bound native/CLI check. Private input paths are supplied externally.

RAW reference arrays are validation inputs only. The converter never receives
their paths. No private photos or precomputed measurements are stored in Git.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import subprocess
import tempfile


def digest(path):
    with Path(path).open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()


def invoke(command, *, env=None, payload=None, timeout=300):
    result = subprocess.run(command, env=env, input=payload, text=True,
                            capture_output=True, timeout=timeout)
    if result.returncode:
        raise AssertionError(f'{command[0]} exited {result.returncode}: {result.stderr[-6000:]}')
    return json.loads(result.stdout)


def native(adapter, request):
    # Two frames exercise the persistent protocol, including response delimiters.
    requests = [dict(schema_version=2, **request),
                dict(schema_version=2, operation='capabilities')]
    result = subprocess.run([str(adapter), '--persistent-json-lines'],
                            input=''.join(json.dumps(x)+'\n' for x in requests),
                            capture_output=True, text=True, timeout=120)
    assert result.returncode == 0, result.stderr[-3000:]
    lines = result.stdout.splitlines()
    assert len(lines) == 2, 'native responses must be separate JSON lines'
    assert json.loads(lines[1])['schema_version'] == 2
    return json.loads(lines[0])


def reference_check(adapter, inputs, bundles, configs, output):
    import numpy as np
    yweights = np.array([.22897456, .69173855, .07928691])
    rows = []
    for config in configs:
        bundle = bundles[config['stem']]
        report = json.loads((bundle/'report.json').read_text())
        w, h = (report['scene'][k] for k in ['width', 'height'])
        with tempfile.TemporaryDirectory(prefix='xd-styles-reference-') as temp:
            d = Path(temp)
            native(adapter, dict(operation='styles-readback', input_path=inputs[config['stem']],
                                 output_path=str(d), width=w, height=h))
            original = np.fromfile(d/'published_hdr.f32', dtype='<f4').reshape(h,w,4)[...,:3]
            native(adapter, dict(operation='styles-readback', input_path=str(bundle/'hdr.heic'),
                                 output_path=str(d), width=w, height=h))
            reconstructed = np.fromfile(d/'published_hdr.f32', dtype='<f4').reshape(h,w,4)[...,:3]
        data = np.load(config['arrays'])
        baseline = data[config.get('baseline', 'hdr')]
        shape = baseline.shape[:2]
        # Geometry was frozen when the independent references were produced.
        # Select only the exact HEIF display orientation using its ORIGINAL HDR,
        # never using RAW or the new reconstruction to fit spatial parameters.
        candidates = []
        for turns in range(4):
            for mirror in [False, True]:
                a = np.rot90(original, turns)
                if mirror: a = a[:,::-1]
                if a.shape[:2] != shape: continue
                cy,by = a@yweights,baseline@yweights
                m = data[config.get('valid','valid')].astype(bool)&(cy>1e-4)&(by>1e-4)
                p = np.log2(cy[m])
                q = np.log2(by[m])
                candidates.append((float(np.corrcoef(p,q)[0,1]),turns,mirror))
        corr, turns, mirror = max(candidates)
        assert corr > .98, f'original HDR orientation/reference mismatch: {corr}'
        before = np.rot90(original, turns)
        after = np.rot90(reconstructed, turns)
        if mirror: before, after = before[:,::-1], after[:,::-1]
        by, ay = before@yweights, after@yweights
        for ref in config['references']:
            ry = data[ref]@yweights
            valid = data[config.get('valid','valid')].astype(bool)
            valid &= np.isfinite(ry) & (ry>1e-6) & (by>1e-6) & (ay>1e-6)
            lo,hi = np.percentile(ry[valid],[2,99.5])
            valid &= (ry>=lo)&(ry<=hi)
            yy,xx = np.indices(shape)
            train = valid & ((yy//64+xx//64)%2==0)
            test = valid & ~train
            assert test.sum()>10000
            metrics = {}
            for name,sy in [('original_hdr',by),('reconstructed_hdr',ay)]:
                residual = np.log2(np.maximum(sy,1e-8))-np.log2(np.maximum(ry,1e-8))
                offset = -float(np.median(residual[train]))
                e = np.abs(residual[test]+offset)
                metrics[name] = dict(mean_abs_ev=float(e.mean()),median_abs_ev=float(np.median(e)),
                                     p90_abs_ev=float(np.percentile(e,90)),exposure_fit_ev=offset)
            old,new = (metrics[k]['mean_abs_ev'] for k in ['original_hdr','reconstructed_hdr'])
            assert new<old*.60 and new<.25, f'{config["stem"]}/{ref}: {old} -> {new} EV'
            rows.append(dict(source=config['stem'],reference=ref,arrays_sha256=digest(config['arrays']),
                             geometry_correlation=corr,orientation=dict(turns=turns,mirror=mirror),
                             heldout_pixels=int(test.sum()),metrics=metrics))
    output.write_text(json.dumps(rows,indent=2)+'\n')
    return rows


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--cli', type=Path, required=True)
    p.add_argument('--adapter', type=Path, required=True)
    p.add_argument('--inputs-json', type=Path, required=True)
    p.add_argument('--output-dir', type=Path, required=True)
    p.add_argument('--reference-config', type=Path)
    args = p.parse_args()
    args.output_dir.mkdir(parents=True,exist_ok=False)
    sources = json.loads(args.inputs_json.read_text())
    before = {path:digest(path) for path in sources}
    env = dict(os.environ, XDREMUX_APPLE_ADAPTER=str(args.adapter.resolve()))
    cmd = [str(args.cli.resolve()),'reconstruct-styles','--output-dir',str(args.output_dir/'photos'),'--max-edge','1024']
    for source in sources: cmd += ['--input',source]
    result = invoke(cmd,env=env,timeout=900)
    assert result['failures']==0 and len(result['results'])==len(sources)
    bundles, inputs, versions, orientations = {}, {}, set(), set()
    for item in result['results']:
        report = item['report']; path = Path(item['input']); bundle = Path(item['output'])
        bundles[path.stem],inputs[path.stem] = bundle,item['input']
        assert report['source_sha256']==before[item['input']]
        assert report['source_preserved'] and report['scene']['runtime_raw_inputs']==0
        heic = report['outputs']['hdr.heic']
        assert heic['compressed_sdr_preserved'] and heic['sdr_readback_max_rgb_error']==0
        assert heic['hdr_target_mae_ev']<.15
        assert all(digest(bundle/name)==facts['sha256'] for name,facts in report['outputs'].items())
        native_facts = native(args.adapter,dict(operation='imageio-auxiliary-facts',input_path=str(bundle/'hdr.heic')))
        assert native_facts['auxiliary']['iso_gain_map'], 'native consumer must discover the ISO Gain Map'
        versions.add(report['native']['metadata_version']); orientations.add(report['native']['orientation'])
    assert versions=={8,9,15,16},versions
    assert {1,3,6}.issubset(orientations),orientations
    # No-overwrite check through the public command, including the manifest.
    first = sources[0]; bundle = bundles[Path(first).stem]; saved = digest(bundle/'report.json')
    again = subprocess.run([str(args.cli),'reconstruct-styles','--input',first,'--output-dir',str(args.output_dir/'photos'),'--max-edge','1024'],env=env,capture_output=True,text=True,timeout=30)
    assert again.returncode==1 and digest(bundle/'report.json')==saved
    # Missing resources must fail without publishing a bundle.
    invalid = args.output_dir/'invalid.heic'; invalid.write_bytes(b'not a HEIF')
    bad = subprocess.run([str(args.cli),'reconstruct-styles','--input',str(invalid),'--output-dir',str(args.output_dir/'rejected')],env=env,capture_output=True,text=True,timeout=30)
    assert bad.returncode==1 and not (args.output_dir/'rejected/invalid.reconstructed').exists()
    # Reject unknown metadata BEFORE calling native reverse selectors or opening aux images.
    d = args.output_dir/'unknown-version';d.mkdir()
    (d/'styles.plist').write_bytes(plistlib.dumps({'0':255,'1':bytes(51840),'2':True}))
    request=dict(schema_version=2,operation='styles-decode-reverse',input_path=first,output_path=str(d),max_edge=1024,supported_metadata_versions=[8,9,15,16])
    unknown = subprocess.run([str(args.adapter)],input=json.dumps(request),capture_output=True,text=True,timeout=30)
    assert unknown.returncode!=0 and 'Unsupported Styles metadata version' in unknown.stderr
    assert not (d/'unstyled.f32').exists()
    references=[]
    if args.reference_config:
        references=reference_check(args.adapter,inputs,bundles,json.loads(args.reference_config.read_text()),args.output_dir/'raw-reference-results.json')
    assert all(digest(path)==value for path,value in before.items())
    summary=dict(passed=True,photos=len(sources),metadata_versions=sorted(versions),orientations=sorted(orientations),
                 source_hashes=before,results=result,raw_reference_comparisons=references,
                 checks=['all-photo conversion','compressed SDR identity','zero SDR pixel difference','native ISO discovery',
                         'HDR target readback','persistent JSON framing','no overwrite','missing-resource rejection',
                         'unknown-version rejection','source integrity','RAW spatial holdout' if references else 'no RAW references supplied'])
    (args.output_dir/'validation.json').write_text(json.dumps(summary,indent=2)+'\n')
    print(json.dumps(dict(passed=True,photos=len(sources),metadata_versions=sorted(versions),raw_comparisons=len(references)),indent=2))


if __name__=='__main__':
    main()
