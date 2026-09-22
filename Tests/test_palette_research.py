"""Palette research contracts; none of these assert a Gallery/device verdict."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from research.palette import probe

ROOT = Path(__file__).resolve().parents[1]


def sample(records=None, prefix=b"IMAGE", tag=b"jxrs"):
    if records is None:
        records = [dict(name="hdr.transform.data", offset=3, length=3, version=7, future={"value": "keep"})]
    text = json.dumps(records, separators=(",", ":")).encode()
    return prefix + b"GAPabc" + text + b"\0" + tag + struct.pack("<I", len(text) + 9)


class PaletteResearchTests(unittest.TestCase):
    def test_filter_layout_matches_both_original_hypotheses(self):
        for kind in ("palette-default", "default"):
            value = probe.filter_info(kind, "common")
            self.assertEqual(len(value), 220)
            self.assertAlmostEqual(struct.unpack_from("<f", value)[0], 5.1, places=5)
            self.assertEqual(struct.unpack_from("<6i", value, 4), (100, 0, 0, 0, 0, 1))
            self.assertEqual(value[28:128].split(b"\0")[0], kind.encode())
            self.assertEqual(struct.unpack_from("<4i", value, 140), (1, 1, 0, 0))
            self.assertEqual(value[156:206].split(b"\0")[0], b"common")
            self.assertEqual(value[206:208], bytes(2))
            self.assertAlmostEqual(struct.unpack_from("<f", value, 208)[0], .18, places=6)
        for kind, mode in (("other", "common"), ("default", "x" * 50), ("default", "a\0b")):
            with self.assertRaises(ValueError):
                probe.filter_info(kind, mode)

    def test_append_preserves_gaps_unknown_fields_order_and_original_prefix(self):
        original = sample()
        old = probe.parse(original)
        result = probe.replace_filter_info(original, "default", "common")
        new = probe.parse(result)
        self.assertEqual(result[:old.json_start], original[:old.json_start])
        self.assertEqual(new.entries[0].fields, dict(old.entries[0].fields, offset=223))
        self.assertEqual(new.payload(new.entries[0]), b"abc")
        self.assertEqual([e.name for e in new.entries], ["hdr.transform.data", "filter.info"])
        self.assertEqual(new.payload(new.entries[1]), probe.filter_info("default", "common"))
        self.assertEqual(result, probe.replace_filter_info(original, "default", "common"))

    def test_replacement_preserves_unreferenced_old_payload_and_other_records(self):
        records = [dict(name="filter.info", offset=6, length=3, version=1),
                   dict(name="other", offset=3, length=3, version=9)]
        original = sample(records)
        old = probe.parse(original)
        result = probe.replace_filter_info(original, "palette-default", "common")
        new = probe.parse(result)
        self.assertEqual([e.name for e in new.entries], ["filter.info", "other"])
        self.assertEqual(result[:old.json_start], original[:old.json_start])
        self.assertEqual(new.payload(new.entries[1]), b"abc")
        self.assertEqual(new.entries[1].fields["version"], 9)

    def test_context_omits_nonrender_data_and_retains_source_tag(self):
        records = [dict(name="watermark.device", offset=6, length=3),
                   dict(name="hdr.transform.data", offset=3, length=3, future=7)]
        source = sample(records, tag=b"test")
        base = struct.pack(">I4s4sI", 16, b"ftyp", b"heic", 0)
        result = probe.carry_context(source, base)
        parsed = probe.parse(result)
        self.assertTrue(result.startswith(base))
        self.assertEqual(parsed.tag, b"test")
        self.assertEqual([e.name for e in parsed.entries], ["hdr.transform.data"])
        self.assertEqual(parsed.payload(parsed.entries[0]), b"abc")
        self.assertEqual(parsed.entries[0].fields["future"], 7)
        with self.assertRaises(ValueError):
            probe.carry_context(source, b"not a HEIC")
        with self.assertRaises(ValueError):
            probe.carry_context(sample([dict(name="watermark", offset=3, length=3)]), base)

    def test_malformed_and_ambiguous_manifests_are_rejected(self):
        cases = [b"", b"x" * 30, sample()[:-1], sample()[:-4] + struct.pack("<I", 2**32-1)]
        for record in (dict(name="a", offset=True, length=1), dict(name="a", offset="3", length=1),
                       dict(name="a", offset=2, length=3), dict(name="a", offset=1000, length=1),
                       dict(name="a", offset=3, length=-1)):
            cases.append(sample([record]))
        cases += [sample([]), sample([dict(name="a", offset=3, length=3)] * 2),
                  sample([dict(name="a", offset=3, length=3), dict(name="b", offset=2, length=2)])]
        for data in cases:
            with self.subTest(data=data[-80:]), self.assertRaises(ValueError):
                probe.parse(data)
        for text in (b'[{"name":"a","name":"b","offset":3,"length":3}]',
                     b'[{"name":"a","offset":3,"length":3,"bad":NaN}]'):
            with self.assertRaises(ValueError):
                probe.parse(b"abc" + text + b"\0jxrs" + struct.pack("<I", len(text)+9))

    def test_atomic_publication_does_not_overwrite_and_cleans_scratch(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            destination = root / "out.heic"
            probe.publish_new(destination, b"complete")
            with self.assertRaises(FileExistsError):
                probe.publish_new(destination, b"other")
            self.assertEqual(destination.read_bytes(), b"complete")
            link = root / "link.heic"
            link.symlink_to(destination)
            with self.assertRaises(FileExistsError):
                probe.publish_new(link, b"other")
            with patch.object(probe.os, "link", side_effect=OSError("no hard links")):
                with self.assertRaises(OSError):
                    probe.publish_new(root / "unsupported.heic", b"data")
            self.assertEqual(sorted(p.name for p in root.iterdir()), ["link.heic", "out.heic"])
            self.assertEqual(destination.read_bytes(), b"complete")

    def test_real_proxdr_fixture_preserves_every_source_payload_and_byte_prefix(self):
        path = ROOT / "fixtures/proxdr/oppo/find-x9-ultra/uhdr-hr-01.heic"
        source = path.read_bytes()
        identity = hashlib.sha256(source).hexdigest()
        self.assertIn(f"{identity}  {path.relative_to(ROOT / 'fixtures')}", (ROOT / "fixtures/SHA256SUMS").read_text())
        old = probe.parse(source)
        for kind in ("default", "palette-default"):
            result = probe.replace_filter_info(source, kind, "common")
            new = probe.parse(result)
            self.assertEqual(result[:old.json_start], source[:old.json_start])
            actual = {e.name: new.payload(e) for e in new.entries}
            for entry in old.entries:
                self.assertEqual(actual[entry.name], old.payload(entry))
        self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), identity)

    def test_cli_reports_hypothesis_not_consumer_success_and_refuses_source_output(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "source.heic"
            source.write_bytes(sample())
            output = Path(directory) / "result.heic"
            command = [sys.executable, str(ROOT / "research/palette/probe.py"), "filter-info", str(source)]
            completed = subprocess.run(command + [str(output)], capture_output=True, text=True, check=True)
            report = json.loads(completed.stdout)
            self.assertEqual(report["consumer_verdict"], "unverified")
            self.assertTrue(report["research_only"])
            self.assertEqual(report["output_sha256"], hashlib.sha256(output.read_bytes()).hexdigest())
            failed = subprocess.run(command + [str(source)], capture_output=True, text=True)
            self.assertNotEqual(failed.returncode, 0)
            self.assertEqual(source.read_bytes(), sample())


if __name__ == "__main__":
    unittest.main()
