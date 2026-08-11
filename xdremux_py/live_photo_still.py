"""Cross-platform Apple Live Photo still writer.

JPEG Motion Photos are encoded as HEIC with pillow-heif. Ultra HDR gain maps
flow through XDRemux's existing ISO 21496 writer. HEIC Motion Photo stills are
not decoded: their encoded image/gain-map graph is preserved and only metadata
item locations are redirected to replacement EXIF/XMP bytes.

No ImageIO, CoreGraphics, PyObjC, or other Apple runtime API is used.
"""

from __future__ import annotations

import io
import math
import re
import struct
import xml.etree.ElementTree as ET
from dataclasses import dataclass
from pathlib import Path

from PIL import Image, ImageOps

from . import heif_io
from .isobmff_patch import _boxes, _fullbox, _parse_all_items, _rn, _wn
from .motion_photo import ByteRange, MotionPhotoAsset, MotionPhotoError, jpeg_resource_ranges

APPLE_MAKERNOTE_TAG = 37500
MOTION_XMP_MARKERS = (b"MotionPhoto", b"MicroVideo", b"GContainer", b"Container:Directory")


class LivePhotoStillError(ValueError):
    pass


def build_apple_makernote(content_identifier: str) -> bytes:
    """Build an Apple iOS MakerNote carrying tag 17 ContentIdentifier."""
    try:
        identifier = content_identifier.upper().encode("ascii")
    except UnicodeEncodeError as exc:
        raise LivePhotoStillError("Live Photo content identifier must be ASCII") from exc
    if not identifier or b"\0" in identifier:
        raise LivePhotoStillError("invalid Live Photo content identifier")

    entries: list[tuple[int, int, int, int | bytes]] = [
        (0x0001, 9, 1, 16),
        (0x0011, 2, len(identifier) + 1, identifier + b"\0"),
        (0x0014, 9, 1, 12),
        (0x0017, 16, 1, (0).to_bytes(8, "big")),
        (0x001F, 9, 1, 0),
    ]
    header = bytearray(b"Apple iOS\0\0\x01MM")
    header.extend(struct.pack(">H", len(entries)))
    data_base = len(header) + len(entries) * 12 + 4
    variable = bytearray()
    result = bytearray(header)
    for tag, field_type, count, value in entries:
        if isinstance(value, bytes):
            offset = data_base + len(variable)
            result.extend(struct.pack(">HHII", tag, field_type, count, offset))
            variable.extend(value)
            if len(variable) & 1:
                variable.append(0)
        else:
            result.extend(struct.pack(">HHII", tag, field_type, count, value & 0xFFFFFFFF))
    result.extend(struct.pack(">I", 0))
    result.extend(variable)
    return bytes(result)


def _inject_makernote(exif_bytes: bytes | None, content_identifier: str, *, orientation: int | None = None) -> bytes:
    exif = Image.Exif()
    if exif_bytes:
        try:
            exif.load(exif_bytes)
        except Exception:
            exif = Image.Exif()
    exif[APPLE_MAKERNOTE_TAG] = build_apple_makernote(content_identifier)
    if orientation is not None:
        exif[274] = orientation
    return exif.tobytes()


def _clean_motion_xmp(xmp: bytes) -> bytes:
    text = xmp.decode("utf-8", errors="replace")
    text = re.sub(
        r"<(?:G?Container):Directory\b.*?</(?:G?Container):Directory\s*>",
        "", text, flags=re.I | re.S,
    )
    fields = (
        "MotionPhoto", "MotionPhotoVersion", "MotionPhotoPresentationTimestampUs",
        "MicroVideo", "MicroVideoVersion", "MicroVideoOffset",
        "MicroVideoPresentationTimestampUs", "VideoLength",
    )
    for field in fields:
        text = re.sub(
            rf"\s+(?:G?Camera|OpCamera):{field}\s*=\s*([\"']).*?\1",
            "", text, flags=re.I | re.S,
        )
        text = re.sub(
            rf"<(?:G?Camera|OpCamera):{field}\b[^>]*>.*?</(?:G?Camera|OpCamera):{field}\s*>",
            "", text, flags=re.I | re.S,
        )
    return text.encode("utf-8")


