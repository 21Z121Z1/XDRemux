"""Universal-image training for an Apple Photographic Style state.

The native iPhone corpus is the only source of producer ground truth.  This
module deliberately consumes only the styled primary image at inference time;
the paired unstyled image remains an auxiliary training target.  Optional RAW,
gain-map, and metadata inputs are represented by explicit modality masks so a
missing measurement cannot be confused with a real zero.
"""

from __future__ import annotations

import base64
import dataclasses
import json
import math
import subprocess
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Mapping, Sequence

import numpy as np

from xdremux_py.apple_reverse_key1_training import (
    GRID_LONG,
    OUTPUT_COUNT,
    PLANE_COUNT,
    POLYNOMIAL_COUNT,
    ReverseKey1Error,
    _atomic_json,
    _atomic_npz,
    _require_torch,
    canonical_json_bytes,
    coefficient_scales,
    identity_key1,
    load_manifest,
    sha256_file,
)


DATASET_SCHEMA = "xdremux-universal-photographic-style-dataset-v1"
REPORT_SCHEMA = "xdremux-universal-photographic-style-training-v1"
INPUT_SIZE = 256
PRIMARY_CHANNELS = 9
METADATA_FIELDS = (
    "log2_display_width",
    "log2_display_height",
    "display_aspect",
    "orientation",
    "log2_exposure_seconds",
    "log2_f_number",
    "log2_iso",
    "focal_length_mm",
    "log2_color_temperature",
    "hdr_gain",
    "software_major",
    "native_protocol",
    "has_gain_map",
    "has_raw",
    "input_bit_depth",
    "has_alpha",
)
STYLE_SCALAR_FIELDS = (
    "TagH",
    "IOriginalRangeMin",
    "IOriginalRangeMax",
    "IGain",
    "Tag4",
    "Tag5",
)


def primary_image_features(styled: np.ndarray) -> np.ndarray:
    """Build device-agnostic features from one oriented RGB primary image."""

    value = np.asarray(styled, dtype=np.float32)
    if value.shape != (3, INPUT_SIZE, INPUT_SIZE):
        raise ReverseKey1Error(
            f"styled primary must be 3x{INPUT_SIZE}x{INPUT_SIZE}"
        )
    if value.max(initial=0) > 1.0:
        value = value / 255.0
    value = np.clip(value, 0.0, 1.0)
    red, green, blue = value
    luma = 0.2126 * red + 0.7152 * green + 0.0722 * blue
    cb = (blue - luma) * 0.5
    cr = (red - luma) * 0.5
    log_luma = np.log1p(15.0 * luma) / math.log(16.0)
    gradient_x = np.zeros_like(luma)
    gradient_y = np.zeros_like(luma)
    gradient_x[:, 1:] = luma[:, 1:] - luma[:, :-1]
    gradient_y[1:, :] = luma[1:, :] - luma[:-1, :]
    return np.concatenate(
        (
            value,
            luma[None],
            cb[None],
            cr[None],
            log_luma[None],
            gradient_x[None],
            gradient_y[None],
        ),
        axis=0,
    ).astype(np.float32)


def _number(value: Any) -> float | None:
    if isinstance(value, (int, float)):
        result = float(value)
        return result if math.isfinite(result) else None
    if not isinstance(value, str):
        return None
    stripped = value.strip().split(" ", 1)[0]
    try:
        if "/" in stripped:
            numerator, denominator = stripped.split("/", 1)
            result = float(numerator) / float(denominator)
        else:
            result = float(stripped)
    except (ValueError, ZeroDivisionError):
        return None
    return result if math.isfinite(result) else None


def metadata_vector(
    record: Mapping[str, Any],
    tags: Mapping[str, Any],
    *,
    has_gain_map: bool = False,
    has_raw: bool = False,
    input_bit_depth: float | None = None,
    has_alpha: bool = False,
) -> tuple[np.ndarray, np.ndarray]:
    """Return numeric metadata and a parallel missing-value mask."""

    width = _number(record.get("displayWidth"))
    height = _number(record.get("displayHeight"))
    exposure = _number(tags.get("ExposureTime"))
    f_number = _number(tags.get("FNumber"))
    iso = _number(tags.get("ISO"))
    focal = _number(tags.get("FocalLength"))
    temperature = _number(tags.get("ColorTemperature"))
    hdr_gain = _number(tags.get("HDRGain"))
    orientation = _number(tags.get("Orientation"))
    if orientation is None:
        orientation = _number(record.get("Orientation"))
    software = _number(record.get("Software"))
    protocol = _number(record.get("Tag0"))
    raw_values: list[float | None] = [
        math.log2(width) if width and width > 0 else None,
        math.log2(height) if height and height > 0 else None,
        width / height if width and height and height > 0 else None,
        orientation / 8.0 if orientation else None,
        math.log2(exposure) if exposure and exposure > 0 else None,
        math.log2(f_number) if f_number and f_number > 0 else None,
        math.log2(iso) if iso and iso > 0 else None,
        focal / 20.0 if focal is not None else None,
        math.log2(temperature) if temperature and temperature > 0 else None,
        hdr_gain,
        software / 30.0 if software is not None else None,
        math.log2(1.0 + protocol) / 16.0 if protocol is not None and protocol >= 0 else None,
        float(has_gain_map),
        float(has_raw),
        input_bit_depth / 16.0 if input_bit_depth else None,
        float(has_alpha),
    ]
    values = np.zeros(len(METADATA_FIELDS), dtype=np.float32)
    mask = np.zeros(len(METADATA_FIELDS), dtype=np.float32)
    for index, value in enumerate(raw_values):
        if value is not None and math.isfinite(value):
            values[index] = value
            mask[index] = 1.0
    # Modality-presence flags are always observed, even when false.
    for field in ("has_gain_map", "has_raw", "has_alpha"):
        mask[METADATA_FIELDS.index(field)] = 1.0
    return values, mask


