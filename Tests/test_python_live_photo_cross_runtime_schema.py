from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from xdremux_py.live_photo_transaction import JOURNAL_DIRECTORY, SCHEMA_VERSION, recover_transactions


class PythonLivePhotoCrossRuntimeSchemaTests(unittest.TestCase):
    def test_python_recovers_canonical_swift_committed_journal(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            journal_directory = directory / JOURNAL_DIRECTORY
            journal_directory.mkdir()
            transaction_id = "swift-feedface"
            # Swift UUIDs normally contain hyphens; use only the accepted portable character set.
            transaction_id = "feed-face"
            image = directory / "photo.heic"
            video = directory / "photo.mov"
            image_backup = directory / ".photo.heic.feed-face.backup"
            video_backup = directory / ".photo.mov.feed-face.backup"
            image.write_bytes(b"new-image")
            video.write_bytes(b"new-video")
            image_backup.write_bytes(b"old-image")
            video_backup.write_bytes(b"old-video")

            # Exact camelCase wire names emitted by Swift JSONEncoder.
            manifest = {
                "schemaVersion": SCHEMA_VERSION,
                "transactionID": transaction_id,
                "state": "committed",
                "finalImage": image.name,
                "finalVideo": video.name,
                "temporaryImage": ".photo.feed-face.tmp.heic",
                "temporaryVideo": ".photo.feed-face.tmp.mov",
                "backupImage": image_backup.name,
                "backupVideo": video_backup.name,
                "hadImage": True,
                "hadVideo": True,
            }
            (journal_directory / f"{transaction_id}.json").write_text(
                json.dumps(manifest), encoding="utf-8"
            )

            recover_transactions(directory, pair_validator=lambda i, v: False)

            self.assertEqual(image.read_bytes(), b"new-image")
            self.assertEqual(video.read_bytes(), b"new-video")
            self.assertFalse(image_backup.exists())
            self.assertFalse(video_backup.exists())
            self.assertEqual(list(journal_directory.glob("*.json")), [])


if __name__ == "__main__":
    unittest.main()
