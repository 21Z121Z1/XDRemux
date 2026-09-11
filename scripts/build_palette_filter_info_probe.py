#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import struct
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

FILTER_INFO_SIZE = 220
FILTER_TYPE_OFFSET = 28
FILTER_TYPE_SIZE = 100
CAPTURE_MODE_OFFSET = 156
CAPTURE_MODE_SIZE = 50
AVG_LUMA_OFFSET = 208


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _c_string(value: str, size: int) -> bytes:
    raw = value.encode("utf-8")
    if len(raw) >= size:
        raise ValueError(f"string is too long for {size}-byte field: {value!r}")
    return raw + b"\x00" * (size - len(raw))


def _read_c_string(data: bytes) -> str:
    return data.split(b"\x00", 1)[0].decode("utf-8", errors="replace")


@dataclass(frozen=True)
class FilterPhotoInfoV51:
    version: float = 5.1
    filter_intensity: int = 100
    src_orientation: int = 0
    src_mirror: int = 0
    soft_light_enable: int = 0
    soft_light_type: int = 0
    filter_enable: int = 1
    filter_type: str = "palette-default"
    color_temperature: float = 6500.0
    lux_index: float = 100.0
    filter_saturation: float = 0.0
    is_palette: int = 1
    palette_algo_used: int = 1
    portrait_mask_used: int = 0
    filter_tone: int = 0
    capture_mode: str = "common"
    avg_luma: float = 0.18
    bright_luma: float = 0.75
    dark_luma: float = 0.03

    def encode(self) -> bytes:
        out = bytearray(FILTER_INFO_SIZE)
        struct.pack_into("<f", out, 0, self.version)
        struct.pack_into(
            "<6i",
            out,
            4,
            self.filter_intensity,
            self.src_orientation,
            self.src_mirror,
            self.soft_light_enable,
            self.soft_light_type,
            self.filter_enable,
        )
        out[FILTER_TYPE_OFFSET:FILTER_TYPE_OFFSET + FILTER_TYPE_SIZE] = _c_string(
            self.filter_type, FILTER_TYPE_SIZE
        )
        struct.pack_into("<3f", out, 128, self.color_temperature, self.lux_index, self.filter_saturation)
        struct.pack_into(
            "<4i",
            out,
            140,
            self.is_palette,
            self.palette_algo_used,
            self.portrait_mask_used,
            self.filter_tone,
        )
        out[CAPTURE_MODE_OFFSET:CAPTURE_MODE_OFFSET + CAPTURE_MODE_SIZE] = _c_string(
            self.capture_mode, CAPTURE_MODE_SIZE
        )
        # The fixed 50-byte capture-mode field ends at 206. The following
        # float fields are naturally aligned to a four-byte boundary.
        struct.pack_into("<3f", out, AVG_LUMA_OFFSET, self.avg_luma, self.bright_luma, self.dark_luma)
        return bytes(out)

    @classmethod
    def decode(cls, data: bytes) -> "FilterPhotoInfoV51":
        if len(data) < FILTER_INFO_SIZE:
            raise ValueError(f"filter.info is shorter than {FILTER_INFO_SIZE} bytes: {len(data)}")
        version = struct.unpack_from("<f", data, 0)[0]
        ints = struct.unpack_from("<6i", data, 4)
        filter_type = _read_c_string(data[FILTER_TYPE_OFFSET:FILTER_TYPE_OFFSET + FILTER_TYPE_SIZE])
        color_temperature, lux_index, filter_saturation = struct.unpack_from("<3f", data, 128)
        is_palette, palette_algo_used, portrait_mask_used, filter_tone = struct.unpack_from("<4i", data, 140)
        capture_mode = _read_c_string(data[CAPTURE_MODE_OFFSET:CAPTURE_MODE_OFFSET + CAPTURE_MODE_SIZE])
        avg_luma, bright_luma, dark_luma = struct.unpack_from("<3f", data, AVG_LUMA_OFFSET)
        return cls(
            version=version,
            filter_intensity=ints[0],
            src_orientation=ints[1],
            src_mirror=ints[2],
            soft_light_enable=ints[3],
            soft_light_type=ints[4],
            filter_enable=ints[5],
            filter_type=filter_type,
            color_temperature=color_temperature,
            lux_index=lux_index,
            filter_saturation=filter_saturation,
            is_palette=is_palette,
            palette_algo_used=palette_algo_used,
            portrait_mask_used=portrait_mask_used,
            filter_tone=filter_tone,
            capture_mode=capture_mode,
            avg_luma=avg_luma,
            bright_luma=bright_luma,
            dark_luma=dark_luma,
        )


def parse_tail(data: bytes) -> tuple[list[dict[str, Any]], list[tuple[int, dict[str, Any], bytes]], int, bytes]:
    if len(data) < 9 or data[-9] != 0:
        raise ValueError("missing extension footer")
    tag = data[-8:-4]
    if len(tag) != 4 or not all(32 <= value <= 126 for value in tag):
        raise ValueError("invalid extension footer tag")
    span = struct.unpack_from("<I", data, len(data) - 4)[0]
    if span < 9 or span > len(data):
        raise ValueError(f"invalid extension footer span: {span}")
    manifest_len = span - 9
    json_start = len(data) - 9 - manifest_len
    raw_manifest = data[json_start:len(data) - 9]
    records = json.loads(raw_manifest)
    if not isinstance(records, list):
        raise ValueError("extension manifest is not a list")

    physical: list[tuple[int, dict[str, Any], bytes]] = []
    for record in records:
        length = int(record["length"])
        start = json_start - int(record["offset"])
        end = start + length
        if length < 0 or start < 0 or end > json_start:
            raise ValueError(f"entry out of bounds: {record.get('name')}")
        physical.append((start, record, data[start:end]))
    physical.sort(key=lambda item: item[0])
    payload_start = min((item[0] for item in physical), default=json_start)
    return records, physical, payload_start, tag


