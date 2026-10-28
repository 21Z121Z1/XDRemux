"""Build a research input containing only coded SDR and its HDR resources.

This is an allow-list projection, not an in-place deletion. The producer file
never contains original Styles, semantic mattes, SmartStyle state, or unused
mdat bytes. EXIF is rebuilt with numeric HDR fields only. Native SDR/HDR
equivalence is a separate, required measurement.
"""
from __future__ import annotations

import hashlib
import argparse
import json
import math
import struct
import xml.etree.ElementTree as ET
from pathlib import Path

from scripts.diagnostics import inspect_oppo_heif as H
from . import hdr_exif

STYLES_URI = 'tag:apple.com,2023:photo:metadata:styles'
TEXTURE_URI = 'tag:apple.com,2026:photo:metadata:texture_styles'
LINEAR_AUX = 'tag:apple.com,2023:photo:aux:linearthumbnail'
HDR_AUX = 'urn:com:apple:photo:2020:aux:hdrgainmap'
HDR_NAMESPACE = 'http://ns.apple.com/HDRGainMap/1.0/'
HDR_FIELDS = {'HDRGainMapVersion', 'HDRGainMapHeadroom'}
PIXEL_PROPERTIES = {'hvcC', 'ispe', 'pixi', 'colr', 'irot', 'imir', 'clap', 'auxC', 'clli', 'mdcv'}
PIXEL_BRANDS = {b'heic', b'heix', b'mif1', b'miaf', b'MiHB', b'MiHA', b'MiHE', b'MiPr', b'tmap'}
MAX_FILE_BYTES = 256 * 1024 * 1024


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _box(kind: str, payload: bytes) -> bytes:
    if len(payload) + 8 >= 2**32:
        raise ValueError('research projection is too large')
    return struct.pack('>I4s', len(payload) + 8, kind.encode('ascii')) + payload


def _full(kind: str, version: int, flags: int, payload: bytes) -> bytes:
    return _box(kind, bytes([version]) + flags.to_bytes(3, 'big') + payload)


def _cstring(data: bytes, start: int, end: int) -> tuple[str, int]:
    stop = data.find(b'\0', start, end)
    if stop < 0:
        raise ValueError('unterminated item identity')
    return data[start:stop].decode('utf-8'), stop + 1


