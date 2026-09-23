"""License-metadata-filtered public collection and synthetic-only pretraining."""

from __future__ import annotations

import copy
import dataclasses
import hashlib
import html
import json
import io
import math
import os
import tempfile
import random
import re
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any, Mapping, Sequence

import numpy as np
from PIL import Image, ImageOps

from xdremux_py.apple_reverse_key1_training import (
    ReverseKey1Error,
    _atomic_json,
    _require_torch,
    sha256_file,
)
from research.oppo_styles.training import (
    INPUT_SIZE,
    METADATA_FIELDS,
    STYLE_SCALAR_FIELDS,
    build_universal_model,
    primary_image_features,
    normalized_primary_rgb,
)


PUBLIC_CORPUS_SCHEMA = "xdremux-public-style-content-corpus-v1"
SYNTHETIC_REPORT_SCHEMA = "xdremux-public-synthetic-style-pretraining-v2"
COMMONS_API = "https://commons.wikimedia.org/w/api.php"
COMMONS_USER_AGENT = (
    "XDRemuxStyleResearchBot/1.1 "
    "(https://github.com/21Z121Z1/XDRemux; contact via GitHub Issues)"
)
DEFAULT_CATEGORIES = (
    "Taken with iPhone 16",
    "Taken with iPhone 16 Pro",
    "Taken with iPhone 17 Pro",
    "Taken with Oppo A3x 4G",
    "Taken with Oppo F29 5G",
)
CURATED_GITHUB_SAMPLES: tuple[dict[str, str], ...] = (
    {
        "category": "scikit-image public-domain photos",
        "title": "astronaut.png",
        "downloadURL": "https://raw.githubusercontent.com/scikit-image/scikit-image/v0.24.0/skimage/data/astronaut.png",
        "sourceURL": "https://github.com/scikit-image/scikit-image/blob/v0.24.0/skimage/data/astronaut.png",
        "sourceGitBlobSHA1": "834cda0012478c5edc8d43bade96d315dedeaab4",
        "license": "Public domain",
        "licenseURL": "https://scikit-image.org/docs/0.24.x/api/skimage.data.html#skimage.data.astronaut",
        "artist": "NASA",
        "credit": "NASA Great Images",
    },
    {
        "category": "scikit-image CC0 photos",
        "title": "brick.png",
        "downloadURL": "https://raw.githubusercontent.com/scikit-image/scikit-image/v0.24.0/skimage/data/brick.png",
        "sourceURL": "https://github.com/scikit-image/scikit-image/blob/v0.24.0/skimage/data/brick.png",
        "sourceGitBlobSHA1": "79941ecd6605663c653e03c85b4ca7e59b3c4412",
        "license": "CC0",
        "licenseURL": "https://scikit-image.org/docs/0.24.x/api/skimage.data.html#skimage.data.brick",
        "artist": "CC0Textures",
        "credit": "scikit-image data",
    },
    {
        "category": "scikit-image CC0 photos",
        "title": "camera.png",
        "downloadURL": "https://raw.githubusercontent.com/scikit-image/scikit-image/v0.24.0/skimage/data/camera.png",
        "sourceURL": "https://github.com/scikit-image/scikit-image/blob/v0.24.0/skimage/data/camera.png",
        "sourceGitBlobSHA1": "cdb3405d0086639638a13a2c72bfb646060180e2",
        "license": "CC0",
        "licenseURL": "https://scikit-image.org/docs/0.24.x/api/skimage.data.html#skimage.data.camera",
        "artist": "Lav Varshney",
        "credit": "scikit-image data",
    },
    {
        "category": "scikit-image CC0 photos",
        "title": "chelsea.png",
        "downloadURL": "https://raw.githubusercontent.com/scikit-image/scikit-image/v0.24.0/skimage/data/chelsea.png",
        "sourceURL": "https://github.com/scikit-image/scikit-image/blob/v0.24.0/skimage/data/chelsea.png",
        "sourceGitBlobSHA1": "fae212d2befa8926f05d7cf06c799f06b81545ab",
        "license": "CC0",
        "licenseURL": "https://scikit-image.org/docs/0.24.x/api/skimage.data.html#skimage.data.chelsea",
        "artist": "Stefan van der Walt",
        "credit": "scikit-image data",
    },
    {
        "category": "scikit-image CC0 photos",
        "title": "coffee.png",
        "downloadURL": "https://raw.githubusercontent.com/scikit-image/scikit-image/v0.24.0/skimage/data/coffee.png",
        "sourceURL": "https://github.com/scikit-image/scikit-image/blob/v0.24.0/skimage/data/coffee.png",
        "sourceGitBlobSHA1": "f8350bf7e6e22734667f8c86fd230a741370fbcb",
        "license": "CC0",
        "licenseURL": "https://scikit-image.org/docs/0.24.x/api/skimage.data.html#skimage.data.coffee",
        "artist": "Rachel Michetti",
        "credit": "Pikolo Espresso Bar / scikit-image data",
    },
    {
        "category": "scikit-image public-domain photos",
        "title": "hubble_deep_field.jpg",
        "downloadURL": "https://raw.githubusercontent.com/scikit-image/scikit-image/v0.24.0/skimage/data/hubble_deep_field.jpg",
        "sourceURL": "https://github.com/scikit-image/scikit-image/blob/v0.24.0/skimage/data/hubble_deep_field.jpg",
        "sourceGitBlobSHA1": "50c1b81aea1d565d78741387b5279bbed0ba4ce8",
        "license": "Public domain",
        "licenseURL": "https://scikit-image.org/docs/0.24.x/api/skimage.data.html#skimage.data.hubble_deep_field",
        "artist": "NASA",
        "credit": "HubbleSite / scikit-image data",
    },
    {
        "category": "scikit-image public-domain photos",
        "title": "rocket.jpg",
        "downloadURL": "https://raw.githubusercontent.com/scikit-image/scikit-image/v0.24.0/skimage/data/rocket.jpg",
        "sourceURL": "https://github.com/scikit-image/scikit-image/blob/v0.24.0/skimage/data/rocket.jpg",
        "sourceGitBlobSHA1": "32bdd33960c60d012fc71b5fd80a7ccc24d3b906",
        "license": "Public domain",
        "licenseURL": "https://scikit-image.org/docs/0.24.x/api/skimage.data.html#skimage.data.rocket",
        "artist": "SpaceX",
        "credit": "SpaceX Photos / scikit-image data",
    },
)
ALLOWED_LICENSE_POLICY = (
    "CC0 or Public Domain, or CC BY / CC BY-SA versions 1.0 through 4.0; "
    "NC and ND variants are rejected"
)


