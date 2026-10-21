#!/usr/bin/env python3
"""ARCHIVE prototype: matched Styles spans on one Mac; not a product/CI pass.

Uses fixed-source comparison helpers with bounded subprocesses and an
independent retained clone. No shared worktree registration or automatic deletion.
Formal budget and unprofiled ABBA receipts remain separate and unchanged.
"""
from pathlib import Path
import argparse
import hashlib
import json
import os
import platform
import re
import statistics
import sys
from bounded_stage_lifecycle import (Commands, Interrupted, budget,
    install_timers, load_verified_owner, owned_workspace, reference_clone, verify_source)

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

def run_native(a, out, report, commands):
    c, identity, owner_text, candidate_rust = load_verified_owner(a.source, commands, CANDIDATE)
    report['source_identity'] = identity
    report['fixed_source_hashes'] = dict(owner_sha256=sha(owner_text.encode()),
                                       candidate_rust_sha256=sha(candidate_rust.encode()))
    commands.checkpoint()
    fixture_sha = c.checked_fixture(c.ROOT)
    candidate = dict(sha=CANDIDATE, cli=c.binary(a.cli), adapter=c.binary(a.apple_adapter))
    if candidate['cli']['sha256'] != a.cli_sha256 or candidate['adapter']['sha256'] != a.adapter_sha256:
        raise ValueError('candidate binaries differ from supplied build receipt')
    report['candidate'] = candidate
    report['host'] = {command: commands.capture(command.split(), c.ROOT, timeout=30).strip()
                      for command in ('sw_vers', 'uname -m', 'xcodebuild -version', 'rustc --version')}
    owned = owned_workspace(report, commands)
    baseline = reference_clone(c, BASELINE, owned, out, report, commands)
    report['unmodified_baseline_build'] = baseline
    source = owned / 'reference-source'
    file = source / OWNER
    original = commands.capture(['git', '-C', str(source), 'show', BASELINE + ':' + str(OWNER)],
                                source, timeout=30)
    if file.read_bytes() != original.encode():
        raise ValueError('owned baseline source changed before timer insertion')
    modified, proof = insert_timers(original, candidate_rust)
    install_timers(file, original.encode(), modified.encode(), owned, report, commands)
    patch = commands.capture(['git', '-C', str(source), 'diff', '--binary'], source, timeout=30)
    (out / 'baseline-timers.patch').write_text(patch)
    report['instrumentation'] = proof
    commands.checkpoint()
    env = {**os.environ, 'CARGO_TARGET_DIR': str(owned / 'reference-cargo-target')}
    c.run_logged(['cargo', 'build', '--locked', '--release', '-p', 'xdremux-cli'],
                 source, env, out / 'instrumented-baseline-build.log', 1800)
    dirty = commands.git(source, 'status', '--porcelain=v1', '--untracked-files=all')
    if dirty != 'M ' + str(OWNER):
        raise ValueError('unexpected instrumented baseline changes; retained without reset/removal')
    baseline = {**baseline, 'cli': c.binary(owned / 'reference-cargo-target/release/xdremux'),
                'tracked_status': dirty, 'instrumented': True}
    if c.checked_fixture(source) != fixture_sha:
        raise ValueError('fixture mismatch')
    report['instrumented_baseline'] = baseline
    outputs = []
    for ordinal, (warmup, iteration, arm) in enumerate(c.sample_order()):
        logs = out / f'{ordinal:02d}-{arm}'
        logs.mkdir()
        work = owned / f'{ordinal:02d}-{arm}'
        work.mkdir()
        output = work / 'styles.heic'
        row = dict(ordinal=ordinal, arm=arm, warmup=warmup, iteration=iteration,
                   status='running', logs=logs.name)
        report['samples'].append(row)
        commands.checkpoint()
        try:
            row.update(measure(c, arm, candidate if arm == 'candidate' else baseline,
                               c.ROOT / c.FIXTURE, output, logs))
        except BaseException as error:
            row.update(status='interrupted' if isinstance(error, (KeyboardInterrupt, Interrupted))
                       else 'failed', error=type(error).__name__ + ': ' + str(error))
            commands.checkpoint()
            raise
        outputs.append((candidate if arm == 'candidate' else baseline, output, logs, row))
        commands.checkpoint()
    for facts, output, logs, row in outputs:
        try:
            c.run_logged([facts['cli']['path'], 'validate', str(output), '--apple-styles'],
                         c.ROOT, c.conversion_environment(facts), logs / 'validate.log', 300)
            if c.digest(output) != row['output_sha256']:
                raise ValueError('validation changed measured output')
        except BaseException as error:
            row.update(status='validation-failed', error=type(error).__name__ + ': ' + str(error))
            commands.checkpoint()
            raise
        row['status'] = 'validated'
        commands.checkpoint()
    report['stage_comparison'] = summary(report['samples'])
    if verify_source(c.ROOT, commands, CANDIDATE) != identity:
        raise ValueError('candidate source changed during diagnostic')
    report['status'] = 'completed-with-retained-workspace'


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--source', type=Path, required=True)
    p.add_argument('--cli', type=Path, required=True)
    p.add_argument('--apple-adapter', type=Path, required=True)
    p.add_argument('--cli-sha256', required=True)
    p.add_argument('--adapter-sha256', required=True)
    p.add_argument('--output-dir', type=Path, required=True)
    a = p.parse_args()
    out = a.output_dir.resolve()
    out.mkdir(parents=True, exist_ok=False)
    report = dict(status='not-started', diagnostic_only=True, absolute_gate_unchanged=True,
                  source=CANDIDATE, baseline=BASELINE, samples=[],
                  limits=['Portable lifecycle tests are not macOS compilation/execution or stage results.',
                          'Only broad operations are comparable: raster/gain-raster release points differ.',
                          'Nested logging overhead differs; scene delta is not pure algorithm cost.',
                          'Do not add overlapping spans or stage medians as E2E attribution.',
                          'All owned clones, targets and sample media are retained outside text evidence.',
                          'SIGKILL/host loss may leave only the last checkpoint; no cleanup success assumed.'])
    commands = Commands(out, report)
    try:
        if not a.source.is_dir():
            raise ValueError('source checkout missing; owner not loaded')
        if platform.system() != 'Darwin':
            raise RuntimeError('macOS/Apple adapter unavailable; zero builds or native conversions')
        if 'XDREMUX_SYSTEM_GAIN_MAP' in os.environ:
            raise ValueError('system-gain-map opt-in would change the default Styles path')
        with budget(90 * 60):
            run_native(a, out, report, commands)
        return 0
    except (KeyboardInterrupt, Interrupted) as error:
        report.update(status='interrupted', error_type=type(error).__name__, error=str(error))
        return 2
    except BaseException as error:
        report.update(status='timed-out' if isinstance(error, (TimeoutError, __import__('subprocess').TimeoutExpired))
                      else 'failed' if report['commands'] else 'blocked',
                      error_type=type(error).__name__, error=str(error))
        return 2
    finally:
        report['process_cleanup_uncertain'] = any(row.get('cleanup_status') == 'uncertain'
                                                   for row in report['commands'])
        commands.checkpoint()
        print(json.dumps(report, indent=2))


if __name__ == '__main__':
    raise SystemExit(main())
