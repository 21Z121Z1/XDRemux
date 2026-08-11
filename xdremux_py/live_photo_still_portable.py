"""Standards-complete pure-Python Live Photo still entry point.

Android Motion Photo directories may declare a GainMap item with Length=0. Per
the Motion Photo specification that means the GainMap resource is shared with
the preceding Primary item. For JPEG Ultra HDR this is the normal JPEG/R MPF
layout: the primary SDR JPEG and a secondary gain-map JPEG live in one resource.

This module resolves both positive-length GainMap resources and the shared
JPEG/R form, then feeds the existing XDRemux ISO 21496 HEIC writer. It uses no
Apple platform API.
"""

from __future__ import annotations

import io
from pathlib import Path

from PIL import Image, ImageOps

from . import heif_io
from . import live_photo_still as base_writer
from .motion_photo import MotionPhotoAsset, MotionPhotoError, jpeg_resource_ranges
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
        raise LivePhotoStillError("Ultra HDR gain-map JPEG has neither hdrgm XMP nor ISO 21496-1 APP2 metadata")
    return metadata


def _embedded_gainmap_jpeg(static_bytes: bytes) -> tuple[bytes, dict]:
    """Locate JPEG/R's secondary gain-map JPEG after the primary JPEG EOI.

    MPF is the normative locator, but selecting the first *validated* JPEG after
    the primary EOI is equivalent for the JPEG/R layout and avoids trusting a
    potentially malformed MPF offset. A candidate is accepted only if it has
    valid gain-map metadata, so EXIF thumbnails or arbitrary vendor JPEGs cannot
    be mistaken for the gain map.
    """
    primary_end = _jpeg_end(static_bytes, 0)
    search = primary_end
    while search < len(static_bytes):
        candidate = static_bytes.find(b"\xff\xd8", search)
        if candidate < 0:
            break
        try:
            end = _jpeg_end(static_bytes, candidate)
            gain_jpeg = static_bytes[candidate:end]
            metadata = _gain_metadata(gain_jpeg)
            with Image.open(io.BytesIO(gain_jpeg)) as image:
                image.load()
                if image.width <= 0 or image.height <= 0:
                    raise LivePhotoStillError("invalid JPEG/R gain-map dimensions")
            return gain_jpeg, metadata
        except (LivePhotoStillError, OSError):
            search = candidate + 2
    raise LivePhotoStillError("shared Ultra HDR GainMap item has no valid secondary JPEG/R gain map")


def _declared_gainmap_bytes(asset: MotionPhotoAsset) -> bytes | None:
    try:
        ranges = jpeg_resource_ranges(asset.items, asset.source.stat().st_size)
    except MotionPhotoError:
        return None
    for item, byte_range in zip(asset.items, ranges):
        if item.semantic.lower() != "gainmap" or item.length <= 0:
            continue
        if byte_range.end > asset.still_range.end:
            raise LivePhotoStillError("GainMap resource overlaps Motion Photo video")
        data = base_writer._read_range(asset.source, byte_range)
        if data.startswith(b"\xff\xd8"):
            return data
        # Some vendors prefix a positive-length shared container resource. Accept a nested JPEG
        # only after validating its own gain-map metadata.
        cursor = 0
        while True:
            cursor = data.find(b"\xff\xd8", cursor)
            if cursor < 0:
                break
            try:
                end = _jpeg_end(data, cursor)
                candidate = data[cursor:end]
                _gain_metadata(candidate)
                return candidate
            except LivePhotoStillError:
                cursor += 2
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

    declared = _declared_gainmap_bytes(asset)
    if declared is not None:
        return _write_ultrahdr(asset, output, content_identifier, declared, _gain_metadata(declared))

    # A zero-length secondary item is explicitly a shared resource in Motion Photo 1.0. For an
    # Ultra HDR JPEG the shared Primary resource is JPEG/R and contains the secondary gain JPEG.
    if any(item.length == 0 for item in gain_items):
        static_bytes = base_writer._read_range(asset.source, asset.still_range)
        gain_jpeg, metadata = _embedded_gainmap_jpeg(static_bytes)
        return _write_ultrahdr(asset, output, content_identifier, gain_jpeg, metadata)

    # Positive-length GainMap metadata exists but did not resolve to a valid gain JPEG. Do not drop
    # HDR silently by falling back to an SDR still.
    raise LivePhotoStillError("declared Ultra HDR GainMap resource is not a valid gain-map JPEG")


def write_live_photo_still(
    asset: MotionPhotoAsset,
    output: Path,
    content_identifier: str,
) -> bool:
    """Write the Live Photo HEIC without using any Apple framework."""
    if asset.source_kind == "androidHeifMotionPhotoV1":
        return base_writer.write_live_photo_still(asset, output, content_identifier)
    return _write_motion_photo_jpeg(asset, Path(output), content_identifier)