def decode_style_binary(value: str, expected_bytes: int, field: str) -> bytes:
    if not isinstance(value, str) or not value.startswith("base64:"):
        raise ReverseKey1Error(f"{field} is not base64 binary data")
    decoded = base64.b64decode(value[7:], validate=True)
    if len(decoded) != expected_bytes:
        raise ReverseKey1Error(
            f"{field} byte count is {len(decoded)}, expected {expected_bytes}"
        )
    return decoded


@dataclasses.dataclass(frozen=True)
class UniversalPreparationConfig:
    native_manifest: Path
    output: Path
    exiftool: str = "exiftool"


def prepare_universal_dataset(config: UniversalPreparationConfig) -> dict[str, Any]:
    source_manifest = config.native_manifest.resolve()
    source_header, records = load_manifest(source_manifest)
    paths = [str(record["sourcePath"]) for record in records]
    command = [
        config.exiftool,
        "-json",
        "-b",
        "-n",
        "-Tag3",
        "-TagC",
        "-TagD",
        *[f"-{field}" for field in STYLE_SCALAR_FIELDS],
        "-ExposureTime",
        "-FNumber",
        "-ISO",
        "-FocalLength",
        "-ColorTemperature",
        "-HDRGain",
        "-Orientation",
        *paths,
    ]
    result = subprocess.run(command, check=False, capture_output=True)
    if result.returncode != 0:
        diagnostic = result.stderr.decode("utf-8", errors="replace").strip()
        raise ReverseKey1Error(f"exiftool universal inventory failed: {diagnostic}")
    inventory = json.loads(result.stdout)
    by_path = {str(item["SourceFile"]): item for item in inventory}
    if len(by_path) != len(records):
        raise ReverseKey1Error("universal inventory did not preserve one record per sample")

    # Tag3 is a 516-byte mixed-layout GTC resource, not a flat Float16 array.
    # Keep the complete native byte contract.  Treating all bytes after the
    # four-byte prefix as half floats silently turns structural fields into
    # NaNs/Infs on every sample.
    gtc = np.empty((len(records), 516), dtype=np.uint8)
    light_maps = np.empty((len(records), 2, 32, 32), dtype=np.float16)
    scalars = np.zeros((len(records), len(STYLE_SCALAR_FIELDS)), dtype=np.float32)
    scalar_mask = np.zeros_like(scalars)
    metadata = np.zeros((len(records), len(METADATA_FIELDS)), dtype=np.float32)
    metadata_mask = np.zeros_like(metadata)
    prepared_records: list[dict[str, Any]] = []
    source_root = source_manifest.parent
    for index, record in enumerate(records):
        tags = by_path[str(record["sourcePath"])]
        tag3 = decode_style_binary(tags.get("Tag3"), 516, "Tag3")
        tag_c = decode_style_binary(tags.get("TagC"), 2048, "TagC")
        tag_d = decode_style_binary(tags.get("TagD"), 2048, "TagD")
        gtc[index] = np.frombuffer(tag3, dtype=np.uint8)
        light_maps[index, 0] = np.frombuffer(tag_c, dtype="<f2").reshape(32, 32)
        light_maps[index, 1] = np.frombuffer(tag_d, dtype="<f2").reshape(32, 32)
        for scalar_index, field in enumerate(STYLE_SCALAR_FIELDS):
            value = _number(tags.get(field))
            if value is not None:
                scalars[index, scalar_index] = value
                scalar_mask[index, scalar_index] = 1.0
        metadata[index], metadata_mask[index] = metadata_vector(record, tags)
        prepared_records.append(
            {
                "index": index,
                "sourceSHA256": record["sourceSHA256"],
                "sourcePath": record["sourcePath"],
                "samplePath": str((source_root / str(record["samplePath"])).resolve()),
                "captureSession": record["captureSession"],
                "split": record["split"],
                "Model": record.get("Model"),
                "Software": record.get("Software"),
            }
        )
    for name, values in (
        ("lightMaps", light_maps),
        ("scalars", scalars),
        ("metadata", metadata),
    ):
        if not np.isfinite(values).all():
            raise ReverseKey1Error(f"universal {name} labels contain non-finite values")

    output = config.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    labels_path = output / "labels.npz"
    _atomic_npz(
        labels_path,
        gtc=gtc,
        light_maps=light_maps,
        scalars=scalars,
        scalar_mask=scalar_mask,
        metadata=metadata,
        metadata_mask=metadata_mask,
    )
    header = {
        "schema": DATASET_SCHEMA,
        "sourceManifest": str(source_manifest),
        "sourceManifestSHA256": sha256_file(source_manifest),
        "sourceCorpusSHA256": source_header["corpusSHA256"],
        "recordsSHA256": "",
        "labelsSHA256": sha256_file(labels_path),
        "labelBytes": labels_path.stat().st_size,
        "sampleCount": len(prepared_records),
        "sessionCount": len({record["captureSession"] for record in prepared_records}),
        "splitCounts": dict(Counter(record["split"] for record in prepared_records)),
        "modelCounts": dict(Counter(str(record.get("Model")) for record in prepared_records)),
        "inputContract": "single-styled-primary-plus-masked-optional-modalities",
        "outputContract": {
            "key1": "12x9-or-9x12x8x10x3 Float16 lattice",
            "gtc": "complete 516-byte native mixed-layout Tag3 resource",
            "lightMaps": "c/d 2x32x32 Float16",
            "scalars": list(STYLE_SCALAR_FIELDS),
            "uncertainty": "input-conditioned normalized error prediction",
        },
        "metadataFields": list(METADATA_FIELDS),
    }
    header["recordsSHA256"] = __import__("hashlib").sha256(
        canonical_json_bytes(prepared_records)
    ).hexdigest()
    _atomic_json(output / "manifest.json", {"header": header, "samples": prepared_records})
    return header


