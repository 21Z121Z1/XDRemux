"""Cross-platform Android/OPPO Motion Photo parsing.

The parser mirrors XDRemuxCore's Swift MotionPhoto model without importing any
Apple framework. It understands Android Motion Photo V1, legacy MicroVideo,
HEIF ``mpvd`` payloads, and the OPPO LPEX extensions needed for ColorOS 15/16.

Only small metadata windows are loaded into memory. Embedded video/still
resources are represented as checked byte ranges and copied with bounded I/O.
"""

from __future__ import annotations

import json
import os
import re
import struct
import xml.etree.ElementTree as ET
from dataclasses import dataclass, replace
from pathlib import Path
from typing import Iterable

MAX_XMP_SCAN_BYTES = 4 * 1024 * 1024
MAX_DIRECTORY_ITEMS = 64
MAX_METADATA_STRING = 4096
MAX_LPEX_JSON_BYTES = 256 * 1024
MAX_VENDOR_TAIL_SCAN_BYTES = 512 * 1024 * 1024
SCAN_CHUNK_BYTES = 2 * 1024 * 1024


class MotionPhotoError(ValueError):
    """Malformed or unsupported Motion Photo input."""


@dataclass(frozen=True)
class ByteRange:
    start: int
    end: int

    def __post_init__(self) -> None:
        if self.start < 0 or self.end < self.start:
            raise MotionPhotoError("invalid Motion Photo byte range")

    @property
    def length(self) -> int:
        return self.end - self.start


@dataclass(frozen=True)
class MotionPhotoItem:
    mime: str
    semantic: str
    length: int
    padding: int = 0


@dataclass(frozen=True)
class OppoMetadata:
    cover_frame_pts_us: int | None = None
    version: int = 0
    matrix_count: int = 0
    photo_crop_matrix: tuple[float, ...] | None = None
    photo_eis_matrix: tuple[float, ...] | None = None
    matrices: tuple[tuple[int, tuple[float, ...]], ...] = ()
    video_width: int | None = None
    video_height: int | None = None
    origin_photo_width: int | None = None
    origin_photo_height: int | None = None
    photo_eis_crop_factor: tuple[float, ...] | None = None
    eis_crop_factor: tuple[float, ...] | None = None
    photo_crop_factor: float | None = None
    stream_count: int = 1


@dataclass(frozen=True)
class MotionPhotoAsset:
    source: Path
    source_kind: str
    items: tuple[MotionPhotoItem, ...]
    still_range: ByteRange
    video_range: ByteRange
    presentation_timestamp_us: int | None
    presentation_source: str | None
    vendor_metadata: OppoMetadata | None = None


@dataclass(frozen=True)
class _Box:
    offset: int
    size: int
    kind: bytes
    header_size: int

    @property
    def payload_offset(self) -> int:
        return self.offset + self.header_size

    @property
    def end(self) -> int:
        return self.offset + self.size


def _checked_nonnegative(value: str | None, *, default: int = 0) -> int:
    if value is None:
        return default
    if len(value.encode("utf-8")) > 32:
        raise MotionPhotoError("Motion Photo integer metadata is too long")
    try:
        parsed = int(value, 10)
    except ValueError as exc:
        raise MotionPhotoError("invalid Motion Photo integer metadata") from exc
    if parsed < 0 or parsed > (1 << 63) - 1:
        raise MotionPhotoError("invalid Motion Photo item length")
    return parsed


def _local_name(name: str) -> str:
    if name.startswith("{"):
        return name.split("}", 1)[1]
    return name.split(":", 1)[-1]


def _namespace(name: str) -> str:
    if name.startswith("{") and "}" in name:
        return name[1:].split("}", 1)[0]
    return ""


def _extract_xmp_prefix(path: Path) -> bytes | None:
    size = path.stat().st_size
    with path.open("rb") as stream:
        prefix = stream.read(min(size, MAX_XMP_SCAN_BYTES))
    starts = [prefix.find(b"<x:xmpmeta"), prefix.find(b"<xmpmeta")]
    starts = [value for value in starts if value >= 0]
    if not starts:
        return None
    start = min(starts)
    ends: list[int] = []
    for closing in (b"</x:xmpmeta>", b"</xmpmeta>"):
        pos = prefix.find(closing, start)
        if pos >= 0:
            ends.append(pos + len(closing))
    if not ends:
        if size > len(prefix):
            raise MotionPhotoError("Motion Photo XMP exceeds safety limit")
        raise MotionPhotoError("Motion Photo XMP is malformed")
    return prefix[start:min(ends)]


