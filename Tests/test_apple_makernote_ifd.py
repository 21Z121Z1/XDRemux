from __future__ import annotations

import unittest

from PIL import ExifTags, Image

from xdremux_py.live_photo_still import (
    APPLE_MAKERNOTE_TAG,
    _inject_makernote,
    _maker_identifier,
)


class AppleMakerNoteIFDTests(unittest.TestCase):
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
