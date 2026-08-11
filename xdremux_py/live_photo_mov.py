"""Pure-Python QuickTime writer for Apple Live Photo paired video resources.

No AVFoundation, CoreMedia, FFmpeg, ExifTool, PyObjC, or other Apple runtime API
is used. Encoded source media stays at its original file offsets. The old moov
is replaced by an equal-sized free atom, then a tiny metadata mdat and rebuilt
moov are appended. Existing video/audio sample offsets and compressed bytes are
therefore preserved exactly.
"""

from __future__ import annotations

import hashlib
import math
import os
import stat
import struct
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path

from .motion_photo import OppoMetadata

QUICKTIME_EPOCH_OFFSET = 2_082_844_800
METADATA_TIMESCALE = 600
CONTENT_IDENTIFIER_KEY = b"com.apple.quicktime.content.identifier"
STILL_IMAGE_KEY = b"com.apple.quicktime.still-image-time"
TRANSFORM_KEY = b"com.apple.quicktime.live-photo-still-image-transform"
REFERENCE_DIMENSIONS_KEY = b"com.apple.quicktime.live-photo-still-image-transform-reference-dimensions"
COPY_CHUNK_SIZE = 1 << 20


class LivePhotoMovieError(ValueError):
    pass


@dataclass(frozen=True)
class Box:
    offset: int
    size: int
    kind: bytes
    header_size: int = 8
    size32: int | None = None

    @property
    def payload_offset(self) -> int:
        return self.offset + self.header_size

    @property
    def end(self) -> int:
        return self.offset + self.size


def _boxes(data: bytes | bytearray, start: int, end: int) -> tuple[Box, ...]:
    result: list[Box] = []
    offset = start
    while offset < end:
        if offset + 8 > end:
            raise LivePhotoMovieError("truncated ISO BMFF box header")
        size32, kind = struct.unpack_from(">I4s", data, offset)
        header_size = 8
        if size32 == 1:
            if offset + 16 > end:
                raise LivePhotoMovieError("truncated extended ISO BMFF box header")
            size = struct.unpack_from(">Q", data, offset + 8)[0]
            header_size = 16
        elif size32 == 0:
            size = end - offset
        else:
            size = size32
        if size < header_size or offset + size > end:
            raise LivePhotoMovieError(f"invalid {kind!r} box size {size}")
        result.append(Box(offset, int(size), kind, header_size, size32))
        offset += int(size)
    return tuple(result)


def _scan_top_level(path: Path) -> tuple[Box, ...]:
    file_size = path.stat().st_size
    result: list[Box] = []
    with path.open("rb") as stream:
        offset = 0
        while offset < file_size:
            if file_size - offset < 8:
                raise LivePhotoMovieError("trailing bytes after final ISO BMFF box")
            stream.seek(offset)
            header = stream.read(16)
            if len(header) < 8:
                raise LivePhotoMovieError("truncated top-level box")
            size32, kind = struct.unpack_from(">I4s", header, 0)
            header_size = 8
            if size32 == 1:
                if len(header) < 16:
                    raise LivePhotoMovieError("truncated extended top-level box")
                size = struct.unpack_from(">Q", header, 8)[0]
                header_size = 16
            elif size32 == 0:
                size = file_size - offset
            else:
                size = size32
            if size < header_size or offset + size > file_size:
                raise LivePhotoMovieError(f"invalid top-level {kind!r} box")
            result.append(Box(offset, int(size), kind, header_size, size32))
            offset += int(size)
    return tuple(result)


def _box(kind: bytes, payload: bytes) -> bytes:
    if len(kind) != 4:
        raise LivePhotoMovieError("box type must be four bytes")
    size = 8 + len(payload)
    if size <= 0xFFFFFFFF:
        return struct.pack(">I4s", size, kind) + payload
    return struct.pack(">I4sQ", 1, kind, size + 8) + payload


