"""License-audited public-image collection and synthetic style pretraining."""

from __future__ import annotations

import dataclasses
import hashlib
import html
import json
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
from xdremux_py.universal_photographic_style_training import (
    INPUT_SIZE,
    METADATA_FIELDS,
    STYLE_SCALAR_FIELDS,
    build_universal_model,
    primary_image_features,
)


PUBLIC_CORPUS_SCHEMA = "xdremux-public-style-content-corpus-v1"
SYNTHETIC_REPORT_SCHEMA = "xdremux-public-synthetic-style-pretraining-v1"
COMMONS_API = "https://commons.wikimedia.org/w/api.php"
DEFAULT_CATEGORIES = (
    "Taken with iPhone 16",
    "Taken with iPhone 16 Pro",
    "Taken with iPhone 17 Pro",
    "Taken with Oppo A3x 4G",
    "Taken with Oppo F29 5G",
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
        headers={"User-Agent": "XDRemux-public-style-research/1.0"},
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        return commons_candidates(json.load(response), category, request_limit)


def _download_sample(record: Mapping[str, Any], path: Path) -> dict[str, Any]:
    request = urllib.request.Request(
        str(record["downloadURL"]),
        headers={"User-Agent": "XDRemux-public-style-research/1.0"},
    )
    payload: bytes | None = None
    for attempt in range(4):
        try:
            with urllib.request.urlopen(request, timeout=90) as response:
                payload = response.read(24 * 1024 * 1024 + 1)
            break
        except urllib.error.HTTPError as error:
            if error.code not in {429, 500, 502, 503, 504} or attempt == 3:
                raise
            time.sleep(2**attempt)
    if payload is None:
        raise ReverseKey1Error(f"public sample download returned no data: {record['title']}")
    if len(payload) > 24 * 1024 * 1024:
        raise ReverseKey1Error(f"public sample exceeds 24 MiB: {record['title']}")
    try:
        from io import BytesIO

        with Image.open(BytesIO(payload)) as source:
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
    np.save(path, value, allow_pickle=False)
    result = dict(record)
    result.pop("downloadURL", None)
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
    rng = random.Random(seed)
    records: list[dict[str, Any]] = []
    failures: list[dict[str, str]] = []
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
        for candidate in candidates:
            if accepted >= per_category:
                break
            path = image_directory.resolve() / f"{len(records):04d}.npy"
            try:
                records.append(_download_sample(candidate, path))
            except (OSError, ReverseKey1Error) as error:
                failures.append({"title": candidate["title"], "error": str(error)})
                print(
                    json.dumps(
                        {"category": category, "title": candidate["title"], "error": str(error)}
                    ),
                    flush=True,
                )
                continue
            accepted += 1
            time.sleep(0.5)
    if len(records) < max(4, len(categories)):
        raise ReverseKey1Error(
            f"only {len(records)} public samples passed license/decode checks"
        )
    result = {
        "schema": PUBLIC_CORPUS_SCHEMA,
        "seed": seed,
        "allowedLicensePolicy": ALLOWED_LICENSE_POLICY,
        "categories": list(categories),
        "samples": records,
        "failures": failures,
    }
    _atomic_json(output.resolve(), result)
    return result


def synthetic_affine_pair(
    image: np.ndarray, rng: np.random.Generator
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    clean = np.asarray(image, dtype=np.float32)
    if clean.shape != (3, INPUT_SIZE, INPUT_SIZE):
        raise ReverseKey1Error("synthetic source image must be 3x256x256")
    if clean.max(initial=0) > 1:
        clean = clean / 255.0
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
    device: str = "cpu"
    seed: int = 260829


def pretrain_public_synthetic_style(config: PublicPretrainingConfig) -> dict[str, Any]:
    torch, _ = _require_torch()
    if config.device == "mps" and not torch.backends.mps.is_available():
        raise ReverseKey1Error("MPS was requested but is unavailable")
    manifest = config.manifest.resolve()
    value = json.loads(manifest.read_text(encoding="utf-8"))
    if value.get("schema") != PUBLIC_CORPUS_SCHEMA:
        raise ReverseKey1Error("invalid public style corpus manifest")
    records = value.get("samples")
    if not isinstance(records, list) or len(records) < 4:
        raise ReverseKey1Error("public style corpus is too small")
    torch.manual_seed(config.seed)
    np.random.seed(config.seed)
    examples: list[tuple[np.ndarray, np.ndarray, np.ndarray, str]] = []
    for record_index, record in enumerate(records):
        path = Path(str(record["imagePath"]))
        if record.get("tensorSHA256") != sha256_file(path):
            raise ReverseKey1Error(f"public tensor identity mismatch: {path}")
        image = np.load(path, allow_pickle=False)
        for transform_index in range(config.transforms_per_image):
            rng = np.random.default_rng(
                config.seed + record_index * 1009 + transform_index
            )
            styled, clean, key1 = synthetic_affine_pair(image, rng)
            examples.append(
                (
                    primary_image_features(styled),
                    clean[:, ::4, ::4].astype(np.float32),
                    key1,
                    str(record["title"]),
                )
            )
    split = max(1, int(len(records) * 0.8)) * config.transforms_per_image
    train_examples = examples[:split]
    heldout_examples = examples[split:]
    if not heldout_examples:
        raise ReverseKey1Error("public synthetic heldout split is empty")

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
        "train": torch.utils.data.DataLoader(
            Dataset(train_examples), batch_size=config.batch_size, shuffle=True
        ),
        "heldout": torch.utils.data.DataLoader(
            Dataset(heldout_examples), batch_size=config.batch_size, shuffle=False
        ),
    }
    statistics = _synthetic_statistics()
    model = build_universal_model(
        statistics, architecture="multimodal_large"
    ).to(config.device)
    optimizer = torch.optim.AdamW(model.parameters(), lr=config.learning_rate)

    def evaluate() -> dict[str, float]:
        model.eval()
        key_errors: list[float] = []
        unstyled_errors: list[float] = []
        response_errors: list[float] = []
        with torch.no_grad():
            for primary, clean, key1, _titles in loaders["heldout"]:
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

    baseline = evaluate()
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
            key_loss = torch.nn.functional.smooth_l1_loss(output["key1"], key1)
            unstyled_loss = torch.nn.functional.l1_loss(output["unstyled"], clean)
            loss = key_loss + 0.25 * unstyled_loss
            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 2.0)
            optimizer.step()
            losses.append(float(loss.detach().cpu()))
        metrics = evaluate()
        history.append(
            {"epoch": epoch, "trainingLoss": float(np.mean(losses)), "heldout": metrics}
        )
        print(json.dumps(history[-1], sort_keys=True), flush=True)
    output = config.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    checkpoint = {
        "schema": SYNTHETIC_REPORT_SCHEMA,
        "architecture": "UniversalPhotographicStyleStateNet-v3-optional-modalities",
        "architectureConfig": "multimodal_large",
        "epoch": config.epochs,
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
        "syntheticExamples": len(examples),
        "splitExamples": {
            "train": len(train_examples),
            "heldout": len(heldout_examples),
        },
        "architecture": checkpoint["architecture"],
        "device": config.device,
        "baseline": baseline,
        "final": history[-1]["heldout"],
        "history": history,
        "licenses": sorted({str(record["license"]) for record in records}),
        "categories": sorted({str(record["category"]) for record in records}),
        "claimBoundary": (
            "Synthetic affine key1 pretraining only. This is not native Apple or "
            "Neutrino solver supervision and cannot establish production accuracy."
        ),
    }
    _atomic_json(output / "report.json", report)
    return report
