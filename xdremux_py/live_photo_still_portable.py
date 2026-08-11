"""Standards-complete pure-Python Live Photo still entry point.

Android Motion Photo directories do not give the Primary JPEG a byte length.
The primary resource must be determined by parsing the JPEG itself; secondary
resources are then walked forward in directory order. This matters for Ultra HDR
Motion Photos because deriving every resource backwards from EOF can locate the
trailing MotionPhoto video correctly while assigning the wrong bytes to an
intermediate GainMap item.

This module therefore parses the primary JPEG to EOI, walks secondary resources
forward, supports zero-length shared resources, and feeds the existing XDRemux
ISO 21496 HEIC writer. It uses no Apple platform API.
"""

from __future__ import annotations

import io
from pathlib import Path

from PIL import Image, ImageOps

from . import heif_io
from . import live_photo_still as base_writer
from .motion_photo import ByteRange, MotionPhotoAsset
from .ultrahdr_iso import ISO21496JPEGMetadataError, parse_iso21496_jpeg_metadata


LivePhotoStillError = base_writer.LivePhotoStillError
read_apple_content_identifier = base_writer.read_apple_content_identifier


def _jpeg_end(data: bytes, start: int = 0) -> int:
    """Return the byte after EOI for one JPEG, including progressive multi-scan JPEGs."""
    if start < 0 or start + 2 > len(data) or data[start:start + 2] != b"\xff\xd8":
        raise LivePhotoStillError("JPEG/R primary image lacks SOI")
    cursor = start + 2
    entropy = False
    while cursor < len(data):
        if entropy:
            marker_start = data.find(b"\xff", cursor)
            if marker_start < 0:
                break
            marker_cursor = marker_start + 1
            while marker_cursor < len(data) and data[marker_cursor] == 0xFF:
                marker_cursor += 1
            if marker_cursor >= len(data):
                break
            marker = data[marker_cursor]
            if marker == 0x00 or 0xD0 <= marker <= 0xD7:
                cursor = marker_cursor + 1
                continue
            cursor = marker_start
            entropy = False
            continue

        if cursor >= len(data) or data[cursor] != 0xFF:
            raise LivePhotoStillError("malformed JPEG/R marker stream")
        while cursor < len(data) and data[cursor] == 0xFF:
            cursor += 1
        if cursor >= len(data):
            break
        marker = data[cursor]
        cursor += 1
        if marker == 0xD9:
            return cursor
        if marker == 0xD8:
            raise LivePhotoStillError("unexpected nested JPEG SOI")
        if marker == 0x01 or 0xD0 <= marker <= 0xD7:
            continue
        if cursor + 2 > len(data):
            raise LivePhotoStillError("truncated JPEG/R segment length")
        segment_length = int.from_bytes(data[cursor:cursor + 2], "big")
        if segment_length < 2 or cursor + segment_length > len(data):
            raise LivePhotoStillError("invalid JPEG/R segment length")
        is_sos = marker == 0xDA
        cursor += segment_length
        if is_sos:
            entropy = True
    raise LivePhotoStillError("JPEG/R image has no EOI")


def _gain_metadata(gain_jpeg: bytes) -> dict:
    try:
        return base_writer.parse_ultrahdr_metadata(gain_jpeg)
    except LivePhotoStillError as xmp_error:
        if "lacks hdrgm XMP" not in str(xmp_error):
            raise
    try:
        metadata = parse_iso21496_jpeg_metadata(gain_jpeg)
    except ISO21496JPEGMetadataError as exc:
        raise LivePhotoStillError(str(exc)) from exc
    if metadata is None:
        raise LivePhotoStillError(
            "Ultra HDR gain-map JPEG has neither hdrgm XMP nor ISO 21496-1 APP2 metadata"
        )
    return metadata


def _validated_gain_jpeg(data: bytes) -> tuple[bytes, dict] | None:
    """Return the first actual gain-map JPEG in one physical resource."""
    search = 0
    while search < len(data):
        candidate = data.find(b"\xff\xd8", search)
        if candidate < 0:
            return None
        try:
            end = _jpeg_end(data, candidate)
            gain_jpeg = data[candidate:end]
            metadata = _gain_metadata(gain_jpeg)
            with Image.open(io.BytesIO(gain_jpeg)) as image:
                image.load()
                if image.width <= 0 or image.height <= 0:
                    raise LivePhotoStillError("invalid gain-map dimensions")
            return gain_jpeg, metadata
        except (LivePhotoStillError, OSError):
            search = candidate + 2
    return None


