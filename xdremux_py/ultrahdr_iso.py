"""ISO 21496-1 gain-map metadata carried in JPEG APP2 segments.

Android 15-era Ultra HDR encoders may write both Adobe hdrgm XMP and ISO 21496-1
metadata. Some real devices expose the ISO block without a usable hdrgm packet on
the gain-map resource. This module implements the compact binary metadata form
used by Android's Apache-2.0 libultrahdr implementation, independently in
Python, so XDRemux can preserve the same gain map without any platform API.
"""

from __future__ import annotations

import struct

ISO_NAMESPACE = b"urn:iso:std:iso:ts:21496:-1\x00"


class ISO21496JPEGMetadataError(ValueError):
    pass


def _jpeg_app2_payloads(jpeg: bytes):
    if not jpeg.startswith(b"\xff\xd8"):
        raise ISO21496JPEGMetadataError("gain-map resource is not a JPEG")
    cursor = 2
    size = len(jpeg)
    while cursor < size:
        if jpeg[cursor] != 0xFF:
            # Entropy-coded scan data starts only after SOS. APP metadata must precede it.
            raise ISO21496JPEGMetadataError("malformed JPEG marker stream")
        while cursor < size and jpeg[cursor] == 0xFF:
            cursor += 1
        if cursor >= size:
            break
        marker = jpeg[cursor]
        cursor += 1
        if marker in {0xD8, 0xD9} or 0xD0 <= marker <= 0xD7 or marker == 0x01:
            if marker == 0xD9:
                break
            continue
        if cursor + 2 > size:
            raise ISO21496JPEGMetadataError("truncated JPEG segment length")
        segment_length = struct.unpack_from(">H", jpeg, cursor)[0]
        if segment_length < 2 or cursor + segment_length > size:
            raise ISO21496JPEGMetadataError("invalid JPEG segment length")
        payload = jpeg[cursor + 2:cursor + segment_length]
        if marker == 0xE2:
            yield payload
        if marker == 0xDA:  # Start of Scan
            break
        cursor += segment_length


def _reader(data: bytes):
    position = 0

    def take(fmt: str):
        nonlocal position
        length = struct.calcsize(fmt)
        if position + length > len(data):
            raise ISO21496JPEGMetadataError("truncated ISO 21496-1 gain-map metadata")
        value = struct.unpack_from(fmt, data, position)
        position += length
        return value[0] if len(value) == 1 else value

    return take


def _fraction(numerator: int, denominator: int, field: str) -> float:
    if denominator == 0:
        raise ISO21496JPEGMetadataError(f"zero denominator in {field}")
    return numerator / denominator


def decode_iso21496_payload(data: bytes) -> dict:
    """Decode Android/libultrahdr's ISO 21496-1 fraction serialization."""
    take = _reader(data)
    minimum_version = take(">H")
    _writer_version = take(">H")
    if minimum_version != 0:
        raise ISO21496JPEGMetadataError(
            f"unsupported ISO 21496-1 minimum version {minimum_version}"
        )
    flags = take(">B")
    channel_count = 3 if flags & 0x80 else 1
    use_base_color_space = bool(flags & 0x40)
    backward_direction = bool(flags & 0x04)
    common_denominator = bool(flags & 0x08)

    gain_min: list[float] = []
    gain_max: list[float] = []
    gamma: list[float] = []
    base_offset: list[float] = []
    alternate_offset: list[float] = []

    if common_denominator:
        denominator = take(">I")
        if denominator == 0:
            raise ISO21496JPEGMetadataError("zero common ISO 21496-1 denominator")
        base_headroom = _fraction(take(">I"), denominator, "baseHdrHeadroom")
        alternate_headroom = _fraction(take(">I"), denominator, "alternateHdrHeadroom")
        for channel in range(channel_count):
            gain_min.append(_fraction(take(">i"), denominator, f"gainMapMin[{channel}]"))
            gain_max.append(_fraction(take(">i"), denominator, f"gainMapMax[{channel}]"))
            gamma.append(_fraction(take(">I"), denominator, f"gamma[{channel}]"))
            base_offset.append(_fraction(take(">i"), denominator, f"baseOffset[{channel}]"))
            alternate_offset.append(
                _fraction(take(">i"), denominator, f"alternateOffset[{channel}]")
            )
    else:
        base_headroom = _fraction(take(">I"), take(">I"), "baseHdrHeadroom")
        alternate_headroom = _fraction(take(">I"), take(">I"), "alternateHdrHeadroom")
        for channel in range(channel_count):
            gain_min.append(_fraction(take(">i"), take(">I"), f"gainMapMin[{channel}]"))
            gain_max.append(_fraction(take(">i"), take(">I"), f"gainMapMax[{channel}]"))
            gamma.append(_fraction(take(">I"), take(">I"), f"gamma[{channel}]"))
            base_offset.append(_fraction(take(">i"), take(">I"), f"baseOffset[{channel}]"))
            alternate_offset.append(
                _fraction(take(">i"), take(">I"), f"alternateOffset[{channel}]")
            )

    if channel_count == 1:
        gain_min *= 3
        gain_max *= 3
        gamma *= 3
        base_offset *= 3
        alternate_offset *= 3
    if backward_direction:
        # JPEG Ultra HDR requires the SDR representation to be the base. Supporting the inverse
        # direction would require swapping the base/alternate semantics and validating a different
        # image contract, so fail rather than silently producing incorrect HDR.
        raise ISO21496JPEGMetadataError(
            "backward-direction ISO 21496-1 metadata is unsupported for Ultra HDR JPEG"
        )
    return {
        "gainMapMin": gain_min,
        "gainMapMax": gain_max,
        "gamma": gamma,
        "offsetSdr": base_offset,
        "offsetHdr": alternate_offset,
        "hdrCapacityMin": base_headroom,
        "hdrCapacityMax": alternate_headroom,
        "baseRenditionIsHDR": False,
        "useBaseColorSpace": use_base_color_space,
    }


def parse_iso21496_jpeg_metadata(jpeg: bytes) -> dict | None:
    for payload in _jpeg_app2_payloads(jpeg):
        if payload.startswith(ISO_NAMESPACE):
            return decode_iso21496_payload(payload[len(ISO_NAMESPACE):])
    return None