def _full_box(kind: bytes, payload: bytes = b"", *, version: int = 0, flags: int = 0) -> bytes:
    return _box(kind, bytes((version,)) + flags.to_bytes(3, "big") + payload)


def _direct_child(data: bytes | bytearray, parent: Box, kind: bytes) -> Box:
    for child in _boxes(data, parent.payload_offset, parent.end):
        if child.kind == kind:
            return child
    raise LivePhotoMovieError(f"missing {kind.decode('latin1')} box")


def _read_box(path: Path, box: Box) -> bytes:
    with path.open("rb") as stream:
        stream.seek(box.offset)
        data = stream.read(box.size)
    if len(data) != box.size:
        raise LivePhotoMovieError("could not read full ISO BMFF box")
    return data


def _free_box_same_size(size: int) -> bytes:
    if size < 8:
        raise LivePhotoMovieError("cannot replace undersized moov box")
    if size <= 0xFFFFFFFF:
        return struct.pack(">I4s", size, b"free") + b"\0" * (size - 8)
    return struct.pack(">I4sQ", 1, b"free", size) + b"\0" * (size - 16)


def _movie_timescale(moov: bytes) -> tuple[int, int]:
    mvhd = _direct_child(moov, Box(0, len(moov), b"moov"), b"mvhd")
    version = moov[mvhd.payload_offset]
    if version == 0:
        timescale_offset, duration_offset, width = mvhd.offset + 20, mvhd.offset + 24, 4
    elif version == 1:
        timescale_offset, duration_offset, width = mvhd.offset + 28, mvhd.offset + 32, 8
    else:
        raise LivePhotoMovieError(f"unsupported mvhd version {version}")
    timescale = struct.unpack_from(">I", moov, timescale_offset)[0]
    duration = int.from_bytes(moov[duration_offset:duration_offset + width], "big")
    if timescale <= 0:
        raise LivePhotoMovieError("invalid movie timescale")
    return timescale, duration


def _track_id(moov: bytes, track: Box) -> int:
    tkhd = _direct_child(moov, track, b"tkhd")
    version = moov[tkhd.payload_offset]
    return struct.unpack_from(">I", moov, tkhd.offset + (20 if version == 0 else 28))[0]


def _handler_type(moov: bytes, track: Box) -> bytes:
    mdia = _direct_child(moov, track, b"mdia")
    hdlr = _direct_child(moov, mdia, b"hdlr")
    if hdlr.size < 20:
        raise LivePhotoMovieError("truncated media handler")
    return moov[hdlr.payload_offset + 8:hdlr.payload_offset + 12]


def _mdhd_timescale(moov: bytes, track: Box) -> int:
    mdia = _direct_child(moov, track, b"mdia")
    mdhd = _direct_child(moov, mdia, b"mdhd")
    version = moov[mdhd.payload_offset]
    value = struct.unpack_from(">I", moov, mdhd.offset + (20 if version == 0 else 28))[0]
    if value <= 0:
        raise LivePhotoMovieError("invalid media timescale")
    return value