class Graph:
    """Read item identities/relationships using the existing diagnostic parser."""

    def __init__(self, data: bytes):
        if not data or len(data) > MAX_FILE_BYTES:
            raise ValueError('empty or oversized research HEIF')
        self.data = data
        top = list(H.boxes(data, 0, len(data)))
        ftyp = [b for b in top if b['type'] == 'ftyp']
        if len(ftyp) != 1:
            raise ValueError('one file type declaration is required')
        self.file_type = data[ftyp[0]['payload_start']:ftyp[0]['end']]
        if len(self.file_type) < 8 or len(self.file_type) % 4:
            raise ValueError('invalid file type declaration')
        metadata = [b for b in top if b['type'] == 'meta']
        if len(metadata) != 1:
            raise ValueError('one HEIF meta box is required')
        meta = metadata[0]
        children = list(H.boxes(data, meta['payload_start'] + 4, meta['end']))
        by_kind = {}
        for child in children:
            if child['type'] in by_kind:
                raise ValueError('duplicate HEIF metadata box')
            by_kind[child['type']] = child
        if not {'pitm', 'iinf', 'iloc', 'iprp'} <= by_kind.keys():
            raise ValueError('incomplete HEIF graph')
        version, _, pos = H.fullbox(data, by_kind['pitm'])
        if version not in (0, 1):
            raise ValueError('unsupported pitm version')
        width = 4 if version else 2
        if pos + width != by_kind['pitm']['end']:
            raise ValueError('invalid primary item')
        self.primary = int.from_bytes(data[pos:pos + width], 'big')
        self.items = H.parse_iinf(data, by_kind['iinf'])
        if -1 in self.items or self.primary not in self.items:
            raise ValueError('missing primary or inconsistent item count')
        self.identities = {}
        iv, _, ipos = H.fullbox(data, by_kind['iinf'])
        if iv not in (0, 1):
            raise ValueError('unsupported iinf version')
        ipos += 4 if iv else 2
        for item_box in H.boxes(data, ipos, by_kind['iinf']['end']):
            if item_box['type'] != 'infe':
                raise ValueError('unexpected iinf child')
            item_version, _, start = H.fullbox(data, item_box)
            if item_version not in (2, 3):
                raise ValueError('unsupported item identity version')
            iw = 4 if item_version == 3 else 2
            item_id = int.from_bytes(data[start:start + iw], 'big')
            if item_id in self.identities:
                raise ValueError('duplicate item ID')
            name, tail = _cstring(data, start + iw + 6, item_box['end'])
            kind = self.items[item_id].get('type')
            identity = {'kind': kind, 'name': name}
            if kind == 'uri ':
                identity['uri'], tail = _cstring(data, tail, item_box['end'])
            if kind == 'mime':
                identity['content_type'], tail = _cstring(data, tail, item_box['end'])
            self.identities[item_id] = identity
        self.refs = H.parse_iref(data, by_kind['iref']) if 'iref' in by_kind else []
        for ref in self.refs:
            if ref['from'] not in self.items or any(t not in self.items for t in ref['to']):
                raise ValueError('missing item reference dependency')
        self.alternatives = []
        if 'grpl' in by_kind:
            group_box = by_kind['grpl']
            for group in H.boxes(data, group_box['payload_start'], group_box['end']):
                if group['type'] != 'altr':
                    raise ValueError('unverified image entity group')
                gv, gf, pos = H.fullbox(data, group)
                if gv != 0 or gf != 0 or pos + 8 > group['end']:
                    raise ValueError('unsupported image alternate group')
                count = int.from_bytes(data[pos + 4:pos + 8], 'big')
                if pos + 8 + count * 4 != group['end']:
                    raise ValueError('truncated image alternate group')
                members = [int.from_bytes(data[pos + 8 + j * 4:pos + 12 + j * 4], 'big') for j in range(count)]
                if len(set(members)) != count or any(i not in self.items for i in members):
                    raise ValueError('invalid image alternate group members')
                self.alternatives.append(members)
        locations = H.parse_iloc(data, by_kind['iloc'])
        self.storage_methods = {item: location['construction_method'] for item, location in locations.items()}
        self.payloads = {}
        idat = by_kind.get('idat')
        for item in self.items:
            loc = locations.get(item)
            if loc is None or loc['data_reference'] != 0 or loc['construction_method'] not in (0, 1):
                raise ValueError('missing or nonlocal item storage')
            for extent in loc['extents']:
                offset, length = extent['offset'], extent['length']
                if offset < 0 or length < 0:
                    raise ValueError('negative extent')
                if loc['construction_method'] == 1:
                    if idat is None or offset + length > idat['end'] - idat['payload_start']:
                        raise ValueError('invalid idat extent')
                elif offset + length > len(data):
                    raise ValueError('invalid file extent')
            self.payloads[item] = H.payload_for_item(data, loc, idat)
        self.properties, self.associations = self._properties(by_kind['iprp'])

    def _properties(self, container: dict) -> tuple[list[dict], dict]:
        children = list(H.boxes(self.data, container['payload_start'], container['end']))
        ipco = [b for b in children if b['type'] == 'ipco']
        ipma = [b for b in children if b['type'] == 'ipma']
        if len(ipco) != 1 or len(ipma) != 1:
            raise ValueError('one property container/association table is required')
        properties = []
        for p in H.boxes(self.data, ipco[0]['payload_start'], ipco[0]['end']):
            entry = {'kind': p['type'], 'raw': self.data[p['start']:p['end']]}
            if p['type'] == 'auxC':
                entry['aux'], _ = _cstring(self.data, p['payload_start'] + 4, p['end'])
            properties.append(entry)
        version, flags, pos = H.fullbox(self.data, ipma[0])
        if version not in (0, 1) or flags & ~1:
            raise ValueError('unsupported ipma layout')
        end = ipma[0]['end']
        def read(width):
            nonlocal pos
            if pos + width > end:
                raise ValueError('truncated property association')
            result = int.from_bytes(self.data[pos:pos + width], 'big')
            pos += width
            return result
        count = read(4)
        associations = {}
        for _ in range(count):
            item = read(4 if version else 2)
            if item not in self.items or item in associations:
                raise ValueError('invalid property owner')
            values = []
            for _ in range(read(1)):
                value = read(2 if flags & 1 else 1)
                index = value & (0x7fff if flags & 1 else 0x7f)
                essential = bool(value & (0x8000 if flags & 1 else 0x80))
                if index > len(properties):
                    raise ValueError('property index outside container')
                if index:
                    values.append((index, essential))
            associations[item] = values
        if pos != end:
            raise ValueError('trailing property associations')
        return properties, associations

    def props(self, item: int) -> list[dict]:
        return [self.properties[i - 1] for i, _ in self.associations.get(item, [])]

    def aux(self, item: int) -> list[str]:
        return [p['aux'] for p in self.props(item) if p['kind'] == 'auxC']

    def targets(self, item: int, kind: str) -> list[int]:
        return [target for r in self.refs if r['type'] == kind and r['from'] == item for target in r['to']]

    def image_fingerprint(self, item: int, ancestors: tuple = ()) -> str:
        if item in ancestors:
            raise ValueError('cyclic image dependency')
        properties = [sha(p['raw']) for p in self.props(item) if p['kind'] != 'auxC']
        record = [self.identities[item]['kind'], sha(self.payloads[item]), properties,
                  [self.image_fingerprint(t, ancestors + (item,)) for t in self.targets(item, 'dimg')]]
        return sha(json.dumps(record, separators=(',', ':')).encode())

    def metadata_items(self, uri: str) -> list[int]:
        return [i for i, identity in self.identities.items() if identity.get('uri') == uri
                and self.primary in self.targets(i, 'cdsc')]