def _extract_xmp(data: bytes) -> bytes | None:
    starts = [value for value in (data.find(b"<x:xmpmeta"), data.find(b"<xmpmeta")) if value >= 0]
    if not starts:
        return None
    start = min(starts)
    ends = []
    for closing in (b"</x:xmpmeta>", b"</xmpmeta>"):
        pos = data.find(closing, start)
        if pos >= 0:
            ends.append(pos + len(closing))
    return data[start:min(ends)] if ends else None


def _parse_number_list(value: str, default: float) -> list[float]:
    parts = [piece for piece in re.split(r"[,\s]+", value.strip()) if piece]
    if not parts:
        return [default]
    try:
        parsed = [float(piece) for piece in parts]
    except ValueError as exc:
        raise LivePhotoStillError(f"invalid Ultra HDR gain-map value: {value}") from exc
    if any(not math.isfinite(item) for item in parsed):
        raise LivePhotoStillError("non-finite Ultra HDR gain-map value")
    return parsed


def _local_xml_name(name: str) -> str:
    if name.startswith("{") and "}" in name:
        return name.split("}", 1)[1]
    return name.split(":", 1)[-1]


def _xmp_values(root: ET.Element, name: str, default: float) -> list[float]:
    target = name.lower()
    for element in root.iter():
        if _local_xml_name(element.tag).lower() == target:
            children = [
                (child.text or "").strip() for child in element.iter()
                if child is not element and _local_xml_name(child.tag).lower() == "li" and (child.text or "").strip()
            ]
            if children:
                return [_parse_number_list(value, default)[0] for value in children]
            if (element.text or "").strip():
                return _parse_number_list((element.text or "").strip(), default)
        for attr_name, attr_value in element.attrib.items():
            if _local_xml_name(attr_name).lower() == target:
                return _parse_number_list(attr_value, default)
    return [default]


def _xmp_boolean(root: ET.Element, name: str, default: bool = False) -> bool:
    target = name.lower()
    for element in root.iter():
        if _local_xml_name(element.tag).lower() == target and (element.text or "").strip():
            return (element.text or "").strip().lower() in {"1", "true", "yes"}
        for attr_name, attr_value in element.attrib.items():
            if _local_xml_name(attr_name).lower() == target:
                return attr_value.strip().lower() in {"1", "true", "yes"}
    return default


def parse_ultrahdr_metadata(gainmap_jpeg: bytes) -> dict:
    xmp = _extract_xmp(gainmap_jpeg)
    if xmp is None:
        raise LivePhotoStillError("Ultra HDR GainMap resource lacks hdrgm XMP")
    upper = xmp.upper()
    if b"<!DOCTYPE" in upper or b"<!ENTITY" in upper:
        raise LivePhotoStillError("DTD/entity declarations are forbidden in Ultra HDR XMP")
    try:
        root = ET.fromstring(xmp)
    except ET.ParseError as exc:
        raise LivePhotoStillError("Ultra HDR GainMap XMP is malformed") from exc
    gain_max = _xmp_values(root, "GainMapMax", 1.0)
    return {
        "gainMapMin": _xmp_values(root, "GainMapMin", 0.0),
        "gainMapMax": gain_max,
        "gamma": _xmp_values(root, "Gamma", 1.0),
        "offsetSdr": _xmp_values(root, "OffsetSDR", 0.0),
        "offsetHdr": _xmp_values(root, "OffsetHDR", 0.0),
        "hdrCapacityMin": _xmp_values(root, "HDRCapacityMin", 0.0)[0],
        "hdrCapacityMax": _xmp_values(root, "HDRCapacityMax", gain_max[0])[0],
        "baseRenditionIsHDR": _xmp_boolean(root, "BaseRenditionIsHDR", False),
        "useBaseColorSpace": True,
    }


