#!/usr/bin/env python3
from __future__ import annotations

import json
import struct
import subprocess
import tempfile
from pathlib import Path

from xdremux_py.motion_photo import MotionPhotoError, parse_motion_photo

ROOT = Path(__file__).resolve().parents[1]
RUST_ORACLE = ROOT / "target" / "debug" / "examples" / "motion_photo_conformance"


def make_box(kind: bytes, payload: bytes) -> bytes:
    assert len(kind) == 4
    return struct.pack(">I4s", len(payload) + 8, kind) + payload


def fake_mp4(brand: bytes = b"isom", payload_size: int = 120_000, payload_byte: int = 0x44) -> bytes:
    assert len(brand) == 4
    ftyp = make_box(b"ftyp", brand + b"\x00\x00\x00\x00")
    return ftyp + make_box(b"mdat", bytes([payload_byte]) * payload_size)


def standard_xmp(video_length: int, *, include_version: bool = True) -> str:
    version = ' Camera:MotionPhotoVersion="1"' if include_version else ""
    return f"""
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/"
                     xmlns:Container="http://ns.google.com/photos/1.0/container/"
                     xmlns:Item="http://ns.google.com/photos/1.0/container/item/"
                     Camera:MotionPhoto="1"{version}
                     Camera:MotionPhotoPresentationTimestampUs="1634640">
      <Container:Directory><rdf:Seq>
        <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="image/jpeg" Item:Semantic="Primary" Item:Length="0" Item:Padding="0"/></rdf:li>
        <rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="video/mp4" Item:Semantic="MotionPhoto" Item:Length="{video_length}" Item:Padding="0"/></rdf:li>
      </rdf:Seq></Container:Directory>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
"""


def python_metadata(metadata) -> dict | None:
    if metadata is None:
        return None
    return {
        "coverFramePtsUs": metadata.cover_frame_pts_us,
        "version": metadata.version,
        "matrixCount": metadata.matrix_count,
        "videoWidth": metadata.video_width,
        "videoHeight": metadata.video_height,
        "streamCount": metadata.stream_count,
    }


def python_asset(asset) -> dict:
    return {
        "status": "asset",
        "sourceKind": asset.source_kind,
        "items": [
            {
                "mime": item.mime,
                "semantic": item.semantic,
                "length": item.length,
                "padding": item.padding,
            }
            for item in asset.items
        ],
        "still": {"lower": asset.still_range.start, "upper": asset.still_range.end},
        "video": {"lower": asset.video_range.start, "upper": asset.video_range.end},
        "presentationTimestampUs": asset.presentation_timestamp_us,
        "presentationSource": asset.presentation_source,
        "vendorMetadata": python_metadata(asset.vendor_metadata),
    }


def normalize_rust(value: dict) -> dict:
    if value.get("status") != "asset":
        return value
    metadata = value.get("vendorMetadata")
    if isinstance(metadata, dict):
        metadata = {
            "coverFramePtsUs": metadata.get("coverFramePtsUs"),
            "version": metadata.get("version"),
            "matrixCount": metadata.get("matrixCount"),
            "videoWidth": metadata.get("videoWidth"),
            "videoHeight": metadata.get("videoHeight"),
            "streamCount": metadata.get("streamCount"),
        }
    return {
        "status": "asset",
        "sourceKind": value.get("sourceKind"),
        "items": value.get("items"),
        "still": value.get("still"),
        "video": value.get("video"),
        "presentationTimestampUs": value.get("presentationTimestampUs"),
        "presentationSource": value.get("presentationSource"),
        "vendorMetadata": metadata,
    }


