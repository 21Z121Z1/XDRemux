"""OPPO UserComment shooting-mode categorization shared by the Python CLI."""

from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import struct
import tempfile
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, replace
from enum import Enum
from pathlib import Path
from typing import Iterable


class CaptureMode(Enum):
    MASTER = ("master", "大师模式", 0x100000000)
    RICOH_GR = ("ricoh-gr", "RICOH GR", 0x80000000)
    PROFESSIONAL = ("professional", "专业模式", 0x100)
    PORTRAIT = ("portrait", "人像", 0x10)
    NIGHT = ("night", "夜景", 0x800)
    PANORAMA = ("panorama", "全景", 0x4)
    TIME_LAPSE = ("time-lapse", "延时摄影", 0x8)
    ULTRA_HIGH_RESOLUTION = ("ultra-high-resolution", "超清", 0x2000)
    ID_PHOTO = ("id-photo", "证件照", 0x4000)
    STICKER = ("sticker", "贴纸", 0x200)
    ENHANCED_TEXT = ("enhanced-text", "超级文本", 0x1000)
    GROUP_PHOTO = ("group-photo", "合影", 0x400000)
    DOUBLE_EXPOSURE = ("double-exposure", "双重曝光", 0x8000)
    BEAUTY = ("beauty", "美颜", 0x2)
    NORMAL = ("normal", "普通拍照", 0)

    def __init__(self, key: str, folder_name: str, bit: int) -> None:
        self.key = key
        self.folder_name = folder_name
        self.bit = bit


MODE_PRIORITY = tuple(mode for mode in CaptureMode if mode is not CaptureMode.NORMAL)
KNOWN_FLAGS_MASK = 0
for _bit in (
    0x1, 0x2, 0x4, 0x8, 0x10, 0x20, 0x40, 0x80,
    0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000, 0x4000, 0x8000,
    0x10000, 0x20000, 0x40000, 0x80000, 0x100000, 0x200000,
    0x400000, 0x800000, 0x1000000, 0x2000000, 0x4000000,
    0x8000000, 0x10000000, 0x20000000, 0x40000000, 0x80000000,
    0x100000000, 0x200000000, 0x4000000000000000,
):
    KNOWN_FLAGS_MASK |= _bit

SUPPORTED_EXTENSIONS = {".heic", ".heif", ".jpg", ".jpeg"}
COMMENT_PATTERN = re.compile(rb"(?:Oplus|Oppo)_[0-9]+", re.IGNORECASE)
JSON_TAG_PATTERN = re.compile(rb'"oplustag"\s*:\s*"?([0-9]+)"?', re.IGNORECASE)


@dataclass(frozen=True)
class Classification:
    raw_user_comment: str | None
    tag_flags: int | None
    unknown_flags: int
    mode: CaptureMode | None
    status: str


@dataclass(frozen=True)
class CategorizationItem:
    source: Path
    destination: Path
    classification: Classification
    disposition: str
    error: str | None = None


def classify_user_comment(raw: str | None) -> Classification:
    if raw is None or not raw.strip():
        return Classification(raw, None, 0, None, "missing-user-comment")
    normalized = raw.strip()
    flags = _parse_flags(normalized)
    if flags is None:
        return Classification(normalized, None, 0, None, "malformed-user-comment")
    unknown = flags & ~KNOWN_FLAGS_MASK
    mode = next((candidate for candidate in MODE_PRIORITY if flags & candidate.bit), None)
    if mode is None and unknown:
        return Classification(normalized, flags, unknown, None, "unknown-flags")
    return Classification(normalized, flags, unknown, mode or CaptureMode.NORMAL, "categorized")


def classify_path(path: Path) -> Classification:
    try:
        raw = extract_user_comment(path)
    except OSError:
        return Classification(None, None, 0, None, "unreadable-image")
    return classify_user_comment(raw)


def extract_user_comment(path: Path) -> str | None:
    exif_bytes: bytes | None = None
    suffix = path.suffix.lower()
    try:
        if suffix in {".heic", ".heif"}:
            from pillow_heif import open_heif
            heif = open_heif(str(path), convert_hdr_to_8bit=False)
            primary = heif[0] if hasattr(heif, "__getitem__") else heif
            exif_bytes = primary.info.get("exif")
        else:
            from PIL import Image
            with Image.open(path) as image:
                exif_bytes = image.info.get("exif")
                if exif_bytes is None:
                    exif = image.getexif()
                    try:
                        value = exif.get_ifd(0x8769).get(0x9286)
                    except (KeyError, TypeError):
                        value = None
                    decoded = _decode_user_comment_value(value)
                    if decoded:
                        return decoded
    except Exception:
        exif_bytes = None

    if exif_bytes:
        decoded = _extract_tiff_user_comment(exif_bytes)
        if decoded:
            return decoded

    data = path.read_bytes()
    decoded = _extract_tiff_user_comment(data)
    if decoded:
        return decoded
    match = COMMENT_PATTERN.search(data)
    if match:
        return match.group(0).decode("ascii")
    json_match = JSON_TAG_PATTERN.search(data)
    if json_match:
        return '{"oplustag":"' + json_match.group(1).decode("ascii") + '"}'
    return None


def collect_inputs(inputs: Iterable[Path], output_dir: Path) -> list[Path]:
    excluded = output_dir.resolve(strict=False)
    collected: dict[str, Path] = {}
    for source in inputs:
        if not source.exists():
            raise FileNotFoundError(f"input not found: {source}")
        candidates = [source] if source.is_file() else source.rglob("*")
        for candidate in candidates:
            resolved = candidate.resolve(strict=False)
            if resolved == excluded or excluded in resolved.parents:
                continue
            if candidate.is_file() and candidate.suffix.lower() in SUPPORTED_EXTENSIONS:
                collected[str(resolved)] = candidate
    return [collected[key] for key in sorted(collected)]