def _safe_xml_root(xmp: bytes) -> ET.Element:
    upper = xmp.upper()
    if b"<!DOCTYPE" in upper or b"<!ENTITY" in upper:
        raise MotionPhotoError("DTD/entity declarations are forbidden in Motion Photo XMP")
    try:
        return ET.fromstring(xmp)
    except ET.ParseError as exc:
        raise MotionPhotoError("Motion Photo XMP is malformed") from exc


def _find_description_attributes(root: ET.Element) -> dict[str, str]:
    result: dict[str, str] = {}
    for element in root.iter():
        if _local_name(element.tag) != "Description":
            continue
        for key, value in element.attrib.items():
            local = _local_name(key)
            namespace = _namespace(key)
            if local in {
                "MotionPhoto",
                "MotionPhotoVersion",
                "MotionPhotoPresentationTimestampUs",
                "MicroVideo",
                "MicroVideoOffset",
                "MicroVideoPresentationTimestampUs",
            }:
                result[local] = value
                if namespace:
                    result[f"{namespace}|{local}"] = value
    return result


def _parse_directory(root: ET.Element) -> tuple[MotionPhotoItem, ...]:
    items: list[MotionPhotoItem] = []

    def visit(element: ET.Element, inside: bool) -> None:
        local = _local_name(element.tag)
        current_inside = inside or local == "Directory"
        if current_inside and local == "Item":
            attrs = {_local_name(key): value for key, value in element.attrib.items()}
            mime = attrs.get("Mime", "")
            semantic = attrs.get("Semantic", "")
            if not mime or not semantic:
                raise MotionPhotoError("Motion Photo container item lacks Mime/Semantic")
            if len(mime.encode()) > MAX_METADATA_STRING or len(semantic.encode()) > MAX_METADATA_STRING:
                raise MotionPhotoError("Motion Photo metadata string exceeds safety limit")
            items.append(MotionPhotoItem(
                mime=mime,
                semantic=semantic,
                length=_checked_nonnegative(attrs.get("Length"), default=0),
                padding=_checked_nonnegative(attrs.get("Padding"), default=0),
            ))
            if len(items) > MAX_DIRECTORY_ITEMS:
                raise MotionPhotoError("Motion Photo directory exceeds item limit")
        for child in list(element):
            visit(child, current_inside)

    visit(root, False)
    return tuple(items)


def _parse_standard_xmp(xmp: bytes) -> tuple[bool, int | None, int | None, int | None, tuple[MotionPhotoItem, ...]]:
    root = _safe_xml_root(xmp)
    attrs = _find_description_attributes(root)
    flag_raw = attrs.get("MotionPhoto", attrs.get("MicroVideo"))
    enabled = flag_raw == "1"
    version = int(attrs["MotionPhotoVersion"]) if attrs.get("MotionPhotoVersion", "").isdigit() else None
    timestamp: int | None = None
    for name in ("MotionPhotoPresentationTimestampUs", "MicroVideoPresentationTimestampUs"):
        raw = attrs.get(name)
        if raw is None:
            continue
        try:
            value = int(raw)
        except ValueError as exc:
            raise MotionPhotoError("invalid Motion Photo presentation timestamp") from exc
        timestamp = None if value == -1 else value
        break
    legacy_offset = None
    if attrs.get("MicroVideoOffset") is not None:
        legacy_offset = _checked_nonnegative(attrs["MicroVideoOffset"])
    return enabled, version, timestamp, legacy_offset, _parse_directory(root)


def _validate_directory(items: tuple[MotionPhotoItem, ...]) -> None:
    if not (2 <= len(items) <= MAX_DIRECTORY_ITEMS):
        raise MotionPhotoError("invalid Motion Photo container directory")
    if items[0].semantic.lower() != "primary" or items[0].length != 0:
        raise MotionPhotoError("Motion Photo must begin with one zero-length Primary item")
    if any(item.semantic.lower() == "primary" for item in items[1:]):
        raise MotionPhotoError("Motion Photo contains multiple Primary items")
    motion = [index for index, item in enumerate(items) if item.semantic.lower() == "motionphoto"]
    if motion != [len(items) - 1]:
        raise MotionPhotoError("MotionPhoto resource must be unique and last")
    last = items[-1]
    if last.mime.lower() not in {"video/mp4", "video/quicktime"} or last.length <= 0:
        raise MotionPhotoError("invalid MotionPhoto video resource")
    if any(item.length < 0 or item.padding < 0 for item in items):
        raise MotionPhotoError("negative Motion Photo item length")
    if any(item.padding != 0 for item in items[1:]):
        raise MotionPhotoError("secondary Motion Photo padding is unsupported")


