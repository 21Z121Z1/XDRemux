"""Rebuild only Apple's numeric HDR MakerNote fields, without Styles state."""
from __future__ import annotations

import struct

from PIL import Image

HEADER = b'Apple iOS\0\0\x01'
HDR_TAGS = {33, 48}


def redact(payload: bytes) -> bytes | None:
    if len(payload) < 12:
        raise ValueError('truncated source EXIF')
    offset = int.from_bytes(payload[:4], 'big') + 4
    if offset + 8 > len(payload):
        raise ValueError('source EXIF offset outside item')
    raw = payload[offset:]
    exif = Image.Exif()
    exif.load(raw if raw.startswith(b'Exif\0\0') else b'Exif\0\0' + raw)
    note = exif.get_ifd(34665).get(37500, b'')
    if not note.startswith(HEADER):
        return None
    if len(note) < 20 or note[12:14] not in (b'MM', b'II'):
        raise ValueError('unsupported Apple HDR MakerNote layout')
    order = '>' if note[12:14] == b'MM' else '<'
    count = struct.unpack_from(order + 'H', note, 14)[0]
    if 16 + count * 12 + 4 > len(note):
        raise ValueError('truncated source HDR MakerNote')
    values = {}
    for index in range(count):
        tag, kind, length, position = struct.unpack_from(order + 'HHII', note, 16 + 12 * index)
        if tag not in HDR_TAGS:
            continue
        if tag in values or kind != 10 or length != 1 or position + 8 > len(note):
            raise ValueError('ambiguous or invalid numeric HDR MakerNote field')
        numerator, denominator = struct.unpack_from(order + 'ii', note, position)
        if denominator == 0:
            raise ValueError('invalid numeric HDR rational')
        values[tag] = numerator, denominator
    if not values:
        return None
    if exif.get(271) != 'Apple':
        raise ValueError('numeric Apple HDR fields require the source Apple make')
    # Offsets inside an Apple MakerNote are relative to that note, not the TIFF.
    start = 16 + 12 * len(values) + 4
    entries, rationals = [], []
    for index, (tag, value) in enumerate(sorted(values.items())):
        entries.append(struct.pack('>HHII', tag, 10, 1, start + index * 8))
        rationals.append(struct.pack('>ii', *value))
    minimal_note = HEADER + b'MM' + struct.pack('>H', len(values))
    minimal_note += b''.join(entries) + b'\0' * 4 + b''.join(rationals)
    return wrap_note(minimal_note)


def wrap_note(note: bytes) -> bytes:
    """Construct fresh TIFF storage: Make, Exif pointer, and one MakerNote."""
    tiff = b'MM\0*' + struct.pack('>I', 8)
    # IFD0 ends at 38, followed by 'Apple\0', then the Exif IFD at 44.
    tiff += struct.pack('>H', 2)
    tiff += struct.pack('>HHII', 271, 2, 6, 38)
    tiff += struct.pack('>HHII', 34665, 4, 1, 44) + b'\0' * 4 + b'Apple\0'
    tiff += struct.pack('>H', 1)
    tiff += struct.pack('>HHII', 37500, 7, len(note), 62) + b'\0' * 4 + note
    return struct.pack('>I', 6) + b'Exif\0\0' + tiff
