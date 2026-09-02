"""Migration-time photo classification oracle and folder projection helpers.

The canonical product implementation lives in the Rust classification/runtime stack.
This module remains only for conformance evidence while Python migration oracles exist.
"""

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
from typing import Callable, Iterable


CLASSIFICATION_LAYOUT_VERSION = "asset-type-v1"
UNCLASSIFIED_FOLDER_NAME = "未分类"


class AssetType(Enum):
    STATIC_PHOTO = ("static-photo", "静态照片")
    LIVE_PHOTO = ("live-photo", "实况照片")

    def __init__(self, key: str, folder_name: str) -> None:
        self.key = key
        self.folder_name = folder_name

    @property
    def tag_id(self) -> str:
        return f"asset.{self.key}"


class ResourceRole(Enum):
    PRIMARY_IMAGE = "primary-image"
    PAIRED_VIDEO = "paired-video"
    SIDECAR = "sidecar"


@dataclass(frozen=True)
class PhotoResource:
    path: Path
    role: ResourceRole


@dataclass(frozen=True)
class PhotoAsset:
    asset_id: str
    asset_type: AssetType
    resources: tuple[PhotoResource, ...]

    @property
    def primary_image(self) -> Path:
        for resource in self.resources:
            if resource.role is ResourceRole.PRIMARY_IMAGE:
                return resource.path
        raise ValueError("photo asset has no primary image")

    @classmethod
    def static_photo(cls, path: Path) -> "PhotoAsset":
        resolved = Path(path).resolve(strict=False)
        return cls(
            asset_id=str(resolved),
            asset_type=AssetType.STATIC_PHOTO,
            resources=(PhotoResource(Path(path), ResourceRole.PRIMARY_IMAGE),),
        )

    @classmethod
    def live_photo(cls, image: Path, video: Path, asset_id: str | None = None) -> "PhotoAsset":
        image = Path(image)
        video = Path(video)
        resolved_id = asset_id or str(image.resolve(strict=False))
        return cls(
            asset_id=resolved_id,
            asset_type=AssetType.LIVE_PHOTO,
            resources=(
                PhotoResource(image, ResourceRole.PRIMARY_IMAGE),
                PhotoResource(video, ResourceRole.PAIRED_VIDEO),
            ),
        )


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
    # NORMAL is a folder projection fallback, not a semantic tag/activated bit.
    NORMAL = ("normal", "普通拍照", 0)

    def __init__(self, key: str, folder_name: str, bit: int) -> None:
        self.key = key
        self.folder_name = folder_name
        self.bit = bit

    @property
    def tag_id(self) -> str:
        return f"capture.{self.key}"


MODE_PRIORITY = tuple(mode for mode in CaptureMode if mode is not CaptureMode.NORMAL)
MAPPED_FLAGS_MASK = 0
for _mode in MODE_PRIORITY:
    MAPPED_FLAGS_MASK |= _mode.bit

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


class PhotoCapability(Enum):
    PROXDR = "proxdr"
    GAIN_MAP = "gain-map"
    HDR = "hdr"
    DEPTH = "depth"

    @property
    def tag_id(self) -> str:
        return f"capability.{self.value}"


class CameraVendor(Enum):
    OPPO = "oppo"

    @property
    def tag_id(self) -> str:
        return f"vendor.{self.value}"


@dataclass(frozen=True)
class OppoFlagEvidence:
    raw_flags: int
    recognized_flags: int
    known_unmapped_flags: int
    unknown_flags: int