def jpeg_resource_ranges(items: tuple[MotionPhotoItem, ...], file_size: int) -> tuple[ByteRange, ...]:
    """Resolve tightly packed Android JPEG resources in directory order."""
    _validate_directory(items)
    starts = [0] * len(items)
    ends = [0] * len(items)
    cursor = file_size
    for index in range(len(items) - 1, -1, -1):
        item = items[index]
        end = cursor
        if index == 0:
            unpadded = end - item.padding
            if unpadded < 0:
                raise MotionPhotoError("Motion Photo primary padding exceeds file size")
            starts[index], ends[index], cursor = 0, unpadded, 0
        else:
            start = cursor - item.length
            if start < 0:
                raise MotionPhotoError("Motion Photo item range exceeds file size")
            starts[index], ends[index], cursor = start, end, start
    if starts[0] != 0 or ends[-1] != file_size:
        raise MotionPhotoError("invalid Motion Photo resource ranges")
    return tuple(ByteRange(starts[i], ends[i]) for i in range(len(items)))


def _read_box_header(stream, offset: int, upper_bound: int) -> _Box | None:
    if offset < 0 or upper_bound - offset < 8:
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
        size = upper_bound - offset
    else:
        size = size32
    if size < header_size or offset + size > upper_bound:
        return None
    return _Box(offset, int(size), kind, header_size)


def is_ftyp_start(path: Path, offset: int, upper_bound: int) -> bool:
    with path.open("rb") as stream:
        box = _read_box_header(stream, offset, upper_bound)
        if box is None or box.kind != b"ftyp" or box.size < box.header_size + 8:
            return False
        stream.seek(box.payload_offset)
        brand = stream.read(4)
        return len(brand) == 4 and all(0x20 <= value <= 0x7E for value in brand)


def ftyp_offsets(path: Path, byte_range: ByteRange, *, buffer_size: int = 1 << 20) -> tuple[int, ...]:
    if buffer_size < 64:
        raise MotionPhotoError("ftyp scanner buffer is too small")
    rough: set[int] = set()
    with path.open("rb") as stream:
        stream.seek(byte_range.start)
        remaining = byte_range.length
        absolute = byte_range.start
        carry = b""
        while remaining > 0:
            chunk = stream.read(min(buffer_size, remaining))
            if not chunk:
                break
            window = carry + chunk
            window_start = absolute - len(carry)
            pos = 0
            while True:
                found = window.find(b"ftyp", pos)
                if found < 0:
                    break
                if found >= 4:
                    candidate = window_start + found - 4
                    if byte_range.start <= candidate < byte_range.end:
                        rough.add(candidate)
                pos = found + 4
            carry = window[-32:]
            absolute += len(chunk)
            remaining -= len(chunk)
    return tuple(offset for offset in sorted(rough) if is_ftyp_start(path, offset, byte_range.end))


def _heif_ranges(path: Path, items: tuple[MotionPhotoItem, ...], file_size: int) -> tuple[ByteRange, ByteRange]:
    primary, motion = items[0], items[-1]
    if primary.mime.lower() not in {"image/heic", "image/heif"} or primary.padding != 8:
        raise MotionPhotoError("HEIF Motion Photo requires Primary padding=8")
    boxes: list[_Box] = []
    with path.open("rb") as stream:
        cursor = 0
        while cursor < file_size:
            if len(boxes) >= 4096:
                raise MotionPhotoError("too many HEIF top-level boxes")
            box = _read_box_header(stream, cursor, file_size)
            if box is None:
                raise MotionPhotoError("invalid HEIF top-level box")
            boxes.append(box)
            cursor = box.end
    if not boxes or boxes[0].kind != b"ftyp":
        raise MotionPhotoError("HEIF Motion Photo lacks ftyp")
    mpvd = [box for box in boxes if box.kind == b"mpvd"]
    if len(mpvd) != 1:
        raise MotionPhotoError("HEIF Motion Photo must contain one mpvd box")
    box = mpvd[0]
    payload_start = box.payload_offset
    if file_size - motion.length != payload_start:
        raise MotionPhotoError("HEIF Motion Photo directory does not point at mpvd payload")
    if not is_ftyp_start(path, payload_start, box.end):
        raise MotionPhotoError("mpvd payload is not ISO BMFF video")
    return ByteRange(0, box.offset), ByteRange(payload_start, box.end)


