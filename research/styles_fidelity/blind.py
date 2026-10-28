"""Blind resource regeneration and joint calibration against native edit responses."""
from __future__ import annotations

import argparse
import itertools
import json
import math
import os
from pathlib import Path
import subprocess
import sys
import tempfile

import numpy as np

from scripts.apple.check_style_tone_response import prepare_consumer_compatibility, PINNED_RESEARCH_REVISION
from . import corpus, isolate, native
from .texture import measurement_host, primitive, run

TONES = (-0.5, 0.0, 0.5)
DEFAULT = {'baseline_scale': 1.0, 'gain_scale': 1.0, 'baseline_maximum': 10.4}


def native_rgb(path: Path) -> np.ndarray:
    layout = json.loads(path.with_name(path.name + '.json').read_text())
    shape = layout['shape']
    if layout.get('dtype') != '<f4' or len(shape) != 3 or shape[2] != 4 or math.prod(shape) * 4 != path.stat().st_size:
        raise ValueError('native render layout differs from RGBA32F')
    pixels = np.fromfile(path, dtype='<f4').reshape(shape)[..., :3]
    if not np.isfinite(pixels).all():
        raise ValueError('native edit response contains nonfinite RGB')
    return pixels.copy()


def score(reference: np.ndarray, actual: np.ndarray) -> dict:
    if actual.shape != reference.shape or not np.isfinite(actual).all() or not np.isfinite(reference).all():
        raise ValueError('native response geometry or finite-pixel contract differs')
    difference = actual.astype('f8') - reference.astype('f8')
    absolute = np.abs(difference)
    y = reference @ np.array([.22897456, .69173852, .07928691])
    low, high = np.quantile(y, [.1, .9])
    regions = {}
    for label, mask in [('shadows', y <= low), ('midtones', (y > low) & (y < high)), ('highlights', y >= high)]:
        regions[label] = {'pixels': int(mask.sum()),
                          'rmse': float(np.sqrt(np.mean(difference[mask] ** 2))) if mask.any() else None}
    return {'rmse': float(np.sqrt(np.mean(difference ** 2))), 'mae': float(absolute.mean()),
            'maximum_absolute': float(absolute.max()), 'p95_absolute': float(np.quantile(absolute, .95)),
            'components': int(difference.size), 'regions': regions}


def choose_candidate(trials: list[dict]) -> dict:
    valid = [t for t in trials if t.get('passed') and isinstance(t.get('training_rmse'), (int, float))
             and math.isfinite(t['training_rmse'])]
    if not valid:
        raise ValueError('no feasible native calibration trial')
    # Stable order preserves the product default when objectives are identical.
    return min(valid, key=lambda row: row['training_rmse'])


def prepare_renderer(research: Path, output: Path):
    revision = run(['git', '-C', research, 'rev-parse', 'HEAD']).stdout.decode().strip()
    if revision != PINNED_RESEARCH_REVISION:
        raise ValueError('native edit renderer source revision differs')
    original = research / 'consumer-model/harness/native_filter_probe.m'
    source = original.read_text()
    anchor = '[filter setValue:@(tone) forKey:@"inputToneBias"];facts[@"tone_bias"]=@(tone);'
    if source.count(anchor) != 1:
        raise ValueError('native edit control patch context differs')
    source = source.replace(anchor, anchor + '''
        double color=[NSProcessInfo.processInfo.environment[@"XD_COLOR_BIAS"] doubleValue];
        BOOL colorSupported=[filter.inputKeys containsObject:@"inputColorBias"];
        facts[@"color_bias_supported"]=@(colorSupported);facts[@"color_bias"]=@(color);
        if(colorSupported) [filter setValue:@(color) forKey:@"inputColorBias"];
        else if(color!=0) @throw [NSException exceptionWithName:@"ColorControlUnavailable" reason:@"native filter has no inputColorBias" userInfo:nil];
''')
    owned = output / 'edit-response.m'
    owned.write_text(source)
    flags = ['xcrun', 'clang', '-O1', '-fobjc-arc']
    for framework in ('Foundation', 'CoreGraphics', 'CoreImage', 'CoreMedia', 'ImageIO', 'Metal'):
        flags += ['-framework', framework]
    binary = output / 'edit-response'
    run([*flags, owned, '-o', binary])
    product = Path(__file__).resolve().parents[2]
    reverse_source = product / 'Sources/XDStylesNative/Styles.m'
    reverse_probe = Path(__file__).with_name('reverse_probe.m')
    reverse = output / 'reverse-response'
    run([*flags, '-fblocks', '-I', product / 'Sources/XDStylesNative/include',
         reverse_source, reverse_probe, '-o', reverse])
    compatibility, parser_facts = prepare_consumer_compatibility(research, output)
    return binary, reverse, compatibility, {'research_revision': revision,
        'upstream_probe_sha256': isolate.sha(original.read_bytes()),
        'owned_probe_sha256': isolate.sha(owned.read_bytes()), 'parser': parser_facts,
        'reverse_source_sha256': isolate.sha(reverse_source.read_bytes()),
        'reverse_probe_sha256': isolate.sha(reverse_probe.read_bytes()),
        'reverse_assertion_policy': 'diagnostic exception containment; any native assertion remains failure',
        'scope': 'native PISemanticStyleFilter output; no reconstructed Metal consumer model'}


