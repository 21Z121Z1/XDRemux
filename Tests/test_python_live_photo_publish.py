from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from xdremux_py.live_photo_publish import publish_pair, reconcile_pair


class PythonLivePhotoPublishTests(unittest.TestCase):
    def test_publish_replaces_pair(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            image, video = d / "photo.heic", d / "photo.mov"
            ti, tv = d / ".photo.tx.tmp.heic", d / ".photo.tx.tmp.mov"
            image.write_bytes(b"old-image")
            video.write_bytes(b"old-video")
            ti.write_bytes(b"new-image")
            tv.write_bytes(b"new-video")
            publish_pair(ti, tv, image, video)
            self.assertEqual(image.read_bytes(), b"new-image")
            self.assertEqual(video.read_bytes(), b"new-video")
            self.assertFalse(ti.exists())
            self.assertFalse(tv.exists())

    def test_publish_requires_same_directory(self):
        with tempfile.TemporaryDirectory() as tmp, tempfile.TemporaryDirectory() as other:
            d, o = Path(tmp), Path(other)
            image, video = d / "photo.heic", d / "photo.mov"
            ti, tv = o / "photo.tmp.heic", o / "photo.tmp.mov"
            ti.write_bytes(b"image")
            tv.write_bytes(b"video")
            with self.assertRaisesRegex(ValueError, "destination directory/filesystem"):
                publish_pair(ti, tv, image, video)

    def test_reconcile_removes_incomplete_pair_and_stale_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            image, video = d / "photo.heic", d / "photo.mov"
            temp = d / ".photo.deadbeef.tmp.mov"
            backup = d / ".photo.heic.deadbeef.backup"
            legacy = d / ".xdremux-live-photo-transactions"
            image.write_bytes(b"partial")
            temp.write_bytes(b"temp")
            backup.write_bytes(b"backup")
            legacy.mkdir()
            (legacy / "old.json").write_text("{}", encoding="utf-8")
            reconcile_pair(image, video, lambda i, v: False)
            for path in (image, video, temp, backup, legacy):
                self.assertFalse(path.exists())

    def test_reconcile_keeps_valid_pair(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            image, video = d / "photo.heic", d / "photo.mov"
            image.write_bytes(b"image")
            video.write_bytes(b"video")
            reconcile_pair(image, video, lambda i, v: True)
            self.assertTrue(image.exists())
            self.assertTrue(video.exists())

    def test_reconcile_removes_invalid_pair(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            image, video = d / "photo.heic", d / "photo.mov"
            image.write_bytes(b"image")
            video.write_bytes(b"video")
            reconcile_pair(image, video, lambda i, v: False)
            self.assertFalse(image.exists())
            self.assertFalse(video.exists())


if __name__ == "__main__":
    unittest.main()