def parse_android_motion_photo(path: str | os.PathLike[str]) -> MotionPhotoAsset | None:
    source = Path(path)
    size = source.stat().st_size
    if size < 16:
        raise MotionPhotoError("Motion Photo input is too small")
    xmp = _extract_xmp_prefix(source)
    if xmp is None:
        return None
    enabled, version, timestamp, legacy_offset, items = _parse_standard_xmp(xmp)
    if not enabled:
        return None
    if items:
        if version != 1:
            raise MotionPhotoError(f"unsupported Motion Photo version: {version!r}")
        _validate_directory(items)
        if items[0].mime.lower() in {"image/heic", "image/heif"}:
            still_range, video_range = _heif_ranges(source, items, size)
            source_kind = "androidHeifMotionPhotoV1"
        else:
            ranges = jpeg_resource_ranges(items, size)
            still_range = ByteRange(0, ranges[-1].start)
            video_range = ranges[-1]
            if not is_ftyp_start(source, video_range.start, video_range.end):
                raise MotionPhotoError("Motion Photo video is not a valid ISO BMFF stream")
            source_kind = "androidMotionPhotoV1"
        presentation_source = "androidXMP" if timestamp is not None else None
    elif legacy_offset is not None:
        if legacy_offset <= 0 or legacy_offset > size:
            raise MotionPhotoError("invalid legacy MicroVideo offset")
        items = (
            MotionPhotoItem("image/jpeg", "Primary", 0, 0),
            MotionPhotoItem("video/mp4", "MotionPhoto", legacy_offset, 0),
        )
        video_range = ByteRange(size - legacy_offset, size)
        still_range = ByteRange(0, video_range.start)
        if not is_ftyp_start(source, video_range.start, video_range.end):
            raise MotionPhotoError("legacy MicroVideo payload is not ISO BMFF")
        source_kind = "legacyMicroVideoV1b"
        presentation_source = "legacyMicroVideoXMP" if timestamp is not None else None
    else:
        raise MotionPhotoError("Motion Photo directory is missing")
    return MotionPhotoAsset(
        source=source,
        source_kind=source_kind,
        items=items,
        still_range=still_range,
        video_range=video_range,
        presentation_timestamp_us=timestamp,
        presentation_source=presentation_source,
    )


def _extract_balanced_json(data: bytes, brace: int) -> bytes | None:
    depth = 0
    in_string = False
    escaping = False
    for index in range(brace, min(len(data), brace + MAX_LPEX_JSON_BYTES + 1)):
        byte = data[index]
        if in_string:
            if escaping:
                escaping = False
            elif byte == 0x5C:
                escaping = True
            elif byte == 0x22:
                in_string = False
            continue
        if byte == 0x22:
            in_string = True
        elif byte == 0x7B:
            depth += 1
        elif byte == 0x7D:
            depth -= 1
            if depth == 0:
                return data[brace:index + 1]
            if depth < 0:
                return None
    return None


def _matrix(value) -> tuple[float, ...] | None:
    if not isinstance(value, list) or len(value) != 9:
        return None
    try:
        values = tuple(float(item) for item in value)
    except (TypeError, ValueError):
        return None
    if any(not (-1e308 < item < 1e308) for item in values):
        return None
    return values


def _number_tuple(value, limit: int) -> tuple[float, ...] | None:
    if not isinstance(value, list) or len(value) > limit:
        return None
    try:
        values = tuple(float(item) for item in value)
    except (TypeError, ValueError):
        return None
    if any(not (-1e308 < item < 1e308) for item in values):
        return None
    return values