def _sample_pts_seconds(moov: bytes, track: Box) -> tuple[float, ...]:
    timescale = _mdhd_timescale(moov, track)
    mdia = _direct_child(moov, track, b"mdia")
    minf = _direct_child(moov, mdia, b"minf")
    stbl = _direct_child(moov, minf, b"stbl")
    stts = _direct_child(moov, stbl, b"stts")
    cursor = stts.payload_offset + 4
    if cursor + 4 > stts.end:
        raise LivePhotoMovieError("truncated stts")
    entry_count = struct.unpack_from(">I", moov, cursor)[0]
    cursor += 4
    dts: list[int] = []
    clock = 0
    for _ in range(entry_count):
        if cursor + 8 > stts.end:
            raise LivePhotoMovieError("truncated stts entry")
        count, delta = struct.unpack_from(">II", moov, cursor)
        cursor += 8
        if count > 10_000_000 - len(dts):
            raise LivePhotoMovieError("video sample table exceeds safety limit")
        for _ in range(count):
            dts.append(clock)
            clock += delta
    if not dts:
        raise LivePhotoMovieError("video track has no samples")
    offsets = [0] * len(dts)
    ctts = next((box for box in _boxes(moov, stbl.payload_offset, stbl.end) if box.kind == b"ctts"), None)
    if ctts is not None:
        version = moov[ctts.payload_offset]
        cursor = ctts.payload_offset + 4
        if cursor + 4 > ctts.end:
            raise LivePhotoMovieError("truncated ctts")
        entry_count = struct.unpack_from(">I", moov, cursor)[0]
        cursor += 4
        sample_index = 0
        for _ in range(entry_count):
            if cursor + 8 > ctts.end:
                raise LivePhotoMovieError("truncated ctts entry")
            sample_count = struct.unpack_from(">I", moov, cursor)[0]
            value = int.from_bytes(moov[cursor + 4:cursor + 8], "big", signed=(version == 1))
            cursor += 8
            if sample_index + sample_count > len(offsets):
                raise LivePhotoMovieError("ctts sample count exceeds stts")
            offsets[sample_index:sample_index + sample_count] = [value] * sample_count
            sample_index += sample_count
        if sample_index != len(offsets):
            raise LivePhotoMovieError("ctts/stts sample count mismatch")
    return tuple((decode + offset) / timescale for decode, offset in zip(dts, offsets))


def resolve_still_time(source_video: Path, requested_timestamp_us: int | None) -> float:
    top = _scan_top_level(source_video)
    moov_boxes = [box for box in top if box.kind == b"moov"]
    if len(moov_boxes) != 1:
        raise LivePhotoMovieError("embedded video must contain exactly one moov box")
    moov = _read_box(source_video, moov_boxes[0])
    movie_timescale, movie_duration = _movie_timescale(moov)
    root = Box(0, len(moov), b"moov")
    tracks = [box for box in _boxes(moov, root.payload_offset, root.end) if box.kind == b"trak"]
    video_tracks = [track for track in tracks if _handler_type(moov, track) == b"vide"]
    if not video_tracks:
        raise LivePhotoMovieError("embedded video contains no video track")
    pts = _sample_pts_seconds(moov, video_tracks[0])
    duration_seconds = movie_duration / movie_timescale
    if requested_timestamp_us is not None:
        requested = requested_timestamp_us / 1_000_000.0
        if requested < 0 or requested > duration_seconds:
            raise LivePhotoMovieError("Motion Photo still timestamp lies outside the video")
        return min(pts, key=lambda value: abs(value - requested))
    midpoint = duration_seconds * 0.5
    closest = min(range(len(pts)), key=lambda index: abs(pts[index] - midpoint))
    return pts[max(0, closest - 1)]


def _invert3(m: tuple[float, ...]) -> tuple[float, ...] | None:
    if len(m) != 9:
        return None
    a, b, c, d, e, f, g, h, i = m
    det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)
    if not math.isfinite(det) or abs(det) <= 1e-10:
        return None
    inv = 1.0 / det
    return ((e*i-f*h)*inv, (c*h-b*i)*inv, (b*f-c*e)*inv,
            (f*g-d*i)*inv, (a*i-c*g)*inv, (c*d-a*f)*inv,
            (d*h-e*g)*inv, (b*g-a*h)*inv, (a*e-b*d)*inv)


def _multiply3(a: tuple[float, ...], b: tuple[float, ...]) -> tuple[float, ...]:
    return tuple(sum(a[r*3+k] * b[k*3+c] for k in range(3)) for r in range(3) for c in range(3))


