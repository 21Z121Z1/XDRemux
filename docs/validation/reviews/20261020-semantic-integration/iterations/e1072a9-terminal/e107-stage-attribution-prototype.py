#!/usr/bin/env python3
"""ARCHIVE prototype: matched Styles spans on one Mac; not a product/CI pass.

Uses the existing performance owner's build/process/fixture/CPU helpers. The
only source mutation is reversible timer insertion in its owned main worktree.
Formal budget and unprofiled ABBA receipts remain separate and unchanged.
"""
from pathlib import Path
import argparse
import hashlib
import importlib.util
import json
import os
import platform
import re
import signal
import statistics
import sys
import tempfile

BASELINE = '89d806c8aec961a7991eb3e6d6e2b1370aed8791'
CANDIDATE = 'e1072a90e31b12861dfe8b9cf25e07ce75c17c44'
OWNER = Path('crates/xdremux-runtime/src/apple_styles.rs')
COMMON = ('source_analysis','base_conversion','semantic_production','resource_decode',
          'scene_derivation','thumbnail_encode','key1_fit','style_metadata',
          'native_style_properties','style_assembly','texture_generation',
          'final_validation','publication')
DETAIL = ('scene_hdr_reconstruction','scene_linear_area_reduction')
ANCHORS = (
    ('source_analysis', '    let prepared = runtime.analyze_proxdr(source)?;\n', 'after'),
    ('base_conversion', '        .map_err(|error| RuntimeError::external("Photographic Styles base staging", error))?;\n', 'after'),
    ('semantic_production', '    let primary = runtime\n', 'before'),
    ('resource_decode', '    let scene_raster = downsample_raster(&raster, LINEAR_THUMBNAIL_MAX_SIZE)?;\n', 'before'),
    ('scene_derivation', '    let scene_rasters = derive_style_scene(&scene_raster, &gain_raster, &prepared.scale)?;\n', 'after'),
    ('thumbnail_encode', '    let style_data = source_derived_style_data(&scene_rasters)?;\n', 'before'),
    ('key1_fit', '    let style_data = source_derived_style_data(&scene_rasters)?;\n', 'after'),
    ('style_metadata', '    // Validate the exact Rust-owned key-1 resource at the private consumer\n', 'before'),
    ('native_style_properties', '    adapter.semantic_style_properties_facts(&style_properties, &style_data)?;\n', 'after'),
    ('style_assembly', '    let assembled =\n        crate::apple_texture::generate(&adapter, &styles_path, &assembled, staging.path())?;\n', 'before'),
    ('texture_generation', '    let assembled =\n        crate::apple_texture::generate(&adapter, &styles_path, &assembled, staging.path())?;\n', 'after'),
    ('final_validation', '    let mut publisher = AtomicFilePublisher::new(output.to_path_buf());\n', 'before'),
    ('publication', '    let published = publisher.publish_bytes(assembled)?;\n', 'after'),
)

def sha(data):
    return hashlib.sha256(data).hexdigest()

def insert_timers(original: str, candidate: str):
    """Exact fixed revisions only; deleting added bytes must recover original."""
    if 'fn profile_start()' in original or 'let mut profile = profile_start();' in original:
        raise ValueError('baseline already instrumented; refusing double insertion')
    start = candidate.index('fn profile_start()')
    end = candidate.index('// Apple ImageIO accepts', start)
    helper = candidate[start:end].replace('Instant', 'std::time::Instant')
    boundary = '// Apple ImageIO accepts'
    if original.count(boundary) != 1:
        raise ValueError('unexpected helper boundary')
    result = original.replace(boundary, helper + boundary, 1)
    additions = [helper]
    begin = '    let output = output.as_ref();\n'
    if result.count(begin) != 1:
        raise ValueError('unexpected entry boundary')
    entry = '    let mut profile = profile_start();\n'
    result = result.replace(begin, entry + begin, 1)
    additions.append(entry)
    for stage, anchor, side in ANCHORS:
        if result.count(anchor) != 1:
            raise ValueError(f'unexpected {stage} boundary')
        stamp = f'    profile_stage(&mut profile, "{stage}");\n'
        result = result.replace(anchor, anchor+stamp if side=='after' else stamp+anchor, 1)
        additions.append(stamp)
    recovered = result
    for addition in additions:
        if recovered.count(addition) != 1:
            raise ValueError('timer removal is not unique')
        recovered = recovered.replace(addition, '', 1)
    if recovered != original:
        raise ValueError('instrumentation changed existing source')
    return result, dict(original_sha256=sha(original.encode()), instrumented_sha256=sha(result.encode()),
                        removed_insertions_restore_exact_original=True, common_stages=list(COMMON),
                        baseline_scene_boundary='legacy downsample plus scene derivation',
                        candidate_scene_boundary='full-resolution HDR reconstruction plus linear area reduction; nested details excluded from totals')

