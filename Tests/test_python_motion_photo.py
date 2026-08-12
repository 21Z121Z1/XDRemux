from __future__ import annotations

import json
import struct
import tempfile
import unittest
from pathlib import Path

from xdremux_py.live_photo_mov import (
    _box,
    _full_box,
    media_payload_sha256,
    oppo_transform,
    read_content_identifier,
    read_still_time,
    resolve_still_time,
    validate_live_photo_movie,
    write_live_photo_movie,
)
from xdremux_py.live_photo_still import build_apple_makernote, parse_ultrahdr_metadata
from xdremux_py.motion_photo import (
    ByteRange,
    OppoMetadata,
    parse_android_motion_photo,
    parse_oppo_lpex,
)
from xdremux_py.motion_video import strip_trailing_vendor_data


def _fake_video() -> bytes:
    ftyp = _box(b"ftyp", b"isom" + struct.pack(">I", 0) + b"isommp42")
    mvhd = _full_box(
        b"mvhd",
        struct.pack(">IIII", 0, 0, 1000, 300)
        + struct.pack(">I", 0x00010000)
        + struct.pack(">H", 0x0100)
        + b"\0\0" + b"\0" * 8
        + struct.pack(">9i", 0x10000, 0, 0, 0, 0x10000, 0, 0, 0, 0x40000000)
        + b"\0" * 24 + struct.pack(">I", 2),
    )
    tkhd = _full_box(
        b"tkhd",
        struct.pack(">IIIII", 0, 0, 1, 0, 300)
        + b"\0" * 8 + struct.pack(">hhhh", 0, 0, 0, 0)
        + struct.pack(">9i", 0x10000, 0, 0, 0, 0x10000, 0, 0, 0, 0x40000000)
        + struct.pack(">II", 64 << 16, 48 << 16),
        flags=7,
    )
    mdhd = _full_box(b"mdhd", struct.pack(">IIIIHH", 0, 0, 1000, 300, 0x55C4, 0))
    hdlr = _full_box(b"hdlr", b"\0" * 4 + b"vide" + b"\0" * 12 + b"Video\0")
    stts = _full_box(b"stts", struct.pack(">III", 1, 3, 100))
    trak = _box(b"trak", tkhd + _box(b"mdia", mdhd + hdlr + _box(b"minf", _box(b"stbl", stts))))
    return ftyp + _box(b"moov", mvhd + trak) + _box(b"mdat", b"encoded-media-payload")


def _motion_xmp(video_length: int, *, heif: bool = False, presentation: int = 120000) -> bytes:
    image_mime = "image/heic" if heif else "image/jpeg"
    padding = "8" if heif else "0"
    return f'''<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
 xmlns:Camera="http://ns.google.com/photos/1.0/camera/"
 xmlns:Container="http://ns.google.com/photos/1.0/container/"
 xmlns:Item="http://ns.google.com/photos/1.0/container/item/">
<rdf:RDF><rdf:Description Camera:MotionPhoto="1" Camera:MotionPhotoVersion="1"
 Camera:MotionPhotoPresentationTimestampUs="{presentation}">
<Container:Directory><rdf:Seq>
<rdf:li><Container:Item Item:Mime="{image_mime}" Item:Semantic="Primary" Item:Length="0" Item:Padding="{padding}"/></rdf:li>
<rdf:li><Container:Item Item:Mime="video/mp4" Item:Semantic="MotionPhoto" Item:Length="{video_length}" Item:Padding="0"/></rdf:li>
</rdf:Seq></Container:Directory></rdf:Description></rdf:RDF></x:xmpmeta>'''.encode()