def _hdr_xmp(payload: bytes) -> bytes | None:
    if HDR_NAMESPACE.encode() not in payload:
        return None
    if len(payload) > 2 * 1024 * 1024 or b'<!DOCTYPE' in payload or b'<!ENTITY' in payload:
        raise ValueError('unsupported HDR XMP document')
    root = ET.fromstring(payload)
    values = {}
    for element in root.iter():
        for key, value in [*element.attrib.items(), (element.tag, element.text)]:
            if key.startswith('{' + HDR_NAMESPACE + '}'):
                name = key.split('}', 1)[1]
                if name not in HDR_FIELDS or not value or name in values and values[name] != value:
                    raise ValueError('unknown or conflicting HDR XMP field')
                if name == 'HDRGainMapVersion':
                    if not value.isascii() or not value.isdecimal() or not 0 <= int(value) < 2**32:
                        raise ValueError('invalid HDR version')
                else:
                    number = float(value)
                    if not math.isfinite(number) or not 0 <= number <= 1_000_000:
                        raise ValueError('invalid HDR headroom')
                    value = format(number, '.17g')
                values[name] = value
    if not values:
        raise ValueError('HDR namespace has no admitted fields')
    xmp = ET.Element('{adobe:ns:meta/}xmpmeta')
    for prefix, uri in [('x', 'adobe:ns:meta/'), ('rdf', 'http://www.w3.org/1999/02/22-rdf-syntax-ns#'),
                        ('HDRGainMap', HDR_NAMESPACE)]:
        ET.register_namespace(prefix, uri)
    rdf = ET.SubElement(xmp, '{http://www.w3.org/1999/02/22-rdf-syntax-ns#}RDF')
    desc = ET.SubElement(rdf, '{http://www.w3.org/1999/02/22-rdf-syntax-ns#}Description')
    desc.set('{http://www.w3.org/1999/02/22-rdf-syntax-ns#}about', '')
    for name, value in sorted(values.items()):
        ET.SubElement(desc, '{' + HDR_NAMESPACE + '}' + name).text = value
    return ET.tostring(xmp, encoding='utf-8')