def extract(compatibility: Path, photo: Path, output: Path):
    run([sys.executable, compatibility / 'research/vm-native/extract_inputs.py', photo, output])


def responses(adapter: Path, renderer: Path, compatibility: Path, source: Path, root: Path,
              *, color_supported=None, observations=None):
    resources = root / 'resources'
    extract(compatibility, source, resources)
    outputs, control_facts = {}, []
    if observations is None:
        observations = {}
    observations.update({'controls': control_facts, 'remove_style_executed': False})
    controls = [(tone, 0.0) for tone in TONES]
    # Probe real input keys; an unavailable Color axis is reported, never synthesized.
    index = 0
    while index < len(controls):
        tone, color = controls[index]
        directory = root / f'edit-{index}'
        run([renderer, source, resources, directory], env={**os.environ,
            'XD_TONE_BIAS': str(tone), 'XD_COLOR_BIAS': str(color), 'XD_TONE_MAX_EDGE': '512',
            'XD_TONE_THUMB_MODE': 'native-stage-mirror'})
        facts = json.loads((directory / 'facts.json').read_text())
        if facts.get('thumbnail_mode') != 'native-stage-mirror':
            raise ValueError('native response did not decode Apple Log and thumbnail gain')
        if facts.get('exception') or not any(t.get('use_style_engine') is False
                and t.get('output', {}).get('nonfinite') == 0 for t in facts.get('trials', [])):
            raise ValueError('native renderer failed: ' + str(facts.get('exception', facts.get('trials'))))
        available = facts.get('color_bias_supported') is True
        observations['native_input_keys'] = sorted(facts.get('defaults', {}))
        observations['color_supported'] = available
        if color_supported is not None and available != color_supported:
            raise ValueError('native Color capability changed between reference and rebuilt file')
        if index == 0:
            color_supported = available
            if available:
                controls.extend([(0.0, -0.5), (0.0, 0.5)])
        key = f'tone={tone:g},color={color:g}'
        outputs[key] = native_rgb(directory / 'renderer.f32')
        control_facts.append({'tone': tone, 'color': color, 'native_color_control': available,
                              'thumbnail_mode': facts['thumbnail_mode']})
        index += 1
    neutral = outputs['tone=0,color=0']
    observations['response_from_neutral'] = {key: score(neutral, value) for key, value in outputs.items()}
    facts = primitive(adapter, 'styles-decode-reverse', source, resources,
                      max_edge=512, supported_metadata_versions=[8, 9, 15, 16])
    width, height = facts['unstyled']['width'], facts['unstyled']['height']
    raw = np.fromfile(resources / 'unstyled.f32', dtype='<f4')
    if raw.size != width * height * 4 or not np.isfinite(raw).all():
        raise ValueError('invalid native remove-style result')
    outputs['remove-style'] = raw.reshape(height, width, 4)[..., :3].copy()
    observations['remove_style_executed'] = True
    return outputs, observations