class PythonMotionPhotoTests(unittest.TestCase):
    def test_android_jpeg_directory_resolves_video_at_eof(self):
        video = _fake_video()
        xmp = _motion_xmp(len(video))
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "motion.jpg"
            static = b"\xff\xd8" + xmp + b"\xff\xd9"
            path.write_bytes(static + video)
            asset = parse_android_motion_photo(path)
            self.assertIsNotNone(asset)
            assert asset is not None
            self.assertEqual(asset.source_kind, "androidMotionPhotoV1")
            self.assertEqual(asset.still_range, ByteRange(0, len(static)))
            self.assertEqual(asset.video_range, ByteRange(len(static), len(static) + len(video)))
            self.assertEqual(asset.presentation_timestamp_us, 120000)

    def test_heif_mpvd_excludes_trailing_vendor_box(self):
        video = _fake_video()
        trailing = _box(b"sefd", b"vendor")
        xmp = _motion_xmp(len(video) + len(trailing), heif=True)
        ftyp = _box(b"ftyp", b"heic" + struct.pack(">I", 0) + b"mif1heic")
        metadata = _box(b"uuid", xmp)
        mpvd = _box(b"mpvd", video)
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "motion.heic"
            path.write_bytes(ftyp + metadata + mpvd + trailing)
            asset = parse_android_motion_photo(path)
            self.assertIsNotNone(asset)
            assert asset is not None
            self.assertEqual(asset.source_kind, "androidHeifMotionPhotoV1")
            self.assertEqual(asset.still_range.end, len(ftyp) + len(metadata))
            self.assertEqual(asset.video_range.start, len(ftyp) + len(metadata) + 8)
            self.assertEqual(asset.video_range.end, len(ftyp) + len(metadata) + len(mpvd))

    def test_lpex_parser_reads_photo_eis_crop_factor(self):
        payload = {
            "version": 1,
            "photoEisCropFactor": [1.11, 1.12],
            "eisCropFactor": [0.80, 0.81],
        }
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "coloros16.bin"
            path.write_bytes(b"prefix-lpexLivePhotoExtension" + json.dumps(payload).encode() + b"-suffix")
            metadata = parse_oppo_lpex(path)
            self.assertIsNotNone(metadata)
            assert metadata is not None
            self.assertEqual(metadata.photo_eis_crop_factor, (1.11, 1.12))
            self.assertEqual(metadata.eis_crop_factor, (0.80, 0.81))

    def test_coloros16_uses_legacy_eis_compensation_when_no_factor_exists(self):
        identity = (1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0)
        matrix = oppo_transform(OppoMetadata(
            version=1,
            photo_crop_matrix=identity,
            photo_eis_matrix=identity,
        ))
        self.assertIsNotNone(matrix)
        assert matrix is not None
        self.assertAlmostEqual(matrix[0], 0.90, places=12)
        self.assertAlmostEqual(matrix[4], 0.90, places=12)
        self.assertAlmostEqual(matrix[8], 1.0, places=12)

    def test_coloros16_uses_reciprocal_photo_eis_crop_factor(self):
        matrix = oppo_transform(OppoMetadata(
            version=1,
            photo_eis_crop_factor=(1.11, 1.12),
        ))
        self.assertIsNotNone(matrix)
        assert matrix is not None
        self.assertAlmostEqual(matrix[0], 1.0 / 1.11, places=12)
        self.assertAlmostEqual(matrix[4], 1.0 / 1.12, places=12)
        self.assertAlmostEqual(matrix[8], 1.0, places=12)

    def test_coloros16_accepts_legacy_direct_scale_below_one(self):
        matrix = oppo_transform(OppoMetadata(
            version=1,
            eis_crop_factor=(0.91, 0.92),
        ))
        self.assertIsNotNone(matrix)
        assert matrix is not None
        self.assertAlmostEqual(matrix[0], 0.91, places=12)
        self.assertAlmostEqual(matrix[4], 0.92, places=12)

    def test_photo_eis_crop_factor_wins_over_legacy_eis_factor(self):
        matrix = oppo_transform(OppoMetadata(
            version=1,
            photo_eis_crop_factor=(1.11, 1.11),
            eis_crop_factor=(0.80, 0.80),
        ))
        self.assertIsNotNone(matrix)
        assert matrix is not None
        self.assertAlmostEqual(matrix[0], 1.0 / 1.11, places=12)
        self.assertAlmostEqual(matrix[4], 1.0 / 1.11, places=12)

    def test_coloros15_uses_closest_cover_frame_and_inverts_matrix(self):
        matrix = oppo_transform(OppoMetadata(
            cover_frame_pts_us=1100,
            version=0,
            matrix_count=2,
            matrices=(
                (1000, (2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 1.0)),
                (2000, (4.0, 0.0, 0.0, 0.0, 4.0, 0.0, 0.0, 0.0, 1.0)),
            ),
        ))
        self.assertIsNotNone(matrix)
        assert matrix is not None
        self.assertAlmostEqual(matrix[0], 0.5, places=12)
        self.assertAlmostEqual(matrix[4], 0.5, places=12)
        self.assertAlmostEqual(matrix[8], 1.0, places=12)

    def test_pure_python_mov_writer_preserves_media_and_writes_live_photo_metadata(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "source.mp4"
            output = Path(tmp) / "output.mov"
            source.write_bytes(_fake_video())
            source_hashes = media_payload_sha256(source)
            resolved = resolve_still_time(source, 120000)
            self.assertAlmostEqual(resolved, 0.1, places=6)
            write_live_photo_movie(source, output, "ABC-123", resolved)
            self.assertEqual(read_content_identifier(output), "ABC-123")
            self.assertAlmostEqual(read_still_time(output) or -1, 0.1, places=3)
            self.assertEqual(media_payload_sha256(output), source_hashes)
            validate_live_photo_movie(output, "ABC-123", resolved)

    def test_pure_python_mov_writer_embeds_oppo_transform_without_reencoding_media(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "source.mp4"
            output = Path(tmp) / "output.mov"
            source.write_bytes(_fake_video())
            source_hashes = media_payload_sha256(source)
            metadata = OppoMetadata(
                version=1,
                photo_eis_crop_factor=(1.11, 1.12),
                video_width=1728,
                video_height=1296,
            )
            expected = oppo_transform(metadata)
            self.assertIsNotNone(expected)
            assert expected is not None
            write_live_photo_movie(source, output, "ABC-123", 0.1, oppo_metadata=metadata)
            payload = output.read_bytes()
            self.assertIn(struct.pack(">9d", *expected), payload)
            self.assertIn(struct.pack(">2f", 1728.0, 1296.0), payload)
            self.assertEqual(media_payload_sha256(output), source_hashes)

    def test_trailing_vendor_bytes_are_removed_only_after_complete_bmff(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "oppo-stream1.mp4"
            media = _fake_video()
            vendor = b"\x00\x00\x00\x20\x9a\x99\x99?opaque-coloros-tail"
            path.write_bytes(media + vendor)
            removed = strip_trailing_vendor_data(path)
            self.assertEqual(removed, len(vendor))
            self.assertEqual(path.read_bytes(), media)
            self.assertTrue(media_payload_sha256(path))

    def test_ultrahdr_rdf_sequence_metadata_is_parsed(self):
        xmp = b'''<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:hdrgm="http://ns.adobe.com/hdr-gain-map/1.0/"><rdf:RDF><rdf:Description><hdrgm:GainMapMin><rdf:Seq><rdf:li>0</rdf:li><rdf:li>0.1</rdf:li><rdf:li>0.2</rdf:li></rdf:Seq></hdrgm:GainMapMin><hdrgm:GainMapMax>2.0 2.1 2.2</hdrgm:GainMapMax><hdrgm:Gamma>1</hdrgm:Gamma><hdrgm:HDRCapacityMax>2.2</hdrgm:HDRCapacityMax></rdf:Description></rdf:RDF></x:xmpmeta>'''
        meta = parse_ultrahdr_metadata(b"\xff\xd8" + xmp + b"\xff\xd9")
        self.assertEqual(meta["gainMapMin"], [0.0, 0.1, 0.2])
        self.assertEqual(meta["gainMapMax"], [2.0, 2.1, 2.2])
        self.assertEqual(meta["hdrCapacityMax"], 2.2)

    def test_apple_makernote_contains_content_identifier_tag(self):
        maker = build_apple_makernote("ABC-123")
        self.assertTrue(maker.startswith(b"Apple iOS\0\0\x01MM"))
        count = struct.unpack_from(">H", maker, 14)[0]
        found = None
        for index in range(count):
            tag, kind, item_count, value = struct.unpack_from(">HHII", maker, 16 + index * 12)
            if tag == 0x0011:
                self.assertEqual(kind, 2)
                found = maker[value:value + item_count].rstrip(b"\0").decode("ascii")
        self.assertEqual(found, "ABC-123")


if __name__ == "__main__":
    unittest.main()
