"""Inference adapter for the universal Photographic Style state model."""

from __future__ import annotations

import io
import json
import math
import mmap
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Mapping

import numpy as np
from PIL import Image, ImageOps

from xdremux_py.apple_reverse_key1_training import (
    GRID_LONG,
    GRID_SHORT,
    ReverseKey1Error,
    _fit_rgb,
    _require_torch,
    encode_key1,
    sha256_file,
)
from xdremux_py.universal_photographic_style_training import (
    GAIN_MAP_CHANNELS,
    INPUT_SIZE,
    LINEAR_CHANNELS,
    METADATA_FIELDS,
    MODALITY_FIELDS,
    STYLE_SCALAR_FIELDS,
    build_universal_model,
    gain_map_sidecar_features,
    linear_sidecar_features,
    metadata_vector,
    primary_image_features,
)


SUPPORTED_IMAGE_SUFFIXES = frozenset(
    {
        ".avif",
        ".dng",
        ".heic",
        ".heif",
        ".jpeg",
        ".jpg",
        ".png",
        ".tif",
        ".tiff",
        ".webp",
    }
)
GAIN_MAP_MARKERS = (
    b"urn:iso:std:iso:ts:21496:-1",
    b"hdrgm",
    b"local.uhdr.gainmap.data",
    b"hdr_gain_map",
)


@dataclass(frozen=True)
class UniversalImageInput:
    path: Path
    primary: np.ndarray
    metadata: np.ndarray
    metadata_mask: np.ndarray
    display_width: int
    display_height: int
    has_raw: bool
    has_gain_map: bool
    source_sha256: str
    source_make: str | None = None
    source_model: str | None = None
    linear_rgb_features: np.ndarray | None = None
    gain_map_features: np.ndarray | None = None


def _register_heif() -> None:
    try:
        import pillow_heif
    except ImportError:
        return
    pillow_heif.register_heif_opener()


def _dng_preview(path: Path, exiftool: str) -> bytes:
    result = subprocess.run(
        [exiftool, "-b", "-PreviewImage", str(path)],
        capture_output=True,
        check=False,
        timeout=120,
    )
    if result.returncode or not result.stdout:
        raise ReverseKey1Error(
            f"DNG embedded preview extraction failed: {result.stderr.decode(errors='replace')[-600:]}"
        )
    return result.stdout


def _contains_gain_map(path: Path) -> bool:
    if not path.is_file() or path.stat().st_size == 0:
        return False
    with path.open("rb") as source:
        with mmap.mmap(source.fileno(), 0, access=mmap.ACCESS_READ) as data:
            return any(data.find(marker) >= 0 for marker in GAIN_MAP_MARKERS)


def _image_and_size(path: Path, exiftool: str) -> tuple[np.ndarray, int, int, bool]:
    _register_heif()
    suffix = path.suffix.lower()
    source: Any = io.BytesIO(_dng_preview(path, exiftool)) if suffix == ".dng" else path
    try:
        with Image.open(source) as image:
            display = ImageOps.exif_transpose(image)
            width, height = display.size
            has_alpha = "A" in display.getbands()
            return _fit_rgb(display), width, height, has_alpha
    except Exception as error:
        raise ReverseKey1Error(f"unsupported or unreadable image {path}: {error}") from error


def _metadata_tags(path: Path, exiftool: str) -> dict[str, Any]:
    fields = (
        "ExposureTime",
        "FNumber",
        "ISO",
        "FocalLength",
        "ColorTemperature",
        "HDRGain",
        "Orientation",
        "Software",
        "Tag0",
        "BitsPerSample",
        "Make",
        "Model",
    )
    result = subprocess.run(
        [exiftool, "-j", "-n", *[f"-{field}" for field in fields], str(path)],
        capture_output=True,
        text=True,
        check=False,
        timeout=120,
    )
    if result.returncode:
        raise ReverseKey1Error(f"image metadata extraction failed: {result.stderr[-600:]}")
    values = json.loads(result.stdout)
    if not isinstance(values, list) or len(values) != 1:
        raise ReverseKey1Error("image metadata extraction did not return one record")
    return values[0]


def load_universal_image(
    path: Path,
    *,
    exiftool: str = "exiftool",
    linear_rgb_sidecar: Path | None = None,
    gain_map_sidecar: Path | None = None,
) -> UniversalImageInput:
    resolved = path.resolve()
    if resolved.suffix.lower() not in SUPPORTED_IMAGE_SUFFIXES:
        raise ReverseKey1Error(f"unsupported image suffix: {resolved.suffix}")
    styled, width, height, has_alpha = _image_and_size(resolved, exiftool)
    tags = _metadata_tags(resolved, exiftool)
    bit_depth = tags.get("BitsPerSample")
    if isinstance(bit_depth, list):
        bit_depth = max((float(value) for value in bit_depth), default=None)
    elif not isinstance(bit_depth, (int, float)):
        bit_depth = None
    linear_features = (
        linear_sidecar_features(linear_rgb_sidecar.resolve())
        if linear_rgb_sidecar is not None
        else None
    )
    gain_features = (
        gain_map_sidecar_features(gain_map_sidecar.resolve())
        if gain_map_sidecar is not None
        else None
    )
    has_raw = resolved.suffix.lower() == ".dng" or linear_features is not None
    has_gain_map = _contains_gain_map(resolved) or gain_features is not None
    metadata, metadata_mask = metadata_vector(
        {
            "displayWidth": width,
            "displayHeight": height,
            "Orientation": tags.get("Orientation"),
            "Software": tags.get("Software"),
            "Tag0": tags.get("Tag0"),
        },
        tags,
        has_gain_map=has_gain_map,
        has_raw=has_raw,
        input_bit_depth=bit_depth,
        has_alpha=has_alpha,
    )
    return UniversalImageInput(
        path=resolved,
        primary=primary_image_features(styled),
        metadata=metadata,
        metadata_mask=metadata_mask,
        display_width=width,
        display_height=height,
        has_raw=has_raw,
        has_gain_map=has_gain_map,
        source_sha256=sha256_file(resolved),
        source_make=str(tags.get("Make")) if tags.get("Make") is not None else None,
        source_model=str(tags.get("Model")) if tags.get("Model") is not None else None,
        linear_rgb_features=linear_features,
        gain_map_features=gain_features,
    )


