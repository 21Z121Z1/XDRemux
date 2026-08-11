"""Pure-Python normalization for embedded Motion Photo ISO-BMFF video streams.

Some ColorOS 16 Stream 1 payloads accepted by AVFoundation contain opaque vendor
bytes after the last complete BMFF box and before Stream 2 begins. Those bytes
are not referenced media samples and are not part of a standalone MP4/MOV
container. A cross-platform writer must remove only that trailing opaque suffix
before parsing/remuxing the stream.
"""

from __future__ import annotations

import os
import struct
from pathlib import Path


class MotionVideoError(ValueError):
    pass


def _read_box_size(stream, offset: int, file_size: int) -> tuple[int, bytes] | None:
    remaining = file_size - offset
    if remaining < 8:
        return None
    stream.seek(offset)
    header = stream.read(16)
    if len(header) < 8:
        return None
    size32, kind = struct.unpack_from(">I4s", header, 0)
    header_size = 8
    if size32 == 1:
        if len(header) < 16:
            return None
        size = struct.unpack_from(">Q", header, 8)[0]
        header_size = 16
    elif size32 == 0:
        size = remaining
    else:
        size = size32
    if size < header_size or size > remaining:
        return None
    if not all(0x20 <= byte <= 0x7E for byte in kind):
        return None
    return int(size), kind


def standalone_bmff_length(path: Path) -> int:
    """Return the complete top-level BMFF prefix length of an embedded video.

    Strictly requires a normal ``ftyp`` plus ``moov`` and at least one ``mdat``.
    If an invalid suffix appears only *after* those required boxes, the suffix is
    treated as vendor data and excluded. Invalid data before the container is
    complete remains a hard failure.
    """
    path = Path(path)
    file_size = path.stat().st_size
    offset = 0
    kinds: list[bytes] = []
    with path.open("rb") as stream:
        while offset < file_size:
            parsed = _read_box_size(stream, offset, file_size)
            if parsed is None:
                if b"ftyp" in kinds and b"moov" in kinds and b"mdat" in kinds:
                    break
                raise MotionVideoError(f"invalid ISO-BMFF data at offset {offset}")
            size, kind = parsed
            if not kinds and kind != b"ftyp":
                raise MotionVideoError("embedded video does not begin with ftyp")
            kinds.append(kind)
            offset += size
    if not kinds or kinds[0] != b"ftyp" or b"moov" not in kinds or b"mdat" not in kinds:
        raise MotionVideoError("embedded video lacks required ftyp/moov/mdat boxes")
    return offset


def strip_trailing_vendor_data(path: Path) -> int:
    """Truncate only an opaque suffix after a complete standalone BMFF stream.

    Returns the number of removed bytes. Media payloads inside valid BMFF boxes
    are never rewritten.
    """
    path = Path(path)
    original = path.stat().st_size
    clean = standalone_bmff_length(path)
    if clean < original:
        with path.open("r+b") as stream:
            stream.truncate(clean)
            stream.flush()
            os.fsync(stream.fileno())
    return original - clean