def _parse_lpex_object(raw: bytes) -> OppoMetadata | None:
    try:
        obj = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError):
        return None
    if not isinstance(obj, dict):
        return None

    def integer(name: str) -> int | None:
        value = obj.get(name)
        if isinstance(value, bool):
            return None
        try:
            return int(value) if value is not None else None
        except (TypeError, ValueError):
            return None

    def size(name: str) -> tuple[int | None, int | None]:
        value = obj.get(name)
        if not isinstance(value, list) or len(value) < 2:
            return None, None
        try:
            width, height = int(value[0]), int(value[1])
        except (TypeError, ValueError):
            return None, None
        return (width, height) if width > 0 and height > 0 else (None, None)

    matrices: list[tuple[int, tuple[float, ...]]] = []
    raw_matrices = obj.get("matrices")
    if isinstance(raw_matrices, dict) and len(raw_matrices) <= 4096:
        for key, value in raw_matrices.items():
            parsed = _matrix(value)
            if parsed is None:
                continue
            try:
                timestamp = int(key)
            except (TypeError, ValueError):
                continue
            matrices.append((timestamp, parsed))
    matrices.sort(key=lambda pair: pair[0])
    vw, vh = size("videoSize")
    ow, oh = size("originPhotoSize")
    photo_crop_factor = obj.get("photoCropFactor")
    try:
        photo_crop_factor = float(photo_crop_factor) if photo_crop_factor is not None else None
    except (TypeError, ValueError):
        photo_crop_factor = None
    return OppoMetadata(
        cover_frame_pts_us=integer("coverFramePts"),
        version=integer("version") or 0,
        matrix_count=integer("matrixCount") or 0,
        photo_crop_matrix=_matrix(obj.get("photoCropMatrix")),
        photo_eis_matrix=_matrix(obj.get("photoEisMatrix")),
        matrices=tuple(matrices),
        video_width=vw,
        video_height=vh,
        origin_photo_width=ow,
        origin_photo_height=oh,
        photo_eis_crop_factor=_number_tuple(obj.get("photoEisCropFactor"), 8),
        eis_crop_factor=_number_tuple(obj.get("eisCropFactor"), 8),
        photo_crop_factor=photo_crop_factor,
        stream_count=1,
    )


def parse_oppo_lpex(path: str | os.PathLike[str]) -> OppoMetadata | None:
    source = Path(path)
    needles = (b"lpexLivePhotoExtension", b"LivePhotoExtension", b"pexLivePhotoExtension")
    overlap = MAX_LPEX_JSON_BYTES + 128
    carry = b""
    with source.open("rb") as stream:
        while True:
            chunk = stream.read(SCAN_CHUNK_BYTES)
            if not chunk:
                break
            window = carry + chunk
            for needle in needles:
                search = 0
                while True:
                    found = window.find(needle, search)
                    if found < 0:
                        break
                    brace = window.find(b"{", found + len(needle), min(len(window), found + len(needle) + 33))
                    if brace >= 0:
                        raw = _extract_balanced_json(window, brace)
                        if raw is not None:
                            parsed = _parse_lpex_object(raw)
                            if parsed is not None:
                                return parsed
                    search = found + len(needle)
            carry = window[-overlap:]
    return None


def _xmp_text(path: Path) -> str | None:
    xmp = _extract_xmp_prefix(path)
    return xmp.decode("utf-8", errors="replace") if xmp else None


def _xmp_integer(text: str, names: Iterable[str]) -> int | None:
    for name in names:
        escaped = re.escape(name)
        patterns = (
            rf"<{escaped}>([^<]+)</{escaped}>",
            rf"{escaped}\s*=\s*[\"']([^\"']+)[\"']",
        )
        for pattern in patterns:
            match = re.search(pattern, text)
            if match:
                try:
                    return int(match.group(1).strip())
                except ValueError:
                    pass
    return None