def _transpose_gainmap(image: Image.Image, orientation: int) -> Image.Image:
    method = {
        2: Image.Transpose.FLIP_LEFT_RIGHT, 3: Image.Transpose.ROTATE_180,
        4: Image.Transpose.FLIP_TOP_BOTTOM, 5: Image.Transpose.TRANSPOSE,
        6: Image.Transpose.ROTATE_270, 7: Image.Transpose.TRANSVERSE,
        8: Image.Transpose.ROTATE_90,
    }.get(orientation)
    return image.transpose(method) if method is not None else image


def _jpeg_gainmap_range(asset: MotionPhotoAsset) -> ByteRange | None:
    if asset.source_kind == "legacyMicroVideoV1b":
        return None
    try:
        ranges = jpeg_resource_ranges(asset.items, asset.source.stat().st_size)
    except MotionPhotoError:
        return None
    for item, byte_range in zip(asset.items, ranges):
        if item.semantic.lower() == "gainmap" and byte_range.end <= asset.still_range.end:
            return byte_range
    return None


def _read_range(path: Path, byte_range: ByteRange) -> bytes:
    with path.open("rb") as stream:
        stream.seek(byte_range.start)
        data = stream.read(byte_range.length)
    if len(data) != byte_range.length:
        raise LivePhotoStillError("truncated Motion Photo still resource")
    return data


def _jpeg_still(asset: MotionPhotoAsset, output: Path, content_identifier: str) -> bool:
    static_bytes = _read_range(asset.source, asset.still_range)
    gain_range = _jpeg_gainmap_range(asset)
    with Image.open(io.BytesIO(static_bytes)) as source_image:
        source_image.load()
        original_orientation = int(source_image.getexif().get(274, 1) or 1)
        base = ImageOps.exif_transpose(source_image).convert("RGB")
        source_exif = source_image.info.get("exif")
        source_icc = source_image.info.get("icc_profile")
    exif_blob = _inject_makernote(source_exif, content_identifier, orientation=1)
    base.info["exif"] = exif_blob
    if source_icc:
        base.info["icc_profile"] = source_icc
    output.parent.mkdir(parents=True, exist_ok=True)
    if gain_range is not None:
        gain_bytes = _read_range(asset.source, gain_range)
        iso_meta = parse_ultrahdr_metadata(gain_bytes)
        with Image.open(io.BytesIO(gain_bytes)) as gain_source:
            gain_source.load()
            gain = _transpose_gainmap(gain_source.copy(), original_orientation)
        heif_io.write_heic(
            str(output), base, gain, iso_meta,
            oppo_compat=False, lhdr=None, exif_data=exif_blob,
        )
        return True
    import pillow_heif
    heif = pillow_heif.from_pillow(base)
    heif.save(str(output), quality=95, exif=exif_blob, xmp=None, save_nclx_profile=True)
    return False


@dataclass
class _IlocEntry:
    item_id: int
    construction_pos: int | None
    base_pos: int | None
    base_size: int
    extent_offset_pos: int
    extent_offset_size: int
    extent_length_pos: int
    extent_length_size: int
    old_offset: int
    old_length: int