def license_is_allowed(value: str) -> bool:
    if value.casefold() in {"cc0", "public domain", "public-domain"}:
        return True
    return bool(re.fullmatch(r"CC BY(?:-SA)? (?:1\.0|2\.0|2\.5|3\.0|4\.0)", value))


def _metadata_value(metadata: Mapping[str, Any], name: str) -> str:
    value = metadata.get(name)
    if not isinstance(value, Mapping):
        return ""
    return html.unescape(str(value.get("value") or "")).strip()


def commons_candidates(
    payload: Mapping[str, Any], category: str, limit: int
) -> list[dict[str, Any]]:
    pages = payload.get("query", {}).get("pages", [])
    if not isinstance(pages, list):
        raise ReverseKey1Error("Wikimedia Commons response has no page array")
    selected: list[dict[str, Any]] = []
    for page in sorted(pages, key=lambda item: str(item.get("title") or "")):
        imageinfo = page.get("imageinfo")
        if not isinstance(imageinfo, list) or len(imageinfo) != 1:
            continue
        info = imageinfo[0]
        metadata = info.get("extmetadata") or {}
        license_name = _metadata_value(metadata, "LicenseShortName")
        mime = str(info.get("mime") or "")
        download_url = info.get("thumburl") or info.get("url")
        if (
            not license_is_allowed(license_name)
            or mime not in {"image/jpeg", "image/png", "image/webp"}
            or not isinstance(download_url, str)
        ):
            continue
        selected.append(
            {
                "category": category,
                "title": str(page.get("title") or ""),
                "sourceURL": str(info.get("descriptionurl") or ""),
                "downloadURL": download_url,
                "sourceSHA1": str(info.get("sha1") or ""),
                "license": license_name,
                "licenseURL": _metadata_value(metadata, "LicenseUrl"),
                "artist": _metadata_value(metadata, "Artist"),
                "credit": _metadata_value(metadata, "Credit"),
            }
        )
        if len(selected) >= limit:
            break
    return selected


