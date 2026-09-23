"""Failure receipts must remain evidence, not a waiver of carrier parity."""
import contextlib
from fractions import Fraction
import copy
import hashlib
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from research.video_styles import probe, smoke
from Tests.test_video_style_research import media_report


class VideoStyleDiagnosticsTests(unittest.TestCase):
    def test_final_packet_duration_records_track_index_and_exact_rationals(self):
        source, candidate = media_report(), media_report()
        source["streams"][0]["id"] = "0x1"
        candidate["streams"][0]["id"] = "0xa"
        candidate["packets"][1]["duration"] = 3
        with self.assertRaises(probe.TimingMismatch) as raised:
            probe.verify_parity(source, candidate, timed=False)
        details = raised.exception.details
        self.assertEqual(details["sourceTrackID"], "0x1")
        self.assertEqual(details["candidateTrackID"], "0xa")
        self.assertEqual(details["packetIndex"], 1)
        self.assertEqual(details["kind"], "video")
        self.assertEqual(details["field"], "duration")
        self.assertEqual(details["source"], dict(ticks=1, timeBase="1/30", numerator=1, denominator=30))
        self.assertEqual(details["candidate"], dict(ticks=3, timeBase="1/30", numerator=1, denominator=10))
        json.dumps(details, allow_nan=False)

    def test_audio_and_stream_timing_failures_are_not_misattributed(self):
        source = media_report()
        for stream_level in (False, True):
            candidate = copy.deepcopy(source)
            if stream_level:
                candidate["streams"][1]["duration_ts"] += 1
            else:
                candidate["packets"][3]["dts"] += 1
            with self.subTest(stream=stream_level), self.assertRaises(probe.TimingMismatch) as raised:
                probe.verify_parity(source, candidate, timed=False)
            details = raised.exception.details
            self.assertEqual(details["kind"], "audio")
            self.assertEqual(details["sourceStreamIndex"], 1)
            self.assertEqual(details["packetIndex"], None if stream_level else 1)
            self.assertEqual(Fraction(details["candidate"]["numerator"], details["candidate"]["denominator"]),
                             Fraction(3201 if stream_level else 1601, 48000))

    def test_failure_keeps_hashes_without_retaining_or_publishing_media(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, target = root / "source.mov", root / "output.mov"
            source.write_bytes(b"source")
            after = media_report()
            after["packets"][1]["duration"] += 1
            def construct(command, _directory):
                Path(command[2]).write_bytes(b"candidate")
                return dict(schema=1, variant="static", metadataReadback=True)
            with patch.object(probe, "inspect_media", side_effect=[media_report(), after]), \
                 patch.object(probe, "_json_command", side_effect=construct), \
                 self.assertRaises(probe.TimingMismatch) as raised:
                probe.run(source, target, helper=source, variant="static")
            receipt = raised.exception.evidence
            self.assertFalse(receipt["passed"])
            self.assertFalse(receipt["published"])
            self.assertTrue(receipt["sourceUnchanged"])
            self.assertEqual(receipt["sourceSHA256"], hashlib.sha256(b"source").hexdigest())
            self.assertEqual(receipt["candidateSHA256"], hashlib.sha256(b"candidate").hexdigest())
            self.assertEqual(receipt["error"]["packetIndex"], 1)
            self.assertEqual(list(root.iterdir()), [source])

    def test_cli_emits_failed_json_and_unsuccessful_status(self):
        output, errors = io.StringIO(), io.StringIO()
        failure = ValueError("explicit parity failure")
        failure.evidence = probe.failure_evidence(failure, variant="lower", source_hash="a" * 64)
        with patch.object(probe, "run", side_effect=failure), \
             contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
            status = probe.main(["input.mov", "output.mov", "--helper", "helper", "--variant", "lower"])
        self.assertNotEqual(status, 0)
        self.assertFalse(json.loads(output.getvalue())["passed"])
        self.assertIn("parity failure", errors.getvalue())

    def matrix(self, *, fail=None, terminal=True, clobber=False, missing=False):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        helper = root / "helper"
        helper.write_bytes(b"not executed by test")
        helper.chmod(0o700)
        calls = []
        def prepare(root, codec, _extra):
            source = root / f"{codec}.mov"
            source.write_bytes(codec.encode())
            return source
        def run(source, target, *, helper, variant):
            if source.name == "malformed.mov":
                raise ValueError("ffprobe failed (1): invalid input")
            calls.append((source.stem, variant, target.exists()))
            if target.exists():
                if clobber:
                    target.write_bytes(b"corrupted")
                raise FileExistsError(target)
            if fail == (source.stem, variant):
                raise ValueError("intentional parity failure")
            target.write_bytes(b"candidate")
            return {"variant": variant, "passed": True, "published": True,
                    "sourceSHA256": probe.digest(source), "candidateSHA256": probe.digest(target),
                    "native": {"terminalEmptyMarkersSkipped": int(terminal)},
                    "photosEditingValidated": False}
        with patch.object(smoke, "prepare_source", side_effect=prepare), \
             patch.object(smoke, "run", side_effect=run), \
             patch.object(smoke.shutil, "which", return_value=None if missing else "/test/tool"):
            report = smoke.run_matrix(helper, root)
        return report, calls

    def test_matrix_keeps_all_codecs_and_hypotheses_after_first_failure(self):
        report, calls = self.matrix(fail=("h264", "static"))
        self.assertFalse(report["passed"])
        self.assertEqual(report["requiredCases"], 8)
        self.assertEqual(len(report["receipts"]), 8)
        self.assertEqual({(r["fixture"], r["variant"]) for r in report["receipts"]},
                         {(c, v) for c, _ in smoke.CODECS for v in probe.VARIANTS})
        self.assertEqual(sum(r["passed"] for r in report["receipts"]), 7)
        self.assertEqual(len([c for c in calls if not c[2]]), 8)
        self.assertTrue(report["malformedInput"]["passed"])

    def test_terminal_empty_marker_coverage_is_mandatory(self):
        report, _ = self.matrix(terminal=False)
        self.assertFalse(report["passed"])
        self.assertTrue(all(not r["passed"] for r in report["receipts"]))
        self.assertTrue(all("terminal" in r["error"]["message"] for r in report["receipts"]))

    def test_all_eight_no_clobber_controls_are_mandatory(self):
        report, calls = self.matrix()
        self.assertTrue(report["passed"])
        self.assertEqual(len([c for c in calls if c[2]]), 8)
        self.assertTrue(all(r["noClobberValidated"] and r["sourceUnchanged"] for r in report["receipts"]))
        report, _ = self.matrix(clobber=True)
        self.assertFalse(report["passed"])
        self.assertTrue(all(not r["passed"] for r in report["receipts"]))

    def test_failed_prerequisite_is_not_successful_malformed_control(self):
        report, calls = self.matrix(missing=True)
        self.assertFalse(report["passed"])
        self.assertFalse(report["malformedInput"]["passed"])
        self.assertEqual(len(report["receipts"]), 8)
        self.assertEqual(calls, [])