def _iloc_entries(data: bytearray, ds: int, de: int, *, idat_payload_offset: int | None = None) -> dict[int, _IlocEntry]:
    version, _, body = _fullbox(data, ds)
    b0 = data[body]
    offset_size, length_size = (b0 >> 4) & 0xF, b0 & 0xF
    b1 = data[body + 1]
    base_size = (b1 >> 4) & 0xF
    index_size = (b1 & 0xF) if version in (1, 2) else 0
    count_size = 2 if version < 2 else 4
    cursor = body + 2
    count = int.from_bytes(data[cursor:cursor + count_size], "big")
    cursor += count_size
    result: dict[int, _IlocEntry] = {}
    for _ in range(count):
        item_id_size = 2 if version < 2 else 4
        item_id = int.from_bytes(data[cursor:cursor + item_id_size], "big")
        cursor += item_id_size
        construction_pos = None
        construction_method = 0
        if version in (1, 2):
            construction_pos = cursor
            construction_method = int.from_bytes(data[cursor:cursor + 2], "big") & 0xF
            cursor += 2
        data_reference_index = int.from_bytes(data[cursor:cursor + 2], "big")
        cursor += 2
        base_pos = cursor if base_size else None
        base_offset, cursor = _rn(base_size, data, cursor)
        extent_count = int.from_bytes(data[cursor:cursor + 2], "big")
        cursor += 2
        if data_reference_index != 0 or extent_count != 1:
            for _ in range(extent_count):
                cursor += index_size + offset_size + length_size
            continue
        if index_size:
            cursor += index_size
        offset_pos = cursor
        extent_offset, cursor = _rn(offset_size, data, cursor)
        length_pos = cursor
        extent_length, cursor = _rn(length_size, data, cursor)
        if construction_method == 0:
            absolute = base_offset + extent_offset
        elif construction_method == 1 and idat_payload_offset is not None:
            absolute = idat_payload_offset + base_offset + extent_offset
        else:
            absolute = -1
        result[item_id] = _IlocEntry(
            item_id, construction_pos, base_pos, base_size,
            offset_pos, offset_size, length_pos, length_size, absolute, extent_length,
        )
    if cursor > de:
        raise LivePhotoStillError("truncated HEIF iloc")
    return result


def _item_payload(data: bytearray, entry: _IlocEntry) -> bytes | None:
    if entry.old_offset < 0 or entry.old_offset + entry.old_length > len(data):
        return None
    return bytes(data[entry.old_offset:entry.old_offset + entry.old_length])


def _heif_exif_replacement(old_payload: bytes, content_identifier: str) -> bytes:
    if len(old_payload) < 8:
        raise LivePhotoStillError("HEIF Exif item is truncated")
    tiff_offset = struct.unpack_from(">I", old_payload, 0)[0]
    start = 4 + tiff_offset
    if start + 8 > len(old_payload):
        raise LivePhotoStillError("HEIF Exif TIFF offset is invalid")
    modified = _inject_makernote(b"Exif\0\0" + old_payload[start:], content_identifier)
    if not modified.startswith(b"Exif\0\0"):
        raise LivePhotoStillError("Pillow produced invalid EXIF payload")
    return struct.pack(">I", 0) + modified[6:]


