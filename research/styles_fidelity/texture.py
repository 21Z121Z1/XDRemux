"""Replay native Texture effects using resources decoded from a final HDR file."""
from __future__ import annotations

import argparse
import copy
import importlib.util
import itertools
import json
import math
import os
from pathlib import Path
import platform
import subprocess
import sys
import tempfile

import numpy as np

from .isolate import Graph, TEXTURE_URI, sha
from .native import display_geometry

REVISION = '2bc4e3d16fc2c6028cdf708f06d79cf3be42a6d5'
ROLES = {
    'skin': 'semanticskinmattev2', 'face': 'semanticfaceskinmatte',
    'instance': 'semanticpersoninstances', 'person': 'semanticpersonmatte',
    'hair': 'semantichairmatte', 'glasses': 'semanticglassesmattev2',
    'teeth': 'semanticteethmattev2', 'nose': 'semanticnosematte', 'lips': 'semanticlipsmatte',
    'eyebrows': 'semanticeyebrowsmatte', 'body': 'semanticnonfaceskinmatte',
    'hands': 'semantichandsmatte', 'ears': 'semanticearsmatte', 'tattoo': 'semantictattoomatte',
}
EFFECTS = ('Mattify', 'UnderEyeBrighten', 'SkinSmoothStandalone')


def area_axis(values, count, axis):
    """Integrate the same piecewise-constant pixel footprint on one axis."""
    values = np.moveaxis(np.asarray(values, dtype=np.float64), axis, 0)
    length = len(values)
    edges = np.linspace(0, length, count + 1)
    indices = np.minimum(edges.astype(np.int64), length - 1)
    fractions = edges - indices
    cumulative = np.concatenate((np.zeros_like(values[:1]), np.cumsum(values, axis=0)))
    integral = cumulative[indices] + fractions.reshape((-1,) + (1,) * (values.ndim - 1)) * values[indices]
    return np.moveaxis(np.diff(integral, axis=0) / (length / count), 0, axis)


def area_rgba(source, width, height):
    intermediate = np.empty((source.shape[0], width, 4), dtype=np.float32)
    for row in range(0, source.shape[0], 128):
        intermediate[row:row + 128] = area_axis(source[row:row + 128], width, 1)
    result = np.empty((height, width, 4), dtype=np.float32)
    for column in range(0, width, 128):
        result[:, column:column + 128] = area_axis(intermediate[:, column:column + 128], height, 0)
    if not np.isfinite(result).all():
        raise ValueError('nonfinite native HDR working pixels')
    return result


def run(command, *, env=None, timeout=300, diagnostic=None):
    result = subprocess.run([str(v) for v in command], capture_output=True, timeout=timeout, env=env)
    if result.returncode:
        details = {}
        if diagnostic is not None and diagnostic.is_file():
            try:
                report = json.loads(diagnostic.read_text())
                keys = ('stage', 'exception', 'producer_effect', 'person_index', 'native_effect_name',
                        'setup', 'prewarm', 'prepareToProcess:', 'calculateStats', 'process',
                        'finishProcessing', 'native_call_order', 'image_pixels', 'native_GPU_execution')
                details = {key: report[key] for key in keys if key in report}
            except (ValueError, OSError) as error:
                details = {'report_read_error': str(error)}
        raise RuntimeError(f'{Path(command[0]).name} exited {result.returncode}: '
                           + result.stderr.decode(errors='replace')[-1600:]
                           + '; stdout: ' + result.stdout.decode(errors='replace')[-1600:]
                           + '; native report: ' + json.dumps(details, allow_nan=False))
    return result


def primitive(adapter: Path, operation: str, source: Path, output: Path, **configuration):
    request = {'schema_version': 2, 'operation': operation, 'input_path': str(source),
               'output_path': str(output), **configuration}
    result = subprocess.run([str(adapter)], input=(json.dumps(request) + '\n').encode(),
                            capture_output=True, timeout=300)
    if result.returncode:
        raise RuntimeError(f'{operation} exited {result.returncode}: '
                           + result.stderr.decode(errors='replace')[-3000:]
                           + '; stdout: ' + result.stdout.decode(errors='replace')[-1600:])
    return json.loads(result.stdout)