def load_universal_model(checkpoint_path: Path, device: str = "auto") -> tuple[Any, dict[str, Any], str]:
    torch, _ = _require_torch()
    selected = "mps" if device == "auto" and torch.backends.mps.is_available() else device
    if selected == "auto":
        selected = "cpu"
    if selected == "mps" and not torch.backends.mps.is_available():
        raise ReverseKey1Error("MPS was requested but is unavailable")
    checkpoint = torch.load(checkpoint_path.resolve(), map_location="cpu", weights_only=False)
    statistics = {
        name: np.asarray(value, dtype=np.float32)
        for name, value in checkpoint["statistics"].items()
    }
    architecture = str(checkpoint.get("architectureConfig") or "base")
    model = build_universal_model(statistics, architecture=architecture)
    model.load_state_dict(checkpoint["model"])
    model.to(selected).eval()
    return model, checkpoint, selected


def predict_universal_state(
    image: UniversalImageInput,
    model: Any,
    *,
    device: str,
) -> tuple[dict[str, np.ndarray], float]:
    torch, _ = _require_torch()
    primary = torch.from_numpy(image.primary).unsqueeze(0).to(device)
    metadata = torch.from_numpy(image.metadata).unsqueeze(0).to(device)
    metadata_mask = torch.from_numpy(image.metadata_mask).unsqueeze(0).to(device)
    linear_rgb = np.zeros(
        (LINEAR_CHANNELS, INPUT_SIZE, INPUT_SIZE), dtype=np.float32
    )
    gain_map = np.zeros(
        (GAIN_MAP_CHANNELS, INPUT_SIZE, INPUT_SIZE), dtype=np.float32
    )
    modality_mask = np.zeros(len(MODALITY_FIELDS), dtype=np.float32)
    if image.linear_rgb_features is not None:
        linear_rgb = image.linear_rgb_features
        modality_mask[0] = 1.0
    if image.gain_map_features is not None:
        gain_map = image.gain_map_features
        modality_mask[1] = 1.0
    linear_tensor = torch.from_numpy(linear_rgb).unsqueeze(0).to(device)
    gain_tensor = torch.from_numpy(gain_map).unsqueeze(0).to(device)
    modality_tensor = torch.from_numpy(modality_mask).unsqueeze(0).to(device)
    if device == "mps":
        torch.mps.synchronize()
    started = time.perf_counter()
    with torch.no_grad():
        predicted = model(
            primary,
            metadata,
            metadata_mask,
            linear_tensor,
            gain_tensor,
            modality_tensor,
        )
    if device == "mps":
        torch.mps.synchronize()
    elapsed = time.perf_counter() - started
    result = {
        name: value.detach().cpu().numpy()[0].astype(np.float32)
        for name, value in predicted.items()
    }
    if any(not np.isfinite(value).all() for value in result.values()):
        raise ReverseKey1Error("universal model produced non-finite state")
    return result, elapsed


def native_state_resources(
    image: UniversalImageInput,
    prediction: Mapping[str, np.ndarray],
) -> dict[str, Any]:
    key1 = np.asarray(prediction["key1"], dtype=np.float32)
    gtc = np.asarray(prediction["gtc"], dtype=np.float32)
    light = np.asarray(prediction["lightMaps"], dtype=np.float32)
    scalars = np.asarray(prediction["scalars"], dtype=np.float32)
    if gtc.shape != (516,) or light.shape != (2, 32, 32):
        raise ReverseKey1Error("universal state output shape is invalid")
    if scalars.shape != (len(STYLE_SCALAR_FIELDS),):
        raise ReverseKey1Error("universal scalar output shape is invalid")
    if any(not np.isfinite(value).all() for value in (key1, gtc, light, scalars)):
        raise ReverseKey1Error("universal state resource contains non-finite values")
    landscape = image.display_width >= image.display_height
    key1_bytes = encode_key1(
        key1,
        width_slots=GRID_LONG if landscape else GRID_SHORT,
        height_slots=GRID_SHORT if landscape else GRID_LONG,
    )
    gtc_bytes = np.rint(np.clip(gtc, 0.0, 1.0) * 255.0).astype(np.uint8).tobytes()
    c_bytes = light[0].astype("<f2").tobytes()
    d_bytes = light[1].astype("<f2").tobytes()
    uncertainty = float(np.exp(np.asarray(prediction["key1LogVariance"])).mean())
    if not math.isfinite(uncertainty):
        raise ReverseKey1Error("universal state uncertainty is non-finite")
    return {
        "key1": key1_bytes,
        "gtc": gtc_bytes,
        "c": c_bytes,
        "d": d_bytes,
        "scalars": {
            name: float(value) for name, value in zip(STYLE_SCALAR_FIELDS, scalars)
        },
        "uncertainty": uncertainty,
    }