def oppo_transform(metadata: OppoMetadata | None) -> tuple[float, ...] | None:
    if metadata is None:
        return None
    if metadata.version >= 1:
        result = (0.90, 0.0, 0.0, 0.0, 0.90, 0.0, 0.0, 0.0, 1.0)
        if metadata.photo_crop_matrix and (inverse := _invert3(metadata.photo_crop_matrix)):
            result = _multiply3(result, inverse)
        if metadata.photo_eis_matrix and (inverse := _invert3(metadata.photo_eis_matrix)):
            result = _multiply3(result, inverse)
    else:
        if metadata.matrix_count <= 0 or not metadata.matrices or metadata.cover_frame_pts_us is None:
            return None
        _, matrix = min(metadata.matrices, key=lambda pair: abs(pair[0] - metadata.cover_frame_pts_us))
        result = _invert3(matrix) or matrix
    identity = (1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0)
    return None if all(abs(x-y) <= 1e-6 for x, y in zip(result, identity)) else result


def _metadata_key_atom(local_id: int, name: bytes, type_code: int) -> bytes:
    keyd = _box(b"keyd", b"mdta" + name)
    dtyp = _box(b"dtyp", struct.pack(">II", 0, type_code))
    return _box(struct.pack(">I", local_id), keyd + dtyp)


def _metadata_sample(transform: tuple[float, ...] | None, dimensions: tuple[float, float] | None) -> bytes:
    values = [_box(struct.pack(">I", 1), b"\x00")]
    if transform is not None:
        values.append(_box(struct.pack(">I", 2), struct.pack(">9d", *transform)))
    if dimensions is not None:
        values.append(_box(struct.pack(">I", 3), struct.pack(">2f", *dimensions)))
    return b"".join(values)


def _metadata_track(track_id: int, movie_timescale: int, still_time_seconds: float,
                    chunk_offset: int, *, transform: tuple[float, ...] | None,
                    dimensions: tuple[float, float] | None) -> tuple[bytes, bytes]:
    sample = _metadata_sample(transform, dimensions)
    timestamp = int(time.time()) + QUICKTIME_EPOCH_OFFSET
    empty_duration = max(0, round(still_time_seconds * movie_timescale))
    marker_duration = max(1, round(movie_timescale / METADATA_TIMESCALE))
    track_duration = empty_duration + marker_duration
    if track_duration > 0xFFFFFFFF:
        raise LivePhotoMovieError("metadata track duration exceeds version-0 tkhd")
    matrix = struct.pack(">9i", 0x10000, 0, 0, 0, 0x10000, 0, 0, 0, 0x40000000)
    tkhd = _full_box(
        b"tkhd",
        struct.pack(">IIIII", timestamp, timestamp, track_id, 0, track_duration)
        + b"\0" * 8 + struct.pack(">hhhh", 0, 0, 0, 0) + matrix + struct.pack(">II", 0, 0),
        flags=0x0F,
    )
    edits = []
    if empty_duration > 0:
        edits.append(struct.pack(">IiHH", empty_duration, -1, 1, 0))
    edits.append(struct.pack(">IiHH", marker_duration, 0, 1, 0))
    edts = _box(b"edts", _full_box(b"elst", struct.pack(">I", len(edits)) + b"".join(edits)))
    mdhd = _full_box(b"mdhd", struct.pack(">IIIIHH", timestamp, timestamp, METADATA_TIMESCALE, 1, 0x55C4, 0))
    media_name = b"Core Media Metadata"
    media_handler = _full_box(
        b"hdlr", b"mhlrmetaappl" + struct.pack(">II", 1, 0) + bytes((len(media_name),)) + media_name
    )
    gmhd = _box(b"gmhd", _full_box(b"gmin", struct.pack(">HHHHhH", 0x40, 0x8000, 0x8000, 0x8000, 0, 0)))
    data_name = b"Core Media Data Handler"
    data_handler = _full_box(
        b"hdlr", b"dhlralisappl" + struct.pack(">II", 0, 0) + bytes((len(data_name),)) + data_name
    )
    dinf = _box(b"dinf", _full_box(b"dref", struct.pack(">I", 1) + _full_box(b"alis", flags=1)))
    key_atoms = [_metadata_key_atom(1, STILL_IMAGE_KEY, 65)]
    if transform is not None:
        key_atoms.append(_metadata_key_atom(2, TRANSFORM_KEY, 79))
    if dimensions is not None:
        key_atoms.append(_metadata_key_atom(3, REFERENCE_DIMENSIONS_KEY, 71))
    keys = _box(b"keys", b"".join(key_atoms))
    mebx = _box(b"mebx", b"\0" * 6 + struct.pack(">H", 1) + keys)
    stsd = _full_box(b"stsd", struct.pack(">I", 1) + mebx)
    stts = _full_box(b"stts", struct.pack(">III", 1, 1, 1))
    stsc = _full_box(b"stsc", struct.pack(">IIII", 1, 1, 1, 1))
    stsz = _full_box(b"stsz", struct.pack(">II", len(sample), 1))
    chunk = _full_box(b"co64", struct.pack(">IQ", 1, chunk_offset)) if chunk_offset > 0xFFFFFFFF else _full_box(b"stco", struct.pack(">II", 1, chunk_offset))
    stbl = _box(b"stbl", stsd + stts + stsc + stsz + chunk)
    minf = _box(b"minf", gmhd + data_handler + dinf + stbl)
    mdia = _box(b"mdia", mdhd + media_handler + minf)
    return _box(b"trak", tkhd + edts + mdia), sample


