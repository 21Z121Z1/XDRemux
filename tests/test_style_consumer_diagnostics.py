"""Exercise the product CI's read-only Styles diagnostic entry point."""
import json
from pathlib import Path
import plistlib
import subprocess
import struct
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]


class StyleConsumerDiagnosticTests(unittest.TestCase):
    def diagnostic(self, *, method=0, version=15, data_reference=0, truncated=False):
        # Location-level fixture: these bytes are not a native HEVC/Photos proof.
        payload = plistlib.dumps({"0": version, "1": bytes(51840), "3": bytes(516)},
                                 fmt=plistlib.FMT_BINARY)
        texture = b"tag:apple.com,2026:photo:metadata:texture_styles\0existing-texture"
        source = b"header" + struct.pack(">I4s", 8 + len(payload), b"idat") + payload + texture
        location = {"construction_method": method, "data_reference": data_reference,
                    "extents": [{"offset": 14 if method == 0 else 0,
                                 "length": len(payload) + (len(texture) + 1 if truncated else 0)}]}
        report = {"items": [{"type": "uri ", "name": "styleMetadata", "location": location}],
                  "meta_children": [] if method == 0 else [
                      {"type": "idat", "start": 6, "end": 14 + len(payload),
                       "size": 8 + len(payload)}]}
        script = (ROOT / "scripts/apple/check_rust_style_consumer.sh").read_text()
        marker = 'python3 - "$target" "$inspection" <<\'PY\'\n'
        self.assertEqual(script.count(marker), 1)
        entry = script.split(marker, 1)[1].split("\nPY", 1)[0]
        with tempfile.TemporaryDirectory(prefix="xdremux-style-diagnostic-") as tmp:
            image = Path(tmp) / "native-v15.heic"
            inspection = Path(tmp) / "inspection.json"
            image.write_bytes(source)
            inspection.write_text(json.dumps(report))
            result = subprocess.run([sys.executable, "-c", entry, str(image), str(inspection)],
                                    cwd=ROOT, capture_output=True, text=True, check=False)
            self.assertEqual(image.read_bytes(), source, "diagnostics must never patch the carrier")
            self.assertEqual(sorted(p.name for p in Path(tmp).iterdir()),
                             ["inspection.json", "native-v15.heic"])
            return result

    def test_current_native_file_backed_v15_is_read_without_an_idat(self):
        result = self.diagnostic()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Photos Tone/Color admission remains UNVERIFIED", result.stdout)

    def test_idat_backed_v15_remains_supported(self):
        result = self.diagnostic(method=1)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_wrong_version_does_not_pass_the_product_diagnostic(self):
        self.assertNotEqual(self.diagnostic(method=1, version=16).returncode, 0)

    def test_external_and_truncated_extents_are_rejected(self):
        self.assertNotEqual(self.diagnostic(data_reference=1).returncode, 0)
        self.assertNotEqual(self.diagnostic(truncated=True).returncode, 0)


if __name__ == "__main__":
    unittest.main()
