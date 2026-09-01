#!/usr/bin/env python3
"""Strict real-fixture gate for the pure-Python Motion Photo -> Live Photo path."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

from xdremux_py.live_photo import convert_motion_photo, validate_pair
from xdremux_py.live_photo_mov import media_payload_sha256
from xdremux_py.live_photo_still_portable import _jpeg_end
from xdremux_py.motion_photo import copy_range, parse_motion_photo, primary_video_range
from xdremux_py.motion_video import strip_trailing_vendor_data


@dataclass(frozen=True)
class FixtureSpec:
    filename: str
    sha256: str
    source_kind: str
    still_end: int
    video_start: int
    video_end: int
    presentation_timestamp_us: int
    stream_count: int
    expects_gain_map: bool
    primary_video_end: int | None = None


FIXTURES = (
    FixtureSpec("motion-photo/oppo/coloros15-ultrahdr-01.jpg", "83a4f9f3c978f541e1255bff3bd89cffe0da182aef5558c1d9d081c41f4cdb01", "oppoLivePhoto", 5_212_915, 5_212_915, 15_165_684, 1_469_600, 1, True),
    FixtureSpec("motion-photo/oppo/coloros15-ultrahdr-02.jpg", "3f5cc79c1cf26f18acf22522964e7b8e009bf35b36c4c509d7618b1fd7cd6707", "oppoLivePhoto", 4_610_334, 4_610_334, 13_359_471, 1_433_190, 1, True),
    FixtureSpec("motion-photo/oppo/coloros15-ultrahdr-03.jpg", "20afbcfb3f6fbcd7ea7b2ca306b8208dbfd10eaeb7a9fb91cf86a5a9b21c3920", "oppoLivePhoto", 19_365_654, 19_365_654, 30_680_658, 1_666_600, 1, True),
    FixtureSpec("motion-photo/oppo/coloros16-dualstream-ultrahdr-01.jpg", "5b555b0fffcec9ffb64a082a0532822431b59fc0490b677cc557e9810b764e70", "oppoLivePhoto", 6_809_684, 6_809_684, 24_929_781, 1_533_287, 2, True, 23_211_122),
    FixtureSpec("motion-photo/oppo/coloros16-dualstream-ultrahdr-02.jpg", "15c19972c3328da9c4bfb8ad9134f92764c6c51827853f8118d5d2d986e967ff", "oppoLivePhoto", 13_591_436, 13_591_436, 29_199_130, 1_298_732, 2, True, 27_234_826),
    FixtureSpec("motion-photo/xiaomi/android-v1-ultrahdr-01.jpg", "18f5d5b9243dec290626b446f6812d7bf41399bdc66d7feb794e562a9ffca4dc", "androidMotionPhotoV1", 9_541_876, 9_541_876, 10_550_148, 430_574, 1, True),
    FixtureSpec("motion-photo/samsung/jpeg-ultrahdr-01.jpg", "d95c3bfe772d681c3b7b4c33ab39f6a9da46517b3e88209fe263843dfa49cfa4", "androidMotionPhotoV1", 2_689_001, 2_689_001, 6_842_570, 1_573_888, 1, True),
    FixtureSpec("motion-photo/samsung/jpeg-ultrahdr-02.jpg", "c9e97669689fcc975f3d511cc15274b047c6b340d12c434fd04ceaa249bfee9b", "androidMotionPhotoV1", 2_690_459, 2_690_459, 3_752_096, 1_585_246, 1, True),
    FixtureSpec("motion-photo/samsung/heif-ultrahdr-01.heic", "06eb244bc69ae464bd7b0a60b769f4fc3429dc543451481f5331586a7536b8d0", "androidHeifMotionPhotoV1", 1_232_154, 1_232_162, 5_181_667, 1_540_401, 1, True),
    FixtureSpec("motion-photo/samsung/heif-ultrahdr-01-duplicate-r002.heic", "06eb244bc69ae464bd7b0a60b769f4fc3429dc543451481f5331586a7536b8d0", "androidHeifMotionPhotoV1", 1_232_154, 1_232_162, 5_181_667, 1_540_401, 1, True),
    FixtureSpec("motion-photo/samsung/heif-ultrahdr-02.heic", "d33f502276f0d8e8a0f49c9f5674ed1728812f7432f355a5a3325007fc780f1f", "androidHeifMotionPhotoV1", 1_217_171, 1_217_179, 5_586_957, 2_518_658, 1, True),
    FixtureSpec("motion-photo/samsung/heif-ultrahdr-02-duplicate-r003.heic", "d33f502276f0d8e8a0f49c9f5674ed1728812f7432f355a5a3325007fc780f1f", "androidHeifMotionPhotoV1", 1_217_171, 1_217_179, 5_586_957, 2_518_658, 1, True),
    FixtureSpec("motion-photo/vivo/android-v1-sdr-01.jpg", "f71104787d3ce236e5543a71cfc50f8208fd9acbaeef057178350dfbacecd277", "androidMotionPhotoV1", 3_307_962, 3_307_962, 6_031_584, 1_333_944, 1, False),
    FixtureSpec("motion-photo/vivo/android-v1-sdr-02.jpg", "7a00f4a63b51abfde5d1a93bc08053b3f4f28222b2234212da030ab8ed12d321", "androidMotionPhotoV1", 3_036_474, 3_036_474, 9_638_904, 838_055, 1, False),
)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(1 << 20):
            digest.update(chunk)
    return digest.hexdigest()


def index_fixtures(root: Path) -> dict[str, Path]:
    expected = {spec.filename for spec in FIXTURES}
    found: dict[str, Path] = {}
    for path in root.rglob("*"):
        if not path.is_file():
            continue
        relative = path.relative_to(root).as_posix()
        if relative in expected:
            if relative in found and path.resolve() != found[relative].resolve():
                raise RuntimeError(f"duplicate fixture path: {relative}")
            found[relative] = path
    missing = sorted(expected - set(found))
    if missing:
        raise RuntimeError("missing real Motion Photo fixtures: " + ", ".join(missing))
    return found


def _safe_geometry(source: Path, asset) -> str:
    """Emit offsets/metadata marker positions only; never print image bytes."""
    items = ",".join(
        f"{item.semantic}:{item.mime}:len={item.length}:pad={item.padding}"
        for item in asset.items
    )
    if source.suffix.lower() not in {".jpg", ".jpeg"}:
        return f"items=[{items}]"
    with source.open("rb") as stream:
        static = stream.read(asset.still_range.end)
    primary_end = _jpeg_end(static, 0)
    soi_offsets: list[int] = []
    cursor = primary_end
    while len(soi_offsets) < 8:
        found = static.find(b"\xff\xd8", cursor)
        if found < 0:
            break
        soi_offsets.append(found)
        cursor = found + 2
    markers = {
        "hdrgm": static.find(b"hdrgm"),
        "iso21496": static.find(b"urn:iso:std:iso:ts:21496:-1"),
        "mpf": static.find(b"MPF\x00"),
    }
    return (
        f"items=[{items}] primary_eoi={primary_end} static_end={asset.still_range.end} "
        f"secondary_soi={soi_offsets} markers={markers}"
    )


def _characterize_all(sources: dict[str, Path]):
    characterized = {}
    for spec in FIXTURES:
        source = sources[spec.filename]
        before = sha256(source)
        if before != spec.sha256:
            raise RuntimeError(f"fixture SHA-256 mismatch: {spec.filename}: {before}")
        asset = parse_motion_photo(source)
        if asset is None:
            raise RuntimeError(f"parser rejected real fixture: {spec.filename}")
        expected = (
            spec.source_kind, spec.still_end, spec.video_start, spec.video_end,
            spec.presentation_timestamp_us, spec.stream_count,
        )
        actual = (
            asset.source_kind, asset.still_range.end, asset.video_range.start, asset.video_range.end,
            asset.presentation_timestamp_us,
            asset.vendor_metadata.stream_count if asset.vendor_metadata else 1,
        )
        if actual != expected:
            raise RuntimeError(f"characterization drift for {spec.filename}: {actual!r} != {expected!r}")
        primary = primary_video_range(asset)
        if primary.start != spec.video_start or primary.end != (spec.primary_video_end or spec.video_end):
            raise RuntimeError(f"primary video stream range drift for {spec.filename}: {primary}")
        print(f"GEOMETRY {spec.filename}: {_safe_geometry(source, asset)}", flush=True)
        characterized[spec.filename] = (source, before, asset, primary)
    return characterized


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixture-root", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    args = parser.parse_args()
    output_root = args.output_root.resolve()
    output_root.mkdir(parents=True, exist_ok=True)
    sources = index_fixtures(args.fixture_root.resolve())
    characterized = _characterize_all(sources)
    manifest_entries: list[dict[str, object]] = []

    for index, spec in enumerate(FIXTURES, start=1):
        source, before, asset, primary = characterized[spec.filename]
        extracted = output_root / f"source-{index:02d}.mp4"
        copy_range(source, primary, extracted)
        removed_vendor_bytes = strip_trailing_vendor_data(extracted)
        source_media = media_payload_sha256(extracted)
        if not source_media:
            raise RuntimeError(f"fixture has no media-data payload: {spec.filename}")
        output_image = output_root / f"fixture-{index:02d}.heic"
        result = convert_motion_photo(source, output_image)
        validate_pair(result.image_path, result.video_path, result.content_identifier, result.still_time_seconds)
        if result.source_had_gain_map != spec.expects_gain_map:
            raise RuntimeError(
                f"gain-map characterization changed for {spec.filename}: "
                f"{result.source_had_gain_map} != {spec.expects_gain_map}"
            )
        if media_payload_sha256(result.video_path) != source_media:
            raise RuntimeError(f"compressed media changed for {spec.filename}")
        if sha256(source) != before:
            raise RuntimeError(f"Python conversion modified source fixture: {spec.filename}")
        extracted.unlink()
        manifest_entries.append({
            "sourceFilename": spec.filename,
            "sourcePath": str(source),
            "sourceKind": spec.source_kind,
            "outputImagePath": str(result.image_path),
            "outputVideoPath": str(result.video_path),
            "contentIdentifier": result.content_identifier,
            "stillImageTimeSeconds": result.still_time_seconds,
            "expectsGainMap": spec.expects_gain_map,
            "removedTrailingVendorBytes": removed_vendor_bytes,
        })
        print(
            f"PASS {spec.filename}: {result.source_kind}, gainmap={result.source_had_gain_map}, "
            f"still={result.still_time_seconds:.6f}s, trailing_vendor_bytes={removed_vendor_bytes}",
            flush=True,
        )

    args.manifest.parent.mkdir(parents=True, exist_ok=True)
    args.manifest.write_text(json.dumps({"fixtures": manifest_entries}, indent=2), encoding="utf-8")
    print(f"all {len(FIXTURES)} pure-Python Motion Photo fixtures passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