@dataclass(frozen=True)
class Classification:
    raw_user_comment: str | None
    tag_flags: int | None
    recognized_flags: int
    known_unmapped_flags: int
    unknown_flags: int
    capture_modes: frozenset[CaptureMode]
    metadata_status: str
    asset_type: AssetType = AssetType.STATIC_PHOTO
    capabilities: frozenset[PhotoCapability] = frozenset()
    vendor: CameraVendor | None = None

    @property
    def evidence(self) -> OppoFlagEvidence | None:
        if self.tag_flags is None:
            return None
        return OppoFlagEvidence(
            raw_flags=self.tag_flags,
            recognized_flags=self.recognized_flags,
            known_unmapped_flags=self.known_unmapped_flags,
            unknown_flags=self.unknown_flags,
        )

    @property
    def status(self) -> str:
        """Legacy compatibility view; new code should use metadata_status + unknown_flags."""
        if self.metadata_status != "ok":
            return self.metadata_status
        if self.capture_modes or self.unknown_flags == 0:
            return "categorized"
        return "unknown-flags"

    @property
    def mode(self) -> CaptureMode | None:
        """Compatibility view: the single folder projection mode, never the semantic source of truth."""
        return FolderProjection.primary_capture_mode(self)

    @property
    def tags(self) -> tuple[str, ...]:
        values = {self.asset_type.tag_id}
        values.update(mode.tag_id for mode in self.capture_modes)
        values.update(capability.tag_id for capability in self.capabilities)
        if self.vendor is not None:
            values.add(self.vendor.tag_id)
        return tuple(sorted(values))

    def with_asset_type(self, asset_type: AssetType) -> "Classification":
        return replace(self, asset_type=asset_type)


class FolderProjection:
    """Deterministic many-tag -> one physical directory projection."""

    layout_version = CLASSIFICATION_LAYOUT_VERSION
    root_folder_names = frozenset(asset.folder_name for asset in AssetType)

    @staticmethod
    def primary_capture_mode(classification: Classification) -> CaptureMode | None:
        for candidate in MODE_PRIORITY:
            if candidate in classification.capture_modes:
                return candidate
        # "Normal" is a presentation fallback only when OPPO metadata parsed cleanly and there are
        # no completely unknown bits. Known-but-unmapped flags retain the historical normal folder.
        if classification.vendor is CameraVendor.OPPO and classification.metadata_status == "ok" and classification.unknown_flags == 0:
            return CaptureMode.NORMAL
        return None

    @classmethod
    def relative_directory(cls, classification: Classification) -> Path:
        mode = cls.primary_capture_mode(classification)
        mode_folder = mode.folder_name if mode is not None else UNCLASSIFIED_FOLDER_NAME
        return Path(classification.asset_type.folder_name) / mode_folder


def parse_oppo_tag_flag(user_comment: str | None) -> OppoFlagEvidence | None:
    if not user_comment:
        return None
    stripped = user_comment.strip().rstrip("\x00")
    value: int | None = None

    if stripped.startswith("{"):
        try:
            parsed = json.loads(stripped)
        except json.JSONDecodeError:
            parsed = None
        if isinstance(parsed, dict):
            for key, raw_value in parsed.items():
                if str(key).lower() == "oplustag":
                    try:
                        value = int(raw_value)
                    except (TypeError, ValueError):
                        return None
                    break

    if value is None:
        matched = re.search(r"(?i)(?:oplus|oppo)_([0-9]+)", stripped)
        if matched is None:
            return None
        value = int(matched.group(1))

    recognized = value & MAPPED_FLAGS_MASK
    known_unmapped = value & (KNOWN_FLAGS_MASK & ~MAPPED_FLAGS_MASK)
    unknown = value & ~KNOWN_FLAGS_MASK
    return OppoFlagEvidence(
        raw_flags=value,
        recognized_flags=recognized,
        known_unmapped_flags=known_unmapped,
        unknown_flags=unknown,
    )


