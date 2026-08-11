"""Standards-complete pure-Python Live Photo still entry point.

The base writer handles Adobe Ultra HDR XMP and direct HEIF metadata rewriting.
This adapter adds ISO 21496-1 JPEG APP2 metadata decoding for Android 15-era
inputs whose GainMap resource does not expose a usable hdrgm XMP packet.
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


def _write_iso_ultrahdr_jpeg(
    asset: MotionPhotoAsset,
    output: Path,
    content_identifier: str,
) -> bool | None:
    gain_range = base_writer._jpeg_gainmap_range(asset)
    if gain_range is None:
        return None
    gain_bytes = base_writer._read_range(asset.source, gain_range)
    try:
        iso_meta = parse_iso21496_jpeg_metadata(gain_bytes)
    except ISO21496JPEGMetadataError as exc:
        raise LivePhotoStillError(str(exc)) from exc
    if iso_meta is None:
        return None

    static_bytes = base_writer._read_range(asset.source, asset.still_range)
    with Image.open(io.BytesIO(static_bytes)) as source_image:
        source_image.load()
        original_orientation = int(source_image.getexif().get(274, 1) or 1)
        primary = ImageOps.exif_transpose(source_image).convert("RGB")
        source_exif = source_image.info.get("exif")
        source_icc = source_image.info.get("icc_profile")
    exif_blob = base_writer._inject_makernote(
        source_exif,
        content_identifier,
        orientation=1,
    )
    primary.info["exif"] = exif_blob
    if source_icc:
        primary.info["icc_profile"] = source_icc

    with Image.open(io.BytesIO(gain_bytes)) as gain_source:
        gain_source.load()
        gain = base_writer._transpose_gainmap(gain_source.copy(), original_orientation)

    output.parent.mkdir(parents=True, exist_ok=True)
    heif_io.write_heic(
        str(output),
        primary,
        gain,
        iso_meta,
        oppo_compat=False,
        lhdr=None,
        exif_data=exif_blob,
    )
    return True


def write_live_photo_still(
    asset: MotionPhotoAsset,
    output: Path,
    content_identifier: str,
) -> bool:
    """Write the Live Photo HEIC without using any Apple framework.

    Prefer the existing Adobe hdrgm path. If a JPEG GainMap has no hdrgm XMP,
    fall back to its ISO 21496-1 APP2 metadata. HEIF Motion Photos continue down
    the encoded-bitstream-preserving path in ``live_photo_still``.
    """
    if asset.source_kind == "androidHeifMotionPhotoV1":
        return base_writer.write_live_photo_still(asset, output, content_identifier)
    try:
        return base_writer.write_live_photo_still(asset, output, content_identifier)
    except LivePhotoStillError as original:
        if "lacks hdrgm XMP" not in str(original):
            raise
        result = _write_iso_ultrahdr_jpeg(asset, Path(output), content_identifier)
        if result is None:
            raise original
        return result
