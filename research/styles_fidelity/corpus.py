"""Inventory supplied archives without running their code or mixing labels into inputs."""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path
import plistlib
import struct
import zipfile

from PIL import Image

from . import isolate


def _file_sha(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()


def _plist(graph: isolate.Graph, uri: str) -> dict | None:
    items = graph.metadata_items(uri)
    if len(items) > 1:
        raise ValueError('ambiguous primary-associated ' + uri)
    if not items:
        return None
    raw = graph.payloads[items[0]]
    if len(raw) > 2 * 1024 * 1024:
        raise ValueError('oversized property list')
    value = plistlib.loads(raw)
    if not isinstance(value, dict):
        raise ValueError('metadata is not a property-list dictionary')
    return value


def _maker84(raw: bytes) -> dict | None:
    if not raw.startswith(b'Apple iOS\0\0\x01') or len(raw) < 16:
        return None
    if raw[12:14] not in (b'MM', b'II'):
        return None
    order = '>' if raw[12:14] == b'MM' else '<'
    count = struct.unpack_from(order + 'H', raw, 14)[0]
    if 16 + count * 12 + 4 > len(raw):
        raise ValueError('truncated Apple MakerNote')
    selected = []
    for index in range(count):
        pos = 16 + index * 12
        tag, kind, length, offset = struct.unpack_from(order + 'HHII', raw, pos)
        if tag != 84:
            continue
        if kind != 7 or offset + length > len(raw) or length > 1024 * 1024:
            raise ValueError('invalid SmartStyle MakerNote storage')
        value = plistlib.loads(raw[offset:offset + length])
        if not isinstance(value, dict):
            raise ValueError('SmartStyle state is not a dictionary')
        selected.append({k: v for k, v in value.items() if k in {str(i) for i in range(13)}
                         and isinstance(v, (int, float, bool, str))})
    if len(selected) > 1:
        raise ValueError('ambiguous SmartStyle MakerNote')
    return selected[0] if selected else None


def _exif(graph: isolate.Graph) -> dict:
    items = [i for i, ident in graph.identities.items() if ident['kind'] == 'Exif'
             and graph.primary in graph.targets(i, 'cdsc')]
    if len(items) > 1:
        raise ValueError('ambiguous primary EXIF')
    if not items:
        return {'maker84': None}
    raw = graph.payloads[items[0]]
    if len(raw) < 8:
        return {'maker84': None, 'parse_error': 'truncated EXIF'}
    offset = int.from_bytes(raw[:4], 'big')
    if offset + 4 > len(raw):
        return {'maker84': None, 'parse_error': 'EXIF offset outside payload'}
    raw = raw[4 + offset:]
    if not raw.startswith(b'Exif\0\0'):
        raw = b'Exif\0\0' + raw
    try:
        exif = Image.Exif()
        exif.load(raw)
        nested = exif.get_ifd(34665)
        maker = _maker84(nested.get(37500, b''))
        return {'make': exif.get(271), 'model': exif.get(272), 'software': exif.get(305),
                'capture_time': nested.get(36867), 'maker84': maker}
    except (ValueError, TypeError, SyntaxError, struct.error, plistlib.InvalidFileException) as error:
        return {'maker84': None, 'parse_error': type(error).__name__ + ': ' + str(error)}


def inspect_bytes(data: bytes) -> dict:
    graph = isolate.Graph(data)
    style = _plist(graph, isolate.STYLES_URI)
    texture = _plist(graph, isolate.TEXTURE_URI)
    exif = _exif(graph)
    state = exif['maker84']
    standard = None if state is None or not {'1', '2', '3', '6'} <= state.keys() else (
        state['1'] == 0 and state['2'] == 0 and state['3'] == 1 and state['6'] == 4)
    texture_standard = None if texture is None else texture.get('Preset') == 'Standard'
    style_facts = None
    if style is not None:
        baseline = style.get('4')
        if isinstance(baseline, bool) or not isinstance(baseline, (int, float)) or not math.isfinite(baseline):
            raise ValueError('invalid reference baseline exposure')
        style_facts = {'version': style.get('0'), 'baseline_exposure': baseline,
                       'baseline_outside_current_product_range': not 4.0 <= baseline <= 10.4,
                       'scene_type': style.get('5'), 'face_boost': style.get('j'),
                       'base_gain': style.get('h'),
                       'resource_bytes': {k: len(style[k]) for k in ('1', '3', 'c', 'd') if isinstance(style.get(k), bytes)},
                       'resource_sha256': {k: isolate.sha(style[k]) for k in ('1', '3', 'c', 'd') if isinstance(style.get(k), bytes)}}
    aux = sorted({name for item in graph.items for name in graph.aux(item)})
    complete = style_facts is not None and set(style_facts['resource_bytes']) == {'1', '3', 'c', 'd'}
    return {'source_sha256': isolate.sha(data), 'bytes': len(data),
            'coded_primary_fingerprint': graph.image_fingerprint(graph.primary),
            'capture_claims': {k: v for k, v in exif.items() if k != 'maker84'},
            'smartstyle_state': state, 'standard_semantic_state': standard,
            'standard_texture_state': texture_standard, 'style': style_facts,
            'texture': None if texture is None else {
                'preset': texture.get('Preset'), 'hardware_tuning': texture.get('HardwareModel'),
                'people': len(texture.get('TextureStylePostProcessedPeopleData', [])),
                'people_data_version': texture.get('TextureStylePeopleDataVersion')},
            'auxiliary_roles': aux,
            'standard_calibration_candidate': bool(complete and exif.get('make') == 'Apple' and standard is True
                and texture_standard is not False and isolate.LINEAR_AUX in aux
                and isolate.HDR_AUX in aux),
            'capture_authenticity_certified': False,
            'prior_research_exposure': 'not established',
            'native_render_verified': False}


def catalog(archives: list[Path], output: Path) -> dict:
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    originals = output / 'originals'
    originals.mkdir()
    records, archive_records, errors, entries = {}, [], [], 0
    for archive in sorted({Path(p).resolve() for p in archives}):
        identity = {'path': str(archive), 'sha256': _file_sha(archive), 'bytes': archive.stat().st_size}
        archive_records.append(identity)
        with zipfile.ZipFile(archive) as stream:
            identity['media_counts'] = dict(Counter(Path(m.filename).suffix.lower()
                for m in stream.infolist() if not m.is_dir() and '__MACOSX' not in m.filename))
            for member in stream.infolist():
                if member.is_dir() or '__MACOSX' in member.filename or not member.filename.lower().endswith(('.heic', '.heif')):
                    continue
                entries += 1
                source = {'archive_sha256': identity['sha256'], 'member': member.filename}
                try:
                    if member.file_size <= 0 or member.file_size > isolate.MAX_FILE_BYTES:
                        raise ValueError('archive member exceeds research size limit')
                    data = stream.read(member)
                    digest = isolate.sha(data)
                    if digest in records:
                        records[digest]['sources'].append(source)
                        continue
                    facts = inspect_bytes(data)
                    path = originals / (digest + '.heic')
                    with path.open('xb') as target:
                        target.write(data)
                    facts.update({'path': str(path), 'sources': [source],
                                  'scene_group': 'unreviewed-' + facts['coded_primary_fingerprint'],
                                  'split': 'unassigned', 'scene_group_reviewed': False})
                    records[digest] = facts
                except (ValueError, OSError, RuntimeError, struct.error, KeyError, UnicodeError, plistlib.InvalidFileException) as error:
                    errors.append({**source, 'error': type(error).__name__ + ': ' + str(error)})
        if _file_sha(archive) != identity['sha256']:
            raise ValueError('source archive changed during cataloging')
    samples = sorted(records.values(), key=lambda row: row['source_sha256'])
    style_samples = [s for s in samples if s['style'] is not None]
    eligible = [s for s in samples if s['standard_calibration_candidate']]
    report = {'schema': 'styles-fidelity-corpus-v1', 'archives': archive_records,
              'samples': samples, 'errors': errors,
              'summary': {'archives': len(archive_records), 'archive_heif_entries': entries,
                          'unique_heif_files': len(samples), 'inspection_errors': len(errors),
                          'with_styles': len(style_samples), 'standard_candidates': len(eligible),
                          'style_versions': dict(Counter(str(s['style']['version']) for s in style_samples)),
                          'texture_presets': dict(Counter(str(s['texture']['preset']) for s in samples if s['texture'])),
                          'baseline_exposure_range': ([min(s['style']['baseline_exposure'] for s in style_samples),
                                                       max(s['style']['baseline_exposure'] for s in style_samples)] if style_samples else None),
                          'baseline_outside_current_product_range': sum(s['style']['baseline_outside_current_product_range'] for s in style_samples),
                          'scene_split_frozen': False, 'native_cases_executed': 0},
              'scope': 'Reference labels stay outside every isolated producer input. Archive origin and metadata do not certify camera provenance.'}
    (output / 'corpus.json').write_text(json.dumps(report, ensure_ascii=False, indent=2, allow_nan=False) + '\n')
    return report


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archives', nargs='+', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args(argv)
    report = catalog(args.archives, args.output)
    print(json.dumps(report['summary'], ensure_ascii=False, indent=2))
    return 1 if report['errors'] else 0


if __name__ == '__main__':
    raise SystemExit(main())