def classify_user_comment(
    user_comment: str | None,
    *,
    asset_type: AssetType = AssetType.STATIC_PHOTO,
    capabilities: Iterable[PhotoCapability] = (),
) -> Classification:
    evidence = parse_oppo_tag_flag(user_comment)
    capture_modes: frozenset[CaptureMode]
    if evidence is None:
        capture_modes = frozenset()
    else:
        capture_modes = frozenset(mode for mode in MODE_PRIORITY if evidence.recognized_flags & mode.bit)
    status = "ok" if evidence is not None else ("missing-user-comment" if user_comment is None else "malformed-user-comment")
    return Classification(
        raw_user_comment=user_comment,
        tag_flags=evidence.raw_flags if evidence is not None else None,
        recognized_flags=evidence.recognized_flags if evidence is not None else 0,
        known_unmapped_flags=evidence.known_unmapped_flags if evidence is not None else 0,
        unknown_flags=evidence.unknown_flags if evidence is not None else 0,
        capture_modes=capture_modes,
        metadata_status=status,
        asset_type=asset_type,
        capabilities=frozenset(capabilities),
        vendor=CameraVendor.OPPO if evidence is not None else None,
    )


def _decode_user_comment(raw: bytes) -> str | None:
    prefixes = (b"ASCII\x00\x00\x00", b"UNICODE\x00", b"JIS\x00\x00\x00\x00\x00")
    payload = raw
    if raw.startswith(prefixes[0]):
        payload = raw[len(prefixes[0]):]
    elif raw.startswith(prefixes[1]):
        payload = raw[len(prefixes[1]):]
        try:
            return payload.decode("utf-16-be", errors="replace").rstrip("\x00")
        except UnicodeDecodeError:
            return None
    elif raw.startswith(prefixes[2]):
        payload = raw[len(prefixes[2]):]
    return payload.decode("utf-8", errors="replace").rstrip("\x00")


def _read_tiff_value(data: bytes, base: int, endian: str, entry_offset: int) -> bytes | None:
    if entry_offset + 12 > len(data):
        return None
    tag, field_type, count, value_or_offset = struct.unpack_from(endian + "HHII", data, entry_offset)
    if tag != 0x9286 or field_type != 7 or count == 0:
        return None
    if count <= 4:
        return data[entry_offset + 8 : entry_offset + 8 + count]
    absolute = base + value_or_offset
    end = absolute + count
    if absolute < base or end > len(data):
        return None
    return data[absolute:end]


def _extract_user_comment_from_tiff(data: bytes, base: int = 0) -> str | None:
    if base < 0 or base + 8 > len(data):
        return None
    byte_order = data[base:base + 2]
    if byte_order == b"II":
        endian = "<"
    elif byte_order == b"MM":
        endian = ">"
    else:
        return None
    if struct.unpack_from(endian + "H", data, base + 2)[0] != 42:
        return None
    ifd0_offset = base + struct.unpack_from(endian + "I", data, base + 4)[0]
    if ifd0_offset + 2 > len(data):
        return None
    count = struct.unpack_from(endian + "H", data, ifd0_offset)[0]
    exif_ifd_offset: int | None = None
    for index in range(count):
        entry = ifd0_offset + 2 + index * 12
        if entry + 12 > len(data):
            return None
        tag, field_type, value_count, value_or_offset = struct.unpack_from(endian + "HHII", data, entry)
        if tag == 0x9286:
            raw = _read_tiff_value(data, base, endian, entry)
            return _decode_user_comment(raw) if raw is not None else None
        if tag == 0x8769 and field_type == 4 and value_count == 1:
            exif_ifd_offset = base + value_or_offset
    if exif_ifd_offset is None or exif_ifd_offset + 2 > len(data):
        return None
    exif_count = struct.unpack_from(endian + "H", data, exif_ifd_offset)[0]
    for index in range(exif_count):
        entry = exif_ifd_offset + 2 + index * 12
        raw = _read_tiff_value(data, base, endian, entry)
        if raw is not None:
            return _decode_user_comment(raw)
    return None


