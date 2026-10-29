"""Locate Tone error by substituting reference resources in the evaluator only."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import tempfile

from . import blind, corpus, isolate, native
from .texture import measurement_host, run

MASKS = tuple(key + '.heic' for key in (
    'inputSubjectMatteImage', 'inputSkinMatteImage', 'inputSkyMatteImage', 'inputGainMapImage'))
GROUPS = {
    'linear_exposure': (('4', 'h', 'i'), ('linear.heic',)),
    'thumbnail_pixels': ((), ('linear.heic',)),
    'baseline_exposure': (('4',), ()),
    'base_gain': (('h',), ()),
    'thumbnail_encoding': (('i',), ()),
    'tone_curve': (('3',), ()),
    'light_maps': (('c', 'd', 'e', 'f', 'g'), ()),
    'statistics_scene': (('5', '6', '7', 'j', 'k'), ()),
    'masks': ((), MASKS),
    'metadata_mode': (('0', '2'), ()),
}


def substitute(base: Path, reference: Path, output: Path, groups):
    """Keep other generated resources intact, including the unused key-1 data."""
    shutil.copytree(base, output)
    generated = plistlib.loads((base / 'styles.plist').read_bytes())
    original = plistlib.loads((reference / 'styles.plist').read_bytes())
    for group in groups:
        keys, files = GROUPS[group]
        for key in keys:
            if key in original:
                generated[key] = original[key]
            else:
                generated.pop(key, None)
        for name in files:
            if (reference / name).is_file():
                shutil.copyfile(reference / name, output / name)
            else:
                (output / name).unlink(missing_ok=True)
    (output / 'styles.plist').write_bytes(plistlib.dumps(generated, fmt=plistlib.FMT_BINARY))


def render(renderer: Path, source: Path, resources: Path, directory: Path, tone: float):
    run([renderer, source, resources, directory], env={**os.environ,
        'XD_TONE_BIAS': str(tone), 'XD_COLOR_BIAS': '0',
        'XD_TONE_MAX_EDGE': '512', 'XD_TONE_THUMB_MODE': 'native-stage-mirror'})
    facts = json.loads((directory / 'facts.json').read_text())
    if facts.get('exception') or not any(t.get('use_style_engine') is False
            and t.get('output', {}).get('nonfinite') == 0 for t in facts.get('trials', [])):
        raise ValueError('native Tone renderer failed')
    return blind.native_rgb(directory / 'renderer.f32')


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('adapter', 'generator', 'research-root', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--input', type=Path, action='append', required=True)
    args = parser.parse_args(argv)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = {'schema': 'styles-tone-attribution-v1', 'passed': False, 'cases': [],
        'oracle_resources_used_for_diagnosis': True, 'product_candidate': False,
        'product_defaults_changed': False,
        'native_consumer_scope': 'PISemanticStyleFilter useStyleEngine=NO; not full engine or reversal',
        'groups': {k: {'metadata_keys': v[0], 'files': v[1]} for k, v in GROUPS.items()}}
    try:
        report['host'] = measurement_host(args.adapter)
        report['experiment_sha256'] = isolate.sha(Path(__file__).read_bytes())
        with tempfile.TemporaryDirectory(prefix='tone-attribution-', dir=output) as temporary:
            work = Path(temporary)
            renderer, _, compatibility, report['renderer'] = blind.prepare_renderer(args.research_root, work)
            for index, source in enumerate(args.input):
                source = source.resolve()
                identity = isolate.sha(source.read_bytes())
                if corpus.inspect_bytes(source.read_bytes())['standard_semantic_state'] is not True:
                    raise ValueError('Tone attribution requires a Standard reference')
                root = work / str(index)
                root.mkdir()
                equivalence = native.check_case(args.adapter, source, root)
                if not equivalence['passed']:
                    raise ValueError('blind source changed native SDR/HDR pixels')
                isolate.write_case(source, root / 'producer')
                rebuilt = root / 'rebuilt.heic'
                metadata = blind.regenerate(args.generator, args.adapter, root / 'producer/input.heic',
                    rebuilt, equivalence['reference_headroom'], blind.DEFAULT, verify_pixels=True)
                # Generation has ended before the evaluator can access reference resources.
                original, generated = root / 'reference', root / 'generated'
                blind.extract(compatibility, source, original)
                blind.extract(compatibility, rebuilt, generated)
                case = {'name': source.name, 'source_sha256': identity,
                        'native_equivalence': equivalence, 'generated_metadata': metadata, 'variants': {}}
                reference_metadata = plistlib.loads((original / 'styles.plist').read_bytes())
                case['reference_linear_metadata'] = {key: reference_metadata.get(key)
                                                     for key in ('4', 'h', 'i')}
                report['cases'].append(case)
                references = {tone: render(renderer, source, original, root / f'ref-{tone}', tone)
                              for tone in blind.TONES}
                # Prior measurements found no effect from the other groups on
                # these controls. Keep them in closure, not repeated singles.
                singles = ('thumbnail_pixels', 'baseline_exposure', 'base_gain',
                           'thumbnail_encoding', 'linear_exposure', 'tone_curve', 'light_maps')
                variants = [('generated', ()), *[(key, (key,)) for key in singles],
                            ('brightness_bundle', ('linear_exposure', 'tone_curve', 'light_maps')),
                            ('all_renderer_inputs', tuple(GROUPS))]
                for label, groups in variants:
                    resources = root / ('bundle-' + label)
                    substitute(generated, original, resources, groups)
                    metrics = {f'{tone:g}': blind.score(references[tone],
                        render(renderer, source, resources, root / f'{label}-{tone}', tone))
                        for tone in blind.TONES}
                    case['variants'][label] = {'metrics': metrics,
                        'mean_nonzero_tone_rmse': (metrics['-0.5']['rmse'] + metrics['0.5']['rmse']) / 2}
                    # Observe what the native consumer actually decoded, rather
                    # than inferring it from pre-encoding Rust samples.
                    facts = json.loads((root / f'{label}-0.5/facts.json').read_text())
                    case['variants'][label]['input_observation_tone'] = 0.5
                    case['variants'][label]['native_inputs'] = {
                        key: facts[key] for key in ('primary_input', 'thumbnail_input',
                                                   'native_curve_image', 'gtc_calls')}
                closure = case['variants']['all_renderer_inputs']['metrics']
                if any(m['maximum_absolute'] != 0 for m in closure.values()):
                    raise ValueError('substituted renderer inputs do not reproduce the reference; attribution incomplete')
                if isolate.sha(source.read_bytes()) != identity:
                    raise ValueError('reference source changed')
                (output / 'report.json').write_text(json.dumps(report, indent=2, allow_nan=False) + '\n')
            report['passed'] = bool(report['cases'])
    except (ValueError, RuntimeError, OSError, KeyError, subprocess.TimeoutExpired) as error:
        report['error'] = str(error)
    finally:
        (output / 'report.json').write_text(json.dumps(report, indent=2, allow_nan=False) + '\n')
    print(json.dumps(report, indent=2))
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
