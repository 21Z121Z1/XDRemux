"""Photo asset classification and folder projection shared by the Python CLI."""

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
        if classification.metadata_status == "ok" and classification.tag_flags is not None and classification.unknown_flags == 0:
            return CaptureMode.NORMAL
        return None

    @staticmethod
    def relative_directory(classification: Classification, asset_type: AssetType | None = None) -> Path:
        resolved_asset_type = asset_type or classification.asset_type
        mode = FolderProjection.primary_capture_mode(classification)
        leaf = mode.folder_name if mode is not None else UNCLASSIFIED_FOLDER_NAME
        return Path(resolved_asset_type.folder_name) / leaf


SUPPORTED_EXTENSIONS = {".heic", ".heif", ".jpg", ".jpeg"}
COMMENT_PATTERN = re.compile(rb"(?:Oplus|Oppo)_[0-9]+", re.IGNORECASE)
JSON_TAG_PATTERN = re.compile(rb'"oplustag"\s*:\s*"?([0-9]+)"?', re.IGNORECASE)


@dataclass(frozen=True)
class CategorizationItem:
    source: Path
    destination: Path
    classification: Classification
    disposition: str
    error: str | None = None



def classify_user_comment(raw: str | None) -> Classification:
    if raw is None or not raw.strip():
        return Classification(raw, None, 0, 0, 0, frozenset(), "missing-user-comment")
    normalized = raw.strip()
    flags = _parse_flags(normalized)
    if flags is None:
        return Classification(normalized, None, 0, 0, 0, frozenset(), "malformed-user-comment")

    recognized = flags & MAPPED_FLAGS_MASK
    known_unmapped = flags & KNOWN_FLAGS_MASK & ~MAPPED_FLAGS_MASK
    unknown = flags & ~KNOWN_FLAGS_MASK
    modes = frozenset(mode for mode in MODE_PRIORITY if flags & mode.bit)
    return Classification(
        normalized,
        flags,
        recognized,
        known_unmapped,
        unknown,
        modes,
        "ok",
        vendor=CameraVendor.OPPO,
    )



def classify_asset(asset: PhotoAsset) -> Classification:
    """Classify the primary image while preserving the asset-level resource identity."""
    return classify_path(asset.primary_image, asset.asset_type)


def classify_path(path: Path, asset_type: AssetType | None = None) -> Classification:
    resolved_asset_type = asset_type or infer_asset_type(path)
    try:
        raw = extract_user_comment(path)
        data = path.read_bytes()
    except OSError:
        return Classification(None, None, 0, 0, 0, frozenset(), "unreadable-image", asset_type=resolved_asset_type)
    base = classify_user_comment(raw)
    return replace(
        base,
        asset_type=resolved_asset_type,
        capabilities=frozenset(_detect_capabilities(data)),
    )




def infer_asset_type(path: Path) -> AssetType:
    suffix = Path(path).suffix.lower()
    if suffix not in {".jpg", ".jpeg", ".heic", ".heif"}:
        return AssetType.STATIC_PHOTO
    try:
        from .motion_photo import MotionPhotoError, parse_motion_photo
    except (ImportError, ValueError):
        return AssetType.STATIC_PHOTO
    try:
        return AssetType.LIVE_PHOTO if parse_motion_photo(Path(path)) is not None else AssetType.STATIC_PHOTO
    except (OSError, MotionPhotoError):
        return AssetType.STATIC_PHOTO

def _detect_capabilities(data: bytes) -> set[PhotoCapability]:
    """Emit only capabilities backed by container evidence understood by both implementations."""
    capabilities: set[PhotoCapability] = set()
    has_private_gain_map = (
        b'"local.uhdr.gainmap.data"' in data
        or b'"local.uhdr.gainmap.info"' in data
    )
    if has_private_gain_map:
        capabilities.update({PhotoCapability.PROXDR, PhotoCapability.GAIN_MAP, PhotoCapability.HDR})
    if b'\"rear.depth\"' in data and b'\"rear.depth.config\"' in data:
        capabilities.add(PhotoCapability.DEPTH)
    return capabilities



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
    input_paths = [Path(item) for item in inputs]
    excluded = output_dir.resolve(strict=False)
    collected: dict[str, Path] = {}
    in_place_skip_roots = (
        {asset.folder_name for asset in AssetType}
        | {mode.folder_name for mode in CaptureMode}
    )
    for source in input_paths:
        if not source.exists():
            raise FileNotFoundError(f"input not found: {source}")
        candidates = [source] if source.is_file() else source.rglob("*")
        for candidate in candidates:
            resolved = candidate.resolve(strict=False)
            if resolved == excluded or excluded in resolved.parents:
                continue
            if not candidate.is_file() or candidate.suffix.lower() not in SUPPORTED_EXTENSIONS:
                continue
            skip = False
            for input_root in input_paths:
                if not input_root.is_dir():
                    continue
                try:
                    relative = candidate.relative_to(input_root)
                except ValueError:
                    continue
                if len(relative.parts) > 1 and relative.parts[0] in in_place_skip_roots:
                    skip = True
                    break
            if not skip:
                collected[str(resolved)] = candidate
    return [collected[key] for key in sorted(collected)]