def manifested_entries(data: bytes) -> tuple[list[dict[str, Any]], dict[str, bytes], int, bytes]:
    records, physical, payload_start, tag = parse_tail(data)
    payloads = {str(record.get("name", "")): payload for _, record, payload in physical}
    return records, payloads, payload_start, tag


def guess_capture_mode(payloads: dict[str, bytes]) -> str:
    raw = payloads.get("capture.mode")
    if raw:
        text = _read_c_string(raw).strip()
        if text and len(text.encode("utf-8")) < CAPTURE_MODE_SIZE and all(ch.isprintable() for ch in text):
            return text
    return "common"


def rebuild_with_filter_info(source: bytes, filter_info: bytes) -> tuple[bytes, dict[str, Any]]:
    _, physical, payload_start, tag = parse_tail(source)
    original_by_name = {str(record.get("name", "")): payload for _, record, payload in physical}

    ordered: list[tuple[dict[str, Any], bytes]] = []
    replaced = False
    for _, record, payload in physical:
        name = str(record.get("name", ""))
        rec = dict(record)
        if name == "filter.info":
            payload = filter_info
            replaced = True
        ordered.append((rec, payload))
    if not replaced:
        ordered.append(({"name": "filter.info", "version": 1}, filter_info))

    payload = bytearray()
    starts: list[int] = []
    for _, block in ordered:
        starts.append(len(payload))
        payload.extend(block)
    payload_len = len(payload)

    rebuilt_records: list[dict[str, Any]] = []
    for (record, block), start in zip(ordered, starts):
        rec = dict(record)
        rec["length"] = len(block)
        rec["offset"] = payload_len - start
        rec.setdefault("version", 1)
        rebuilt_records.append(rec)

    manifest = json.dumps(rebuilt_records, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    footer = b"\x00" + tag + struct.pack("<I", len(manifest) + 9)
    output = source[:payload_start] + bytes(payload) + manifest + footer

    _, out_payloads, _, out_tag = manifested_entries(output)
    unchanged = {}
    for name, block in original_by_name.items():
        if name == "filter.info":
            continue
        unchanged[name] = sha256(block) == sha256(out_payloads.get(name, b""))
    if not all(unchanged.values()):
        failed = [name for name, ok in unchanged.items() if not ok]
        raise ValueError(f"non-filter entries changed: {failed}")

    return output, {
        "footer_tag": out_tag.decode("ascii", errors="replace"),
        "filter_info_replaced": replaced,
        "non_filter_entries_preserved": unchanged,
    }


def summarize_source(source: bytes) -> dict[str, Any]:
    records, payloads, payload_start, tag = manifested_entries(source)
    existing = payloads.get("filter.info")
    existing_decoded: dict[str, Any] | None = None
    if existing is not None and len(existing) >= FILTER_INFO_SIZE:
        try:
            existing_decoded = asdict(FilterPhotoInfoV51.decode(existing))
        except Exception:
            existing_decoded = None
    return {
        "sha256": sha256(source),
        "size": len(source),
        "footer_tag": tag.decode("ascii", errors="replace"),
        "payload_start": payload_start,
        "capture_mode_guess": guess_capture_mode(payloads),
        "entries": [
            {
                "name": str(record.get("name", "")),
                "length": len(payloads.get(str(record.get("name", "")), b"")),
                "sha256": sha256(payloads.get(str(record.get("name", "")), b"")),
            }
            for record in records
        ],
        "existing_filter_info_decoded": existing_decoded,
    }


def build_case(source: bytes, out_path: Path, report_path: Path, info: FilterPhotoInfoV51) -> None:
    encoded = info.encode()
    round_trip = FilterPhotoInfoV51.decode(encoded)
    if round_trip.filter_type != info.filter_type or round_trip.capture_mode != info.capture_mode:
        raise ValueError("filter.info round-trip failed")
    output, mutation = rebuild_with_filter_info(source, encoded)
    out_path.write_bytes(output)
    report = {
        "output": out_path.name,
        "sha256": sha256(output),
        "size": len(output),
        "filter_info_sha256": sha256(encoded),
        "filter_info_size": len(encoded),
        "filter_info": asdict(round_trip),
        "mutation": mutation,
        "device_acceptance": "unverified",
    }
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--outdir", type=Path, required=True)
    args = parser.parse_args()

    source = args.source.read_bytes()
    args.outdir.mkdir(parents=True, exist_ok=True)
    source_report = summarize_source(source)
    capture_mode = source_report["capture_mode_guess"]

    cases = {
        "palette-prefix": FilterPhotoInfoV51(filter_type="palette-default", capture_mode=capture_mode),
        "palette-semantic": FilterPhotoInfoV51(filter_type="default", capture_mode=capture_mode),
    }
    case_reports = []
    for name, info in cases.items():
        out = args.outdir / f"{name}.heic"
        rep = args.outdir / f"{name}.json"
        build_case(source, out, rep, info)
        case_reports.append(json.loads(rep.read_text(encoding="utf-8")))

    aggregate = {
        "source": args.source.as_posix(),
        "source_report": source_report,
        "schema": {
            "filter_info_size": FILTER_INFO_SIZE,
            "filter_type_offset": FILTER_TYPE_OFFSET,
            "capture_mode_offset": CAPTURE_MODE_OFFSET,
            "avg_luma_offset": AVG_LUMA_OFFSET,
        },
        "cases": case_reports,
    }
    (args.outdir / "report.json").write_text(
        json.dumps(aggregate, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(json.dumps(aggregate, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