def _movie_metadata(content_identifier: str) -> bytes:
    try:
        identifier = content_identifier.encode("ascii")
    except UnicodeEncodeError as exc:
        raise LivePhotoMovieError("Live Photo content identifier must be ASCII") from exc
    if not identifier or b"\0" in identifier:
        raise LivePhotoMovieError("invalid Live Photo content identifier")
    handler = _box(b"hdlr", b"\0" * 8 + b"mdta" + b"\0" * 14)
    keys = _full_box(b"keys", struct.pack(">I", 1) + _box(b"mdta", CONTENT_IDENTIFIER_KEY))
    value = _box(b"data", struct.pack(">II", 1, 0) + identifier)
    ilst = _box(b"ilst", _box(struct.pack(">I", 1), value))
    return _box(b"meta", handler + keys + ilst)


def _patch_next_track_id(raw: bytes, next_track_id: int) -> bytes:
    data = bytearray(raw)
    struct.pack_into(">I", data, len(data) - 4, next_track_id)
    return bytes(data)


def _rebuild_moov(original: bytes, metadata_track: bytes, movie_metadata: bytes, new_track_id: int) -> bytes:
    root = Box(0, len(original), b"moov")
    rebuilt: list[bytes] = []
    for child in _boxes(original, root.payload_offset, root.end):
        if child.kind == b"meta":
            continue
        raw = original[child.offset:child.end]
        if child.kind == b"mvhd":
            raw = _patch_next_track_id(raw, new_track_id + 1)
        rebuilt.append(raw)
    return _box(b"moov", b"".join(rebuilt) + metadata_track + movie_metadata)


def _copy_range(stream_in, stream_out, offset: int, length: int) -> None:
    stream_in.seek(offset)
    remaining = length
    while remaining:
        chunk = stream_in.read(min(COPY_CHUNK_SIZE, remaining))
        if not chunk:
            raise LivePhotoMovieError("source movie ended during copy")
        stream_out.write(chunk)
        remaining -= len(chunk)


