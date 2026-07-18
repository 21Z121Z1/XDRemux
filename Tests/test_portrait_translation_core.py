import base64
import json
import platform
import re
import struct
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PORTRAIT_SOURCE = (
    ROOT
    / "Sources"
    / "XDRemuxAppleFeatures"
    / "Portrait"
    / "PortraitConversionPipeline.swift"
)
DYNAMIC_IDS = set(range(0x0190, 0x019A)) | set(range(0x01C2, 0x01C6))


def static_rend_profiles() -> list[bytes]:
    source = PORTRAIT_SOURCE.read_text(encoding="utf-8")
    matches = re.findall(
        r"portraitStaticRenderingProfile[^=]+Base64 = \"\"\"(.*?)\"\"\"",
        source,
        flags=re.DOTALL,
    )
    return [base64.b64decode("".join(match.split())) for match in matches]


class PortraitTranslationCoreTests(unittest.TestCase):
    def test_static_profiles_exclude_all_per_scene_records(self) -> None:
        profiles = static_rend_profiles()
        self.assertEqual(len(profiles), 4)
        for payload in profiles:
            self.assertEqual(payload[:4], b"REND")
            self.assertEqual(struct.unpack_from("<I", payload, 8)[0], len(payload))
            self.assertEqual((len(payload) - 16) % 8, 0)
            identifiers = {
                struct.unpack_from("<H", payload, offset)[0]
                for offset in range(16, len(payload), 8)
            }
            self.assertEqual(len(identifiers), 153)
            self.assertFalse(identifiers & DYNAMIC_IDS)

    @unittest.skipUnless(platform.system() == "Darwin", "Swift/ImageIO CLI is macOS-only")
    def test_swift_rend_parser_and_per_scene_builder(self) -> None:
        subprocess.run(
            ["swift", "build", "--product", "xdremux-dev"],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        completed = subprocess.run(
            [str(ROOT / ".build" / "debug" / "xdremux-dev"), "portrait-self-test"],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        report = json.loads(completed.stdout)
        self.assertTrue(report["passed"])
        self.assertTrue(report["byteStableRoundTrip"])
        self.assertTrue(report["perSceneRENDIsDistinct"])
        self.assertTrue(report["nativeXHLRBVectorMatched"])
        self.assertTrue(report["nativeXHLRBDefaultsMatched"])
        self.assertTrue(report["focusDispatchVectorsMatched"])
        self.assertTrue(report["petNoRectHistogramFractionMatched"])
        self.assertTrue(report["nativeDepthRoundTripMatched"])
        self.assertTrue(report["malformedLengthRejected"])
        self.assertTrue(report["duplicateRecordRejected"])
        self.assertTrue(report["scratchFileNameIsBounded"])
        self.assertLess(report["scratchFileNameLength"], 255)
        self.assertFalse(report["staticProfileContainsDynamicRecords"])
        self.assertNotEqual(report["nearRENDSHA256"], report["farRENDSHA256"])


if __name__ == "__main__":
    unittest.main()
