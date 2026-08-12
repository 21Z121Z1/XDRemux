from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from xdremux_py.live_photo_transaction import (
    JOURNAL_DIRECTORY,
    SCHEMA_VERSION,
    commit_pair,
    recover_transactions,
)


class PythonLivePhotoTransactionTests(unittest.TestCase):
    def test_commit_replaces_both_resources_and_removes_journal(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            image = directory / "photo.heic"
            video = directory / "photo.mov"
            temp_image = directory / ".photo.tx.tmp.heic"
            temp_video = directory / ".photo.tx.tmp.mov"
            image.write_bytes(b"old-image")
            video.write_bytes(b"old-video")
            temp_image.write_bytes(b"new-image")
            temp_video.write_bytes(b"new-video")

            commit_pair(temp_image, temp_video, image, video, pair_validator=lambda i, v: True)

            self.assertEqual(image.read_bytes(), b"new-image")
            self.assertEqual(video.read_bytes(), b"new-video")
            journal_dir = directory / JOURNAL_DIRECTORY
            self.assertEqual(list(journal_dir.glob("*.json")), [])
            self.assertEqual(list(directory.glob("*.backup")), [])

    def test_commit_rejects_cross_directory_temporary_files_before_rename(self):
        with tempfile.TemporaryDirectory() as tmp, tempfile.TemporaryDirectory() as other:
            directory = Path(tmp)
            image = directory / "photo.heic"
            video = directory / "photo.mov"
            temp_image = Path(other) / "photo.tmp.heic"
            temp_video = Path(other) / "photo.tmp.mov"
            temp_image.write_bytes(b"image")
            temp_video.write_bytes(b"video")
            with self.assertRaisesRegex(ValueError, "destination directory/filesystem"):
                commit_pair(temp_image, temp_video, image, video)

    def test_recovery_restores_originals_when_crash_occurs_after_image_install(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            journal_dir = directory / JOURNAL_DIRECTORY
            journal_dir.mkdir()
            transaction_id = "abc123"
            image = directory / "photo.heic"
            video = directory / "photo.mov"
            temp_image = directory / ".photo.abc123.tmp.heic"
            temp_video = directory / ".photo.abc123.tmp.mov"
            image_backup = directory / ".photo.heic.abc123.backup"
            video_backup = directory / ".photo.mov.abc123.backup"

            image.write_bytes(b"new-image")
            temp_video.write_bytes(b"new-video")
            image_backup.write_bytes(b"old-image")
            video_backup.write_bytes(b"old-video")
            manifest = {
                "schema_version": SCHEMA_VERSION,
                "transaction_id": transaction_id,
                "state": "image_installed",
                "final_image": image.name,
                "final_video": video.name,
                "temporary_image": temp_image.name,
                "temporary_video": temp_video.name,
                "backup_image": image_backup.name,
                "backup_video": video_backup.name,
                "had_image": True,
                "had_video": True,
            }
            (journal_dir / f"{transaction_id}.json").write_text(json.dumps(manifest), encoding="utf-8")

            recover_transactions(directory, pair_validator=lambda i, v: False)

            self.assertEqual(image.read_bytes(), b"old-image")
            self.assertEqual(video.read_bytes(), b"old-video")
            self.assertFalse(temp_video.exists())
            self.assertEqual(list(journal_dir.glob("*.json")), [])

    def test_recovery_handles_crash_between_rename_and_state_update(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            journal_dir = directory / JOURNAL_DIRECTORY
            journal_dir.mkdir()
            transaction_id = "deadbeef"
            image = directory / "photo.heic"
            video = directory / "photo.mov"
            temp_image = directory / ".photo.deadbeef.tmp.heic"
            temp_video = directory / ".photo.deadbeef.tmp.mov"

            image.write_bytes(b"new-image")
            temp_video.write_bytes(b"new-video")
            manifest = {
                "schema_version": SCHEMA_VERSION,
                "transaction_id": transaction_id,
                "state": "originals_backed_up",
                "final_image": image.name,
                "final_video": video.name,
                "temporary_image": temp_image.name,
                "temporary_video": temp_video.name,
                "backup_image": ".photo.heic.deadbeef.backup",
                "backup_video": ".photo.mov.deadbeef.backup",
                "had_image": False,
                "had_video": False,
            }
            (journal_dir / f"{transaction_id}.json").write_text(json.dumps(manifest), encoding="utf-8")

            recover_transactions(directory, pair_validator=lambda i, v: False)

            self.assertFalse(image.exists())
            self.assertFalse(video.exists())
            self.assertFalse(temp_video.exists())
            self.assertEqual(list(journal_dir.glob("*.json")), [])

    def test_pair_installed_recovery_finalizes_valid_pair_instead_of_rolling_back(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            journal_dir = directory / JOURNAL_DIRECTORY
            journal_dir.mkdir()
            transaction_id = "feedface"
            image = directory / "photo.heic"
            video = directory / "photo.mov"
            image.write_bytes(b"new-image")
            video.write_bytes(b"new-video")
            image_backup = directory / ".photo.heic.feedface.backup"
            video_backup = directory / ".photo.mov.feedface.backup"
            image_backup.write_bytes(b"old-image")
            video_backup.write_bytes(b"old-video")
            manifest = {
                "schema_version": SCHEMA_VERSION,
                "transaction_id": transaction_id,
                "state": "pair_installed",
                "final_image": image.name,
                "final_video": video.name,
                "temporary_image": ".photo.feedface.tmp.heic",
                "temporary_video": ".photo.feedface.tmp.mov",
                "backup_image": image_backup.name,
                "backup_video": video_backup.name,
                "had_image": True,
                "had_video": True,
            }
            (journal_dir / f"{transaction_id}.json").write_text(json.dumps(manifest), encoding="utf-8")

            recover_transactions(directory, pair_validator=lambda i, v: i == image and v == video)

            self.assertEqual(image.read_bytes(), b"new-image")
            self.assertEqual(video.read_bytes(), b"new-video")
            self.assertFalse(image_backup.exists())
            self.assertFalse(video_backup.exists())
            self.assertEqual(list(journal_dir.glob("*.json")), [])

    def test_committed_recovery_never_rolls_back_during_cleanup(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            journal_dir = directory / JOURNAL_DIRECTORY
            journal_dir.mkdir()
            transaction_id = "c0ffee"
            image = directory / "photo.heic"
            video = directory / "photo.mov"
            image.write_bytes(b"new-image")
            video.write_bytes(b"new-video")
            # Simulate a crash after cleanup removed the image backup but before removing video backup.
            video_backup = directory / ".photo.mov.c0ffee.backup"
            video_backup.write_bytes(b"old-video")
            manifest = {
                "schema_version": SCHEMA_VERSION,
                "transaction_id": transaction_id,
                "state": "committed",
                "final_image": image.name,
                "final_video": video.name,
                "temporary_image": ".photo.c0ffee.tmp.heic",
                "temporary_video": ".photo.c0ffee.tmp.mov",
                "backup_image": ".photo.heic.c0ffee.backup",
                "backup_video": video_backup.name,
                "had_image": True,
                "had_video": True,
            }
            (journal_dir / f"{transaction_id}.json").write_text(json.dumps(manifest), encoding="utf-8")

            recover_transactions(directory, pair_validator=lambda i, v: False)

            self.assertEqual(image.read_bytes(), b"new-image")
            self.assertEqual(video.read_bytes(), b"new-video")
            self.assertFalse(video_backup.exists())
            self.assertEqual(list(journal_dir.glob("*.json")), [])


if __name__ == "__main__":
    unittest.main()
