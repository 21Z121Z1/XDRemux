from pathlib import Path


def replace(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text(encoding="utf-8")
    if old not in text:
        raise SystemExit(f"expected text not found in {path}")
    p.write_text(text.replace(old, new, 1), encoding="utf-8")


replace(
    "Sources/XDRemuxCLI/Commands/MotionPhotoCLIIntegration.swift",
    """                // Single-file conversion historically replaces its deterministic output on rerun.\n                // Ignore unrelated pre-existing outputs here; only protect the source path itself.\n                fileExists: { _ in false }\n""",
    """                // Treat any existing HEIC or companion MOV as a collision. Single-file\n                // conversion must never infer ownership of an unrelated destination resource.\n                fileExists: { FileManager.default.fileExists(atPath: $0.path) }\n""",
)
replace(
    "Sources/XDRemuxCLI/Commands/MotionPhotoCLIIntegration.swift",
    """        try validateLivePhotoOutputExtension(outputImageURL)\n\n        let result = try AppleLivePhotoConversionEngine.convert(\n""",
    """        try validateLivePhotoOutputExtension(outputImageURL)\n        if outputWasExplicit {\n            let outputVideoURL = AppleLivePhotoConversionEngine.companionVideoURL(for: outputImageURL)\n            if FileManager.default.fileExists(atPath: outputImageURL.path)\n                || FileManager.default.fileExists(atPath: outputVideoURL.path) {\n                throw CLIError.invalidValue(\n                    option: \"--output\",\n                    value: \"target HEIC/MOV already exists; refusing to overwrite an output pair with unknown provenance\"\n                )\n            }\n        }\n\n        let result = try AppleLivePhotoConversionEngine.convert(\n""",
)

replace(
    "xdremux_py/live_photo.py",
    """def default_output_image(input_path: Path) -> Path:\n    \"\"\"Preserve the user's basename; only avoid overwriting a same-path HEIC source.\"\"\"\n    input_path = Path(input_path)\n    output = input_path.with_suffix(\".heic\")\n    if output.resolve() == input_path.resolve():\n        return output.with_name(f\"{output.stem} (2){output.suffix}\")\n    return output\n""",
    """def default_output_image(input_path: Path) -> Path:\n    \"\"\"Preserve the basename while never claiming an existing HEIC/MOV namespace.\"\"\"\n    input_path = Path(input_path)\n    base = input_path.with_suffix(\".heic\")\n    sequence = 1\n    while True:\n        candidate = (\n            base\n            if sequence == 1\n            else base.with_name(f\"{base.stem} ({sequence}){base.suffix}\")\n        )\n        video = companion_video_path(candidate)\n        if (\n            candidate.resolve() != input_path.resolve()\n            and not candidate.exists()\n            and not video.exists()\n        ):\n            return candidate\n        sequence += 1\n""",
)
replace(
    "xdremux_py/live_photo.py",
    """    output_image = Path(output_image) if output_image is not None else default_output_image(input_path)\n    if output_image.suffix.lower() not in {\".heic\", \".heif\"}:\n""",
    """    output_was_explicit = output_image is not None\n    output_image = Path(output_image) if output_was_explicit else default_output_image(input_path)\n    if output_image.suffix.lower() not in {\".heic\", \".heif\"}:\n""",
)
replace(
    "xdremux_py/live_photo.py",
    """    output_video = companion_video_path(output_image)\n    output_directory = output_image.parent\n""",
    """    output_video = companion_video_path(output_image)\n    if output_was_explicit and (output_image.exists() or output_video.exists()):\n        raise LivePhotoConversionError(\n            \"explicit Live Photo output HEIC/MOV already exists; \"\n            \"refusing to overwrite an output pair with unknown provenance\"\n        )\n    output_directory = output_image.parent\n""",
)

Path("Tests/XDRemuxCLITests/MotionPhotoOutputSafetyTests.swift").write_text(r'''import CryptoKit
import Foundation
import XCTest
import XDRemuxAppleFeatures
@testable import XDRemuxCLI

final class MotionPhotoOutputSafetyTests: XCTestCase {
    private let fixtureName = "IMG20260710191114_ColorOS_16.jpg"

    func testImplicitConvertPreservesForeignSameBasenamePairAndUsesNumberedOutput() throws {
        let sourceFixture = try fixtureURL()
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-motion-output-safety-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let source = directory.appendingPathComponent(fixtureName)
        try FileManager.default.copyItem(at: sourceFixture, to: source)
        let stem = source.deletingPathExtension().lastPathComponent
        let foreignImage = directory.appendingPathComponent(stem).appendingPathExtension("heic")
        let foreignVideo = directory.appendingPathComponent(stem).appendingPathExtension("mov")
        try Data("foreign-heic-do-not-touch".utf8).write(to: foreignImage, options: .atomic)
        try Data("foreign-mov-do-not-touch".utf8).write(to: foreignVideo, options: .atomic)
        let imageDigest = try digest(foreignImage)
        let videoDigest = try digest(foreignVideo)

        XCTAssertTrue(try MotionPhotoCLIIntegration.handleIfNeeded([
            "convert", "--input", source.path,
        ]))

        XCTAssertEqual(try digest(foreignImage), imageDigest)
        XCTAssertEqual(try digest(foreignVideo), videoDigest)
        let outputImage = directory.appendingPathComponent("\(stem) (2).heic")
        let outputVideo = directory.appendingPathComponent("\(stem) (2).mov")
        XCTAssertTrue(FileManager.default.fileExists(atPath: outputImage.path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: outputVideo.path))
        XCTAssertTrue(AppleLivePhotoValidator.isValidPair(imageURL: outputImage, videoURL: outputVideo))
    }

    func testExplicitConvertRefusesExistingForeignOutputAndPreservesBytes() throws {
        let sourceFixture = try fixtureURL()
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("xdremux-motion-explicit-output-safety-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let source = directory.appendingPathComponent(fixtureName)
        try FileManager.default.copyItem(at: sourceFixture, to: source)
        let outputImage = directory.appendingPathComponent("user-owned.heic")
        let outputVideo = directory.appendingPathComponent("user-owned.mov")
        try Data("foreign-explicit-heic".utf8).write(to: outputImage, options: .atomic)
        try Data("foreign-explicit-mov".utf8).write(to: outputVideo, options: .atomic)
        let imageDigest = try digest(outputImage)
        let videoDigest = try digest(outputVideo)

        XCTAssertThrowsError(try MotionPhotoCLIIntegration.handleIfNeeded([
            "convert", "--input", source.path, "--output", outputImage.path,
        ]))
        XCTAssertEqual(try digest(outputImage), imageDigest)
        XCTAssertEqual(try digest(outputVideo), videoDigest)
    }

    private func fixtureURL() throws -> URL {
        let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath, isDirectory: true)
        let fixture = root.appendingPathComponent("fixtures").appendingPathComponent(fixtureName)
        guard FileManager.default.fileExists(atPath: fixture.path) else {
            throw XCTSkip("repository Motion Photo fixture is unavailable: \(fixture.path)")
        }
        return fixture
    }

    private func digest(_ url: URL) throws -> SHA256.Digest {
        SHA256.hash(data: try Data(contentsOf: url))
    }
}
''', encoding="utf-8")

Path("Tests/test_python_live_photo_output_safety.py").write_text(r'''import hashlib
import shutil
import tempfile
import unittest
from pathlib import Path

from xdremux_py.live_photo import LivePhotoConversionError, convert_motion_photo, existing_pair_is_valid


class PythonLivePhotoOutputSafetyTests(unittest.TestCase):
    fixture_name = "IMG20260710191114_ColorOS_16.jpg"

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
''', encoding="utf-8")

replace(
    ".github/workflows/python-motion-photo-real-fixtures.yml",
    '      - "Tests/test_python_live_photo_portability.py"\n',
    '      - "Tests/test_python_live_photo_portability.py"\n      - "Tests/test_python_live_photo_output_safety.py"\n',
)

style_workflow = Path(".github/workflows/macos26-photographic-styles-smoke.yml")
text = style_workflow.read_text(encoding="utf-8")
if "  pull_request:\n" not in text:
    text = text.replace(
        "  workflow_dispatch:\n",
        """  pull_request:\n    paths:\n      - \"Sources/XDRemuxAppleFeatures/LivePhoto/**\"\n      - \"Sources/XDRemuxAppleFeatures/PhotographicStyles/**\"\n      - \"Sources/XDRemuxCLI/Commands/MotionPhoto*.swift\"\n      - \"Tests/XDRemuxAppleFeaturesTests/PhotographicStylesRunnerSmokeTests.swift\"\n      - \"fixtures/IMG20260710191114_ColorOS_16.jpg\"\n      - \".github/workflows/macos26-photographic-styles-smoke.yml\"\n  workflow_dispatch:\n""",
        1,
    )
text = text.replace("IMG20260801190843_ColorOS_16.jpg", "IMG20260710191114_ColorOS_16.jpg")
style_workflow.write_text(text, encoding="utf-8")

smoke = Path("Tests/XDRemuxAppleFeaturesTests/PhotographicStylesRunnerSmokeTests.swift")
text = smoke.read_text(encoding="utf-8")
text = text.replace("IMG20260801190843_ColorOS_16.jpg", "IMG20260710191114_ColorOS_16.jpg")
text = text.replace("13_591_436", "6_809_684", 1)
old = '''        XCTAssertTrue(\n            AppleLivePhotoStillWriter.hasGainMap(outputImageURL),\n            "combined output lost the Ultra HDR gain map"\n        )\n\n        let report = try AppleFeatureConversionEngine.validationReport(\n'''
new = '''        XCTAssertTrue(\n            AppleLivePhotoStillWriter.hasGainMap(outputImageURL),\n            "combined output lost the Ultra HDR gain map"\n        )\n        XCTAssertTrue(\n            result.diagnostics.contains { $0.contains("Vision Track5 cover alignment accepted") },\n            "fixture must exercise the accepted ColorOS 16 Vision Track5 alignment path"\n        )\n\n        let report = try AppleFeatureConversionEngine.validationReport(\n'''
if old not in text:
    raise SystemExit("style smoke insertion point not found")
text = text.replace(old, new, 1)
old_summary = '''            "gainMapPreserved": true,\n            "photographicStylesPassed": report["passed"] as? Bool ?? false,\n'''
new_summary = '''            "gainMapPreserved": true,\n            "visionTrack5Accepted": result.diagnostics.contains { $0.contains("Vision Track5 cover alignment accepted") },\n            "photographicStylesPassed": report["passed"] as? Bool ?? false,\n'''
if old_summary not in text:
    raise SystemExit("style smoke summary insertion point not found")
smoke.write_text(text.replace(old_summary, new_summary, 1), encoding="utf-8")