def regenerate(generator: Path, adapter: Path, source: Path, output: Path, headroom: float, calibration: dict,
               *, verify_pixels=False):
    request = {**calibration, 'adapter': str(adapter), 'input': str(source),
               'output': str(output), 'maximum_stops': math.log2(headroom)}
    path = output.with_suffix('.request.json')
    path.write_text(json.dumps(request))
    result = run([generator, path], timeout=900)
    receipt = json.loads(result.stdout)
    if receipt.get('source_preserved') is not True or receipt.get('input_sha256') != isolate.sha(source.read_bytes()):
        raise ValueError('resource producer did not preserve its blind input')
    before, after = isolate.Graph(source.read_bytes()), isolate.Graph(output.read_bytes())
    if before.image_fingerprint(before.primary) != after.image_fingerprint(after.primary):
        raise ValueError('resource regeneration altered the coded primary')
    preservation = None
    if verify_pixels:
        width, height = native.display_geometry(before)
        with tempfile.TemporaryDirectory(prefix='base-check-', dir=output.parent) as temporary:
            work = Path(temporary)
            left = native.readback(adapter, source, work / 'input', width, height)
            right = native.readback(adapter, output, work / 'rebuilt', width, height)
            comparisons = {kind: native.compare_buffers(work / 'input' / f'published_{kind}.f32',
                work / 'rebuilt' / f'published_{kind}.f32', width, height) for kind in ('sdr', 'hdr')}
            if left['hdr_content_headroom'] != right['hdr_content_headroom'] or not all(c['passed'] for c in comparisons.values()):
                raise ValueError('resource assembly changed native SDR/HDR base pixels: ' + json.dumps(comparisons))
            preservation = {'headroom': right['hdr_content_headroom'], 'comparisons': comparisons}
    return {'style': corpus.inspect_bytes(output.read_bytes())['style'],
            'native_base_preservation': preservation}