def _forward_secondary_ranges(asset: MotionPhotoAsset) -> tuple[tuple[object, ByteRange | None], ...]:
    """Resolve JPEG Motion Photo resources in their normative forward order.

    The Primary length is determined by parsing the primary JPEG. A secondary
    item with Length=0 shares the immediately preceding physical resource and
    consumes no bytes. The MotionPhoto item is cross-checked against the already
    validated video start from the generic parser.
    """
    if not asset.items or asset.items[0].semantic.lower() != "primary":
        raise LivePhotoStillError("Motion Photo directory does not begin with Primary")
    static_bytes = base_writer._read_range(asset.source, asset.still_range)
    primary_end = _jpeg_end(static_bytes, 0)
    primary_padding = asset.items[0].padding
    cursor = primary_end + primary_padding
    if cursor > asset.still_range.end:
        raise LivePhotoStillError("Primary JPEG plus padding exceeds static resource")

    result: list[tuple[object, ByteRange | None]] = []
    previous_physical = ByteRange(0, primary_end)
    for item in asset.items[1:]:
        semantic = item.semantic.lower()
        if semantic == "motionphoto":
            if cursor != asset.video_range.start:
                raise LivePhotoStillError(
                    "Motion Photo secondary resource lengths do not reach the declared video start "
                    f"({cursor} != {asset.video_range.start})"
                )
            result.append((item, asset.video_range))
            break
        if item.length == 0:
            result.append((item, None))
            continue
        end = cursor + item.length
        if end < cursor or end > asset.still_range.end:
            raise LivePhotoStillError("secondary Motion Photo resource exceeds static image range")
        byte_range = ByteRange(cursor, end)
        result.append((item, byte_range))
        previous_physical = byte_range
        cursor = end + item.padding
        if cursor > asset.still_range.end:
            raise LivePhotoStillError("secondary Motion Photo padding exceeds static image range")

    if not any(item.semantic.lower() == "motionphoto" for item in asset.items):
        raise LivePhotoStillError("Motion Photo directory has no MotionPhoto item")
    return tuple(result)


def _embedded_gainmap_jpeg(static_bytes: bytes) -> tuple[bytes, dict]:
    """Locate a validated secondary gain JPEG in a shared JPEG/R resource."""
    primary_end = _jpeg_end(static_bytes, 0)
    validated = _validated_gain_jpeg(static_bytes[primary_end:])
    if validated is None:
        raise LivePhotoStillError("shared Ultra HDR GainMap item has no valid secondary JPEG/R gain map")
    return validated


def _declared_gainmap(asset: MotionPhotoAsset) -> tuple[bytes, dict] | None:
    static_bytes = base_writer._read_range(asset.source, asset.still_range)
    for item, byte_range in _forward_secondary_ranges(asset):
        if item.semantic.lower() != "gainmap":
            continue
        if byte_range is None:
            # Length=0 explicitly shares an earlier physical resource. For JPEG Ultra HDR the
            # complete static resource is JPEG/R; search only after the primary JPEG EOI and accept
            # a candidate only when its own gain-map metadata validates.
            return _embedded_gainmap_jpeg(static_bytes)
        resource = base_writer._read_range(asset.source, byte_range)
        validated = _validated_gain_jpeg(resource)
        if validated is None:
            raise LivePhotoStillError(
                "declared Ultra HDR GainMap resource contains no validated gain-map JPEG"
            )
        return validated
    return None


def _write_ultrahdr(
    asset: MotionPhotoAsset,
    output: Path,
    content_identifier: str,
    gain_jpeg: bytes,
    metadata: dict,
) -> bool:
    static_bytes = base_writer._read_range(asset.source, asset.still_range)
    with Image.open(io.BytesIO(static_bytes)) as source_image:
        source_image.load()
        original_orientation = int(source_image.getexif().get(274, 1) or 1)
        primary = ImageOps.exif_transpose(source_image).convert("RGB")
        source_exif = source_image.info.get("exif")
        source_icc = source_image.info.get("icc_profile")
    exif_blob = base_writer._inject_makernote(source_exif, content_identifier, orientation=1)
    primary.info["exif"] = exif_blob
    if source_icc:
        primary.info["icc_profile"] = source_icc

    with Image.open(io.BytesIO(gain_jpeg)) as gain_source:
        gain_source.load()
        gain = base_writer._transpose_gainmap(gain_source.copy(), original_orientation)

    output.parent.mkdir(parents=True, exist_ok=True)
    heif_io.write_heic(
        str(output), primary, gain, metadata,
        oppo_compat=False, lhdr=None, exif_data=exif_blob,
    )
    return True


def _write_motion_photo_jpeg(
    asset: MotionPhotoAsset,
    output: Path,
    content_identifier: str,
) -> bool:
    gain_items = [item for item in asset.items if item.semantic.lower() == "gainmap"]
    if not gain_items:
        return base_writer.write_live_photo_still(asset, output, content_identifier)
    declared = _declared_gainmap(asset)
    if declared is None:
        raise LivePhotoStillError("Motion Photo declares GainMap semantics but no GainMap resource")
    gain_jpeg, metadata = declared
    return _write_ultrahdr(asset, output, content_identifier, gain_jpeg, metadata)


def write_live_photo_still(
    asset: MotionPhotoAsset,
    output: Path,
    content_identifier: str,
) -> bool:
    """Write the Live Photo HEIC without using any Apple framework."""
    if asset.source_kind == "androidHeifMotionPhotoV1":
        return base_writer.write_live_photo_still(asset, output, content_identifier)
    return _write_motion_photo_jpeg(asset, Path(output), content_identifier)