def _oppo_fallback(path: Path, lpex: OppoMetadata | None) -> MotionPhotoAsset | None:
    text = _xmp_text(path) or ""
    has_signature = bool(lpex) or "OpCamera:" in text or re.search(r"oppo|oplus", text, re.I) is not None
    if not has_signature:
        return None
    size = path.stat().st_size
    tail = ByteRange(max(0, size - MAX_VENDOR_TAIL_SCAN_BYTES), size)
    offsets = ftyp_offsets(path, tail)
    if not offsets:
        return None
    declared_lengths: list[int] = []
    for match in re.finditer(r"(?:Item:)?Length\s*(?:=\s*[\"']?|>)(\d+)", text):
        try:
            value = int(match.group(1))
        except ValueError:
            continue
        if value > 100_000:
            declared_lengths.append(value)
    for name in ("OpCamera:VideoLength", "GCamera:VideoLength", "VideoLength"):
        value = _xmp_integer(text, [name])
        if value and value > 100_000:
            declared_lengths.append(value)
    presentation = _xmp_integer(text, (
        "GCamera:MotionPhotoPresentationTimestampUs",
        "MotionPhotoPresentationTimestampUs",
        "GCamera:MicroVideoPresentationTimestampUs",
    ))
    stream_count = 1
    if lpex and lpex.version >= 1 and len(offsets) >= 2:
        video_start = offsets[-2]
        stream_count = 2
    else:
        video_start = -1
        for length in sorted(declared_lengths, reverse=True):
            if 0 < length <= size and is_ftyp_start(path, size - length, size):
                video_start = size - length
                break
        if video_start < 0:
            video_start = offsets[-1]
    metadata = replace(lpex or OppoMetadata(), stream_count=stream_count)
    selected = presentation if presentation is not None else metadata.cover_frame_pts_us
    source_name = "androidXMP" if presentation is not None else ("oppoCoverFrame" if metadata.cover_frame_pts_us is not None else None)
    video_range = ByteRange(video_start, size)
    return MotionPhotoAsset(
        source=path,
        source_kind="oppoLivePhoto",
        items=(
            MotionPhotoItem("image/jpeg", "Primary", 0, 0),
            MotionPhotoItem("video/mp4", "MotionPhoto", video_range.length, 0),
        ),
        still_range=ByteRange(0, video_start),
        video_range=video_range,
        presentation_timestamp_us=selected,
        presentation_source=source_name,
        vendor_metadata=metadata,
    )


def parse_motion_photo(path: str | os.PathLike[str]) -> MotionPhotoAsset | None:
    source = Path(path)
    lpex = parse_oppo_lpex(source)
    try:
        base = parse_android_motion_photo(source)
    except MotionPhotoError:
        fallback = _oppo_fallback(source, lpex)
        if fallback is not None:
            return fallback
        raise
    if base is None:
        return _oppo_fallback(source, lpex)
    if lpex is None:
        return base
    size = source.stat().st_size
    offsets = ftyp_offsets(source, ByteRange(max(0, size - MAX_VENDOR_TAIL_SCAN_BYTES), size))
    metadata = lpex
    still_range, video_range = base.still_range, base.video_range
    if lpex.version >= 1 and len(offsets) >= 2:
        still_range = ByteRange(0, offsets[-2])
        video_range = ByteRange(offsets[-2], size)
        metadata = replace(lpex, stream_count=2)
    else:
        inside = [value for value in offsets if base.video_range.start <= value < base.video_range.end]
        metadata = replace(lpex, stream_count=max(1, len(inside)))
    selected = base.presentation_timestamp_us if base.presentation_timestamp_us is not None else metadata.cover_frame_pts_us
    selected_source = base.presentation_source if base.presentation_timestamp_us is not None else (
        "oppoCoverFrame" if metadata.cover_frame_pts_us is not None else None
    )
    return MotionPhotoAsset(
        source=source,
        source_kind="oppoLivePhoto",
        items=base.items,
        still_range=still_range,
        video_range=video_range,
        presentation_timestamp_us=selected,
        presentation_source=selected_source,
        vendor_metadata=metadata,
    )


def primary_video_range(asset: MotionPhotoAsset) -> ByteRange:
    metadata = asset.vendor_metadata
    if asset.source_kind != "oppoLivePhoto" or metadata is None or metadata.stream_count < 2:
        return asset.video_range
    offsets = ftyp_offsets(asset.source, asset.video_range)
    if len(offsets) < 2:
        return asset.video_range
    return ByteRange(offsets[-2], offsets[-1])


def copy_range(source: Path, byte_range: ByteRange, destination: Path, *, chunk_size: int = 1 << 20) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    remaining = byte_range.length
    with source.open("rb") as src, destination.open("wb") as dst:
        src.seek(byte_range.start)
        while remaining:
            chunk = src.read(min(chunk_size, remaining))
            if not chunk:
                raise MotionPhotoError("source ended while copying Motion Photo resource")
            dst.write(chunk)
            remaining -= len(chunk)