def _heif_still(asset: MotionPhotoAsset, output: Path, content_identifier: str) -> bool:
    data = bytearray(_read_range(asset.source, asset.still_range))
    meta_info = next(((ds, de) for kind, ds, de, _, _ in _boxes(data, 0, len(data)) if kind == "meta"), None)
    if meta_info is None:
        raise LivePhotoStillError("HEIC Motion Photo static resource lacks meta box")
    meta_ds, meta_de = meta_info
    _, _, meta_body = _fullbox(data, meta_ds)
    children = {kind: (ds, de) for kind, ds, de, _, _ in _boxes(data, meta_body, meta_de)}
    if "iinf" not in children or "iloc" not in children:
        raise LivePhotoStillError("HEIC Motion Photo lacks iinf/iloc")
    idat_payload_offset = children.get("idat", (None, None))[0]
    item_types, _ = _parse_all_items(data, *children["iinf"])
    entries = _iloc_entries(data, *children["iloc"], idat_payload_offset=idat_payload_offset)
    exif_ids = [item_id for item_id, kind in item_types.items() if kind == "Exif"]
    if not exif_ids:
        raise LivePhotoStillError("HEIC Motion Photo has no Exif item to carry Live Photo identifier")
    exif_entry = entries.get(exif_ids[0])
    if exif_entry is None or (old_exif := _item_payload(data, exif_entry)) is None:
        raise LivePhotoStillError("HEIC Exif item has unsupported iloc geometry")
    replacements: list[tuple[_IlocEntry, bytes]] = [
        (exif_entry, _heif_exif_replacement(old_exif, content_identifier))
    ]
    for item_id, kind in item_types.items():
        if kind != "mime" or (entry := entries.get(item_id)) is None:
            continue
        old = _item_payload(data, entry)
        if old is not None and any(marker in old for marker in MOTION_XMP_MARKERS):
            replacements.append((entry, _clean_motion_xmp(old)))

    payload = bytearray()
    absolute_base = len(data) + 8
    for entry, replacement in replacements:
        absolute = absolute_base + len(payload)
        if entry.extent_offset_size == 0 or entry.extent_length_size == 0:
            raise LivePhotoStillError("HEIF metadata iloc cannot address replacement payload")
        if absolute >= 1 << (entry.extent_offset_size * 8) or len(replacement) >= 1 << (entry.extent_length_size * 8):
            raise LivePhotoStillError("HEIF metadata replacement exceeds iloc field width")
        if entry.construction_pos is not None:
            word = int.from_bytes(data[entry.construction_pos:entry.construction_pos + 2], "big")
            data[entry.construction_pos:entry.construction_pos + 2] = (word & ~0xF).to_bytes(2, "big")
        if entry.base_pos is not None:
            _wn(entry.base_size, 0, data, entry.base_pos)
        _wn(entry.extent_offset_size, absolute, data, entry.extent_offset_pos)
        _wn(entry.extent_length_size, len(replacement), data, entry.extent_length_pos)
        payload.extend(replacement)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(bytes(data) + struct.pack(">I4s", 8 + len(payload), b"mdat") + payload)
    return _heif_has_iso_gainmap(output)


def _heif_has_iso_gainmap(path: Path) -> bool:
    data = path.read_bytes()
    return b"urn:iso:std:iso:ts:21496:-1" in data or b"hdrgm" in data or b"tmap" in data


def write_live_photo_still(asset: MotionPhotoAsset, output: Path, content_identifier: str) -> bool:
    if asset.source_kind == "androidHeifMotionPhotoV1":
        return _heif_still(asset, Path(output), content_identifier)
    return _jpeg_still(asset, Path(output), content_identifier)


def _maker_identifier(maker: bytes | bytearray) -> str | None:
    data = bytes(maker)
    if not data.startswith(b"Apple iOS") or len(data) < 16:
        return None
    endian = ">" if data[12:14] == b"MM" else "<" if data[12:14] == b"II" else None
    if endian is None:
        return None
    count = struct.unpack_from(endian + "H", data, 14)[0]
    for index in range(count):
        cursor = 16 + index * 12
        if cursor + 12 > len(data):
            return None
        tag, field_type, item_count, value_or_offset = struct.unpack_from(endian + "HHII", data, cursor)
        if tag != 0x0011 or field_type != 2:
            continue
        if item_count <= 4:
            raw = struct.pack(endian + "I", value_or_offset)[:item_count]
        else:
            if value_or_offset + item_count > len(data):
                return None
            raw = data[value_or_offset:value_or_offset + item_count]
        try:
            return raw.rstrip(b"\0").decode("ascii")
        except UnicodeDecodeError:
            return None
    return None


def read_apple_content_identifier(path: Path) -> str | None:
    import pillow_heif
    pillow_heif.register_heif_opener()
    try:
        with Image.open(path) as image:
            maker = image.getexif().get(APPLE_MAKERNOTE_TAG)
    except Exception:
        return None
    return _maker_identifier(maker) if isinstance(maker, (bytes, bytearray)) else None