def extract_user_comment(path: Path) -> str | None:
    data = Path(path).read_bytes()
    # JPEG APP1 Exif.
    if data.startswith(b"\xff\xd8"):
        position = 2
        while position + 4 <= len(data) and data[position] == 0xFF:
            marker = data[position + 1]
            position += 2
            if marker in (0xD8, 0xD9) or 0xD0 <= marker <= 0xD7:
                continue
            if position + 2 > len(data):
                break
            length = int.from_bytes(data[position:position + 2], "big")
            if length < 2 or position + length > len(data):
                break
            payload = data[position + 2:position + length]
            if marker == 0xE1 and payload.startswith(b"Exif\x00\x00"):
                parsed = _extract_user_comment_from_tiff(payload, 6)
                if parsed is not None:
                    return parsed
            position += length
    parsed = _extract_user_comment_from_tiff(data, 0)
    if parsed is not None:
        return parsed

    # HEIF Exif can be stored in an item payload. Search for a TIFF header and parse candidates.
    for signature in (b"II*\x00", b"MM\x00*"):
        start = 0
        while True:
            index = data.find(signature, start)
            if index < 0:
                break
            parsed = _extract_user_comment_from_tiff(data, index)
            if parsed is not None:
                return parsed
            start = index + 1

    # Migration fallback for legacy fixtures and vendor metadata that is not exposed as a normal
    # Exif item yet. Keep this evidence-only path explicit instead of treating it as generic Exif.
    matched = re.search(rb"(?i)(?:Oplus|Oppo)_([0-9]+)", data)
    if matched is not None:
        return matched.group(0).decode("ascii")
    json_tag = re.search(rb'(?i)"oplustag"\s*:\s*"?([0-9]+)"?', data)
    if json_tag is not None:
        return f"Oplus_{json_tag.group(1).decode('ascii')}"
    return None


def _probe_capabilities(path: Path) -> frozenset[PhotoCapability]:
    data = path.read_bytes()
    capabilities: set[PhotoCapability] = set()
    if b"local.lhdr.gainmap" in data or b"local.uhdr.gainmap" in data:
        capabilities.update((PhotoCapability.PROXDR, PhotoCapability.GAIN_MAP, PhotoCapability.HDR))
    if b"rear.depth" in data and b"rear.depth.config" in data:
        capabilities.add(PhotoCapability.DEPTH)
    return frozenset(capabilities)


def classify_path(path: Path, *, asset_type: AssetType = AssetType.STATIC_PHOTO) -> Classification:
    return classify_user_comment(
        extract_user_comment(path),
        asset_type=asset_type,
        capabilities=_probe_capabilities(path),
    )


def collect_inputs(inputs: Iterable[Path], output_dir: Path | None = None) -> list[Path]:
    output_root = output_dir.resolve(strict=False) if output_dir is not None else None
    collected: set[Path] = set()
    for raw in inputs:
        path = Path(raw)
        if not path.exists():
            raise FileNotFoundError(path)
        candidates = path.rglob("*") if path.is_dir() else (path,)
        for candidate in candidates:
            if not candidate.is_file() or candidate.suffix.lower() not in {".jpg", ".jpeg", ".heic", ".heif"}:
                continue
            resolved = candidate.resolve(strict=False)
            if output_root is not None and (resolved == output_root or output_root in resolved.parents):
                continue
            if any(parent.name in FolderProjection.root_folder_names for parent in resolved.parents):
                continue
            collected.add(candidate)
    return sorted(collected, key=lambda item: str(item.resolve(strict=False)))


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while True:
            chunk = handle.read(1024 * 1024)
            if not chunk:
                break
            digest.update(chunk)
    return digest.hexdigest()


@dataclass(frozen=True)
class PlanItem:
    source: Path
    destination: Path
    classification: Classification
    disposition: str
    asset_id: str = ""
    role: ResourceRole = ResourceRole.PRIMARY_IMAGE


@dataclass(frozen=True)
class ExecutionResult:
    source: Path
    destination: Path
    classification: Classification
    disposition: str
    error: str | None = None
    asset_id: str = ""
    role: ResourceRole = ResourceRole.PRIMARY_IMAGE