def compare_responses(reference, actual):
    if reference.keys() != actual.keys():
        raise ValueError('reference and regenerated files did not exercise identical native edits')
    return {key: score(reference[key], actual[key]) for key in reference}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--adapter', type=Path, required=True)
    parser.add_argument('--generator', type=Path, required=True)
    parser.add_argument('--research-root', type=Path, required=True)
    parser.add_argument('--calibration', type=Path, action='append', required=True)
    parser.add_argument('--holdout', type=Path, action='append', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args(argv)
    adapter, generator = args.adapter.resolve(), args.generator.resolve()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = {'schema': 'styles-blind-joint-calibration-v1', 'passed': False, 'trials': [],
              'samples': [], 'selected': None,
              'scope': 'same-photo native edit equivalence; public semantic-Standard controls; no Photos device claim'}
    try:
        report['host'] = measurement_host(adapter)
        report['experiment_sha256'] = isolate.sha(Path(__file__).read_bytes())
        with tempfile.TemporaryDirectory(prefix='blind-fit-', dir=output) as temporary:
            work = Path(temporary)
            renderer, reverse, compatibility, report['renderer'] = prepare_renderer(args.research_root.resolve(), work)
            cases, identities = [], set()
            # This scene assignment is fixed before any native response is evaluated.
            for split, paths in [('calibration', args.calibration), ('holdout', args.holdout)]:
                for path in paths:
                    source = path.resolve()
                    facts = corpus.inspect_bytes(source.read_bytes())
                    identity = facts['source_sha256']
                    if identity in identities or facts['standard_semantic_state'] is not True:
                        raise ValueError('duplicate or non-Standard semantic reference')
                    identities.add(identity)
                    texture = facts['texture']
                    if texture and texture['preset'] != 'Standard' and texture['people']:
                        raise ValueError('non-Standard active Texture could confound the reference')
                    root = work / identity
                    root.mkdir()
                    equivalence = native.check_case(adapter, source, root)
                    sample = {'sample_id': identity, 'name': source.name, 'split': split,
                              'semantic_standard': True, 'texture_reference': texture,
                              'native_equivalence': equivalence,
                              'prior_research_exposure': 'published controls; not a new unseen dataset'}
                    report['samples'].append(sample)
                    if not equivalence['passed']:
                        raise ValueError(source.name + ': blind input failed full native SDR/HDR equivalence')
                    isolate.write_case(source, root / 'producer')
                    cases.append({'root': root, 'source': source, 'input': root / 'producer/input.heic',
                                  'sample': sample, 'headroom': equivalence['reference_headroom']})
            report['split_frozen'] = True
            # Only calibration responses are computed before selection.
            for case in cases:
                if case['sample']['split'] == 'calibration':
                    refs = case['root'] / 'reference'
                    refs.mkdir()
                    case['sample']['edit_controls'] = {}
                    case['reference'], case['sample']['edit_controls'] = responses(reverse, renderer,
                        compatibility, case['source'], refs, observations=case['sample']['edit_controls'])
            calibrations = [DEFAULT, *({'baseline_scale': b, 'gain_scale': g, 'baseline_maximum': 64.0}
                for b, g in itertools.product((.75, 1.0, 1.25), repeat=2))]
            for index, calibration in enumerate(calibrations):
                trial = {'index': index, 'calibration': calibration, 'passed': False, 'samples': []}
                try:
                    for case in cases:
                        if case['sample']['split'] != 'calibration':
                            continue
                        with tempfile.TemporaryDirectory(prefix=f'trial-{index}-', dir=case['root']) as directory:
                            directory = Path(directory)
                            rebuilt = directory / 'rebuilt.heic'
                            metadata = regenerate(generator, adapter, case['input'], rebuilt, case['headroom'], calibration,
                                                  verify_pixels=index == 0)
                            render = directory / 'response'
                            render.mkdir()
                            actual, controls = responses(reverse, renderer, compatibility, rebuilt, render,
                                color_supported=case['sample']['edit_controls']['color_supported'])
                            metrics = compare_responses(case['reference'], actual)
                            trial['samples'].append({'sample_id': case['sample']['sample_id'],
                                'metrics': metrics, 'generated_metadata': metadata, 'controls': controls})
                    trial['training_rmse'] = float(np.mean([m['rmse'] for c in trial['samples'] for m in c['metrics'].values()]))
                    trial['passed'] = True
                except (ValueError, RuntimeError, OSError, KeyError, subprocess.TimeoutExpired) as error:
                    trial['error'] = str(error)
                report['trials'].append(trial)
                (output / 'report.json').write_text(json.dumps(report, indent=2, allow_nan=False) + '\n')
                if index == 0 and not trial['passed']:
                    raise ValueError('product default failed; alternative fits cannot waive that failure: '
                                     + trial['error'])
            selected = choose_candidate(report['trials'])
            report['selected'] = selected['calibration']
            report['training_default_rmse'] = report['trials'][0]['training_rmse']
            report['training_selected_rmse'] = selected['training_rmse']
            holdout = []
            for case in cases:
                if case['sample']['split'] != 'holdout':
                    continue
                refs = case['root'] / 'reference'
                refs.mkdir()
                reference, controls = responses(reverse, renderer, compatibility, case['source'], refs)
                item = {'sample_id': case['sample']['sample_id'], 'controls': controls}
                for label, calibration in [('default', DEFAULT), ('selected', selected['calibration'])]:
                    directory = case['root'] / label
                    directory.mkdir()
                    rebuilt = directory / 'rebuilt.heic'
                    metadata = regenerate(generator, adapter, case['input'], rebuilt, case['headroom'], calibration,
                                          verify_pixels=True)
                    render = directory / 'response'
                    render.mkdir()
                    actual, _ = responses(reverse, renderer, compatibility, rebuilt, render,
                                          color_supported=controls['color_supported'])
                    item[label] = {'metrics': compare_responses(reference, actual), 'generated_metadata': metadata}
                holdout.append(item)
            report['holdout'] = holdout
            report['passed'] = bool(holdout) and len([t for t in report['trials'] if t['passed']]) >= 2
            report['product_defaults_changed'] = False
    except (ValueError, OSError, RuntimeError, KeyError, subprocess.TimeoutExpired) as error:
        report['error'] = str(error)
    finally:
        (output / 'report.json').write_text(json.dumps(report, indent=2, allow_nan=False) + '\n')
    print(json.dumps(report, indent=2))
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
