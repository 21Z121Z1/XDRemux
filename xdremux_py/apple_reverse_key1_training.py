"""Dataset preparation and structured training for Apple's reverse key1.

This module deliberately keeps the first learning boundary narrow:

    styled thumbnail + disabled/native reverse thumbnail -> key1

Device, OS, and lens metadata are audit covariates only.  They are never fed to
the first model.  Private HEIC inputs and generated training caches remain
outside Git under ``.codex``.
"""

from __future__ import annotations

import concurrent.futures
import datetime as dt
import hashlib
import json
import math
import os
import subprocess
import tempfile
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable, Mapping, Sequence

import numpy as np
from PIL import Image, ImageOps


DATASET_SCHEMA = "xdremux-reverse-key1-dataset-v1"
SAMPLE_SCHEMA = "xdremux-reverse-key1-sample-v1"
REPORT_SCHEMA = "xdremux-reverse-key1-training-report-v1"
KEY1_BYTE_LENGTH = 51_840
KEY1_VALUE_COUNT = 25_920
GRID_LONG = 12
GRID_SHORT = 9
PLANE_COUNT = 8
POLYNOMIAL_COUNT = 10
OUTPUT_COUNT = 3
INPUT_SIZE = 256
INPUT_CHANNELS = 12


class ReverseKey1Error(RuntimeError):
    """The reverse-key1 data or training contract is invalid."""


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def sha256_file(path: Path, block_size: int = 1 << 20) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(block_size), b""):
            digest.update(block)
    return digest.hexdigest()


def _atomic_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = canonical_json_bytes(value) + b"\n"
    with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as handle:
        temporary = Path(handle.name)
        handle.write(payload)
    os.replace(temporary, path)


def _atomic_npz(path: Path, **arrays: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        dir=path.parent, suffix=".npz", delete=False
    ) as handle:
        temporary = Path(handle.name)
    try:
        np.savez(temporary, **arrays)
        os.replace(temporary, path)
    finally:
        if temporary.exists():
            temporary.unlink()


def identity_key1() -> np.ndarray:
    """Return the native quadratic identity template for a padded 12x12 grid."""
    result = np.zeros(
        (
            GRID_LONG,
            GRID_LONG,
            PLANE_COUNT,
            POLYNOMIAL_COUNT,
            OUTPUT_COUNT,
        ),
        dtype=np.float32,
    )
    result[:, :, :, 1, 0] = 1.0
    result[:, :, :, 2, 1] = 1.0
    result[:, :, :, 3, 2] = 1.0
    return result


def decode_key1(
    payload: bytes,
    *,
    display_width: int,
    display_height: int,
) -> tuple[np.ndarray, np.ndarray, int, int]:
    """Decode persisted x-major Float16 key1 into a padded y/x training grid."""
    if len(payload) != KEY1_BYTE_LENGTH:
        raise ReverseKey1Error(
            f"key1 must contain {KEY1_BYTE_LENGTH} bytes, got {len(payload)}"
        )
    if display_width < 1 or display_height < 1:
        raise ReverseKey1Error("display dimensions must be positive")
    landscape = display_width >= display_height
    width_slots = GRID_LONG if landscape else GRID_SHORT
    height_slots = GRID_SHORT if landscape else GRID_LONG
    values = np.frombuffer(payload, dtype="<f2")
    if values.size != KEY1_VALUE_COUNT:
        raise ReverseKey1Error("key1 Float16 value count is invalid")
    x_major = values.reshape(
        width_slots,
        height_slots,
        PLANE_COUNT,
        POLYNOMIAL_COUNT,
        OUTPUT_COUNT,
    )
    y_major = np.transpose(x_major, (1, 0, 2, 3, 4))
    padded = np.zeros(
        (
            GRID_LONG,
            GRID_LONG,
            PLANE_COUNT,
            POLYNOMIAL_COUNT,
            OUTPUT_COUNT,
        ),
        dtype=np.float16,
    )
    padded[:height_slots, :width_slots] = y_major
    mask = np.zeros((GRID_LONG, GRID_LONG), dtype=np.bool_)
    mask[:height_slots, :width_slots] = True
    return padded, mask, width_slots, height_slots


def encode_key1(
    padded: np.ndarray,
    *,
    width_slots: int,
    height_slots: int,
) -> bytes:
    expected = {GRID_LONG, GRID_SHORT}
    if {width_slots, height_slots} != expected:
        raise ReverseKey1Error("key1 grid must be 12x9 or 9x12")
    value = np.asarray(padded)
    expected_shape = (
        GRID_LONG,
        GRID_LONG,
        PLANE_COUNT,
        POLYNOMIAL_COUNT,
        OUTPUT_COUNT,
    )
    if value.shape != expected_shape:
        raise ReverseKey1Error(f"padded key1 shape must be {expected_shape}")
    y_major = value[:height_slots, :width_slots]
    x_major = np.transpose(y_major, (1, 0, 2, 3, 4))
    return np.asarray(x_major, dtype="<f2").tobytes(order="C")


