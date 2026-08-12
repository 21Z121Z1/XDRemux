from __future__ import annotations

import struct
import unittest

from PIL import ExifTags, Image

from xdremux_py.live_photo_still import (
    APPLE_MAKERNOTE_TAG,
    _inject_makernote,
    _maker_identifier,
    build_apple_makernote,
)


class AppleMakerNoteIFDTests(unittest.TestCase):
    def test_minimal_makernote_contains_only_content_identifier(self):
        identifier = "DF64C2AE-ED3C-4778-BFCA-C15277E521D2"
        maker = build_apple_makernote(identifier)

        self.assertEqual(maker[:14], b"Apple iOS\0\0\x01MM")
        self.assertEqual(struct.unpack_from(">H", maker, 14)[0], 1)
        tag, field_type, count, offset = struct.unpack_from(">HHII", maker, 16)
        self.assertEqual((tag, field_type), (0x0011, 2))
        self.assertEqual(count, len(identifier) + 1)
        self.assertEqual(offset, 32)
        self.assertEqual(maker[28:32], b"\0\0\0\0")
        self.assertEqual(maker[offset:offset + count], identifier.encode("ascii") + b"\0")
        self.assertEqual(len(maker), 32 + count)
        self.assertEqual(_maker_identifier(maker), identifier)

    def test_makernote_is_serialized_in_exif_ifd_not_ifd0(self):
        payload = _inject_makernote(None, "ABC-123", orientation=1)
        exif = Image.Exif()
        exif.load(payload)

        self.assertNotIn(APPLE_MAKERNOTE_TAG, exif)
        maker = exif.get_ifd(ExifTags.IFD.Exif).get(APPLE_MAKERNOTE_TAG)
        self.assertIsInstance(maker, bytes)
        self.assertEqual(_maker_identifier(maker), "ABC-123")
        self.assertEqual(exif.get(274), 1)

    def test_existing_exif_ifd_metadata_survives_makernote_injection(self):
        source = Image.Exif()
        source[274] = 6
        source_exif = source.get_ifd(ExifTags.IFD.Exif)
        source_exif[36867] = "2026:08:12 08:53:00"

        payload = _inject_makernote(source.tobytes(), "ABC-123", orientation=1)
        result = Image.Exif()
        result.load(payload)
        result_exif = result.get_ifd(ExifTags.IFD.Exif)

        self.assertEqual(result.get(274), 1)
        self.assertEqual(result_exif.get(36867), "2026:08:12 08:53:00")
        self.assertEqual(
            _maker_identifier(result_exif.get(APPLE_MAKERNOTE_TAG)),
            "ABC-123",
        )


if __name__ == "__main__":
    unittest.main()