def spans(stderr, arm):
    expected = COMMON if arm=='baseline' else COMMON[:4]+DETAIL+COMMON[4:]
    rows=[]
    for line in stderr.splitlines():
        if not line.startswith('xdremux_styles_profile'):
            continue
        m=re.fullmatch(r'xdremux_styles_profile stage=([a-z0-9_]+) seconds=([0-9]+(?:\.[0-9]+)?)',line)
        if not m:
            raise ValueError('malformed stage receipt')
        value=float(m[2])
        if not (0<=value<float('inf')):
            raise ValueError('nonfinite stage receipt')
        rows.append(dict(stage=m[1],wall_seconds=value))
    if tuple(r['stage'] for r in rows)!=expected:
        raise ValueError('missing/duplicate/out-of-order stage receipt')
    return rows

def summary(rows):
    values={}
    for arm in ('baseline','candidate'):
        group=[r for r in rows if r['arm']==arm and not r['warmup']]
        if len(group)!=5 or any(r['status']!='validated' for r in group):
            raise ValueError('five independently validated measurements per arm required')
        values[arm]={stage:statistics.median(next(s['wall_seconds'] for s in r['stages'] if s['stage']==stage) for r in group) for stage in COMMON}
    return {stage:dict(baseline_seconds=values['baseline'][stage],candidate_seconds=values['candidate'][stage],
                       candidate_minus_baseline_seconds=values['candidate'][stage]-values['baseline'][stage]) for stage in COMMON}

def measure(c, arm, facts, fixture, output, logs):
    env=c.conversion_environment(facts,profile=True)
    command=[str(c.TIME),'-l',facts['cli']['path'],'convert','--input',str(fixture),'--output',str(output),'--apple-styles']
    with (logs/'stdout.log').open('w') as stdout,(logs/'stderr.log').open('w') as stderr:
        status=c.run_process(command,c.ROOT,env,stdout,stderr,1200)
    if status or not output.is_file() or not output.stat().st_size:
        raise RuntimeError(f'{arm} conversion failed with {status}; see {logs}')
    text=(logs/'stderr.log').read_text()
    stages=spans(text,arm)
    digest=c.digest(output)
    row=dict(status='converted',output_sha256=digest,output_bytes=output.stat().st_size,
             stages=stages,**c.time_facts(text))
    # Never sum nested detail spans. This is a partition check on one sample,
    # not a sum of stage medians or a claim of total CPU attribution.
    common_sum=sum(s['wall_seconds'] for s in stages if s['stage'] in COMMON)
    if common_sum>row['time_real_seconds']+0.1:
        raise ValueError('stage partition exceeds elapsed command time')
    row['common_span_sum_seconds']=common_sum
    row['outside_spans_seconds']=row['time_real_seconds']-common_sum
    return row