def split_for_session(session_id: str) -> str:
    bucket = int(hashlib.sha256(session_id.encode()).hexdigest()[:8], 16) % 100
    if bucket < 70:
        return "train"
    if bucket < 85:
        return "calibration"
    return "heldout"


def _timestamp(record: Mapping[str, Any]) -> dt.datetime | None:
    value = record.get("DateTimeOriginal")
    if not isinstance(value, str):
        return None
    try:
        parsed = dt.datetime.strptime(value, "%Y:%m:%d %H:%M:%S")
    except ValueError:
        return None
    fraction = "".join(ch for ch in str(record.get("SubSecTimeOriginal", "")) if ch.isdigit())
    if fraction:
        parsed = parsed.replace(microsecond=int((fraction + "000000")[:6]))
    return parsed


def assign_sessions(records: list[dict[str, Any]], gap_seconds: int = 120) -> None:
    """Assign deterministic capture sessions without using device IDs as inputs."""
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for record in records:
        camera = "|".join(
            str(record.get(key) or "unknown") for key in ("Model", "LensModel")
        )
        grouped[camera].append(record)
    for camera, items in grouped.items():
        items.sort(
            key=lambda item: (
                item.get("_captureTime") is None,
                item.get("_captureTime") or dt.datetime.max,
                item["sourcePath"],
            )
        )
        previous: dt.datetime | None = None
        session_seed = ""
        ordinal = -1
        for item in items:
            captured = item.get("_captureTime")
            if (
                previous is None
                or captured is None
                or (captured - previous).total_seconds() > gap_seconds
            ):
                ordinal += 1
                session_seed = (
                    f"{camera}|{ordinal}|"
                    f"{captured.isoformat() if captured else item['sourcePath']}"
                )
            session = "session-" + hashlib.sha256(
                session_seed.encode()
            ).hexdigest()[:16]
            item["captureSession"] = session
            item["split"] = split_for_session(session)
            if captured is not None:
                previous = captured


def _exif_inventory(paths: Sequence[Path], exiftool: str) -> list[dict[str, Any]]:
    tags = (
        "FileName",
        "Directory",
        "Model",
        "Software",
        "LensModel",
        "DateTimeOriginal",
        "SubSecTimeOriginal",
        "OffsetTimeOriginal",
        "ImageWidth",
        "ImageHeight",
        "Orientation",
        "Tag0",
    )
    result: list[dict[str, Any]] = []
    for start in range(0, len(paths), 48):
        command = [exiftool, "-j", "-n"]
        command.extend(f"-{tag}" for tag in tags)
        command.extend(str(path) for path in paths[start : start + 48])
        completed = subprocess.run(
            command,
            capture_output=True,
            text=True,
            check=False,
            timeout=180,
        )
        if completed.returncode:
            raise ReverseKey1Error(
                f"exiftool inventory failed: {completed.stderr[-2000:]}"
            )
        values = json.loads(completed.stdout)
        if not isinstance(values, list):
            raise ReverseKey1Error("exiftool inventory root is not an array")
        result.extend(value for value in values if isinstance(value, dict))
    if len(result) != len(paths):
        raise ReverseKey1Error(
            f"exiftool returned {len(result)} records for {len(paths)} inputs"
        )
    return result