def run_rust(path: Path) -> dict:
    completed = subprocess.run(
        [str(RUST_ORACLE), "oppo", str(path)],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(completed.stdout)


def assert_asset_case(name: str, data: bytes) -> None:
    with tempfile.TemporaryDirectory(prefix="xdremux-oppo-python-rust-") as directory:
        path = Path(directory) / f"{name}.jpg"
        path.write_bytes(data)
        py = parse_motion_photo(path)
        rust = run_rust(path)
        if py is None:
            expected = {"status": "none"}
            if rust != expected:
                raise AssertionError(f"{name}: Python none != Rust {rust!r}")
            return
        expected = python_asset(py)
        actual = normalize_rust(rust)
        if actual != expected:
            raise AssertionError(
                f"{name}: Python/Rust asset mismatch\n"
                f"python={json.dumps(expected, sort_keys=True)}\n"
                f"rust={json.dumps(actual, sort_keys=True)}"
            )


def assert_error_case(name: str, data: bytes) -> None:
    with tempfile.TemporaryDirectory(prefix="xdremux-oppo-python-rust-") as directory:
        path = Path(directory) / f"{name}.jpg"
        path.write_bytes(data)
        try:
            parse_motion_photo(path)
        except MotionPhotoError:
            pass
        else:
            raise AssertionError(f"{name}: Python unexpectedly accepted malformed input")
        rust = run_rust(path)
        if rust.get("status") != "error":
            raise AssertionError(f"{name}: Rust unexpectedly returned {rust!r}")


def main() -> None:
    if not RUST_ORACLE.exists():
        raise SystemExit(f"Rust Motion Photo oracle is not built: {RUST_ORACLE}")

    unsigned = b"\xff\xd8\xff\xd9" + fake_mp4(payload_size=128)
    assert_asset_case("unsigned", unsigned)

    single_video = fake_mp4(payload_byte=0x22)
    single_xmp = f"""
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF><rdf:Description xmlns:OpCamera="http://ns.oppo.com/photos/1.0/camera/"
                            OpCamera:VideoLength="{len(single_video)}"
                            GCamera:MotionPhotoPresentationTimestampUs="1634640"/></rdf:RDF>
</x:xmpmeta>
"""
    assert_asset_case("single", b"\xff\xd8" + single_xmp.encode() + b"\xff\xd9" + single_video)

    stale_video = fake_mp4(payload_byte=0x33)
    stale_xmp = """
<x:xmpmeta><rdf:RDF><rdf:Description xmlns:OpCamera="http://ns.oppo.com/photos/1.0/camera/"
                                     OpCamera:VideoLength="100001"/></rdf:RDF></x:xmpmeta>
"""
    assert_asset_case("stale", b"\xff\xd8" + stale_xmp.encode() + b"\xff\xd9" + stale_video)

    stream1 = fake_mp4(brand=b"isom", payload_size=1024, payload_byte=0x44)
    stream2 = fake_mp4(brand=b"mp42", payload_size=1024, payload_byte=0x55)
    dual_xmp = f"""
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF><rdf:Description xmlns:OpCamera="http://ns.oppo.com/photos/1.0/camera/"
                            OpCamera:VideoLength="{len(stream2)}"
                            GCamera:MotionPhotoPresentationTimestampUs="1634640"/></rdf:RDF>
</x:xmpmeta>
"""
    lpex = b'lpexLivePhotoExtension {"version":1,"coverFramePts":1666666,"matrixCount":0,"videoSize":[1920,1080]}'
    dual_still = b"\xff\xd8" + dual_xmp.encode() + lpex + b"\xff\xd9"
    assert_asset_case("dual_fallback", dual_still + stream1 + stream2)

    standard_still = b"\xff\xd8" + standard_xmp(len(stream2)).encode() + lpex + b"\xff\xd9"
    assert_asset_case("dual_standard", standard_still + stream1 + stream2)

    cover_video = fake_mp4(payload_size=128, payload_byte=0x88)
    cover_lpex = b'lpexLivePhotoExtension {"version":0,"coverFramePts":777777}'
    assert_asset_case("cover_only", b"\xff\xd8" + cover_lpex + b"\xff\xd9" + cover_video)

    sentinel_video = fake_mp4(payload_size=128, payload_byte=0x89)
    sentinel_xmp = f"""
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF><rdf:Description xmlns:OpCamera="http://ns.oppo.com/photos/1.0/camera/"
                            OpCamera:VideoLength="{len(sentinel_video)}"
                            GCamera:MotionPhotoPresentationTimestampUs="-1"/></rdf:RDF>
</x:xmpmeta>
"""
    sentinel_lpex = b'lpexLivePhotoExtension {"version":0,"coverFramePts":777777}'
    assert_asset_case(
        "sentinel_cover_fallback",
        b"\xff\xd8" + sentinel_xmp.encode() + sentinel_lpex + b"\xff\xd9" + sentinel_video,
    )

    recover_video = fake_mp4(payload_byte=0x99)
    malformed_oppo = f"""
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/"
                     xmlns:OpCamera="http://ns.oppo.com/photos/1.0/camera/"
                     Camera:MotionPhoto="1"
                     OpCamera:VideoLength="{len(recover_video)}"/>
  </rdf:RDF>
</x:xmpmeta>
"""
    assert_asset_case("recover_android_error", b"\xff\xd8" + malformed_oppo.encode() + b"\xff\xd9" + recover_video)

    malformed_generic = standard_xmp(len(recover_video), include_version=False)
    assert_error_case(
        "preserve_generic_error",
        b"\xff\xd8" + malformed_generic.encode() + b"\xff\xd9" + recover_video,
    )

    print("Python/Rust OPPO Motion Photo conformance: PASS")


if __name__ == "__main__":
    main()
