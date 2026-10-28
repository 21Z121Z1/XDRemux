"""Verify that a blind projection preserves Apple's full SDR and HDR readback.

Pixel buffers remain in an owned temporary directory. The exported report has
hashes, dimensions and numeric comparisons, never pixels or source metadata.
"""
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
import platform
import subprocess
import tempfile

import numpy as np

from . import isolate


def display_geometry(graph: isolate.Graph) -> tuple[int, int]:
    sizes = [p for p in graph.props(graph.primary) if p['kind'] == 'ispe']
    if len(sizes) != 1 or len(sizes[0]['raw']) != 20:
        raise ValueError('one valid primary ispe is required')
    raw = sizes[0]['raw']
    width, height = int.from_bytes(raw[12:16], 'big'), int.from_bytes(raw[16:20], 'big')
    for prop in graph.props(graph.primary):
        if prop['kind'] == 'clap':
            raise ValueError('clean-aperture native geometry is not yet supported by this experiment')
        if prop['kind'] == 'irot':
            if len(prop['raw']) != 9 or prop['raw'][8] & ~3:
                raise ValueError('invalid primary rotation')
            if prop['raw'][8] & 1:
                width, height = height, width
    if width <= 0 or height <= 0 or width * height > 100_000_000:
        raise ValueError('invalid native display geometry')
    return width, height


def compare_buffers(reference: Path, candidate: Path, width: int, height: int) -> dict:
    expected = width * height * 16
    if reference.stat().st_size != expected or candidate.stat().st_size != expected:
        raise ValueError('native readback buffer length differs from declared RGBA32F geometry')
    left = np.memmap(reference, dtype='<f4', mode='r', shape=(height, width, 4))
    right = np.memmap(candidate, dtype='<f4', mode='r', shape=(height, width, 4))
    maximum = 0.0
    total = 0.0
    differing = 0
    nonfinite = 0
    for start in range(0, height, 128):
        a, b = left[start:start + 128], right[start:start + 128]
        finite = np.isfinite(a) & np.isfinite(b)
        nonfinite += int(np.size(finite) - np.count_nonzero(finite))
        if not finite.all():
            continue
        difference = np.abs(a.astype('f8') - b.astype('f8'))
        maximum = max(maximum, float(difference.max()))
        total += float(difference.sum())
        differing += int(np.count_nonzero(a.view('u4') != b.view('u4')))
    del left, right
    return {'components': width * height * 4, 'different_float32_components': differing,
            'nonfinite_components': nonfinite,
            'maximum_absolute_error': maximum if nonfinite == 0 else None,
            'mean_absolute_error': total / (width * height * 4) if nonfinite == 0 else None,
            'passed': nonfinite == 0 and maximum <= 1e-6}


def readback(adapter: Path, source: Path, directory: Path, width: int, height: int) -> dict:
    directory.mkdir()
    request = {'schema_version': 2, 'operation': 'styles-readback', 'input_path': str(source),
               'output_path': str(directory), 'width': width, 'height': height, 'apply_orientation': True}
    process = subprocess.run([str(adapter)], input=(json.dumps(request) + '\n').encode(),
                             capture_output=True, timeout=300)
    (directory / 'stderr.log').write_bytes(process.stderr)
    if process.returncode != 0:
        raise ValueError('native readback failed with exit ' + str(process.returncode))
    facts = json.loads(process.stdout)
    if facts.get('schema_version') != 2:
        raise ValueError('native response schema differs')
    for name in ('sdr', 'hdr'):
        if facts.get(name, {}).get('width') != width or facts.get(name, {}).get('height') != height:
            raise ValueError('native readback geometry differs')
    headroom = facts.get('hdr_content_headroom')
    if isinstance(headroom, bool) or not isinstance(headroom, (float, int)) or not math.isfinite(headroom) or headroom < 1:
        raise ValueError('native HDR headroom is missing or invalid')
    return facts


def check_case(adapter: Path, source: Path, directory: Path, *, keep_numeric_hdr_exif: bool = True) -> dict:
    original = source.read_bytes()
    projection, projection_report = isolate.isolate_bytes(original, keep_numeric_hdr_exif=keep_numeric_hdr_exif)
    width, height = display_geometry(isolate.Graph(original))
    if display_geometry(isolate.Graph(projection)) != (width, height):
        raise ValueError('projection changed display geometry')
    with tempfile.TemporaryDirectory(prefix='native-pixels-', dir=directory) as temporary:
        work = Path(temporary)
        isolated = work / 'input.heic'
        isolated.write_bytes(projection)
        before = readback(adapter, source, work / 'reference', width, height)
        after = readback(adapter, isolated, work / 'projection', width, height)
        comparisons = {name: compare_buffers(work / 'reference' / ('published_' + name + '.f32'),
                                             work / 'projection' / ('published_' + name + '.f32'), width, height)
                       for name in ('sdr', 'hdr')}
        hdr_difference = compare_buffers(work / 'reference' / 'published_sdr.f32',
                                         work / 'reference' / 'published_hdr.f32', width, height)
    unchanged = isolate.sha(source.read_bytes()) == isolate.sha(original)
    headroom_matches = before['hdr_content_headroom'] == after['hdr_content_headroom']
    hdr_exercised = (before['hdr_content_headroom'] > 1
                     and hdr_difference['nonfinite_components'] == 0
                     and hdr_difference['maximum_absolute_error'] > 1e-6)
    passed = unchanged and headroom_matches and hdr_exercised and all(v['passed'] for v in comparisons.values())
    return {**projection_report, 'width': width, 'height': height,
            'source_preserved': unchanged, 'native_headroom_preserved': headroom_matches,
            'reference_headroom': before['hdr_content_headroom'],
            'reference_hdr_expansion_exercised': hdr_exercised,
            'projection_headroom': after['hdr_content_headroom'], 'comparisons': comparisons,
            'native_hdr_equivalence_verified': passed, 'passed': passed}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--adapter', type=Path, required=True)
    parser.add_argument('--input', type=Path, action='append', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--ablate-hdr-exif', action='store_true',
                        help='also measure a stricter projection without numeric HDR EXIF')
    args = parser.parse_args(argv)
    if platform.system() != 'Darwin' or platform.machine() != 'arm64':
        parser.error('this measurement requires the supported arm64/macOS Apple adapter')
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = {'schema': 'styles-blind-native-equivalence-v1',
              'host': {'system': platform.system(), 'release': platform.release(), 'architecture': platform.machine()},
              'adapter_sha256': isolate.sha(args.adapter.read_bytes()), 'cases': [], 'passed': False,
              'scope': 'Source isolation only; no reconstruction, slider fidelity or Photos device claim.'}
    try:
        for source in args.input:
            try:
                case = check_case(args.adapter.resolve(), source.resolve(), output)
                if args.ablate_hdr_exif:
                    case['without_hdr_exif'] = check_case(args.adapter.resolve(), source.resolve(), output,
                                                          keep_numeric_hdr_exif=False)
                report['cases'].append(case)
            except (ValueError, OSError, subprocess.TimeoutExpired) as error:
                report['cases'].append({'source_sha256': isolate.sha(source.read_bytes()) if source.is_file() else None,
                                        'passed': False, 'error': type(error).__name__ + ': ' + str(error)})
        report['passed'] = bool(report['cases']) and all(case['passed'] for case in report['cases'])
    finally:
        (output / 'report.json').write_text(json.dumps(report, indent=2, allow_nan=False) + '\n')
    print(json.dumps(report, indent=2))
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
