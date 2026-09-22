import copy
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from research.video_styles import probe


def media_report(*, timed=False):
    streams = [dict(index=0, codec_type="video", codec_name="h264", time_base="1/30",
                    start_pts=0, duration_ts=2, color_primaries="bt709", pix_fmt="yuv420p",
                    extradata_hash="SHA256:" + "a" * 64,
                    side_data_list=[{"side_data_type": "Display Matrix", "rotation": 90}]),
               dict(index=1, codec_type="audio", codec_name="aac", time_base="1/48000",
                    start_pts=0, duration_ts=3200, channels=2, sample_rate="48000")]
    packets = []
    for stream in streams:
        for index in range(2):
            duration = 1 if stream["index"] == 0 else 1600
            packets.append(dict(stream_index=stream["index"], pts=index * duration,
                                dts=index * duration, duration=duration, size="20", flags="K_",
                                data_hash="SHA256:" + hashlib.sha256(bytes([stream["index"], index])).hexdigest()))
    if timed:
        streams.append(dict(index=2, codec_type="data", codec_tag_string="mebx"))
    return dict(streams=streams, packets=packets, format={"tags": {"title": "source title"}}, chapters=[])


class VideoStyleResearchTests(unittest.TestCase):
    def test_complete_packet_parity_accepts_time_base_rescaling(self):
        source = media_report()
        candidate = media_report(timed=True)
        candidate["streams"][0].update(time_base="1/30000", duration_ts=2000)
        for packet in candidate["packets"]:
            if packet["stream_index"] == 0:
                for key in ("pts", "dts", "duration"):
                    packet[key] *= 1000
        result = probe.verify_parity(source, candidate, timed=True)
        self.assertEqual(result["tracks"], [{"kind": "video", "packets": 2}, {"kind": "audio", "packets": 2}])
        self.assertFalse(result["photosEditingValidated"])

    def test_failures_after_first_frame_are_not_hidden(self):
        source = media_report()
        mutations = [
            lambda r: r["packets"][1].update(data_hash="SHA256:" + "b" * 64),
            lambda r: r["packets"].pop(),
            lambda r: r["packets"][3].update(pts=1601),
            lambda r: r["streams"][0].update(color_primaries="bt2020"),
            lambda r: r["streams"][0].update(pix_fmt="yuv420p10le"),
            lambda r: r["streams"][0].update(side_data_list=[]),
            lambda r: r["streams"].pop(),
            lambda r: r["format"].update(tags={}),
            lambda r: r["streams"][0].update(extradata_hash="SHA256:" + "c" * 64),
        ]
        for mutate in mutations:
            candidate = copy.deepcopy(source)
            mutate(candidate)
            with self.subTest(mutation=mutate), self.assertRaises(ValueError):
                probe.verify_parity(source, candidate, timed=False)

    def test_unknown_tracks_chapters_and_missing_timing_fail_closed(self):
        for key in ("pts", "dts", "duration", "data_hash"):
            source = media_report()
            source["packets"][0].pop(key)
            with self.subTest(key=key), self.assertRaises(ValueError):
                probe.verify_parity(source, source, timed=False)
        source = media_report(timed=True)
        with self.assertRaisesRegex(ValueError, "source tracks"):
            probe.verify_parity(source, source, timed=True)
        source = media_report()
        source["chapters"] = [{}]
        with self.assertRaisesRegex(ValueError, "chapter"):
            probe.verify_parity(source, source, timed=False)

    def test_existing_destination_and_source_never_replaced(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "source.mov"
            path.write_bytes(b"sentinel")
            with self.assertRaises(FileExistsError):
                probe.run(path, path, helper=path, variant="static")
            self.assertEqual(path.read_bytes(), b"sentinel")

    def test_invalid_native_or_packet_results_are_not_published(self):
        for corrupt in ("native", "packet"):
            with self.subTest(corrupt=corrupt), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                source = root / "source.mov"
                target = root / "output.mov"
                source.write_bytes(b"source")
                def construct(command, _directory):
                    Path(command[2]).write_bytes(b"candidate")
                    return dict(schema=1, variant="static", metadataReadback=corrupt != "native")
                after = media_report()
                if corrupt == "packet":
                    after["packets"].pop()
                with patch.object(probe, "inspect_media", side_effect=[media_report(), after]), \
                     patch.object(probe, "_json_command", side_effect=construct), \
                     self.assertRaises(ValueError):
                    probe.run(source, target, helper=source, variant="static")
                self.assertFalse(target.exists())
                self.assertEqual(source.read_bytes(), b"source")
                self.assertEqual(sorted(p.name for p in root.iterdir()), ["source.mov"])

    def test_success_publishes_once_and_racing_destination_is_not_clobbered(self):
        for race in (False, True):
            with self.subTest(race=race), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                source, target = root / "source.mov", root / "output.mov"
                source.write_bytes(b"source")
                def construct(command, _directory):
                    Path(command[2]).write_bytes(b"candidate")
                    if race:
                        target.write_bytes(b"other writer")
                    return dict(schema=1, variant="static", metadataReadback=True)
                with patch.object(probe, "inspect_media", side_effect=[media_report(), media_report()]), \
                     patch.object(probe, "_json_command", side_effect=construct):
                    if race:
                        with self.assertRaises(FileExistsError):
                            probe.run(source, target, helper=source, variant="static")
                    else:
                        result = probe.run(source, target, helper=source, variant="static")
                        self.assertEqual(result["candidateSHA256"], hashlib.sha256(b"candidate").hexdigest())
                self.assertEqual(target.read_bytes(), b"other writer" if race else b"candidate")
                self.assertEqual(source.read_bytes(), b"source")
                self.assertEqual(len(list(root.iterdir())), 2)