def load_owner(source):
    p=source/'scripts/performance/compare_apple_styles_revisions.py'
    spec=importlib.util.spec_from_file_location('xdremux_comparison_owner',p)
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    return module

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--source',type=Path,required=True)
    p.add_argument('--cli',type=Path,required=True)
    p.add_argument('--apple-adapter',type=Path,required=True)
    p.add_argument('--cli-sha256',required=True)
    p.add_argument('--adapter-sha256',required=True)
    p.add_argument('--output-dir',type=Path,required=True)
    a=p.parse_args();out=a.output_dir.resolve();out.mkdir(parents=True,exist_ok=False)
    report=dict(status='not-started',diagnostic_only=True,absolute_gate_unchanged=True,
                source=CANDIDATE,baseline=BASELINE,samples=[],
                limits=['Timer-only main mutation; instrumented binary is not unmodified-main acceptance.',
                        'Extra profiles do not replace or amend the failed absolute budget or prior ABBA.',
                        'Nested candidate scene details excluded from common partition and totals.',
                        'Stage medians/deltas are separate statistics; do not add them as E2E attribution.'])
    c=None
    try:
        if platform.system()!='Darwin':
            raise RuntimeError('macOS/Apple adapter unavailable; zero builds or native conversions')
        if 'XDREMUX_SYSTEM_GAIN_MAP' in os.environ:
            raise ValueError('system-gain-map opt-in would change the default Styles path')
        def deadline(signum,frame):
            raise TimeoutError('90-minute prototype deadline; stop only owned command groups/worktree')
        signal.signal(signal.SIGALRM,deadline)
        signal.alarm(90*60)
        c=load_owner(a.source.resolve())
        if c.commit(c.ROOT,'HEAD')!=CANDIDATE or c.git(c.ROOT,'status','--porcelain=v1'):
            raise ValueError('expected exact clean e107 candidate source')
        fixture_sha=c.checked_fixture(c.ROOT)
        candidate=dict(sha=CANDIDATE,cli=c.binary(a.cli),adapter=c.binary(a.apple_adapter))
        if candidate['cli']['sha256']!=a.cli_sha256 or candidate['adapter']['sha256']!=a.adapter_sha256:
            raise ValueError('candidate binaries differ from supplied build receipt')
        report['candidate']=candidate
        report['host']={cmd:__import__('subprocess').check_output(cmd.split(),text=True).strip() for cmd in ('sw_vers','uname -m','xcodebuild -version','rustc --version')}
        with tempfile.TemporaryDirectory(prefix='xdremux-stage-prototype-') as raw:
            owned=Path(raw)
            # Existing owner creates/builds/cleans exactly its own worktree and
            # target. Keep formal normal-main identity before instrumentation.
            with c.reference_build(c.ROOT,BASELINE,owned,out,report) as baseline:
                report['unmodified_baseline_build']=baseline
                source=owned/'reference-source'
                file=source/OWNER
                original=file.read_text()
                modified,proof=insert_timers(original,(c.ROOT/OWNER).read_text())
                file.write_text(modified)
                (out/'baseline-timers.patch').write_bytes(__import__('subprocess').check_output(['git','-C',str(source),'diff','--binary']))
                report['instrumentation']=proof
                env={**os.environ,'CARGO_TARGET_DIR':str(owned/'reference-cargo-target')}
                c.run_logged(['cargo','build','--locked','--release','-p','xdremux-cli'],source,env,out/'instrumented-baseline-build.log',1800)
                dirty=c.git(source,'status','--porcelain=v1')
                if dirty!='M '+str(OWNER):
                    raise ValueError('unexpected instrumented baseline changes')
                baseline={**baseline,'cli':c.binary(owned/'reference-cargo-target/release/xdremux'),
                          'tracked_status':dirty,'instrumented':True}
                if c.checked_fixture(source)!=fixture_sha:
                    raise ValueError('fixture mismatch')
                report['instrumented_baseline']=baseline
                outputs=[]
                for ordinal,(warmup,iteration,arm) in enumerate(c.sample_order()):
                    logs=out/f'{ordinal:02d}-{arm}';logs.mkdir()
                    work=owned/f'{ordinal:02d}-{arm}';work.mkdir()
                    output=work/'styles.heic'
                    row=dict(ordinal=ordinal,arm=arm,warmup=warmup,iteration=iteration,status='running',logs=logs.name)
                    report['samples'].append(row);c.write_report(out,report)
                    try:
                        row.update(measure(c,arm,candidate if arm=='candidate' else baseline,c.ROOT/c.FIXTURE,output,logs))
                    except Exception as error:
                        row.update(status='failed',error=str(error));c.write_report(out,report)
                        raise
                    outputs.append((candidate if arm=='candidate' else baseline,output,logs,row));c.write_report(out,report)
                # All independent native readbacks are outside timed spans.
                for facts,output,logs,row in outputs:
                    c.run_logged([facts['cli']['path'],'validate',str(output),'--apple-styles'],c.ROOT,c.conversion_environment(facts),logs/'validate.log',300)
                    if c.digest(output)!=row['output_sha256']:
                        raise ValueError('validation changed measured output')
                    row['status']='validated';c.write_report(out,report)
                report['stage_comparison']=summary(report['samples'])
                if c.commit(c.ROOT,'HEAD')!=CANDIDATE or c.git(c.ROOT,'status','--porcelain=v1'):
                    raise ValueError('candidate source changed during diagnostic')
        if report.get('owned_worktree_cleanup',{}).get('returncode')!=0:
            raise RuntimeError('owned worktree cleanup not confirmed')
        report['status']='completed'
        return 0
    except Exception as e:
        report.update(status='blocked' if c is None else 'failed',error=str(e))
        return 2
    finally:
        if platform.system()=='Darwin':
            signal.alarm(0)
        (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
        print(json.dumps(report,indent=2))

if __name__=='__main__':
    raise SystemExit(main())