def _resolved_asset(
    image: Path,
    live_photo_pair_validator: Callable[[Path, Path], bool] | None,
) -> PhotoAsset:
    video = image.with_suffix(".mov")
    if video.is_file() and live_photo_pair_validator is not None and live_photo_pair_validator(image, video):
        return PhotoAsset.live_photo(image, video)
    return PhotoAsset.static_photo(image)


def make_plan(
    inputs: Iterable[Path],
    output_dir: Path,
    *,
    live_photo_pair_validator: Callable[[Path, Path], bool] | None = None,
) -> list[PlanItem]:
    output_dir = Path(output_dir)
    source_files = collect_inputs(inputs, output_dir)
    resources_to_skip: set[Path] = set()
    plans: list[PlanItem] = []

    for source in source_files:
        if source in resources_to_skip:
            continue
        asset = _resolved_asset(source, live_photo_pair_validator)
        for resource in asset.resources:
            resources_to_skip.add(resource.path)
        classification = classify_path(asset.primary_image, asset_type=asset.asset_type)
        relative_directory = FolderProjection.relative_directory(classification)
        resource_fingerprints = {resource.path: _sha256(resource.path) for resource in asset.resources}
        sequence = 1
        while True:
            conflicts = False
            resource_destinations: dict[Path, Path] = {}
            for resource in asset.resources:
                stem = resource.path.stem if sequence == 1 else f"{resource.path.stem} ({sequence})"
                candidate = output_dir / relative_directory / f"{stem}{resource.path.suffix.lower()}"
                resource_destinations[resource.path] = candidate
                if candidate.exists() and _sha256(candidate) != resource_fingerprints[resource.path]:
                    conflicts = True
            if not conflicts:
                break
            sequence += 1

        for resource in asset.resources:
            destination = resource_destinations[resource.path]
            disposition = (
                "duplicate"
                if destination.exists() and _sha256(destination) == resource_fingerprints[resource.path]
                else "copy"
            )
            plans.append(
                PlanItem(
                    source=resource.path,
                    destination=destination,
                    classification=classification,
                    disposition=disposition,
                    asset_id=asset.asset_id,
                    role=resource.role,
                )
            )
    return plans


def _copy_atomically(source: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary_name = tempfile.mkstemp(prefix=f".{destination.name}.", dir=destination.parent)
    os.close(fd)
    temporary = Path(temporary_name)
    try:
        shutil.copy2(source, temporary)
        os.replace(temporary, destination)
    finally:
        temporary.unlink(missing_ok=True)


def execute_plan(plan: Iterable[PlanItem], jobs: int = 1, dry_run: bool = False) -> list[ExecutionResult]:
    items = list(plan)
    if dry_run:
        return [
            ExecutionResult(
                item.source,
                item.destination,
                item.classification,
                "dry-run" if item.disposition != "duplicate" else "duplicate",
                asset_id=item.asset_id,
                role=item.role,
            )
            for item in items
        ]

    def run(item: PlanItem) -> ExecutionResult:
        if item.disposition == "duplicate":
            return ExecutionResult(
                item.source,
                item.destination,
                item.classification,
                "duplicate",
                asset_id=item.asset_id,
                role=item.role,
            )
        try:
            _copy_atomically(item.source, item.destination)
            return ExecutionResult(
                item.source,
                item.destination,
                item.classification,
                "copied",
                asset_id=item.asset_id,
                role=item.role,
            )
        except Exception as error:  # pragma: no cover - platform/filesystem dependent
            return ExecutionResult(
                item.source,
                item.destination,
                item.classification,
                "failed",
                str(error),
                asset_id=item.asset_id,
                role=item.role,
            )

    with ThreadPoolExecutor(max_workers=max(1, jobs)) as executor:
        return list(executor.map(run, items))