def write_live_photo_movie(source: Path, destination: Path, content_identifier: str,
                           still_time_seconds: float, *, oppo_metadata: OppoMetadata | None = None) -> None:
    source, destination = Path(source), Path(destination)
    if source.resolve() == destination.resolve():
        raise LivePhotoMovieError("source and destination must differ")
    if not math.isfinite(still_time_seconds) or still_time_seconds < 0:
        raise LivePhotoMovieError("invalid Live Photo still time")
    top = _scan_top_level(source)
    ftyp_boxes = [box for box in top if box.kind == b"ftyp"]
    moov_boxes = [box for box in top if box.kind == b"moov"]
    if len(ftyp_boxes) != 1 or len(moov_boxes) != 1:
        raise LivePhotoMovieError("source video must contain exactly one ftyp and one moov")
    ftyp, moov_box = ftyp_boxes[0], moov_boxes[0]
    if ftyp.size < ftyp.header_size + 8:
        raise LivePhotoMovieError("source ftyp is too small")
    original_moov = _read_box(source, moov_box)
    movie_timescale, _ = _movie_timescale(original_moov)
    root = Box(0, len(original_moov), b"moov")
    tracks = [box for box in _boxes(original_moov, root.payload_offset, root.end) if box.kind == b"trak"]
    new_track_id = max((_track_id(original_moov, track) for track in tracks), default=0) + 1
    transform = oppo_transform(oppo_metadata)
    dimensions = None
    if transform is not None and oppo_metadata and oppo_metadata.video_width and oppo_metadata.video_height:
        dimensions = (float(oppo_metadata.video_width), float(oppo_metadata.video_height))
    marker_payload_offset = source.stat().st_size + 8
    metadata_track, marker_sample = _metadata_track(
        new_track_id, movie_timescale, still_time_seconds, marker_payload_offset,
        transform=transform, dimensions=dimensions,
    )
    marker_mdat = _box(b"mdat", marker_sample)
    new_moov = _rebuild_moov(original_moov, metadata_track, _movie_metadata(content_identifier), new_track_id)
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(mode="wb", prefix=f".{destination.name}.", suffix=".tmp",
                                         dir=destination.parent, delete=False) as out, source.open("rb") as src:
            temporary = Path(out.name)
            for box in top:
                if box.kind == b"moov":
                    out.write(_free_box_same_size(box.size))
                elif box.kind == b"ftyp":
                    raw = bytearray(_read_box(source, box))
                    raw[box.header_size:box.header_size + 4] = b"qt  "
                    raw[box.header_size + 4:box.header_size + 8] = b"\0\0\0\0"
                    out.write(raw)
                elif box.size32 == 0:
                    if box.size > 0xFFFFFFFF:
                        raise LivePhotoMovieError("cannot append after >4GiB size==0 box")
                    out.write(struct.pack(">I4s", box.size, box.kind))
                    _copy_range(src, out, box.payload_offset, box.size - box.header_size)
                else:
                    _copy_range(src, out, box.offset, box.size)
            out.write(marker_mdat)
            out.write(new_moov)
            out.flush()
            os.fsync(out.fileno())
        os.chmod(temporary, stat.S_IMODE(source.stat().st_mode))
        os.replace(temporary, destination)
    finally:
        if temporary is not None and temporary.exists():
            temporary.unlink()


def read_content_identifier(path: Path) -> str | None:
    moov_box = next((box for box in _scan_top_level(path) if box.kind == b"moov"), None)
    if moov_box is None:
        return None
    data = _read_box(path, moov_box)
    root = Box(0, len(data), b"moov")
    meta = next((box for box in _boxes(data, root.payload_offset, root.end) if box.kind == b"meta"), None)
    if meta is None:
        return None
    try:
        children = _boxes(data, meta.payload_offset, meta.end)
    except LivePhotoMovieError:
        children = _boxes(data, meta.payload_offset + 4, meta.end)
    keys = next((box for box in children if box.kind == b"keys"), None)
    ilst = next((box for box in children if box.kind == b"ilst"), None)
    if keys is None or ilst is None:
        return None
    cursor = keys.payload_offset + 4
    if cursor + 4 > keys.end:
        return None
    count = struct.unpack_from(">I", data, cursor)[0]
    cursor += 4
    content_index = None
    for index in range(1, count + 1):
        if cursor + 8 > keys.end:
            return None
        size = struct.unpack_from(">I", data, cursor)[0]
        if size < 8 or cursor + size > keys.end:
            return None
        if data[cursor + 4:cursor + 8] == b"mdta" and data[cursor + 8:cursor + size] == CONTENT_IDENTIFIER_KEY:
            content_index = index
        cursor += size
    if content_index is None:
        return None
    wanted = struct.pack(">I", content_index)
    for item in _boxes(data, ilst.payload_offset, ilst.end):
        if item.kind != wanted:
            continue
        value = next((child for child in _boxes(data, item.payload_offset, item.end) if child.kind == b"data"), None)
        if value is None or value.size < 16:
            return None
        type_indicator, locale = struct.unpack_from(">II", data, value.payload_offset)
        if type_indicator != 1 or locale != 0:
            return None
        try:
            return data[value.payload_offset + 8:value.end].decode("utf-8")
        except UnicodeDecodeError:
            return None
    return None


