import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
READER = ROOT/"scripts/research/read_texture_native_log.py"
spec = importlib.util.spec_from_file_location("texture_log", READER)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class TextureNativeEvidenceTests(unittest.TestCase):
    def log(self, payload='{"native": true}\n', expected_payload=None, name="case/result.json"):
        original = payload if expected_payload is None else expected_payload
        manifest = {"head_sha": "a"*40, "run_id": 123, "files": [{
            "path": "metadata/"+name, "size": len(original.encode()),
            "sha256": hashlib.sha256(original.encode()).hexdigest()}]}
        text = "TEXTURE_JSON_BEGIN "+name+"\n"+payload+"TEXTURE_JSON_END "+name+"\n"
        text += "TEXTURE_JSON_BEGIN manifest.json\n"+json.dumps(manifest)+"\nTEXTURE_JSON_END manifest.json\n"
        return "".join("2026-10-19T16:00:00.0000000Z "+line+"\n" for line in text.splitlines())

    def test_recovers_actual_timestamped_text_without_claiming_archive_or_device(self):
        with tempfile.TemporaryDirectory() as temporary:
            result = module.extract(self.log(), Path(temporary)/"new")
            self.assertEqual(result["verified_payload_count"], 1)
            self.assertFalse(result["archive_bytes_verified"])
            self.assertEqual((Path(temporary)/"new/case/result.json").read_text(), '{"native": true}\n')

    def test_tampered_native_result_fails_original_manifest(self):
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaisesRegex(ValueError, "hash does not match"):
                module.extract(self.log('{"native": false}\n', '{"native": true}\n'), Path(temporary)/"new")

    def test_unsafe_paths_and_incomplete_native_blocks_fail(self):
        for log in [self.log(name="../escape.json"), "TEXTURE_JSON_BEGIN result.json\n{}\n"]:
            with tempfile.TemporaryDirectory() as temporary:
                with self.assertRaises(ValueError):
                    module.extract(log, Path(temporary)/"new")

    def test_cli_preserves_carriage_returns_in_whole_job_log_hash(self):
        log = "native progress\r\n"+self.log()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root/"job.log"
            source.write_bytes(log.encode())
            subprocess.run([sys.executable, str(READER), str(source), "--out", str(root/"new")],
                           check=True, capture_output=True, text=True)
            result = json.loads((root/"new/log-readback.json").read_text())
            self.assertEqual(result["log_sha256"], hashlib.sha256(log.encode()).hexdigest())
