"""Standards-complete pure-Python Live Photo still entry point.

Ultra HDR JPEG stores the SDR primary JPEG and a secondary gain-map JPEG in the
static JPEG/R resource. Android Motion Photo additionally declares a GainMap
semantic item, but real Samsung files place vendor MotionPhoto_Data inside the
Primary Padding in a way that makes a naive directory-derived GainMap byte range
point at unrelated data. The robust portable rule is therefore:

1. parse the primary JPEG to its EOI;
2. search the remaining *static* resource for a complete secondary JPEG;
3. accept it only if its own hdrgm XMP or ISO 21496-1 APP2 metadata validates;
4. when the directory declares a positive GainMap Length, require the validated
   JPEG byte length to match that declaration.

This is compatible with both ordinary JPEG/R Ultra HDR and the supplied Samsung
Motion Photos while refusing arbitrary embedded JPEG thumbnails/vendor blobs.
No Apple platform API is used.
"""

from __future__ import annotations

import io
from pathlib import Path

from PIL import Image, ImageOps

from . import heif_io
from . import live_photo_still as base_writer
from .motion_photo import MotionPhotoAsset
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
    """Return the first complete JPEG whose own gain-map metadata validates."""
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


def _declared_gainmap(asset: MotionPhotoAsset) -> tuple[bytes, dict] | None:
    gain_items = [item for item in asset.items if item.semantic.lower() == "gainmap"]
    if not gain_items:
        return None
    static_bytes = base_writer._read_range(asset.source, asset.still_range)
    primary_end = _jpeg_end(static_bytes, 0)
    validated = _validated_gain_jpeg(static_bytes[primary_end:])
    if validated is None:
        raise LivePhotoStillError(
            "Motion Photo declares GainMap semantics but the static JPEG/R resource "
            "contains no validated gain-map JPEG"
        )
    gain_jpeg, metadata = validated
    declared_lengths = {item.length for item in gain_items if item.length > 0}
    if declared_lengths and len(gain_jpeg) not in declared_lengths:
        raise LivePhotoStillError(
            "validated JPEG/R gain-map length does not match the Motion Photo directory "
            f"({len(gain_jpeg)} not in {sorted(declared_lengths)})"
        )
    return gain_jpeg, metadata


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