def read_still_time(path: Path) -> float | None:
    moov_box = next((box for box in _scan_top_level(path) if box.kind == b"moov"), None)
    if moov_box is None:
        return None
    data = _read_box(path, moov_box)
    timescale, _ = _movie_timescale(data)
    root = Box(0, len(data), b"moov")
    for track in _boxes(data, root.payload_offset, root.end):
        if track.kind != b"trak":
            continue
        raw = data[track.offset:track.end]
        if b"mebx" not in raw or STILL_IMAGE_KEY not in raw:
            continue
        try:
            edts = _direct_child(data, track, b"edts")
            elst = _direct_child(data, edts, b"elst")
        except LivePhotoMovieError:
            return 0.0
        version = data[elst.payload_offset]
        cursor = elst.payload_offset + 4
        count = struct.unpack_from(">I", data, cursor)[0]
        cursor += 4
        if count == 0:
            return 0.0
        if version == 0:
            duration = struct.unpack_from(">I", data, cursor)[0]
            media_time = struct.unpack_from(">i", data, cursor + 4)[0]
        elif version == 1:
            duration = struct.unpack_from(">Q", data, cursor)[0]
            media_time = struct.unpack_from(">q", data, cursor + 8)[0]
        else:
            return None
        return duration / timescale if media_time == -1 else 0.0
    return None


def media_payload_sha256(path: Path) -> tuple[str, ...]:
    """Hash source media mdats, excluding the small Live Photo metadata mdat."""
    hashes: list[str] = []
    boxes = _scan_top_level(path)
    with Path(path).open("rb") as stream:
        for box in boxes:
            if box.kind != b"mdat":
                continue
            stream.seek(box.payload_offset)
            if box.size <= 512:
                probe = stream.read(box.size - box.header_size)
                if STILL_IMAGE_KEY in probe or probe.startswith(struct.pack(">I", 9) + struct.pack(">I", 1)):
                    continue
                stream.seek(box.payload_offset)
            remaining = box.size - box.header_size
            digest = hashlib.sha256()
            while remaining:
                chunk = stream.read(min(COPY_CHUNK_SIZE, remaining))
                if not chunk:
                    raise LivePhotoMovieError("truncated mdat during hashing")
                digest.update(chunk)
                remaining -= len(chunk)
            hashes.append(digest.hexdigest())
    return tuple(hashes)


def validate_live_photo_movie(path: Path, content_identifier: str, still_time_seconds: float) -> None:
    if read_content_identifier(path) != content_identifier:
        raise LivePhotoMovieError("MOV content identifier mismatch")
    actual = read_still_time(path)
    if actual is None:
        raise LivePhotoMovieError("MOV lacks still-image-time metadata track")
    moov = [box for box in _scan_top_level(path) if box.kind == b"moov"]
    if len(moov) != 1:
        raise LivePhotoMovieError("Live Photo MOV must contain one active moov")
    data = _read_box(path, moov[0])
    timescale, _ = _movie_timescale(data)
    if abs(actual - still_time_seconds) > max(1.0 / timescale, 1e-6):
        raise LivePhotoMovieError("MOV still-image-time mismatch")