def _fit_rgb(image: Image.Image, size: int = INPUT_SIZE) -> np.ndarray:
    value = ImageOps.exif_transpose(image).convert("RGB")
    value = ImageOps.contain(value, (size, size), Image.Resampling.LANCZOS)
    canvas = Image.new("RGB", (size, size), (0, 0, 0))
    canvas.paste(value, ((size - value.width) // 2, (size - value.height) // 2))
    return np.transpose(np.asarray(canvas, dtype=np.uint8), (2, 0, 1))


def _read_primary(path: Path) -> tuple[np.ndarray, int, int]:
    try:
        import pillow_heif
    except ImportError as error:
        raise ReverseKey1Error(
            "dataset preparation requires pillow-heif"
        ) from error
    pillow_heif.register_heif_opener()
    with Image.open(path) as image:
        display = ImageOps.exif_transpose(image)
        width, height = display.size
        return _fit_rgb(display), width, height


def _read_key1(path: Path, exiftool: str) -> bytes:
    completed = subprocess.run(
        [exiftool, "-b", "-Tag1", str(path)],
        capture_output=True,
        check=False,
        timeout=120,
    )
    if completed.returncode:
        raise ReverseKey1Error(
            f"key1 extraction failed: {completed.stderr.decode(errors='replace')[-1000:]}"
        )
    return completed.stdout


def _disabled_render(
    helper: Path,
    source: Path,
    output: Path,
    manifest: Path,
) -> dict[str, Any]:
    completed = subprocess.run(
        [
            str(helper),
            "--render-style",
            str(source),
            str(output),
            str(manifest),
            "0",
            "0",
            "1",
            "0",
            str(INPUT_SIZE),
            "Standard",
        ],
        capture_output=True,
        text=True,
        check=False,
        timeout=240,
    )
    try:
        result = json.loads(completed.stdout)
    except json.JSONDecodeError:
        result = {}
    if completed.returncode or result.get("status") != "written" or not output.is_file():
        raise ReverseKey1Error(
            "disabled native render failed: "
            f"exit={completed.returncode} stderr={completed.stderr[-1200:]} "
            f"result={json.dumps(result)[:1200]}"
        )
    return result


@dataclass(frozen=True)
class PreparationConfig:
    corpus: Path
    output: Path
    helper: Path
    exiftool: str = "exiftool"
    workers: int = 4
    maximum_output_bytes: int = 4 * 1024**3


def _prepare_one(
    record: Mapping[str, Any],
    config: PreparationConfig,
) -> dict[str, Any]:
    source = Path(str(record["sourcePath"]))
    source_hash = str(record["sourceSHA256"])
    sample_path = config.output / "samples" / f"{source_hash}.npz"
    if sample_path.is_file():
        try:
            with np.load(sample_path, allow_pickle=False) as existing:
                if (
                    str(existing["schema"].reshape(-1)[0]) == SAMPLE_SCHEMA
                    and str(existing["source_sha256"].reshape(-1)[0]) == source_hash
                ):
                    return {
                        **dict(record),
                        "status": "cached",
                        "samplePath": str(sample_path.relative_to(config.output)),
                        "sampleBytes": sample_path.stat().st_size,
                    }
        except (OSError, KeyError, ValueError):
            pass

    styled, display_width, display_height = _read_primary(source)
    payload = _read_key1(source, config.exiftool)
    if len(payload) != KEY1_BYTE_LENGTH:
        return {
            **dict(record),
            "status": "ineligible_key1",
            "key1Bytes": len(payload),
        }
    key1, mask, width_slots, height_slots = decode_key1(
        payload,
        display_width=display_width,
        display_height=display_height,
    )
    with tempfile.TemporaryDirectory(prefix="xdremux-key1-render-") as directory:
        render_dir = Path(directory)
        output = render_dir / "disabled.png"
        manifest = render_dir / "disabled.json"
        render_result = _disabled_render(config.helper, source, output, manifest)
        with Image.open(output) as image:
            unstyled = _fit_rgb(image)

    images = np.stack((styled, unstyled), axis=0)
    _atomic_npz(
        sample_path,
        schema=np.asarray([SAMPLE_SCHEMA]),
        source_sha256=np.asarray([source_hash]),
        images=images,
        key1=key1,
        mask=mask,
        grid=np.asarray([width_slots, height_slots], dtype=np.int16),
    )
    return {
        **dict(record),
        "status": "prepared",
        "samplePath": str(sample_path.relative_to(config.output)),
        "sampleBytes": sample_path.stat().st_size,
        "displayWidth": display_width,
        "displayHeight": display_height,
        "gridWidth": width_slots,
        "gridHeight": height_slots,
        "renderStageMilliseconds": render_result.get("stageMilliseconds"),
    }


def prepare_dataset(config: PreparationConfig) -> dict[str, Any]:
    corpus = config.corpus.resolve()
    output = config.output.resolve()
    helper = config.helper.resolve()
    if not corpus.is_dir():
        raise ReverseKey1Error(f"missing corpus: {corpus}")
    if not helper.is_file() or not os.access(helper, os.X_OK):
        raise ReverseKey1Error(f"helper is not executable: {helper}")
    if config.workers < 1 or config.workers > 4:
        raise ReverseKey1Error("workers must be between 1 and 4")
    output.mkdir(parents=True, exist_ok=True)
    paths = sorted(
        (
            path
            for path in corpus.iterdir()
            if path.is_file() and path.suffix.lower() in {".heic", ".heif"}
        ),
        key=lambda path: path.name.casefold(),
    )
    if not paths:
        raise ReverseKey1Error("corpus contains no HEIC/HEIF inputs")

    inventory = _exif_inventory(paths, config.exiftool)
    metadata_by_path = {
        Path(str(item["SourceFile"])).resolve(): item for item in inventory
    }
    with concurrent.futures.ThreadPoolExecutor(max_workers=config.workers) as pool:
        hashes = dict(zip(paths, pool.map(sha256_file, paths)))
    canonical_by_hash: dict[str, Path] = {}
    records: list[dict[str, Any]] = []
    duplicate_count = 0
    for path in paths:
        content_hash = hashes[path]
        if content_hash in canonical_by_hash:
            duplicate_count += 1
            continue
        canonical_by_hash[content_hash] = path
        raw = metadata_by_path[path.resolve()]
        record = {
            "sourcePath": str(path.resolve()),
            "relativePath": path.name,
            "sourceSHA256": content_hash,
            "sourceBytes": path.stat().st_size,
            "Model": raw.get("Model"),
            "Software": raw.get("Software"),
            "LensModel": raw.get("LensModel"),
            "Orientation": raw.get("Orientation"),
            "Tag0": raw.get("Tag0"),
            "_captureTime": _timestamp(raw),
        }
        records.append(record)
    assign_sessions(records)

    def run(record: Mapping[str, Any]) -> dict[str, Any]:
        try:
            return _prepare_one(record, config)
        except Exception as error:  # keep the full corpus run resumable
            return {
                **dict(record),
                "status": "failed",
                "error": f"{type(error).__name__}: {error}",
            }

    prepared: list[dict[str, Any]] = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=config.workers) as pool:
        futures = [pool.submit(run, record) for record in records]
        for index, future in enumerate(concurrent.futures.as_completed(futures), 1):
            prepared.append(future.result())
            if index % 25 == 0 or index == len(futures):
                cache_bytes = sum(
                    path.stat().st_size
                    for path in (output / "samples").glob("*.npz")
                )
                if cache_bytes > config.maximum_output_bytes:
                    raise ReverseKey1Error(
                        "training cache exceeded configured storage ceiling: "
                        f"{cache_bytes} > {config.maximum_output_bytes}"
                    )
                print(
                    json.dumps(
                        {
                            "prepared": index,
                            "total": len(futures),
                            "cacheBytes": cache_bytes,
                            "statusCounts": Counter(
                                item.get("status") for item in prepared
                            ),
                        },
                        default=dict,
                        sort_keys=True,
                    ),
                    flush=True,
                )
    for item in prepared:
        item.pop("_captureTime", None)
    prepared.sort(key=lambda item: str(item["relativePath"]).casefold())
    usable = [
        item
        for item in prepared
        if item.get("status") in {"prepared", "cached"}
    ]
    header = {
        "schema": DATASET_SCHEMA,
        "corpusSHA256": hashlib.sha256(
            canonical_json_bytes(
                [(item["relativePath"], item["sourceSHA256"]) for item in prepared]
            )
        ).hexdigest(),
        "inputSize": INPUT_SIZE,
        "inputContract": [
            "styled_rgb",
            "unstyled_rgb",
            "styled_minus_unstyled_rgb",
            "ycbcr_difference",
        ],
        "labelContract": "padded-y-x-12x12x8x10x3-with-native-orientation-mask",
        "deviceMetadataPolicy": "audit_only_not_model_input",
        "unstyledProvenance": "PLPhotoEditRenderer disabled SemanticStyle render",
        "styledProvenance": "oriented native HEIC primary downsample",
        "counts": {
            "files": len(paths),
            "uniqueContent": len(records),
            "duplicateExtraCopies": duplicate_count,
            "usable": len(usable),
            "failed": sum(item.get("status") == "failed" for item in prepared),
            "ineligibleKey1": sum(
                item.get("status") == "ineligible_key1" for item in prepared
            ),
            "independentSessions": len(
                {item["captureSession"] for item in usable}
            ),
        },
        "splitCounts": dict(Counter(item["split"] for item in usable)),
        "modelCounts": dict(Counter(str(item.get("Model")) for item in usable)),
        "softwareCounts": dict(
            Counter(str(item.get("Software")) for item in usable)
        ),
        "protocolCounts": dict(Counter(str(item.get("Tag0")) for item in usable)),
        "cacheBytes": sum(int(item.get("sampleBytes", 0)) for item in usable),
    }
    manifest = {"header": header, "samples": prepared}
    header["recordsSHA256"] = hashlib.sha256(
        canonical_json_bytes(prepared)
    ).hexdigest()
    _atomic_json(output / "manifest.json", manifest)
    return header


def load_manifest(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    value = json.loads(path.read_text(encoding="utf-8"))
    header = value.get("header")
    samples = value.get("samples")
    if not isinstance(header, dict) or header.get("schema") != DATASET_SCHEMA:
        raise ReverseKey1Error("invalid reverse-key1 dataset header")
    if not isinstance(samples, list):
        raise ReverseKey1Error("reverse-key1 samples must be an array")
    expected = hashlib.sha256(canonical_json_bytes(samples)).hexdigest()
    if header.get("recordsSHA256") != expected:
        raise ReverseKey1Error("reverse-key1 manifest record hash mismatch")
    usable = [
        sample
        for sample in samples
        if sample.get("status") in {"prepared", "cached"}
    ]
    sessions_by_split: dict[str, set[str]] = defaultdict(set)
    for sample in usable:
        sessions_by_split[str(sample["split"])].add(str(sample["captureSession"]))
    splits = list(sessions_by_split)
    for index, left in enumerate(splits):
        for right in splits[index + 1 :]:
            overlap = sessions_by_split[left] & sessions_by_split[right]
            if overlap:
                raise ReverseKey1Error(
                    f"capture-session leakage between {left} and {right}"
                )
    return header, usable


def input_features(images: np.ndarray) -> np.ndarray:
    value = np.asarray(images, dtype=np.float32) / 255.0
    if value.shape != (2, 3, INPUT_SIZE, INPUT_SIZE):
        raise ReverseKey1Error("cached image pair has an invalid shape")
    styled, unstyled = value
    difference = styled - unstyled
    matrix = np.asarray(
        [
            [0.2126, 0.7152, 0.0722],
            [-0.114572, -0.385428, 0.5],
            [0.5, -0.454153, -0.045847],
        ],
        dtype=np.float32,
    )
    ycbcr_difference = np.einsum("oc,chw->ohw", matrix, difference)
    return np.concatenate((styled, unstyled, difference, ycbcr_difference), axis=0)


def _require_torch() -> tuple[Any, Any]:
    try:
        import torch
        import torch.nn as nn
    except ImportError as error:
        raise ReverseKey1Error(
            "training requires PyTorch; install the project training extra"
        ) from error
    return torch, nn


def build_model(scales: np.ndarray | None = None) -> Any:
    torch, nn = _require_torch()

    class ResidualBlock(nn.Module):
        def __init__(self, channels: int):
            super().__init__()
            self.layers = nn.Sequential(
                nn.Conv2d(channels, channels, 3, padding=1),
                nn.GroupNorm(8, channels),
                nn.SiLU(),
                nn.Conv2d(channels, channels, 3, padding=1),
                nn.GroupNorm(8, channels),
            )

        def forward(self, value: Any) -> Any:
            return torch.nn.functional.silu(value + self.layers(value))

    class ReverseKey1Net(nn.Module):
        def __init__(self):
            super().__init__()
            channels = (32, 48, 64, 96, 128)
            layers: list[Any] = []
            incoming = INPUT_CHANNELS
            for channel in channels:
                layers.extend(
                    [
                        nn.Conv2d(incoming, channel, 3, stride=2, padding=1),
                        nn.GroupNorm(8, channel),
                        nn.SiLU(),
                        ResidualBlock(channel),
                    ]
                )
                incoming = channel
            self.encoder = nn.Sequential(*layers)
            self.plane_embedding = nn.Embedding(PLANE_COUNT, 32)
            output_layer = nn.Linear(128, POLYNOMIAL_COUNT * OUTPUT_COUNT)
            nn.init.zeros_(output_layer.weight)
            nn.init.zeros_(output_layer.bias)
            self.head = nn.Sequential(
                nn.Linear(channels[-1] + 32, 128),
                nn.SiLU(),
                output_layer,
            )
            initial_scales = np.ones(
                (PLANE_COUNT, POLYNOMIAL_COUNT, OUTPUT_COUNT), dtype=np.float32
            ) if scales is None else np.asarray(scales, dtype=np.float32)
            if initial_scales.shape != (
                PLANE_COUNT,
                POLYNOMIAL_COUNT,
                OUTPUT_COUNT,
            ):
                raise ReverseKey1Error("coefficient scale shape is invalid")
            self.register_buffer(
                "coefficient_scales",
                torch.from_numpy(initial_scales).reshape(
                    1, 1, 1, PLANE_COUNT, POLYNOMIAL_COUNT, OUTPUT_COUNT
                ),
            )
            self.register_buffer(
                "identity",
                torch.from_numpy(identity_key1()).reshape(
                    1,
                    GRID_LONG,
                    GRID_LONG,
                    PLANE_COUNT,
                    POLYNOMIAL_COUNT,
                    OUTPUT_COUNT,
                ),
            )

        def forward(self, value: Any) -> Any:
            spatial = torch.nn.functional.interpolate(
                self.encoder(value),
                size=(GRID_LONG, GRID_LONG),
                mode="bilinear",
                align_corners=False,
            ).permute(0, 2, 3, 1)
            batch, height, width, _ = spatial.shape
            spatial = spatial[:, :, :, None, :].expand(
                batch, height, width, PLANE_COUNT, spatial.shape[-1]
            )
            plane = self.plane_embedding.weight.reshape(
                1, 1, 1, PLANE_COUNT, -1
            ).expand(batch, height, width, -1, -1)
            normalized_residual = self.head(torch.cat((spatial, plane), dim=-1))
            normalized_residual = normalized_residual.reshape(
                batch,
                height,
                width,
                PLANE_COUNT,
                POLYNOMIAL_COUNT,
                OUTPUT_COUNT,
            )
            return self.identity + normalized_residual * self.coefficient_scales

    return ReverseKey1Net()


class _CachedDataset:
    def __init__(self, root: Path, samples: Sequence[Mapping[str, Any]]):
        self.root = root
        self.samples = list(samples)

    def __len__(self) -> int:
        return len(self.samples)

    def __getitem__(self, index: int) -> tuple[Any, ...]:
        torch, _ = _require_torch()
        record = self.samples[index]
        path = self.root / str(record["samplePath"])
        with np.load(path, allow_pickle=False) as archive:
            features = input_features(archive["images"])
            key1 = np.asarray(archive["key1"], dtype=np.float32)
            mask = np.asarray(archive["mask"], dtype=np.bool_)
        return (
            torch.from_numpy(features),
            torch.from_numpy(key1),
            torch.from_numpy(mask),
            str(record.get("Model") or "unknown"),
            str(record["captureSession"]),
        )


def coefficient_scales(
    root: Path,
    samples: Sequence[Mapping[str, Any]],
) -> np.ndarray:
    identity = identity_key1()
    residuals: list[np.ndarray] = []
    for sample in samples:
        with np.load(root / str(sample["samplePath"]), allow_pickle=False) as archive:
            key1 = np.asarray(archive["key1"], dtype=np.float32)
            mask = np.asarray(archive["mask"], dtype=np.bool_)
        residuals.append((key1 - identity)[mask])
    if not residuals:
        raise ReverseKey1Error("training split is empty")
    stacked = np.concatenate(residuals, axis=0)
    scale = np.quantile(np.abs(stacked), 0.75, axis=0).astype(np.float32)
    return np.maximum(scale, 1e-3)


@dataclass(frozen=True)
class TrainingConfig:
    manifest: Path
    output: Path
    epochs: int = 60
    batch_size: int = 8
    learning_rate: float = 2e-4
    seed: int = 260819
    device: str = "auto"
    num_workers: int = 0


def _select_device(torch: Any, requested: str) -> str:
    if requested == "auto":
        return "mps" if torch.backends.mps.is_available() else "cpu"
    if requested == "mps" and not torch.backends.mps.is_available():
        raise ReverseKey1Error("MPS was requested but is unavailable")
    if requested not in {"mps", "cpu"}:
        raise ReverseKey1Error("training device must be auto, mps, or cpu")
    return requested


def _masked_losses(
    torch: Any,
    prediction: Any,
    target: Any,
    mask: Any,
    scales: Any,
) -> tuple[Any, dict[str, float]]:
    expanded = mask[:, :, :, None, None, None]
    normalized = (prediction - target) / scales
    selected = normalized[expanded.expand_as(normalized)]
    coefficient = torch.nn.functional.huber_loss(
        selected, torch.zeros_like(selected), delta=1.0
    )
    horizontal_mask = mask[:, :, 1:] & mask[:, :, :-1]
    vertical_mask = mask[:, 1:, :] & mask[:, :-1, :]
    normalized_prediction = (prediction - target.new_tensor(identity_key1())) / scales
    horizontal = normalized_prediction[:, :, 1:] - normalized_prediction[:, :, :-1]
    vertical = normalized_prediction[:, 1:] - normalized_prediction[:, :-1]
    spatial_values = []
    if horizontal_mask.any():
        spatial_values.append(
            horizontal[
                horizontal_mask[:, :, :, None, None, None].expand_as(horizontal)
            ].abs().mean()
        )
    if vertical_mask.any():
        spatial_values.append(
            vertical[
                vertical_mask[:, :, :, None, None, None].expand_as(vertical)
            ].abs().mean()
        )
    spatial = sum(spatial_values) / max(1, len(spatial_values))
    total = coefficient + 2e-4 * spatial
    return total, {
        "coefficientHuber": float(coefficient.detach().cpu()),
        "spatialL1": float(spatial.detach().cpu()),
    }


def _evaluate(
    torch: Any,
    model: Any,
    loader: Any,
    device: str,
    *,
    shuffle_inputs: bool = False,
) -> dict[str, Any]:
    model.eval()
    normalized_absolute: list[Any] = []
    raw_absolute: list[Any] = []
    per_model: dict[str, list[float]] = defaultdict(list)
    scales = model.coefficient_scales
    with torch.no_grad():
        for features, target, mask, models, _sessions in loader:
            features = features.to(device)
            target = target.to(device)
            mask = mask.to(device)
            if shuffle_inputs and len(features) > 1:
                features = torch.roll(features, shifts=1, dims=0)
            prediction = model(features)
            normalized = ((prediction - target) / scales).abs()
            raw = (prediction - target).abs()
            for index, model_name in enumerate(models):
                selected = mask[index, :, :, None, None, None].expand_as(
                    normalized[index]
                )
                normalized_values = normalized[index][selected]
                raw_values = raw[index][selected]
                normalized_absolute.append(normalized_values.cpu())
                raw_absolute.append(raw_values.cpu())
                per_model[str(model_name)].append(
                    float(normalized_values.mean().cpu())
                )
    if not normalized_absolute:
        raise ReverseKey1Error("evaluation split is empty")
    normalized_values = torch.cat(normalized_absolute)
    raw_values = torch.cat(raw_absolute)
    return {
        "normalizedMAE": float(normalized_values.mean()),
        "normalizedRMSE": float(torch.sqrt(torch.mean(normalized_values**2))),
        "normalizedP95Absolute": float(torch.quantile(normalized_values, 0.95)),
        "rawMAE": float(raw_values.mean()),
        "rawP95Absolute": float(torch.quantile(raw_values, 0.95)),
        "perModelNormalizedMAE": {
            name: float(np.mean(values)) for name, values in sorted(per_model.items())
        },
    }


def train(config: TrainingConfig) -> dict[str, Any]:
    torch, _ = _require_torch()
    torch.manual_seed(config.seed)
    np.random.seed(config.seed)
    header, samples = load_manifest(config.manifest.resolve())
    root = config.manifest.resolve().parent
    by_split = {
        split: [sample for sample in samples if sample["split"] == split]
        for split in ("train", "calibration", "heldout")
    }
    if any(not by_split[split] for split in by_split):
        raise ReverseKey1Error("train/calibration/heldout splits must all be non-empty")
    scales = coefficient_scales(root, by_split["train"])
    model = build_model(scales)
    device = _select_device(torch, config.device)
    model.to(device)
    loaders = {
        split: torch.utils.data.DataLoader(
            _CachedDataset(root, values),
            batch_size=config.batch_size,
            shuffle=split == "train",
            num_workers=config.num_workers,
        )
        for split, values in by_split.items()
    }
    optimizer = torch.optim.AdamW(
        model.parameters(), lr=config.learning_rate, weight_decay=1e-4
    )
    scheduler = torch.optim.lr_scheduler.CosineAnnealingLR(
        optimizer, T_max=max(1, config.epochs)
    )
    config.output.mkdir(parents=True, exist_ok=True)
    best = math.inf
    history: list[dict[str, Any]] = []
    baselines = {
        "identityCalibration": _evaluate(
            torch, model, loaders["calibration"], device
        ),
        "identityHeldout": _evaluate(torch, model, loaders["heldout"], device),
    }
    for epoch in range(1, config.epochs + 1):
        model.train()
        epoch_losses: list[float] = []
        for features, target, mask, _models, _sessions in loaders["train"]:
            features = features.to(device)
            target = target.to(device)
            mask = mask.to(device)
            optimizer.zero_grad(set_to_none=True)
            identity_features = torch.cat(
                (
                    features[:, 3:6],
                    features[:, 3:6],
                    torch.zeros_like(features[:, 6:12]),
                ),
                dim=1,
            )
            combined_prediction = model(torch.cat((features, identity_features), dim=0))
            prediction, identity_prediction = combined_prediction.chunk(2, dim=0)
            loss, _parts = _masked_losses(
                torch,
                prediction,
                target,
                mask,
                model.coefficient_scales,
            )
            identity_normalized = (
                identity_prediction - model.identity
            ) / model.coefficient_scales
            identity_selected = identity_normalized[
                mask[:, :, :, None, None, None].expand_as(identity_normalized)
            ]
            identity_loss = torch.nn.functional.huber_loss(
                identity_selected,
                torch.zeros_like(identity_selected),
                delta=1.0,
            )
            loss = loss + 0.01 * identity_loss
            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 5.0)
            optimizer.step()
            epoch_losses.append(float(loss.detach().cpu()))
        scheduler.step()
        calibration = _evaluate(torch, model, loaders["calibration"], device)
        epoch_record = {
            "epoch": epoch,
            "learningRate": float(scheduler.get_last_lr()[0]),
            "trainingLoss": float(np.mean(epoch_losses)),
            "calibration": calibration,
        }
        history.append(epoch_record)
        checkpoint = {
            "schema": REPORT_SCHEMA,
            "epoch": epoch,
            "model": model.state_dict(),
            "optimizer": optimizer.state_dict(),
            "coefficientScales": scales,
            "manifestSHA256": sha256_file(config.manifest.resolve()),
            "sourceCorpusSHA256": header["corpusSHA256"],
            "architecture": "ReverseKey1Net-v1",
            "inputChannels": INPUT_CHANNELS,
            "deviceMetadataPolicy": "audit_only_not_model_input",
        }
        torch.save(checkpoint, config.output / "last.pt")
        if calibration["normalizedMAE"] < best:
            best = calibration["normalizedMAE"]
            torch.save(checkpoint, config.output / "best.pt")
        _atomic_json(config.output / "history.json", history)
        print(json.dumps(epoch_record, sort_keys=True), flush=True)

    best_checkpoint = torch.load(
        config.output / "best.pt", map_location=device, weights_only=False
    )
    model.load_state_dict(best_checkpoint["model"])
    heldout = _evaluate(torch, model, loaders["heldout"], device)
    heldout_shuffled = _evaluate(
        torch,
        model,
        loaders["heldout"],
        device,
        shuffle_inputs=True,
    )
    report = {
        "schema": REPORT_SCHEMA,
        "architecture": "ReverseKey1Net-v1",
        "device": device,
        "epochs": config.epochs,
        "batchSize": config.batch_size,
        "learningRate": config.learning_rate,
        "seed": config.seed,
        "parameterCount": sum(parameter.numel() for parameter in model.parameters()),
        "dataset": header,
        "splitCounts": {name: len(values) for name, values in by_split.items()},
        "baselines": baselines,
        "bestEpoch": int(best_checkpoint["epoch"]),
        "calibrationBestNormalizedMAE": best,
        "heldout": heldout,
        "heldoutShuffledInput": heldout_shuffled,
        "artifacts": {
            "best": "best.pt",
            "last": "last.pt",
            "history": "history.json",
        },
    }
    _atomic_json(config.output / "report.json", report)
    return report


def verify_training_run(manifest: Path, run_directory: Path) -> dict[str, Any]:
    """Verify checkpoint provenance and the minimum offline learning evidence."""
    torch, _ = _require_torch()
    manifest = manifest.resolve()
    run_directory = run_directory.resolve()
    header, samples = load_manifest(manifest)
    report_path = run_directory / "report.json"
    if not report_path.is_file():
        raise ReverseKey1Error(f"missing training report: {report_path}")
    report = json.loads(report_path.read_text(encoding="utf-8"))
    if report.get("schema") != REPORT_SCHEMA:
        raise ReverseKey1Error("training report schema is invalid")
    if report.get("dataset", {}).get("corpusSHA256") != header.get("corpusSHA256"):
        raise ReverseKey1Error("training report is bound to another corpus")
    if report.get("splitCounts") != dict(Counter(sample["split"] for sample in samples)):
        raise ReverseKey1Error("training report split counts do not match the manifest")
    best_path = run_directory / str(report.get("artifacts", {}).get("best", ""))
    last_path = run_directory / str(report.get("artifacts", {}).get("last", ""))
    if not best_path.is_file() or not last_path.is_file():
        raise ReverseKey1Error("best/last checkpoint pair is incomplete")
    checkpoint = torch.load(best_path, map_location="cpu", weights_only=False)
    if checkpoint.get("manifestSHA256") != sha256_file(manifest):
        raise ReverseKey1Error("best checkpoint manifest hash does not match")
    if checkpoint.get("sourceCorpusSHA256") != header.get("corpusSHA256"):
        raise ReverseKey1Error("best checkpoint corpus hash does not match")
    if int(checkpoint.get("epoch", -1)) != int(report.get("bestEpoch", -2)):
        raise ReverseKey1Error("best checkpoint epoch does not match the report")
    heldout = report.get("heldout", {})
    identity = report.get("baselines", {}).get("identityHeldout", {})
    shuffled = report.get("heldoutShuffledInput", {})
    metrics = {
        "heldoutNormalizedMAE": heldout.get("normalizedMAE"),
        "identityNormalizedMAE": identity.get("normalizedMAE"),
        "shuffledNormalizedMAE": shuffled.get("normalizedMAE"),
    }
    if any(not isinstance(value, (int, float)) or not math.isfinite(value) for value in metrics.values()):
        raise ReverseKey1Error("required held-out metrics are missing or non-finite")
    if not metrics["heldoutNormalizedMAE"] < metrics["identityNormalizedMAE"]:
        raise ReverseKey1Error("trained checkpoint does not beat the identity baseline")
    if not metrics["heldoutNormalizedMAE"] < metrics["shuffledNormalizedMAE"]:
        raise ReverseKey1Error("trained checkpoint does not use paired image information")
    return {
        "schema": "xdremux-reverse-key1-training-verification-v1",
        "passed": True,
        "manifestSHA256": sha256_file(manifest),
        "reportSHA256": sha256_file(report_path),
        "bestCheckpointSHA256": sha256_file(best_path),
        "lastCheckpointSHA256": sha256_file(last_path),
        "bestEpoch": int(report["bestEpoch"]),
        "parameterCount": int(report["parameterCount"]),
        "sampleCount": len(samples),
        "metrics": metrics,
    }


def storage_summary(paths: Iterable[Path]) -> dict[str, int]:
    result: dict[str, int] = {}
    for path in paths:
        if path.is_file():
            result[str(path)] = path.stat().st_size
        elif path.is_dir():
            result[str(path)] = sum(
                child.stat().st_size for child in path.rglob("*") if child.is_file()
            )
    return result
