"""Bounded Video Smart Style carrier experiment; not an XDRemux product backend.

Construction is delegated to AVFoundation. Publication requires complete encoded
packet, timing, codec/color and reported source-metadata parity. None of these
checks proves that Photos accepts, renders, or reversibly edits the hypothesis.
"""
from __future__ import annotations

import argparse
from collections import defaultdict
from fractions import Fraction
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
from typing import Any

VARIANTS = ("static", "lower", "compact", "upper")
MAX_INPUT_BYTES = 2 * 1024**3
MAX_REPORT_BYTES = 128 * 1024**2
TIMEOUT_SECONDS = 120
CODEC_FIELDS = (
    "codec_type", "codec_name", "profile", "width", "height", "coded_width",
    "coded_height", "pix_fmt", "level", "sample_aspect_ratio", "field_order",
    "color_range", "color_space", "color_transfer", "color_primaries",
    "chroma_location", "bits_per_raw_sample", "bits_per_sample", "sample_rate",
    "channels", "channel_layout", "extradata_size", "extradata_hash",
)
# These describe the MOV container/producer, not captured scene or media state.
CONTAINER_TAGS = frozenset({"major_brand", "minor_version", "compatible_brands", "encoder"})
STREAM_CONTAINER_TAGS = frozenset({"handler_name", "vendor_id", "encoder"})


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def _json_command(command: list[str], directory: Path) -> dict[str, Any]:
    # A temporary report avoids unbounded stdout accumulation in Python. Inputs
    # and wall time are bounded; the report limit is checked before decoding.
    with tempfile.TemporaryFile(dir=directory) as stream:
        result = subprocess.run(command, stdout=stream, stderr=subprocess.PIPE,
                                timeout=TIMEOUT_SECONDS, check=False)
        if result.returncode:
            raise ValueError(f"{Path(command[0]).name} failed ({result.returncode}): "
                             + result.stderr.decode("utf-8", errors="replace")[-4096:])
        if stream.tell() > MAX_REPORT_BYTES:
            raise ValueError("probe report exceeds the research size limit")
        stream.seek(0)
        document = json.load(stream)
    if not isinstance(document, dict):
        raise ValueError("probe report must be a JSON object")
    return document


def inspect_media(path: Path, directory: Path, ffprobe: str) -> dict[str, Any]:
    return _json_command([ffprobe, "-v", "error", "-protocol_whitelist", "file",
                          "-show_streams", "-show_packets", "-show_format",
                          "-show_chapters", "-show_data_hash", "sha256", "-of", "json",
                          str(path)], directory)


def _time(packet: dict[str, Any], field: str, time_base: Fraction) -> Fraction:
    value = packet.get(field)
    if isinstance(value, bool) or not re.fullmatch(r"-?\d+", str(value)):
        raise ValueError(f"missing or invalid exact {field}: {value!r}")
    return int(value) * time_base


def _packets(report: dict[str, Any]) -> dict[int, list[dict[str, Any]]]:
    result: dict[int, list[dict[str, Any]]] = defaultdict(list)
    for packet in report.get("packets", []):
        result[packet["stream_index"]].append(packet)
    return result


def _tags(source: dict[str, Any], candidate: dict[str, Any], ignored: frozenset[str]) -> None:
    for key, value in source.get("tags", {}).items():
        if key not in ignored and candidate.get("tags", {}).get(key) != value:
            raise ValueError(f"source metadata changed or lost: {key}")