def make_plan(inputs: Iterable[Path], output_dir: Path) -> list[CategorizationItem]:
    reserved: dict[Path, tuple[Path, str]] = {}
    items: list[CategorizationItem] = []
    for source in collect_inputs(inputs, output_dir):
        classification = classify_path(source)
        directory = output_dir / classification.mode.folder_name if classification.mode else output_dir
        source_digest = _sha256(source)
        sequence = 1
        while True:
            destination = _sequenced_destination(directory, source.name, sequence)
            prior = reserved.get(destination)
            if prior:
                if prior[1] == source_digest:
                    items.append(CategorizationItem(source, prior[0], classification, "duplicate"))
                    break
                sequence += 1
                continue
            if destination.exists():
                if _files_match(source, destination):
                    reserved[destination] = (destination, source_digest)
                    items.append(CategorizationItem(source, destination, classification, "duplicate"))
                    break
                sequence += 1
                continue
            reserved[destination] = (destination, source_digest)
            items.append(CategorizationItem(source, destination, classification, "copy"))
            break
    return items


def execute_plan(items: list[CategorizationItem], jobs: int = 4, dry_run: bool = False) -> list[CategorizationItem]:
    def execute(item: CategorizationItem) -> CategorizationItem:
        if item.disposition == "duplicate":
            return item
        if dry_run:
            return replace(item, disposition="dry-run")
        try:
            _copy_atomically(item.source, item.destination)
            return replace(item, disposition="copied")
        except OSError as exc:
            return replace(item, disposition="failed", error=str(exc))

    with ThreadPoolExecutor(max_workers=max(1, jobs)) as pool:
        return list(pool.map(execute, items))


def batch_destinations(files: list[Path], output_dir: Path) -> dict[Path, Path]:
    reserved: set[Path] = set()
    result: dict[Path, Path] = {}
    for source in sorted(files, key=lambda item: str(item.resolve(strict=False))):
        classification = classify_path(source)
        directory = output_dir / classification.mode.folder_name if classification.mode else output_dir
        sequence = 1
        while True:
            destination = _sequenced_destination(directory, source.with_suffix(".heic").name, sequence)
            if destination not in reserved:
                reserved.add(destination)
                result[source] = destination
                break
            sequence += 1
    return result


def _parse_flags(raw: str) -> int | None:
    try:
        parsed = json.loads(raw)
        if isinstance(parsed, dict):
            value = next((value for key, value in parsed.items() if key.lower() == "oplustag"), None)
            if isinstance(value, int) and not isinstance(value, bool) and value >= 0:
                return value
            if isinstance(value, str) and value.strip().isdigit():
                return int(value.strip())
    except (ValueError, TypeError):
        pass
    normalized = raw.replace("\x00", "")
    match = re.search(r"(?:oplus|oppo)_([0-9]+)", normalized, re.IGNORECASE)
    return int(match.group(1)) if match else None


def _decode_user_comment_value(value: object) -> str | None:
    if isinstance(value, str):
        return value.strip("\x00")
    if isinstance(value, bytes):
        payload = value[8:] if len(value) >= 8 else value
        return payload.decode("utf-8", errors="replace").strip("\x00")
    return None


def _extract_tiff_user_comment(exif_bytes: bytes) -> str | None:
    data = exif_bytes
    if len(data) >= 10 and data[:4] == b"\x00\x00\x00\x06" and data[4:10] == b"Exif\x00\x00":
        data = data[10:]
    elif data.startswith(b"Exif\x00\x00"):
        data = data[6:]
    if len(data) < 8 or data[:2] not in {b"II", b"MM"}:
        return None
    endian = "<" if data[:2] == b"II" else ">"
    if struct.unpack_from(endian + "H", data, 2)[0] != 42:
        return None
    pending = [struct.unpack_from(endian + "I", data, 4)[0]]
    visited: set[int] = set()
    while pending:
        offset = pending.pop()
        if offset in visited or offset + 2 > len(data):
            continue
        visited.add(offset)
        count = struct.unpack_from(endian + "H", data, offset)[0]
        if count > 4096:
            return None
        for index in range(count):
            entry = offset + 2 + index * 12
            if entry + 12 > len(data):
                return None
            tag, field_type = struct.unpack_from(endian + "HH", data, entry)
            value_count = struct.unpack_from(endian + "I", data, entry + 4)[0]
            value_offset = struct.unpack_from(endian + "I", data, entry + 8)[0]
            if tag in {0x8769, 0x8825}:
                pending.append(value_offset)
            if tag == 0x9286 and field_type == 7:
                start = entry + 8 if value_count <= 4 else value_offset
                end = start + value_count
                if end <= len(data):
                    return _decode_user_comment_value(data[start:end])
    return None


def _sequenced_destination(directory: Path, name: str, sequence: int) -> Path:
    if sequence == 1:
        return directory / name
    source = Path(name)
    return directory / f"{source.stem} ({sequence}){source.suffix}"


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _files_match(left: Path, right: Path) -> bool:
    return left.stat().st_size == right.stat().st_size and _sha256(left) == _sha256(right)


def _copy_atomically(source: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=".xdremux-categorize-", dir=destination.parent)
    os.close(descriptor)
    temporary = Path(temporary_name)
    try:
        shutil.copy2(source, temporary)
        os.replace(temporary, destination)
    finally:
        temporary.unlink(missing_ok=True)