def isolate_bytes(source: bytes, *, keep_numeric_hdr_exif: bool = True) -> tuple[bytes, dict]:
    graph = Graph(source)
    if any(brand not in PIXEL_BRANDS for brand in [graph.file_type[:4],
            *(graph.file_type[i:i + 4] for i in range(8, len(graph.file_type), 4))]):
        raise ValueError('unverified pixel format brand')
    gain = [i for i in graph.items if graph.aux(i) == [HDR_AUX]
            and graph.targets(i, 'auxl') == [graph.primary]]
    if len(gain) != 1:
        raise ValueError('one primary-associated Apple HDR Gain Map is required')
    tmap = [i for i, identity in graph.identities.items() if identity['kind'] == 'tmap'
            and set(graph.targets(i, 'dimg')) == {graph.primary, gain[0]}]
    if len(tmap) > 1:
        raise ValueError('ambiguous tone-map resource')
    order = []
    def visit(item, ancestors=()):
        if item in ancestors:
            raise ValueError('cyclic image dependency')
        if item in order:
            return
        if graph.identities[item]['kind'] not in {'hvc1', 'grid', 'tmap'}:
            raise ValueError('unsupported image dependency')
        if not graph.payloads[item]:
            raise ValueError('empty image payload')
        order.append(item)
        for target in graph.targets(item, 'dimg'):
            visit(target, ancestors + (item,))
    for root in (graph.primary, gain[0], *tmap):
        visit(root)
    payloads = {i: graph.payloads[i] for i in order}
    selected_refs = [r for r in graph.refs if r['from'] in order
                     and r['type'] in {'dimg', 'auxl'} and all(t in order for t in r['to'])]
    redacted, exif_items = [], []
    for item, identity in graph.identities.items():
        if keep_numeric_hdr_exif and identity['kind'] == 'Exif' and graph.primary in graph.targets(item, 'cdsc'):
            exif_items.append(item)
            if len(exif_items) > 1:
                raise ValueError('ambiguous primary EXIF')
            hdr = hdr_exif.redact(graph.payloads[item])
            if hdr is not None:
                order.append(item)
                payloads[item] = hdr
                redacted.append(item)
                selected_refs.append({'type': 'cdsc', 'from': item,
                                      'to': [t for t in graph.targets(item, 'cdsc') if t in payloads and t != item]})
            continue
        if identity['kind'] != 'mime' or not set(graph.targets(item, 'cdsc')) & set((graph.primary, gain[0], *tmap)):
            continue
        hdr = _hdr_xmp(graph.payloads[item])
        if hdr is not None:
            order.append(item)
            payloads[item] = hdr
            redacted.append(item)
            selected_refs.append({'type': 'cdsc', 'from': item,
                                  'to': [t for t in graph.targets(item, 'cdsc') if t in payloads and t != item]})
    mapping = {old: index + 1 for index, old in enumerate(order)}
    wide = len(order) > 65535
    item_code = 'I' if wide else 'H'
    alternatives = [members for members in graph.alternatives
                    if len(members) == 2 and graph.primary in members
                    and any(t in members for t in tmap)]
    group_bytes = b''.join(_full('altr', 0, 0, struct.pack('>II', len(mapping) + index + 1, len(members))
                                + b''.join(struct.pack('>I', mapping[i]) for i in members))
                           for index, members in enumerate(alternatives))
    properties, associations = [], {}
    for old in order:
        values = []
        for index, essential in graph.associations.get(old, []):
            prop = graph.properties[index - 1]
            if prop['kind'] not in PIXEL_PROPERTIES:
                raise ValueError('unverified pixel property ' + prop['kind'])
            if prop['kind'] == 'auxC' and (old != gain[0] or prop.get('aux') != HDR_AUX):
                raise ValueError('non-HDR auxiliary property in selected image graph')
            if prop['raw'] not in properties:
                properties.append(prop['raw'])
            values.append((properties.index(prop['raw']) + 1, essential))
        if values:
            associations[mapping[old]] = values
    infos = []
    for old in order:
        kind = graph.identities[old]['kind']
        tail = b'application/rdf+xml\0\0' if kind == 'mime' else b''
        info = struct.pack('>' + item_code + 'H4s', mapping[old], 0, kind.encode()) + b'image\0' + tail
        flags = 1 if old in redacted else graph.items[old]['flags']
        if flags & ~1:
            raise ValueError('unsupported image item flags')
        infos.append(_full('infe', 3 if wide else 2, flags, info))
    ipma = struct.pack('>I', len(associations))
    wide_properties = len(properties) > 127
    for item, values in associations.items():
        if len(values) > 255:
            raise ValueError('too many properties')
        ipma += struct.pack('>' + item_code + 'B', item, len(values))
        ipma += b''.join(struct.pack('>H' if wide_properties else '>B',
                                    i | ((0x8000 if wide_properties else 0x80) if essential else 0))
                         for i, essential in values)
    references = b''.join(_box(r['type'], struct.pack('>' + item_code + 'H', mapping[r['from']], len(r['to']))
                              + b''.join(struct.pack('>' + item_code, mapping[t]) for t in r['to'])) for r in selected_refs)
    idat_payload = b''.join(payloads[i] for i in order if graph.storage_methods[i] == 1)
    mdat_payload = b''.join(payloads[i] for i in order if graph.storage_methods[i] == 0)
    def metadata(start):
        offsets, positions = [], {0: start, 1: 0}
        for old in order:
            payload = payloads[old]
            method = graph.storage_methods[old]
            offsets.append(struct.pack('>' + item_code + 'HHHQQ', mapping[old], method, 0, 1,
                                       positions[method], len(payload)))
            positions[method] += len(payload)
        iloc = _full('iloc', 2 if wide else 1, 0, b'\x88\x00'
                     + struct.pack('>' + item_code, len(order)) + b''.join(offsets))
        hdlr = _full('hdlr', 0, 0, b'\0' * 4 + b'pict' + b'\0' * 12 + b'\0')
        dinf = _box('dinf', _full('dref', 0, 0, struct.pack('>I', 1) + _full('url ', 0, 1, b'')))
        return _full('meta', 0, 0, hdlr + dinf
                     + _full('pitm', int(wide), 0, struct.pack('>' + item_code, mapping[graph.primary]))
                     + _full('iinf', int(wide), 0, struct.pack('>' + item_code, len(infos)) + b''.join(infos)) + iloc
                     + _full('iref', int(wide), 0, references)
                     + _box('iprp', _box('ipco', b''.join(properties))
                            + _full('ipma', int(wide), int(wide_properties), ipma))
                     + (_box('grpl', group_bytes) if group_bytes else b'')
                     + (_box('idat', idat_payload) if idat_payload else b''))
    prefix = _box('ftyp', graph.file_type)
    first = metadata(0)
    output = prefix + metadata(len(prefix) + len(first) + 8) + _box('mdat', mdat_payload)
    actual = Graph(output)
    for old, new in mapping.items():
        if actual.payloads[new] != payloads[old]:
            raise ValueError('projection changed an admitted payload')
    if graph.image_fingerprint(graph.primary) != actual.image_fingerprint(actual.primary):
        raise ValueError('projection changed coded primary geometry or color')
    if graph.image_fingerprint(gain[0]) != actual.image_fingerprint(mapping[gain[0]]):
        raise ValueError('projection changed coded HDR gain geometry or color')
    if any(v['kind'] == 'uri ' for v in actual.identities.values()):
        raise ValueError('withheld metadata reached producer input')
    return output, {'schema': 'styles-blind-projection-v1', 'source_sha256': sha(source),
                    'producer_input_sha256': sha(output), 'source_items': len(graph.items),
                    'retained_items': len(order), 'retained_image_payloads': len(order) - len(redacted),
                    'redacted_hdr_descriptors': len(redacted), 'coded_primary_preserved': True,
                    'hdr_alternate_groups': len(alternatives),
                    'coded_hdr_gain_preserved': True, 'original_styles_available_to_producer': False,
                    'original_mattes_available_to_producer': False,
                    'original_exif_available_to_producer': False,
                    'rebuilt_numeric_hdr_exif': any(graph.identities[i]['kind'] == 'Exif' for i in redacted),
                    'native_hdr_equivalence_verified': False}


def write_case(source: Path, output: Path) -> dict:
    data = source.read_bytes()
    candidate, report = isolate_bytes(data)
    output.mkdir(parents=True, exist_ok=False)
    try:
        (output / 'input.heic').write_bytes(candidate)
        if sha(source.read_bytes()) != sha(data):
            raise ValueError('source changed during projection')
        (output / 'projection.json').write_text(json.dumps(report, indent=2) + '\n')
    except BaseException:
        for path in output.iterdir():
            path.unlink()
        output.rmdir()
        raise
    return report


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args(argv)
    report = write_case(args.input.resolve(), args.output.resolve())
    print(json.dumps(report, indent=2))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
