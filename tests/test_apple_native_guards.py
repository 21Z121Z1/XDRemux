"""Structural guardrails for native error paths without public fault-injection hooks.

These are source contracts; macOS integration evidence is still required.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class AppleNativeSafetyTests(unittest.TestCase):
    def test_passthrough_reader_must_complete_before_writer_finishes(self):
        source = (ROOT / "Sources/XDCinematicNative/cinematic_envelope.m").read_text()
        writer = source.split("static BOOL XDWriteMediaAndMetadata(", 1)[1].split(
            "int xdremux_cinematic_generate(", 1
        )[0]
        completion = writer.index("reader.status != AVAssetReaderStatusCompleted")
        finalize = writer.index("[writer finishWritingWithCompletionHandler:")
        self.assertLess(completion, finalize)
        self.assertIn("[writer cancelWriting];", writer[completion:finalize])
        self.assertIn("reader.error.localizedDescription", writer[completion:finalize])

    def test_uncaught_cinematic_exception_cleans_scratch(self):
        source = (ROOT / "Sources/XDCinematicNative/cinematic_envelope.m").read_text()
        entry = source.split("int xdremux_cinematic_generate(", 1)[1]
        self.assertIn("NSString *tempDirectory = nil;", entry)
        self.assertIn("tempDirectory = [NSTemporaryDirectory()", entry)
        catch = entry.rsplit("} @catch (NSException *exception) {", 1)[1]
        self.assertIn("removeItemAtPath:tempDirectory", catch)

    def test_main10_reports_effective_compressed_sample_color_tags(self):
        source = (ROOT / "Sources/XDRemuxAppleAdapter/VideoToolboxMain10.swift").read_text()
        self.assertIn("CMFormatDescriptionGetExtensions(formatDescription)", source)
        for key in (
            "kCMFormatDescriptionExtension_ColorPrimaries",
            "kCMFormatDescriptionExtension_TransferFunction",
            "kCMFormatDescriptionExtension_YCbCrMatrix",
        ):
            self.assertIn(key, source)
        for name in ("color_primaries", "transfer_function", "ycbcr_matrix"):
            self.assertIn(name, source)


if __name__ == "__main__":
    unittest.main()