LivePhotoPairValidator = Callable[[Path, Path], bool]


def _companion_video(path: Path) -> Path | None:
    stem = path.stem
    try:
        candidates = sorted(
            (
                candidate
                for candidate in path.parent.iterdir()
                if candidate.is_file()
                and candidate.suffix.lower() == ".mov"
                and candidate.stem == stem
            ),
            key=lambda candidate: candidate.name,
        )
    except OSError:
        return None
    return candidates[0] if candidates else None


def _portable_live_photo_pair_validator(image: Path, video: Path) -> bool:
    try:
        from . import live_photo
        return live_photo.existing_pair_is_valid(image, video)
    except Exception:
        return False


def _resolved_asset(source: Path, pair_validator: LivePhotoPairValidator) -> PhotoAsset:
    companion = _companion_video(source)
    if companion is not None and pair_validator(source, companion):
        return PhotoAsset.live_photo(source, companion)
    if infer_asset_type(source) is AssetType.LIVE_PHOTO:
        return PhotoAsset(
            asset_id=str(source.resolve(strict=False)),
            asset_type=AssetType.LIVE_PHOTO,
            resources=(PhotoResource(source, ResourceRole.PRIMARY_IMAGE),),
        )
    return PhotoAsset.static_photo(source)


def make_plan(
    inputs: Iterable[Path],
    output_dir: Path,
    live_photo_pair_validator: LivePhotoPairValidator | None = None,
) -> list[CategorizationItem]:
    """Plan by user-visible photo asset while publishing one item per physical resource.

    A same-basename MOV is never claimed on filename alone. The portable Live Photo validator must
    confirm the HEIC/JPEG and MOV content identifiers before both resources enter one PhotoAsset.
    """
    pair_validator = live_photo_pair_validator or _portable_live_photo_pair_validator
    reserved: dict[Path, tuple[Path, str]] = {}
    items: list[CategorizationItem] = []
    for source in collect_inputs(inputs, output_dir):
        asset = _resolved_asset(source, pair_validator)
        classification = classify_asset(asset)
        directory = output_dir / FolderProjection.relative_directory(classification)
        digests = {resource.path: _sha256(resource.path) for resource in asset.resources}
        sequence = 1
        while True:
            candidates = [
                (resource, _sequenced_destination(directory, resource.path.name, sequence))
                for resource in asset.resources
            ]
            dispositions: dict[Path, str] = {}
            conflict = False
            for resource, destination in candidates:
                digest = digests[resource.path]
                prior = reserved.get(destination)
                if prior is not None:
                    if prior[1] == digest:
                        dispositions[resource.path] = "duplicate"
                    else:
                        conflict = True
                        break
                elif destination.exists():
                    if _files_match(resource.path, destination):
                        dispositions[resource.path] = "duplicate"
                    else:
                        conflict = True
                        break
                else:
                    dispositions[resource.path] = "copy"
            if conflict:
                sequence += 1
                continue

            for resource, destination in candidates:
                digest = digests[resource.path]
                reserved.setdefault(destination, (destination, digest))
                items.append(
                    CategorizationItem(
                        resource.path,
                        destination,
                        classification,
                        dispositions.get(resource.path, "copy"),
                    )
                )
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



def batch_destinations(
    files: list[Path],
    output_dir: Path,
    asset_type: AssetType | None = None,
) -> dict[Path, Path]:
    reserved: set[Path] = set()
    result: dict[Path, Path] = {}
    for source in sorted(files, key=lambda item: str(item.resolve(strict=False))):
        classification = classify_path(source, asset_type)
        directory = output_dir / FolderProjection.relative_directory(classification, asset_type)
        sequence = 1
        while True:
            destination = _sequenced_destination(directory, source.with_suffix(".heic").name, sequence)
            if destination not in reserved:
                reserved.add(destination)
                result[source] = destination
                break
            sequence += 1
    return result



def classification_contract(classification: Classification) -> dict[str, object]:
    """Canonical cross-runtime representation used by golden tests and future JSON surfaces."""
    mode = classification.mode
    return {
        "asset_type": classification.asset_type.key,
        "capture_modes": [candidate.key for candidate in MODE_PRIORITY if candidate in classification.capture_modes],
        "primary_capture_mode": mode.key if mode is not None else None,
        "folder": mode.folder_name if mode is not None else UNCLASSIFIED_FOLDER_NAME,
        "metadata_status": classification.metadata_status,
        "recognized_flags": classification.recognized_flags,
        "known_unmapped_flags": classification.known_unmapped_flags,
        "unknown_flags": classification.unknown_flags,
        "tags": list(classification.tags),
    }



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