def _commons_category(category: str, request_limit: int = 80) -> list[dict[str, Any]]:
    query = urllib.parse.urlencode(
        {
            "action": "query",
            "generator": "categorymembers",
            "gcmtitle": f"Category:{category}",
            "gcmtype": "file",
            "gcmlimit": str(request_limit),
            "prop": "imageinfo",
            "iiprop": "url|sha1|mime|extmetadata",
            "iiurlwidth": "768",
            "format": "json",
            "formatversion": "2",
        }
    )
    request = urllib.request.Request(
        f"{COMMONS_API}?{query}",
        headers={
            "User-Agent": COMMONS_USER_AGENT,
            "Api-User-Agent": COMMONS_USER_AGENT,
        },
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        return commons_candidates(json.load(response), category, request_limit)


def _download_sample(record: Mapping[str, Any], path: Path) -> dict[str, Any]:
    parsed = urllib.parse.urlsplit(str(record["downloadURL"]))
    if parsed.scheme != "https" or parsed.hostname not in {"raw.githubusercontent.com", "upload.wikimedia.org"}:
        raise ReverseKey1Error("public sample download must use the declared HTTPS source hosts")
    request = urllib.request.Request(
        str(record["downloadURL"]),
        headers={"User-Agent": COMMONS_USER_AGENT},
    )
    payload: bytes | None = None
    for attempt in range(3):
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                final = urllib.parse.urlsplit(response.geturl())
                if final.scheme != "https" or final.hostname not in {"raw.githubusercontent.com", "upload.wikimedia.org"}:
                    raise ReverseKey1Error("public sample redirected outside declared HTTPS source hosts")
                payload = response.read(24 * 1024 * 1024 + 1)
            break
        except urllib.error.HTTPError as error:
            if error.code != 429 or attempt == 2:
                raise
            retry_after = error.headers.get("Retry-After")
            delay = (
                float(retry_after)
                if retry_after and retry_after.isdigit()
                else 2 ** (attempt + 1)
            )
            time.sleep(min(delay, 30.0))
    if payload is None:
        raise ReverseKey1Error(
            f"public sample download returned no data: {record['title']}"
        )
    if len(payload) > 24 * 1024 * 1024:
        raise ReverseKey1Error(f"public sample exceeds 24 MiB: {record['title']}")
    expected_blob = record.get("sourceGitBlobSHA1")
    if expected_blob is not None:
        actual_blob = hashlib.sha1(b"blob " + str(len(payload)).encode() + b"\0" + payload).hexdigest()
        if actual_blob != expected_blob:
            raise ReverseKey1Error(f"pinned public image Git blob mismatch: {record['title']}")
    try:
        with Image.open(io.BytesIO(payload)) as source:
            image = ImageOps.fit(
                ImageOps.exif_transpose(source).convert("RGB"),
                (INPUT_SIZE, INPUT_SIZE),
                method=Image.Resampling.LANCZOS,
            )
            value = np.asarray(image, dtype=np.uint8).transpose(2, 0, 1)
    except Exception as error:
        raise ReverseKey1Error(
            f"public sample decode failed for {record['title']}: {error}"
        ) from error
    path.parent.mkdir(parents=True, exist_ok=True)
    # The corpus owns a new directory; existing tensors are never clobbered.
    with path.open("xb") as destination:
        np.save(destination, value, allow_pickle=False)
    result = dict(record)
    result["imagePath"] = str(path.resolve())
    result["downloadSHA256"] = hashlib.sha256(payload).hexdigest()
    result["tensorSHA256"] = sha256_file(path)
    return result


def collect_public_corpus(
    output: Path,
    image_directory: Path,
    *,
    categories: Sequence[str] = DEFAULT_CATEGORIES,
    per_category: int = 3,
    seed: int = 260829,
) -> dict[str, Any]:
    if per_category <= 0:
        raise ReverseKey1Error("per-category sample count must be positive")
    if output.exists() or output.is_symlink():
        raise FileExistsError(output)
    image_directory.resolve().mkdir(parents=True, exist_ok=False)
    rng = random.Random(seed)
    records: list[dict[str, Any]] = []
    failures: list[dict[str, str]] = []
    for candidate in CURATED_GITHUB_SAMPLES:
        path = image_directory.resolve() / f"{len(records):04d}.npy"
        try:
            records.append(_download_sample(candidate, path))
        except (OSError, ReverseKey1Error) as error:
            failures.append({"title": candidate["title"], "error": str(error)})
            print(
                json.dumps(
                    {
                        "category": candidate["category"],
                        "title": candidate["title"],
                        "error": str(error),
                    }
                ),
                flush=True,
            )
    for category in categories:
        candidates = _commons_category(category)
        print(
            json.dumps(
                {"category": category, "licenseCompatibleCandidates": len(candidates)}
            ),
            flush=True,
        )
        rng.shuffle(candidates)
        accepted = 0
        consecutive_failures = 0
        for candidate in candidates:
            if accepted >= per_category:
                break
            if accepted or consecutive_failures:
                time.sleep(1.0)
            path = image_directory.resolve() / f"{len(records):04d}.npy"
            try:
                records.append(_download_sample(candidate, path))
            except (OSError, ReverseKey1Error) as error:
                consecutive_failures += 1
                failures.append({"title": candidate["title"], "error": str(error)})
                print(
                    json.dumps(
                        {"category": category, "title": candidate["title"], "error": str(error)}
                    ),
                    flush=True,
                )
                if consecutive_failures >= 3:
                    break
                continue
            accepted += 1
            consecutive_failures = 0
    if len(records) < 3:
        raise ReverseKey1Error(
            f"only {len(records)} public samples passed license/decode checks"
        )
    result = {
        "schema": PUBLIC_CORPUS_SCHEMA,
        "seed": seed,
        "allowedLicensePolicy": ALLOWED_LICENSE_POLICY,
        "categories": list(categories),
        "requestedSamples": len(CURATED_GITHUB_SAMPLES)
        + len(categories) * per_category,
        "complete": len(records)
        == len(CURATED_GITHUB_SAMPLES) + len(categories) * per_category,
        "samples": records,
        "failures": failures,
    }
    # Keep partial-source status explicit; training separately rejects duplicate
    # source identities rather than allowing the same content into two splits.
    _write_new_json(output.resolve(), result)
    return result


def _write_new_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    staging: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=path.parent,
                                         prefix=".public-corpus-", delete=False) as target:
            staging = Path(target.name)
            json.dump(value, target, indent=2, sort_keys=True, allow_nan=False)
            target.write("\n")
            target.flush()
            os.fsync(target.fileno())
        os.link(staging, path)
    finally:
        if staging is not None:
            staging.unlink(missing_ok=True)