def measurement_host(adapter: Path):
    facts = {'system': platform.system(), 'release': platform.release(),
             'architecture': platform.machine(), 'adapter_sha256': sha(adapter.read_bytes())}
    if facts['system'] == 'Darwin':
        facts['os_build'] = run(['sw_vers', '-buildVersion']).stdout.decode().strip()
        facts['sdk_version'] = run(['xcrun', '--show-sdk-version']).stdout.decode().strip()
    return facts


def pixel_person(person: dict, width: int, height: int) -> dict:
    value = copy.deepcopy(person)
    for key in ('faceROI', 'faceSkinROI', 'instanceROI'):
        for component, scale in (('x', width), ('width', width), ('y', height), ('height', height)):
            number = value[key][component]
            if isinstance(number, bool) or not isinstance(number, (int, float)) or not math.isfinite(number):
                raise ValueError('invalid final-file person rectangle')
            value[key][component] *= scale
    for landmark in value['faceLandmarks']:
        for axis, scale in (('x', width), ('y', height)):
            number = landmark['point'][axis]
            if not isinstance(number, (int, float)) or not math.isfinite(number):
                raise ValueError('invalid final-file landmark')
            landmark['point'][axis] *= scale
    value.pop('faceROIAndLandmarksROIRelativeScalingROI', None)
    return value


def numeric_leaves(value, prefix=''):
    result = {}
    if isinstance(value, dict):
        for key, child in value.items():
            result.update(numeric_leaves(child, prefix + '/' + str(key)))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            result.update(numeric_leaves(child, prefix + '/' + str(index)))
    elif isinstance(value, (int, float)) and not isinstance(value, bool):
        if not math.isfinite(value):
            raise ValueError('nonfinite native statistic')
        result[prefix] = value
    return result


def compare_statistics(person: dict, facts: dict) -> dict:
    group = facts['native_effect_name']
    reference = numeric_leaves(person.get('imageStats', {}).get(group, {}))
    replayed = numeric_leaves(facts.get('native_output_person_stats', {}))
    common = reference.keys() & replayed.keys()
    errors = [abs(reference[k] - replayed[k]) for k in common]
    return {'statistics_group': group, 'statistics_compared_fields': len(errors),
            'statistics_maximum_absolute_difference': max(errors) if errors else None,
            'statistics_mean_absolute_difference': sum(errors) / len(errors) if errors else None,
            'statistics_field_differences': {k: replayed[k] - reference[k] for k in sorted(common)},
            'stored_only_statistics_fields': len(reference.keys() - replayed.keys()),
            'replayed_only_statistics_fields': len(replayed.keys() - reference.keys())}