def load_universal_manifest(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    value = json.loads(path.read_text(encoding="utf-8"))
    header = value.get("header")
    records = value.get("samples")
    if not isinstance(header, dict) or header.get("schema") != DATASET_SCHEMA:
        raise ReverseKey1Error("invalid universal photographic style manifest")
    if not isinstance(records, list):
        raise ReverseKey1Error("universal photographic style records are not an array")
    actual = __import__("hashlib").sha256(canonical_json_bytes(records)).hexdigest()
    if actual != header.get("recordsSHA256"):
        raise ReverseKey1Error("universal photographic style record hash mismatch")
    sessions: dict[str, set[str]] = defaultdict(set)
    for record in records:
        sessions[str(record["split"])].add(str(record["captureSession"]))
    split_names = sorted(sessions)
    for index, left in enumerate(split_names):
        for right in split_names[index + 1 :]:
            if sessions[left] & sessions[right]:
                raise ReverseKey1Error("universal dataset has capture-session leakage")
    return header, records


def universal_state_statistics(
    manifest: Path, records: Sequence[Mapping[str, Any]]
) -> dict[str, np.ndarray]:
    train = [record for record in records if record["split"] == "train"]
    if not train:
        raise ReverseKey1Error("universal training split is empty")
    labels = np.load(manifest.parent / "labels.npz", allow_pickle=False)
    indices = np.asarray([int(record["index"]) for record in train])
    metadata = labels["metadata"][indices].astype(np.float32)
    metadata_mask = labels["metadata_mask"][indices].astype(bool)
    metadata_center = np.zeros(metadata.shape[1], dtype=np.float32)
    metadata_scale = np.ones(metadata.shape[1], dtype=np.float32)
    metadata_active = np.zeros(metadata.shape[1], dtype=np.float32)
    for column in range(metadata.shape[1]):
        selected = metadata[metadata_mask[:, column], column]
        if len(selected):
            metadata_center[column] = np.median(selected)
            robust_scale = float(
                np.quantile(np.abs(selected - metadata_center[column]), 0.90)
            )
            # A constant field has no learned positive example.  Mark it
            # inactive instead of dividing the first OOD value by 1e-3 and
            # feeding an arbitrary 1000x activation into metadata FiLM.
            if robust_scale >= 1e-3:
                metadata_scale[column] = robust_scale
                metadata_active[column] = 1.0
    gtc = labels["gtc"][indices].astype(np.float32) / 255.0
    light = labels["light_maps"][indices].astype(np.float32)
    scalars = labels["scalars"][indices].astype(np.float32)
    scalar_mask = labels["scalar_mask"][indices].astype(bool)
    scalar_center = np.zeros(scalars.shape[1], dtype=np.float32)
    scalar_scale = np.ones(scalars.shape[1], dtype=np.float32)
    scalar_low = np.zeros(scalars.shape[1], dtype=np.float32)
    scalar_high = np.zeros(scalars.shape[1], dtype=np.float32)
    for column in range(scalars.shape[1]):
        selected = scalars[scalar_mask[:, column], column]
        if len(selected):
            scalar_center[column] = np.median(selected)
            scalar_scale[column] = max(
                float(np.quantile(np.abs(selected - scalar_center[column]), 0.75)),
                1e-3,
            )
            scalar_low[column] = np.quantile(selected, 0.005)
            scalar_high[column] = np.quantile(selected, 0.995)
    return {
        "metadataCenter": metadata_center,
        "metadataScale": metadata_scale,
        "metadataActive": metadata_active,
        "key1Scale": coefficient_scales(
            Path("/"), [{"samplePath": record["samplePath"]} for record in train]
        ),
        "gtcCenter": np.median(gtc, axis=0).astype(np.float32),
        "gtcScale": np.maximum(
            np.quantile(np.abs(gtc - np.median(gtc, axis=0)), 0.75, axis=0), 1e-4
        ).astype(np.float32),
        "lightCenter": np.median(light, axis=0).astype(np.float32),
        "lightScale": np.maximum(
            np.quantile(
                np.abs(light - np.median(light, axis=0)), 0.75, axis=(0, 2, 3)
            ),
            1e-4,
        ).astype(np.float32),
        "scalarCenter": scalar_center,
        "scalarScale": scalar_scale,
        "scalarLow": scalar_low,
        "scalarHigh": scalar_high,
    }


def build_universal_model(
    statistics: Mapping[str, np.ndarray], *, architecture: str = "base"
) -> Any:
    torch, nn = _require_torch()
    if architecture not in {"base", "multiscale_large"}:
        raise ReverseKey1Error(f"unsupported universal architecture: {architecture}")

    class ResidualBlock(nn.Module):
        def __init__(self, channels: int) -> None:
            super().__init__()
            self.block = nn.Sequential(
                nn.Conv2d(channels, channels, 3, padding=1, bias=False),
                nn.GroupNorm(8, channels),
                nn.SiLU(),
                nn.Conv2d(channels, channels, 3, padding=1, bias=False),
                nn.GroupNorm(8, channels),
            )

        def forward(self, value: Any) -> Any:
            return torch.nn.functional.silu(value + self.block(value))

    class UniversalPhotographicStyleStateNet(nn.Module):
        def __init__(self) -> None:
            super().__init__()
            channels = (64, 96, 160, 256)
            stages: list[Any] = []
            incoming = PRIMARY_CHANNELS
            for outgoing in channels:
                blocks: list[Any] = [
                    nn.Conv2d(incoming, outgoing, 3, stride=2, padding=1, bias=False),
                    nn.GroupNorm(8, outgoing),
                    nn.SiLU(),
                    ResidualBlock(outgoing),
                ]
                if architecture == "multiscale_large":
                    blocks.append(ResidualBlock(outgoing))
                stages.append(nn.Sequential(*blocks))
                incoming = outgoing
            if architecture == "multiscale_large":
                self.encoder = None
                self.encoder_stages = nn.ModuleList(stages)
                self.scale_projections = nn.ModuleList(
                    nn.Conv2d(channel, 64, 1) for channel in channels
                )
                self.spatial_fusion = nn.Sequential(
                    nn.Conv2d(64 * len(channels), 256, 1, bias=False),
                    nn.GroupNorm(16, 256),
                    nn.SiLU(),
                    ResidualBlock(256),
                    ResidualBlock(256),
                    ResidualBlock(256),
                )
            else:
                self.encoder = nn.Sequential(*stages)
                self.encoder_stages = None
                self.scale_projections = None
                self.spatial_fusion = None
            self.metadata = nn.Sequential(
                nn.Linear(len(METADATA_FIELDS) * 2, 256),
                nn.SiLU(),
                nn.Linear(256, 512),
            )
            self.context = nn.ModuleList(
                [
                    nn.TransformerEncoderLayer(
                        d_model=256,
                        nhead=8,
                        dim_feedforward=768,
                        dropout=0.1,
                        activation="gelu",
                        batch_first=True,
                        norm_first=True,
                    )
                    for _ in range(2)
                ]
            )
            self.plane_embedding = nn.Embedding(PLANE_COUNT, 64)
            self.key_head = nn.Sequential(
                nn.Linear(320, 256),
                nn.SiLU(),
                nn.Dropout(0.1),
                nn.Linear(256, POLYNOMIAL_COUNT * OUTPUT_COUNT),
            )
            self.uncertainty_head = nn.Sequential(
                nn.Linear(256, 256),
                nn.SiLU(),
                nn.Linear(256, PLANE_COUNT * POLYNOMIAL_COUNT * OUTPUT_COUNT),
            )
            self.gtc_head = nn.Sequential(nn.Linear(256, 384), nn.SiLU(), nn.Linear(384, 516))
            self.light_head = nn.Sequential(
                nn.Conv2d(256, 128, 3, padding=1),
                nn.SiLU(),
                nn.Conv2d(128, 2, 1),
            )
            self.scalar_head = nn.Sequential(
                nn.Linear(256, 128), nn.SiLU(), nn.Linear(128, len(STYLE_SCALAR_FIELDS))
            )
            self.unstyled_head = nn.Sequential(
                nn.Conv2d(256, 96, 3, padding=1),
                nn.SiLU(),
                nn.Conv2d(96, 3, 1),
                nn.Sigmoid(),
            )
            self.task_log_variances = nn.Parameter(torch.zeros(6))
            nn.init.zeros_(self.key_head[-1].weight)
            nn.init.zeros_(self.key_head[-1].bias)
            for name, shape in (
                ("identity", identity_key1()[None].shape),
                ("key1_scale", np.asarray(statistics["key1Scale"])[None, None, None].shape),
            ):
                value = identity_key1()[None] if name == "identity" else np.asarray(
                    statistics["key1Scale"], dtype=np.float32
                )[None, None, None]
                self.register_buffer(name, torch.from_numpy(value.astype(np.float32)))
            for name, key in (
                ("metadata_center", "metadataCenter"),
                ("metadata_scale", "metadataScale"),
                ("gtc_center", "gtcCenter"),
                ("gtc_scale", "gtcScale"),
                ("light_center", "lightCenter"),
                ("light_scale", "lightScale"),
                ("scalar_center", "scalarCenter"),
                ("scalar_scale", "scalarScale"),
            ):
                self.register_buffer(
                    name,
                    torch.from_numpy(np.asarray(statistics[key], dtype=np.float32)),
                )
            self.register_buffer(
                "metadata_active",
                torch.from_numpy(
                    np.asarray(
                        statistics.get(
                            "metadataActive",
                            np.ones(len(METADATA_FIELDS), dtype=np.float32),
                        ),
                        dtype=np.float32,
                    )
                ),
                persistent=False,
            )
            for name, fallback in (
                ("scalar_low", np.full(len(STYLE_SCALAR_FIELDS), -np.inf)),
                ("scalar_high", np.full(len(STYLE_SCALAR_FIELDS), np.inf)),
            ):
                key = "scalarLow" if name == "scalar_low" else "scalarHigh"
                self.register_buffer(
                    name,
                    torch.from_numpy(
                        np.asarray(statistics.get(key, fallback), dtype=np.float32)
                    ),
                    persistent=False,
                )

        def forward(
            self, primary: Any, metadata: Any, metadata_mask: Any
        ) -> dict[str, Any]:
            if self.encoder_stages is not None:
                value = primary
                pyramid = []
                for stage, projection in zip(
                    self.encoder_stages, self.scale_projections
                ):
                    value = stage(value)
                    pyramid.append(
                        torch.nn.functional.interpolate(
                            projection(value),
                            size=(GRID_LONG, GRID_LONG),
                            mode="bilinear",
                            align_corners=False,
                        )
                    )
                spatial = self.spatial_fusion(torch.cat(pyramid, dim=1))
            else:
                spatial = self.encoder(primary)
                # MPS does not implement adaptive pooling when the input size
                # is not divisible by the output size (16 -> 12 here).
                spatial = torch.nn.functional.interpolate(
                    spatial,
                    size=(GRID_LONG, GRID_LONG),
                    mode="bilinear",
                    align_corners=False,
                )
            active_mask = metadata_mask * self.metadata_active
            normalized_metadata = (metadata - self.metadata_center) / self.metadata_scale
            normalized_metadata = normalized_metadata.clamp(-8.0, 8.0) * active_mask
            gamma, beta = self.metadata(
                torch.cat((normalized_metadata, active_mask), dim=-1)
            ).chunk(2, dim=-1)
            spatial = spatial * (1.0 + gamma[:, :, None, None])
            spatial = spatial + beta[:, :, None, None]
            tokens = spatial.flatten(2).transpose(1, 2)
            for block in self.context:
                tokens = block(tokens)
            spatial = tokens.transpose(1, 2).reshape(
                primary.shape[0], 256, GRID_LONG, GRID_LONG
            )
            global_feature = tokens.mean(dim=1)
            node = spatial.permute(0, 2, 3, 1)[:, :, :, None, :].expand(
                -1, -1, -1, PLANE_COUNT, -1
            )
            plane = self.plane_embedding.weight.reshape(1, 1, 1, PLANE_COUNT, 64)
            plane = plane.expand(primary.shape[0], GRID_LONG, GRID_LONG, -1, -1)
            key_residual = self.key_head(torch.cat((node, plane), dim=-1)).reshape(
                primary.shape[0],
                GRID_LONG,
                GRID_LONG,
                PLANE_COUNT,
                POLYNOMIAL_COUNT,
                OUTPUT_COUNT,
            )
            key1 = self.identity + key_residual * self.key1_scale
            log_variance = self.uncertainty_head(global_feature).reshape(
                primary.shape[0], PLANE_COUNT, POLYNOMIAL_COUNT, OUTPUT_COUNT
            ).clamp(-8.0, 6.0)
            gtc = self.gtc_center + self.gtc_head(global_feature) * self.gtc_scale
            light_residual = self.light_head(
                torch.nn.functional.interpolate(
                    spatial, size=(32, 32), mode="bilinear", align_corners=False
                )
            )
            light = self.light_center[None] + light_residual * self.light_scale[None, :, None, None]
            scalars = self.scalar_center + self.scalar_head(global_feature) * self.scalar_scale
            scalars = torch.maximum(
                torch.minimum(scalars, self.scalar_high), self.scalar_low
            )
            unstyled = self.unstyled_head(
                torch.nn.functional.interpolate(
                    spatial, size=(64, 64), mode="bilinear", align_corners=False
                )
            )
            return {
                "key1": key1,
                "key1LogVariance": log_variance,
                "gtc": gtc,
                "lightMaps": light,
                "scalars": scalars,
                "unstyled": unstyled,
            }

    return UniversalPhotographicStyleStateNet()


class _UniversalDataset:
    def __init__(
        self,
        manifest: Path,
        records: Sequence[Mapping[str, Any]],
        metadata_dropout: float = 0.0,
    ) -> None:
        self.manifest = manifest
        self.records = list(records)
        self.labels = np.load(manifest.parent / "labels.npz", allow_pickle=False)
        self.metadata_dropout = metadata_dropout

    def __len__(self) -> int:
        return len(self.records)

    def __getitem__(self, item: int) -> tuple[Any, ...]:
        torch, _ = _require_torch()
        record = self.records[item]
        index = int(record["index"])
        with np.load(str(record["samplePath"]), allow_pickle=False) as sample:
            images = np.asarray(sample["images"], dtype=np.uint8)
            key1 = np.asarray(sample["key1"], dtype=np.float32)
            key_mask = np.asarray(sample["mask"], dtype=np.bool_)
        primary = primary_image_features(images[0])
        unstyled = images[1, :, ::4, ::4].astype(np.float32) / 255.0
        metadata = self.labels["metadata"][index].astype(np.float32)
        metadata_mask = self.labels["metadata_mask"][index].astype(np.float32)
        if self.metadata_dropout and float(torch.rand(())) < self.metadata_dropout:
            metadata = np.zeros_like(metadata)
            metadata_mask = np.zeros_like(metadata_mask)
        return (
            torch.from_numpy(primary),
            torch.from_numpy(metadata),
            torch.from_numpy(metadata_mask),
            torch.from_numpy(key1),
            torch.from_numpy(key_mask),
            torch.from_numpy(self.labels["gtc"][index].astype(np.float32) / 255.0),
            torch.from_numpy(self.labels["light_maps"][index].astype(np.float32)),
            torch.from_numpy(self.labels["scalars"][index].astype(np.float32)),
            torch.from_numpy(self.labels["scalar_mask"][index].astype(np.bool_)),
            torch.from_numpy(unstyled),
            str(record["captureSession"]),
            str(record.get("Model") or "unknown"),
        )


def _consumer_quadratic_proxy(torch: Any, key1: Any, primary: Any) -> Any:
    """Differentiable global quadratic proxy for the native key1 consumer.

    This is deliberately a training regularizer, not a runtime renderer: it
    averages the spatial/directional lattice and applies the verified ten-term
    encoded-RGB basis to the paired styled thumbnail.
    """
    rgb = primary[:, :3, ::4, ::4]
    red, green, blue = rgb[:, 0], rgb[:, 1], rgb[:, 2]
    terms = torch.stack(
        (torch.ones_like(red), red, green, blue, red.square(), red * green,
         red * blue, green.square(), green * blue, blue.square()), dim=1
    )
    coefficients = key1.mean(dim=(1, 2, 3))
    return torch.einsum("bthw,btc->bchw", terms, coefficients).clamp(0.0, 1.0)


def _losses(
    torch: Any,
    model: Any,
    output: Mapping[str, Any],
    batch: Sequence[Any],
    consumer_weight: float = 0.0,
) -> tuple[Any, dict[str, float]]:
    (
        _primary,
        _metadata,
        _metadata_mask,
        key1,
        key_mask,
        gtc,
        light,
        scalars,
        scalar_mask,
        unstyled,
        _sessions,
        _models,
    ) = batch
    expanded = key_mask[:, :, :, None, None, None].expand_as(key1)
    normalized_key = (output["key1"] - key1) / model.key1_scale
    key_loss = torch.nn.functional.huber_loss(
        normalized_key[expanded], torch.zeros_like(normalized_key[expanded]), delta=1.0
    )
    normalized_gtc = (output["gtc"] - gtc) / model.gtc_scale
    gtc_loss = torch.nn.functional.huber_loss(normalized_gtc, torch.zeros_like(normalized_gtc))
    normalized_light = (output["lightMaps"] - light) / model.light_scale[None, :, None, None]
    light_loss = torch.nn.functional.huber_loss(
        normalized_light, torch.zeros_like(normalized_light)
    )
    normalized_scalars = (output["scalars"] - scalars) / model.scalar_scale
    scalar_loss = torch.nn.functional.huber_loss(
        normalized_scalars[scalar_mask], torch.zeros_like(normalized_scalars[scalar_mask])
    )
    unstyled_loss = torch.nn.functional.l1_loss(output["unstyled"], unstyled)
    consumer_proxy = _consumer_quadratic_proxy(torch, output["key1"], _primary)
    consumer_loss = torch.nn.functional.smooth_l1_loss(consumer_proxy, unstyled)
    squared = normalized_key.square()
    spatial_mask = key_mask[:, :, :, None, None, None].expand_as(squared)
    numerator = (squared * spatial_mask).sum(dim=(1, 2))
    denominator = spatial_mask.sum(dim=(1, 2)).clamp_min(1)
    target_log_variance = torch.log(numerator / denominator + 1e-5).detach()
    uncertainty_loss = torch.nn.functional.smooth_l1_loss(
        output["key1LogVariance"], target_log_variance
    )
    values = (key_loss, gtc_loss, light_loss, scalar_loss, unstyled_loss, uncertainty_loss)
    task_noise = model.task_log_variances.clamp(-3.0, 3.0)
    total = sum(torch.exp(-task_noise[i]) * loss + task_noise[i] for i, loss in enumerate(values))
    total = total + float(consumer_weight) * consumer_loss
    return total, {
        "key1": float(key_loss.detach().cpu()),
        "gtc": float(gtc_loss.detach().cpu()),
        "lightMaps": float(light_loss.detach().cpu()),
        "scalars": float(scalar_loss.detach().cpu()),
        "unstyled": float(unstyled_loss.detach().cpu()),
        "uncertainty": float(uncertainty_loss.detach().cpu()),
        "consumerProxy": float(consumer_loss.detach().cpu()),
    }


def _move_batch(batch: Sequence[Any], device: str) -> tuple[Any, ...]:
    return tuple(value.to(device) if hasattr(value, "to") else value for value in batch)


def _evaluate(torch: Any, model: Any, loader: Any, device: str) -> dict[str, Any]:
    model.eval()
    key_errors: list[float] = []
    predicted_uncertainty: list[float] = []
    by_model: dict[str, list[float]] = defaultdict(list)
    gtc_errors: list[float] = []
    light_errors: list[float] = []
    scalar_errors: list[float] = []
    unstyled_errors: list[float] = []
    consumer_proxy_errors: list[float] = []
    with torch.no_grad():
        for original in loader:
            batch = _move_batch(original, device)
            output = model(batch[0], batch[1], batch[2])
            consumer_proxy = _consumer_quadratic_proxy(torch, output["key1"], batch[0])
            consumer_delta = consumer_proxy - batch[9]
            consumer_proxy_errors.extend(
                consumer_delta.abs().mean(dim=(1, 2, 3)).cpu().tolist()
            )
            key1, key_mask = batch[3], batch[4]
            normalized = ((output["key1"] - key1) / model.key1_scale).abs()
            for index, model_name in enumerate(original[-1]):
                selected = key_mask[index, :, :, None, None, None].expand_as(normalized[index])
                error = float(normalized[index][selected].mean().cpu())
                key_errors.append(error)
                by_model[str(model_name)].append(error)
                predicted_uncertainty.append(
                    float(output["key1LogVariance"][index].exp().mean().cpu())
                )
            gtc_errors.extend(
                ((output["gtc"] - batch[5]) / model.gtc_scale).abs().mean(dim=1).cpu().tolist()
            )
            light_errors.extend(
                ((output["lightMaps"] - batch[6]) / model.light_scale[None, :, None, None])
                .abs().mean(dim=(1, 2, 3)).cpu().tolist()
            )
            normalized_scalars = ((output["scalars"] - batch[7]) / model.scalar_scale).abs()
            for index in range(len(normalized_scalars)):
                scalar_errors.append(float(normalized_scalars[index][batch[8][index]].mean().cpu()))
            unstyled_errors.extend(
                (output["unstyled"] - batch[9]).abs().mean(dim=(1, 2, 3)).cpu().tolist()
            )
    if not key_errors:
        raise ReverseKey1Error("universal evaluation split is empty")
    uncertainty_correlation = 0.0
    if np.std(predicted_uncertainty) > 0 and np.std(key_errors) > 0:
        uncertainty_correlation = float(np.corrcoef(predicted_uncertainty, key_errors)[0, 1])
    return {
        "key1NormalizedMAE": float(np.mean(key_errors)),
        "key1P95SampleMAE": float(np.quantile(key_errors, 0.95)),
        "key1PerModelNormalizedMAE": {
            name: float(np.mean(values)) for name, values in sorted(by_model.items())
        },
        "gtcNormalizedMAE": float(np.mean(gtc_errors)),
        "lightMapsNormalizedMAE": float(np.mean(light_errors)),
        "scalarsNormalizedMAE": float(np.mean(scalar_errors)),
        "unstyledMAE": float(np.mean(unstyled_errors)),
        "consumerProxyMAE": float(np.mean(consumer_proxy_errors)),
        "consumerProxyRMSE8": float(
            np.sqrt(np.mean(np.asarray(consumer_proxy_errors) ** 2)) * 255.0
        ),
        "uncertaintyErrorCorrelation": uncertainty_correlation,
    }


@dataclasses.dataclass(frozen=True)
class UniversalTrainingConfig:
    manifest: Path
    output: Path
    epochs: int = 40
    batch_size: int = 6
    learning_rate: float = 2e-4
    device: str = "auto"
    seed: int = 260820
    metadata_dropout: float = 0.25
    architecture: str = "base"
    consumer_weight: float = 0.0
    resume: Path | None = None


def train_universal_model(config: UniversalTrainingConfig) -> dict[str, Any]:
    torch, _ = _require_torch()
    if config.device == "auto":
        device = "mps" if torch.backends.mps.is_available() else "cpu"
    else:
        device = config.device
    if device == "mps" and not torch.backends.mps.is_available():
        raise ReverseKey1Error("MPS was requested but is unavailable")
    if not 0 <= config.metadata_dropout <= 1:
        raise ReverseKey1Error("metadata dropout must be in [0, 1]")
    if config.consumer_weight < 0:
        raise ReverseKey1Error("consumer weight must be non-negative")
    if config.architecture not in {"base", "multiscale_large"}:
        raise ReverseKey1Error(
            f"unsupported universal architecture: {config.architecture}"
        )
    torch.manual_seed(config.seed)
    np.random.seed(config.seed)
    manifest = config.manifest.resolve()
    header, records = load_universal_manifest(manifest)
    by_split = {
        split: [record for record in records if record["split"] == split]
        for split in ("train", "calibration", "heldout")
    }
    if any(not values for values in by_split.values()):
        raise ReverseKey1Error("universal train/calibration/heldout splits must be non-empty")
    statistics = universal_state_statistics(manifest, records)
    model = build_universal_model(
        statistics, architecture=config.architecture
    ).to(device)
    if config.resume is not None:
        checkpoint = torch.load(config.resume.resolve(), map_location=device, weights_only=False)
        if checkpoint.get("manifestSHA256") != sha256_file(manifest):
            raise ReverseKey1Error("resume checkpoint manifest hash does not match dataset")
        model.load_state_dict(checkpoint["model"])
    loaders = {
        split: torch.utils.data.DataLoader(
            _UniversalDataset(
                manifest,
                values,
                metadata_dropout=config.metadata_dropout if split == "train" else 0.0,
            ),
            batch_size=config.batch_size,
            shuffle=split == "train",
            num_workers=0,
        )
        for split, values in by_split.items()
    }
    optimizer = torch.optim.AdamW(model.parameters(), lr=config.learning_rate, weight_decay=1e-4)
    output = config.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    best_score = float("inf")
    best_epoch = 0
    history = []
    manifest_hash = sha256_file(manifest)
    for epoch in range(1, config.epochs + 1):
        model.train()
        totals = []
        components: dict[str, list[float]] = defaultdict(list)
        for original in loaders["train"]:
            batch = _move_batch(original, device)
            optimizer.zero_grad(set_to_none=True)
            prediction = model(batch[0], batch[1], batch[2])
            total, details = _losses(
                torch, model, prediction, batch, consumer_weight=config.consumer_weight
            )
            total.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 2.0)
            optimizer.step()
            totals.append(float(total.detach().cpu()))
            for name, value in details.items():
                components[name].append(value)
        calibration = _evaluate(torch, model, loaders["calibration"], device)
        score = calibration["key1NormalizedMAE"] + 0.15 * (
            calibration["gtcNormalizedMAE"] + calibration["lightMapsNormalizedMAE"]
        )
        row = {
            "epoch": epoch,
            "trainingLoss": float(np.mean(totals)),
            "trainingComponents": {
                name: float(np.mean(values)) for name, values in components.items()
            },
            "calibration": calibration,
            "selectionScore": score,
        }
        history.append(row)
        checkpoint = {
            "schema": REPORT_SCHEMA,
            "architecture": (
                "UniversalPhotographicStyleStateNet-v2-multiscale-large"
                if config.architecture == "multiscale_large"
                else "UniversalPhotographicStyleStateNet-v1"
            ),
            "architectureConfig": config.architecture,
            "epoch": epoch,
            "manifestSHA256": manifest_hash,
            "model": model.state_dict(),
            "statistics": {name: value.tolist() for name, value in statistics.items()},
            "metadataFields": list(METADATA_FIELDS),
            "styleScalarFields": list(STYLE_SCALAR_FIELDS),
        }
        torch.save(checkpoint, output / "last.pt")
        if score < best_score:
            best_score = score
            best_epoch = epoch
            torch.save(checkpoint, output / "best.pt")
        _atomic_json(output / "history.json", history)
        print(json.dumps(row, sort_keys=True), flush=True)
    best = torch.load(output / "best.pt", map_location=device, weights_only=False)
    model.load_state_dict(best["model"])
    heldout = _evaluate(torch, model, loaders["heldout"], device)
    calibration = _evaluate(torch, model, loaders["calibration"], device)
    report = {
        "schema": REPORT_SCHEMA,
        "architecture": best["architecture"],
        "architectureConfig": config.architecture,
        "device": device,
        "manifestSHA256": manifest_hash,
        "sourceCorpusSHA256": header["sourceCorpusSHA256"],
        "splitCounts": {name: len(values) for name, values in by_split.items()},
        "bestEpoch": best_epoch,
        "bestSelectionScore": best_score,
        "calibration": calibration,
        "heldout": heldout,
        "parameterCount": sum(parameter.numel() for parameter in model.parameters()),
        "inputContract": header["inputContract"],
        "outputContract": header["outputContract"],
        "claimBoundary": (
            "Native iPhone session holdout only; arbitrary-device quality and Photos behavior "
            "require separate acceptance."
        ),
        "consumerWeight": config.consumer_weight,
        "resume": str(config.resume.resolve()) if config.resume else None,
    }
    _atomic_json(output / "report.json", report)
    return report