def split_public_records(records: Sequence[Mapping[str, Any]], seed: int) -> dict[str, list[Mapping[str, Any]]]:
    """Lock source-disjoint train/calibration/heldout splits before any fitting."""
    if len(records) < 3:
        raise ReverseKey1Error("public corpus needs at least three independent source images")
    seen: dict[str, set[str]] = {name: set() for name in
                              ("tensorSHA256", "downloadSHA256", "sourceGitBlobSHA1", "sourceSHA1", "sourceURL")}
    for record in records:
        if not isinstance(record, Mapping) or not license_is_allowed(str(record.get("license", ""))):
            raise ReverseKey1Error("public sample must retain an accepted source license label")
        for field, values in seen.items():
            value = record.get(field)
            if field in {"tensorSHA256", "downloadSHA256"} and (
                    not isinstance(value, str) or re.fullmatch(r"[0-9a-f]{64}", value) is None):
                raise ReverseKey1Error("public source and tensor SHA-256 identities are mandatory")
            if value:
                if str(value) in values:
                    raise ReverseKey1Error(f"duplicate public source identity: {field}")
                values.add(str(value))
    ordered = sorted(records, key=lambda row: str(row["downloadSHA256"]))
    random.Random(seed).shuffle(ordered)
    heldout_count = max(1, len(ordered) // 5)
    calibration_count = max(1, len(ordered) // 5)
    train_end = len(ordered) - heldout_count - calibration_count
    return {"train": ordered[:train_end], "calibration": ordered[train_end:-heldout_count],
            "heldout": ordered[-heldout_count:]}


def _verified_public_tensor(record: Mapping[str, Any]) -> np.ndarray:
    path = Path(str(record["imagePath"]))
    payload = path.read_bytes()
    if hashlib.sha256(payload).hexdigest() != record["tensorSHA256"]:
        raise ReverseKey1Error(f"public tensor identity mismatch: {path}")
    return np.asarray(np.load(io.BytesIO(payload), allow_pickle=False))


def synthetic_affine_pair(
    image: np.ndarray, rng: np.random.Generator
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    clean = normalized_primary_rgb(image)
    clean = np.clip(clean, 0.08, 0.92)
    delta = rng.normal(0.0, 0.055, size=(3, 3)).astype(np.float32)
    delta *= np.asarray(
        [[1.0, 0.35, 0.35], [0.35, 1.0, 0.35], [0.35, 0.35, 1.0]],
        dtype=np.float32,
    )
    bias = rng.normal(0.0, 0.025, size=3).astype(np.float32)
    for strength in (1.0, 0.5, 0.25, 0.125):
        matrix = np.eye(3, dtype=np.float32) + delta * strength
        offset = bias * strength
        styled = np.einsum("ci,ihw->chw", matrix, clean) + offset[:, None, None]
        if float(styled.min()) >= 0 and float(styled.max()) <= 1:
            break
    else:
        raise ReverseKey1Error("no bounded synthetic affine transform was found")
    inverse = np.linalg.inv(matrix).astype(np.float32)
    coefficients = np.zeros((10, 3), dtype=np.float32)
    coefficients[0] = -inverse @ offset
    coefficients[1:4] = inverse.T
    key1 = np.broadcast_to(
        coefficients, (12, 12, 8, 10, 3)
    ).copy()
    return styled.astype(np.float32), clean, key1


def _synthetic_statistics() -> dict[str, np.ndarray]:
    return {
        "metadataCenter": np.zeros(len(METADATA_FIELDS), dtype=np.float32),
        "metadataScale": np.ones(len(METADATA_FIELDS), dtype=np.float32),
        "metadataActive": np.zeros(len(METADATA_FIELDS), dtype=np.float32),
        "key1Scale": np.ones((8, 10, 3), dtype=np.float32),
        "gtcCenter": np.zeros(516, dtype=np.float32),
        "gtcScale": np.ones(516, dtype=np.float32),
        "lightCenter": np.zeros((2, 32, 32), dtype=np.float32),
        "lightScale": np.ones(2, dtype=np.float32),
        "scalarCenter": np.zeros(len(STYLE_SCALAR_FIELDS), dtype=np.float32),
        "scalarScale": np.ones(len(STYLE_SCALAR_FIELDS), dtype=np.float32),
        "scalarLow": np.full(len(STYLE_SCALAR_FIELDS), -10.0, dtype=np.float32),
        "scalarHigh": np.full(len(STYLE_SCALAR_FIELDS), 10.0, dtype=np.float32),
    }


@dataclasses.dataclass(frozen=True)
class PublicPretrainingConfig:
    manifest: Path
    output: Path
    epochs: int = 1
    batch_size: int = 2
    learning_rate: float = 2e-4
    transforms_per_image: int = 2
    key_loss_weight: float = 8.0
    unstyled_loss_weight: float = 0.1
    device: str = "cpu"
    seed: int = 260829


def pretrain_public_synthetic_style(config: PublicPretrainingConfig) -> dict[str, Any]:
    torch, _ = _require_torch()
    for count in (config.epochs, config.transforms_per_image, config.batch_size):
        if type(count) is not int or count <= 0:
            raise ReverseKey1Error("epochs, batch size and transforms-per-image must be positive integers")
    if (not all(math.isfinite(value) for value in (config.key_loss_weight, config.unstyled_loss_weight, config.learning_rate))
            or config.key_loss_weight <= 0 or config.unstyled_loss_weight < 0 or config.learning_rate <= 0):
        raise ReverseKey1Error("loss weights and learning rate must be finite with valid positive/nonnegative ranges")
    if config.device == "mps" and not torch.backends.mps.is_available():
        raise ReverseKey1Error("MPS was requested but is unavailable")
    manifest = config.manifest.resolve()
    value = json.loads(manifest.read_text(encoding="utf-8"))
    if value.get("schema") != PUBLIC_CORPUS_SCHEMA:
        raise ReverseKey1Error("invalid public style corpus manifest")
    records = value.get("samples")
    if not isinstance(records, list):
        raise ReverseKey1Error("public style corpus samples must be an array")
    by_split = split_public_records(records, config.seed)
    torch.manual_seed(config.seed)
    np.random.seed(config.seed)
    examples: dict[str, list[tuple[np.ndarray, np.ndarray, np.ndarray, str]]] = {}
    for split_name, split_records in by_split.items():
        examples[split_name] = []
        for record in split_records:
            image = _verified_public_tensor(record)
            for transform_index in range(config.transforms_per_image):
                rng = np.random.default_rng(config.seed + int(record["downloadSHA256"][:16], 16) + transform_index)
                styled, clean, key1 = synthetic_affine_pair(image, rng)
                examples[split_name].append((primary_image_features(styled),
                    clean[:, ::4, ::4].astype(np.float32), key1, str(record["title"])))

    class Dataset:
        def __init__(self, rows: Sequence[tuple[np.ndarray, ...]]) -> None:
            self.rows = rows

        def __len__(self) -> int:
            return len(self.rows)

        def __getitem__(self, index: int) -> tuple[Any, ...]:
            primary, clean, key1, title = self.rows[index]
            return (
                torch.from_numpy(primary),
                torch.from_numpy(clean),
                torch.from_numpy(key1),
                title,
            )

    loaders = {
        name: torch.utils.data.DataLoader(Dataset(rows), batch_size=config.batch_size,
                                         shuffle=name == "train")
        for name, rows in examples.items()
    }
    statistics = _synthetic_statistics()
    model = build_universal_model(
        statistics, architecture="multimodal_large"
    ).to(config.device)
    optimizer = torch.optim.AdamW(model.parameters(), lr=config.learning_rate)

    def evaluate(split: str) -> dict[str, float]:
        model.eval()
        key_errors: list[float] = []
        unstyled_errors: list[float] = []
        response_errors: list[float] = []
        with torch.no_grad():
            for primary, clean, key1, _titles in loaders[split]:
                primary = primary.to(config.device)
                clean = clean.to(config.device)
                key1 = key1.to(config.device)
                output = model(
                    primary,
                    torch.zeros((len(primary), len(METADATA_FIELDS)), device=config.device),
                    torch.zeros((len(primary), len(METADATA_FIELDS)), device=config.device),
                )
                key_errors.extend(
                    (output["key1"] - key1)
                    .abs()
                    .mean(dim=(1, 2, 3, 4, 5))
                    .cpu()
                    .tolist()
                )
                unstyled_errors.extend(
                    (output["unstyled"] - clean)
                    .abs()
                    .mean(dim=(1, 2, 3))
                    .cpu()
                    .tolist()
                )
                predicted_coefficients = output["key1"].mean(dim=(1, 2, 3))
                rgb = primary[:, :3, ::4, ::4]
                red, green, blue = rgb[:, 0], rgb[:, 1], rgb[:, 2]
                terms = torch.stack(
                    (
                        torch.ones_like(red), red, green, blue, red.square(),
                        red * green, red * blue, green.square(), green * blue,
                        blue.square(),
                    ),
                    dim=1,
                )
                rendered = torch.einsum(
                    "bthw,btc->bchw", terms, predicted_coefficients
                ).clamp(0, 1)
                response_errors.extend(
                    (rendered - clean)
                    .square()
                    .mean(dim=(1, 2, 3))
                    .sqrt()
                    .mul(255)
                    .cpu()
                    .tolist()
                )
        return {
            "key1MAE": float(np.mean(key_errors)),
            "unstyledMAE": float(np.mean(unstyled_errors)),
            "syntheticResponseRMSE8": float(np.mean(response_errors)),
        }

    baseline = evaluate("calibration")
    best_epoch = 0
    best_metrics = dict(baseline)
    best_state = copy.deepcopy(model.state_dict())
    history: list[dict[str, Any]] = []
    for epoch in range(1, config.epochs + 1):
        model.train()
        losses: list[float] = []
        for primary, clean, key1, _titles in loaders["train"]:
            primary = primary.to(config.device)
            clean = clean.to(config.device)
            key1 = key1.to(config.device)
            optimizer.zero_grad(set_to_none=True)
            output = model(
                primary,
                torch.zeros((len(primary), len(METADATA_FIELDS)), device=config.device),
                torch.zeros((len(primary), len(METADATA_FIELDS)), device=config.device),
            )
            key_loss = torch.nn.functional.l1_loss(output["key1"], key1)
            unstyled_loss = torch.nn.functional.l1_loss(output["unstyled"], clean)
            loss = (
                config.key_loss_weight * key_loss
                + config.unstyled_loss_weight * unstyled_loss
            )
            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 2.0)
            optimizer.step()
            losses.append(float(loss.detach().cpu()))
        metrics = evaluate("calibration")
        history.append(
            {"epoch": epoch, "trainingLoss": float(np.mean(losses)), "calibration": metrics}
        )
        selection = (metrics["key1MAE"], metrics["syntheticResponseRMSE8"])
        best_selection = (
            best_metrics["key1MAE"], best_metrics["syntheticResponseRMSE8"]
        )
        if selection < best_selection:
            best_epoch = epoch
            best_metrics = dict(metrics)
            best_state = copy.deepcopy(model.state_dict())
        print(json.dumps(history[-1], sort_keys=True), flush=True)
    model.load_state_dict(best_state)
    # No heldout access occurs before the selected weights are frozen.
    heldout = evaluate("heldout")
    output = config.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    checkpoint = {
        "schema": SYNTHETIC_REPORT_SCHEMA,
        "architecture": "UniversalPhotographicStyleStateNet-v3-optional-modalities",
        "architectureConfig": "multimodal_large",
        "epoch": best_epoch,
        "manifestSHA256": sha256_file(manifest),
        "model": model.state_dict(),
        "statistics": {name: array.tolist() for name, array in statistics.items()},
        "metadataFields": list(METADATA_FIELDS),
        "styleScalarFields": list(STYLE_SCALAR_FIELDS),
        "syntheticPretrainingOnly": True,
    }
    torch.save(checkpoint, output / "synthetic-pretrained.pt")
    report = {
        "schema": SYNTHETIC_REPORT_SCHEMA,
        "manifestSHA256": sha256_file(manifest),
        "sourceSamples": len(records),
        "syntheticExamples": sum(len(rows) for rows in examples.values()),
        "splitExamples": {name: len(rows) for name, rows in examples.items()},
        "splitSourceSHA256": {name: [row["downloadSHA256"] for row in rows] for name, rows in by_split.items()},
        "architecture": checkpoint["architecture"],
        "device": config.device,
        "lossWeights": {
            "key1L1": config.key_loss_weight,
            "unstyledL1": config.unstyled_loss_weight,
        },
        "baselineCalibration": baseline,
        "bestEpoch": best_epoch,
        "selectedCalibration": best_metrics,
        "heldout": heldout,
        "lastEpochCalibration": history[-1]["calibration"],
        "history": history,
        "licenses": sorted({str(record["license"]) for record in records}),
        "licenseEvidence": "Source-supplied license metadata; not independent legal adjudication",
        "categories": sorted({str(record["category"]) for record in records}),
        "claimBoundary": (
            "Synthetic affine key1 pretraining only. This is not native Apple or "
            "Neutrino solver supervision and cannot establish production accuracy."
        ),
    }
    _atomic_json(output / "report.json", report)
    return report