def verify_parity(source: dict[str, Any], candidate: dict[str, Any], *, timed: bool) -> dict[str, Any]:
    """Require all A/V packets, not just the first frame, to survive exactly.

    Stream indices and container layout may change. Packet order *within each
    track*, rational timestamps, payloads, codec configuration, color and display
    matrix may not. Unknown/extra source tracks and chapters are not discarded.
    """
    before = source.get("streams", [])
    after = candidate.get("streams", [])
    if not before or any(s.get("codec_type") not in {"video", "audio"} for s in before):
        raise ValueError("only audio/video source tracks are supported by this research probe")
    if sum(s["codec_type"] == "video" for s in before) != 1:
        raise ValueError("exactly one source video track is required")
    if source.get("chapters") or candidate.get("chapters"):
        raise ValueError("chapter semantics are not supported")
    media = [s for s in after if s.get("codec_type") in {"video", "audio"}]
    extra = [s for s in after if s.get("codec_type") not in {"video", "audio"}]
    if len(media) != len(before) or len(extra) != int(timed):
        raise ValueError("unexpected track count")
    if extra and extra[0].get("codec_tag_string") != "mebx":
        raise ValueError("expected an AVFoundation boxed metadata track")
    src_packets, dst_packets = _packets(source), _packets(candidate)
    counts = []
    for old, new in zip(before, media, strict=True):
        for field in CODEC_FIELDS:
            if old.get(field) != new.get(field):
                raise ValueError(f"codec/color contract changed: {field}")
        if old.get("side_data_list", []) != new.get("side_data_list", []):
            raise ValueError("display matrix or stream side data changed")
        _tags(old, new, STREAM_CONTAINER_TAGS)
        old_base, new_base = Fraction(old["time_base"]), Fraction(new["time_base"])
        if old_base <= 0 or new_base <= 0:
            raise ValueError("invalid stream time base")
        for field in ("start_pts", "duration_ts"):
            if _time(old, field, old_base) != _time(new, field, new_base):
                raise ValueError(f"stream {field} changed")
        left, right = src_packets[old["index"]], dst_packets[new["index"]]
        if not left or len(left) != len(right):
            raise ValueError("encoded packet count changed or track is empty")
        for index, (a, b) in enumerate(zip(left, right, strict=True)):
            if not re.fullmatch(r"SHA256:[0-9a-fA-F]{64}", str(a.get("data_hash"))):
                raise ValueError("packet payload checksum is missing")
            for field in ("data_hash", "size", "flags", "side_data_list"):
                if a.get(field) != b.get(field):
                    raise ValueError(f"packet {index} {field} changed")
            for field in ("pts", "dts", "duration"):
                if _time(a, field, old_base) != _time(b, field, new_base):
                    raise ValueError(f"packet {index} {field} changed")
        counts.append({"kind": old["codec_type"], "packets": len(left)})
    _tags(source.get("format", {}), candidate.get("format", {}), CONTAINER_TAGS)
    return {"encodedMediaParity": True, "tracks": counts,
            "evidenceLevel": "encoded-packets-and-reported-metadata",
            "photosEditingValidated": False}


def run(source: Path, destination: Path, *, helper: Path, variant: str,
        ffprobe: str = "ffprobe") -> dict[str, Any]:
    if variant not in VARIANTS:
        raise ValueError("unknown metadata hypothesis")
    source = source.resolve(strict=True)
    helper = helper.resolve(strict=True)
    destination = destination.parent.resolve(strict=True) / destination.name
    if not source.is_file() or not 0 < source.stat().st_size <= MAX_INPUT_BYTES:
        raise ValueError("source must be a nonempty file no larger than 2 GiB")
    if os.path.lexists(destination):
        raise FileExistsError(destination)
    original = digest(source)
    with tempfile.TemporaryDirectory(prefix=".video-style-", dir=destination.parent) as temporary:
        directory = Path(temporary)
        candidate = directory / "candidate.mov"
        before = inspect_media(source, directory, ffprobe)
        # Reject unsupported source tracks before invoking a writer.
        if any(s.get("codec_type") not in {"video", "audio"} for s in before.get("streams", [])):
            raise ValueError("source contains unsupported non-audio/video tracks")
        native = _json_command([str(helper), str(source), str(candidate), variant], directory)
        if native.get("schema") != 1 or native.get("variant") != variant or native.get("metadataReadback") is not True:
            raise ValueError("native metadata readback did not validate the requested hypothesis")
        after = inspect_media(candidate, directory, ffprobe)
        parity = verify_parity(before, after, timed=variant != "static")
        if digest(source) != original:
            raise ValueError("source changed during the experiment")
        candidate_hash = digest(candidate)
        with candidate.open("rb") as stream:
            os.fsync(stream.fileno())
        # Same-filesystem link is atomic and never replaces an existing name.
        # A filesystem without hard links fails explicitly; there is no copy or
        # replace fallback which could expose a partial or clobbered result.
        os.link(candidate, destination)
    return {"schema": 1, "variant": variant, "sourceSHA256": original,
            "candidateSHA256": candidate_hash, "native": native, **parity}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--helper", required=True, type=Path)
    parser.add_argument("--variant", required=True, choices=VARIANTS)
    parser.add_argument("--ffprobe", default="ffprobe")
    args = parser.parse_args()
    try:
        result = run(args.source, args.destination, helper=args.helper,
                     variant=args.variant, ffprobe=args.ffprobe)
    except (OSError, ValueError, KeyError, TypeError, subprocess.TimeoutExpired) as error:
        parser.exit(2, f"video-style probe: {error}\n")
    print(json.dumps(result, sort_keys=True, allow_nan=False))


if __name__ == "__main__":
    main()
