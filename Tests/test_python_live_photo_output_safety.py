import hashlib
import shutil
import tempfile
import unittest
from pathlib import Path

from xdremux_py.live_photo import LivePhotoConversionError, convert_motion_photo, existing_pair_is_valid


class PythonLivePhotoOutputSafetyTests(unittest.TestCase):
    fixture_name = "motion-photo/oppo/coloros16-dualstream-ultrahdr-01.jpg"

    def fixture(self) -> Path:
        path = Path(__file__).resolve().parents[1] / "fixtures" / self.fixture_name
        if not path.is_file():
            self.skipTest(f"repository Motion Photo fixture is unavailable: {path}")
        return path

    @staticmethod
    def digest(path: Path) -> str:
        return hashlib.sha256(path.read_bytes()).hexdigest()

    def test_implicit_convert_preserves_foreign_same_basename_pair_and_sequences_output(self):
        with tempfile.TemporaryDirectory(prefix="xdremux-py-output-safety-") as tmp:
            root = Path(tmp)
            source = root / self.fixture_name
            shutil.copy2(self.fixture(), source)
            foreign_image = source.with_suffix(".heic")
            foreign_video = source.with_suffix(".mov")
            foreign_image.write_bytes(b"foreign-heic-do-not-touch")
            foreign_video.write_bytes(b"foreign-mov-do-not-touch")
            image_digest = self.digest(foreign_image)
            video_digest = self.digest(foreign_video)

            result = convert_motion_photo(source)

            self.assertEqual(self.digest(foreign_image), image_digest)
            self.assertEqual(self.digest(foreign_video), video_digest)
            self.assertEqual(result.image_path.name, f"{source.stem} (2).heic")
            self.assertEqual(result.video_path.name, f"{source.stem} (2).mov")
            self.assertTrue(existing_pair_is_valid(result.image_path, result.video_path))

    def test_explicit_convert_refuses_existing_foreign_pair_and_preserves_bytes(self):
        with tempfile.TemporaryDirectory(prefix="xdremux-py-explicit-output-safety-") as tmp:
            root = Path(tmp)
            source = root / self.fixture_name
            shutil.copy2(self.fixture(), source)
            output_image = root / "user-owned.heic"
            output_video = root / "user-owned.mov"
            output_image.write_bytes(b"foreign-explicit-heic")
            output_video.write_bytes(b"foreign-explicit-mov")
            image_digest = self.digest(output_image)
            video_digest = self.digest(output_video)

            with self.assertRaises(LivePhotoConversionError):
                convert_motion_photo(source, output_image)

            self.assertEqual(self.digest(output_image), image_digest)
            self.assertEqual(self.digest(output_video), video_digest)


if __name__ == "__main__":
    unittest.main()