def prepare_tools(research: Path, output: Path):
    revision = run(['git', '-C', research, 'rev-parse', 'HEAD']).stdout.decode().strip()
    root = research / 'research/texture-producer-20261020'
    if revision != REVISION or run(['git', '-C', research, 'status', '--porcelain', '--', root]).stdout:
        raise ValueError('Texture replay requires the unmodified pinned research sources')
    sys.path.insert(0, str(root))
    spec = importlib.util.spec_from_file_location('final_texture_projection', root / 'project_final.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    source = (root / 'generate_stats.m').read_text()
    before = 'if (storedPerson) [nativeStats addEntriesFromDictionary:storedPerson[@"imageStats"][@"Mattify"]];'
    condition = ('if ([effectKind isEqual:@"Mattify"] && [parameterSource isEqual:@"Studio"] '
                 '&& exposure == 0 && ![report[@"calculateStats"] intValue])')
    if source.count(before) != 1 or source.count(condition) != 1:
        raise ValueError('pinned Texture replay patch context changed')
    source = source.replace(before, '').replace(condition,
        'if ([parameterSource isEqual:@"Studio"] && exposure == 0 && ![report[@"calculateStats"] intValue])')
    tuning_anchor = 'report[@"native_effect_type"] = Call(effectDescriptor, @"type", @[]);'
    if source.count(tuning_anchor) != 1:
        raise ValueError('pinned effect identity context changed')
    source = source.replace(tuning_anchor, tuning_anchor + '''
                id statisticsGroup=Call(NSClassFromString(@"CMITextureStylesProcessor"),@"effectTypeToEffectName:",@[report[@"native_effect_type"]]);
                report[@"native_effect_name"]=statisticsGroup;
                NSDictionary *existing=storedPerson[@"imageStats"][statisticsGroup];
                if(existing && ![NSProcessInfo.processInfo.environment[@"TEXTURE_RECOMPUTE_FINAL_STATS"] boolValue])
                    [nativeStats addEntriesFromDictionary:existing];
''')
    render_control = 'Call(effect, @"setSkipRendering:", @[@NO]);'
    if source.count(render_control) != 1:
        raise ValueError('pinned render control context changed')
    source = source.replace(render_control, '''
                    BOOL hasSkipControl=[effect respondsToSelector:NSSelectorFromString(@"setSkipRendering:")];
                    report[@"render_skip_control_present"]=@(hasSkipControl);
                    if(hasSkipControl) Call(effect,@"setSkipRendering:",@[@NO]);
''')
    person_anchor = 'NSDictionary *personDictionary = storedPerson ?: selected[@"native_person_without_stats"];'
    if source.count(person_anchor) != 1:
        raise ValueError('pinned person input context changed')
    source = source.replace(person_anchor, person_anchor + '''
            if([NSProcessInfo.processInfo.environment[@"TEXTURE_RECOMPUTE_FINAL_STATS"] boolValue]) {
                NSMutableDictionary *geometry=[personDictionary mutableCopy];geometry[@"imageStats"]=@{};
                personDictionary=geometry;
                report[@"provided_precomputed_image_stats_from_final_HEIC"]=@NO;
            }
            report[@"geometry_from_final_HEIC"]=@YES;
''')
    upload = '[command commit]; [command waitUntilCompleted];'
    if source.count(upload) != 1:
        raise ValueError('pinned working image upload context changed')
    source = source.replace(upload, upload + '''
            NSString *linearPath=NSProcessInfo.processInfo.environment[@"TEXTURE_FINAL_LINEAR_RGBA16F"];
            if(linearPath) {
                NSData *pixels=[NSData dataWithContentsOfFile:linearPath];
                if(pixels.length!=width*height*8) @throw [NSException exceptionWithName:@"LinearInput" reason:@"geometry mismatch" userInfo:nil];
                [inputTexture replaceRegion:MTLRegionMake2D(0,0,width,height) mipmapLevel:0 withBytes:pixels.bytes bytesPerRow:width*8];
                [outputTexture replaceRegion:MTLRegionMake2D(0,0,width,height) mipmapLevel:0 withBytes:pixels.bytes bytesPerRow:width*8];
                report[@"color_domain"]=@"native final-file linear Display P3 HDR, area reduced, RGBA16F";
            }
''')
    anchor = 'const __fp16 *renderedValues = renderedPixels.bytes;'
    if source.count(anchor) != 1:
        raise ValueError('pinned output capture context changed')
    source = source.replace(anchor, anchor + '\n                    [renderedPixels writeToFile:[out stringByAppendingString:@".rgba16f"] atomically:YES];')
    owned = output / 'replay.m'
    owned.write_text(source)
    flags = ['xcrun', 'clang', '-O1', '-fobjc-arc', '-fblocks', '-I', root]
    for framework in ('Foundation', 'CoreGraphics', 'CoreImage', 'CoreVideo', 'ImageIO', 'Vision', 'Metal', 'MetalKit'):
        flags += ['-framework', framework]
    run([*flags, root / 'decode_final_masks.m', '-o', output / 'decode-masks'])
    run([*flags, owned, '-o', output / 'replay'])
    return module, {'research_revision': revision, 'upstream_replay_sha256': sha((root / 'generate_stats.m').read_bytes()),
                    'owned_replay_sha256': sha(owned.read_bytes()),
                    'patch_scope': 'select stored statistics per effect; render all three; optionally upload measured HDR pixels; retain output pixels locally'}


def replay_case(adapter: Path, source: Path, tools: Path, projector, directory: Path):
    original = source.read_bytes()
    graph = Graph(original)
    if len(graph.metadata_items(TEXTURE_URI)) != 1:
        raise ValueError('final product has no unique Texture metadata')
    directory.mkdir()
    admission_path = directory / 'admission.json'
    primitive(adapter, 'texture-readback', source, admission_path)
    admission = json.loads(admission_path.read_text())
    people = admission.get('native_person_data_readback', [])
    if not people or admission.get('texture_person_count') != len(people):
        raise ValueError('portrait replay requires final-file native people: ' + json.dumps({
            'native_people': len(people), 'texture_person_count': admission.get('texture_person_count'),
            'asset_loaded': admission.get('asset_loaded'),
            'texture_properties_present': admission.get('texture_properties_present')}))
    primitive(adapter, 'texture-prepare', source, directory,
              texture={'max_dimension': 1024})
    prepared = json.loads((directory / 'input.json').read_text())
    width, height = prepared['working_width'], prepared['working_height']
    for index, person in enumerate(people):
        (directory / f'stored-person-{index}-pixels.json').write_text(
            json.dumps(pixel_person(person, width, height)) + '\n')
    projections = []
    for role, suffix in ROLES.items():
        uri = ('urn:com:apple:photo:2019:aux:' if role == 'hair' else 'tag:apple.com,2026:photo:aux:') + suffix
        payload, facts = projector.project(original, uri, people[0]['instanceMaskReferenceKey'] if role == 'instance' else None)
        (directory / f'stored-{role}.heic').write_bytes(payload)
        projections.append({**facts, 'role': role})
    for index, person in enumerate(people):
        payload, facts = projector.project(original, 'tag:apple.com,2026:photo:aux:semanticpersoninstances', person['instanceMaskReferenceKey'])
        role = f'instance-{index}'
        (directory / f'stored-{role}.heic').write_bytes(payload)
        projections.append({**facts, 'role': role})
    (directory / 'final-mask-projections.json').write_text(json.dumps({'resources': projections}))
    run([tools / 'decode-masks', directory])
    decoded = json.loads((directory / 'final-mask-decode.json').read_text())
    if len(decoded.get('rows', [])) != len(projections) or any(
            (r['width'], r['height']) != (width, height) for r in decoded['rows']):
        raise ValueError('encoded final masks differ from the working geometry; no resize fallback')
    hdr_directory = directory / 'hdr'
    hdr_directory.mkdir()
    native_width, native_height = display_geometry(graph)
    hdr_facts = primitive(adapter, 'styles-readback', source, hdr_directory,
                          width=native_width, height=native_height, apply_orientation=True)
    native_hdr = np.memmap(hdr_directory / 'published_hdr.f32', dtype='<f4', mode='r',
                           shape=(native_height, native_width, 4))
    hdr_working = area_rgba(native_hdr, width, height)
    del native_hdr
    peak = float(hdr_working[..., :3].max())
    if hdr_facts['hdr_content_headroom'] <= 1 or peak <= 1:
        raise ValueError('final input did not exercise values above SDR white')
    hdr_pixels = directory / 'native-hdr.rgba16f'
    hdr_pixels.write_bytes(hdr_working.astype('<f2').tobytes())
    del hdr_working
    effects, comparisons, captured = [], [], {}
    for index, person in enumerate(people):
        for effect, domain, mode in itertools.product(EFFECTS, ('sdr-analysis', 'hdr-linear'), ('stored', 'recomputed')):
            output = directory / f'person-{index}-{effect}-{domain}-{mode}.json'
            environment = {**os.environ, 'TEXTURE_REPLAY_DIR': str(directory),
                           'TEXTURE_PERSON_INDEX': str(index), 'TEXTURE_EFFECT': effect}
            environment['TEXTURE_RECOMPUTE_FINAL_STATS'] = '1' if mode == 'recomputed' else '0'
            if domain == 'hdr-linear':
                environment['TEXTURE_FINAL_LINEAR_RGBA16F'] = str(hdr_pixels)
            else:
                environment.pop('TEXTURE_FINAL_LINEAR_RGBA16F', None)
            run([tools / 'replay', directory / 'working.png', output, '1', 'Studio', '0'],
                env=environment, diagnostic=output)
            facts = json.loads(output.read_text())
            rendered = facts.get('rendered_pixels', {})
            passed = (facts.get('native_statistics_operations_completed') is True
                and facts.get('render_command_status') == 4
                and facts.get('render_prepare_status') == 0
                and facts.get('render_calculate_stats_status') == 0
                and facts.get('render_process_status') == 0
                and rendered.get('nonfinite') == 0
                and not facts.get('native_GPU_execution', {}).get('failures')
                and facts.get('provided_precomputed_image_stats_from_final_HEIC') == (mode == 'stored')
                and facts.get('geometry_from_final_HEIC') is True
                and facts.get('fresh_Vision_observations_used_as_replay_inputs') is False)
            pixels = np.fromfile(output.with_name(output.name + '.rgba16f'), dtype='<f2')
            if pixels.size != width * height * 4 or not np.isfinite(pixels).all():
                raise ValueError('native effect output does not match its finite RGBA16F layout')
            pixels = pixels.reshape(height, width, 4)[..., :3].astype('f4')
            key = (index, effect, domain)
            if mode == 'stored':
                captured[key] = pixels
            else:
                difference = pixels - captured.pop(key)
                comparisons.append({'person': index, 'effect': effect, 'domain': domain,
                    'recomputed_vs_stored_rmse': float(np.sqrt(np.mean(difference.astype('f8') ** 2))),
                    'recomputed_vs_stored_maximum_absolute': float(np.abs(difference).max()),
                    'changed_rgb_components': int(np.count_nonzero(difference))})
            effects.append({'person': index, 'effect': effect, 'domain': domain, 'statistics_mode': mode, 'passed': passed,
                'rendered_pixels': rendered, 'gpu_command_status': facts.get('render_command_status'),
                **compare_statistics(person, facts),
                'native_parameters': facts.get('native_parameters'),
                'exception': facts.get('exception')})
    unchanged = sha(source.read_bytes()) == sha(original)
    return {'input_sha256': sha(original), 'source_preserved': unchanged,
            'hdr_headroom': hdr_facts['hdr_content_headroom'], 'people': len(people),
            'working_hdr_maximum': peak,
            'working_width': width, 'working_height': height,
            'domain': '1024-edge SDR analysis and native linear HDR, with final-file encoded masks and person data',
            'mask_resources': decoded['rows'], 'effects': effects, 'comparisons': comparisons,
            'scope': 'per-person native effect replay; no Photos UI or full HDR effect-composition claim',
            'passed': unchanged and all(e['passed'] for e in effects)}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--adapter', type=Path, required=True)
    parser.add_argument('--research-root', type=Path, required=True)
    parser.add_argument('--input', type=Path, action='append', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args(argv)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = {'schema': 'final-hdr-texture-replay-v1', 'cases': [], 'passed': False}
    try:
        report['host'] = measurement_host(args.adapter.resolve())
        report['experiment_sha256'] = sha(Path(__file__).read_bytes())
        with tempfile.TemporaryDirectory(prefix='texture-replay-', dir=output) as temporary:
            tools = Path(temporary)
            projector, report['producer'] = prepare_tools(args.research_root.resolve(), tools)
            for source in args.input:
                try:
                    report['cases'].append(replay_case(args.adapter.resolve(), source.resolve(), tools,
                        projector, tools / sha(source.read_bytes())))
                except (ValueError, OSError, RuntimeError, KeyError, subprocess.TimeoutExpired) as error:
                    report['cases'].append({'input_sha256': sha(source.read_bytes()), 'passed': False,
                                            'error': str(error)})
        report['passed'] = bool(report['cases']) and all(c['passed'] for c in report['cases'])
    finally:
        (output / 'report.json').write_text(json.dumps(report, indent=2, allow_nan=False) + '\n')
    print(json.dumps(report, indent=2))
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
