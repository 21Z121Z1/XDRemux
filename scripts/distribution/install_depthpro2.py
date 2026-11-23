#!/usr/bin/env python3
"""Download pluggable Apple Depth Pro 2 CoreML models and install xdremux-depthpro2."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shlex
import tempfile
import urllib.parse
import urllib.request


ROOT = Path(__file__).resolve().parents[2]
HF_REPO = "apple/DepthPro2-coreml"
HF_REVISION = "4e607a682f993bbf95035054fc78ba9ffc81faf2"
AMLR_LICENSE_NAME = "Apple Machine Learning Research License (AMLR)"
AMLR_LICENSE_URL = f"https://huggingface.co/{HF_REPO}/blob/{HF_REVISION}/LICENSE"
LICENSE_RECEIPT_NAME = ".apple-amlr-license-accepted.json"

MODEL_CATALOG: dict[str, dict[str, str]] = {
    "s": {
        "id": "depthpro2-s_w4",
        "package": "depthpro2-s_w4.mlpackage",
        "weight_sha256": "24b741e5608cbdd99bc2b26d4925c410b2d415637c36b1c0b51050af9c9be1db",
    },
    "b": {
        "id": "depthpro2-b_w4",
        "package": "depthpro2-b_w4.mlpackage",
        "weight_sha256": "6f47f78fcc7054239bd1f9653b82179e622b0adff0c30877b495f62bf56c37bc",
    },
    "fov": {
        "id": "depthpro2-fov_512x896",
        "package": "depthpro2-fov_512x896.mlpackage",
        "weight_sha256": "1d95a062127b46b7d81d393aac28d58b9dec03f5535409267ee32a14ccf984f5",
    },
}

SELECTOR_ALIASES = {
    "s": "s",
    "small": "s",
    "depthpro2-s": "s",
    "depthpro2-s_w4": "s",
    "b": "b",
    "base": "b",
    "depthpro2-b": "b",
    "depthpro2-b_w4": "b",
    "fov": "fov",
    "depthpro2-fov": "fov",
    "depthpro2-fov_512x896": "fov",
}


def default_models_dir() -> Path:
    override = os.environ.get("XDREMUX_MODEL_DIR", "").strip()
    if override:
        return Path(override).expanduser()
    return Path.home() / "Library" / "Application Support" / "XDRemux" / "Models"


def parse_models_selector(raw: str) -> list[str]:
    selected: list[str] = []
    for token in raw.split(","):
        cleaned = token.strip().lower()
        if not cleaned:
            continue
        if cleaned in {"all", "default", "sb"}:
            for key in ("s", "b", "fov"):
                if key not in selected:
                    selected.append(key)
            continue
        mapped = SELECTOR_ALIASES.get(cleaned)
        if mapped is None:
            raise ValueError(f"unknown Depth Pro 2 model selector: {token!r}")
        if mapped not in selected:
            selected.append(mapped)
    if not selected:
        raise ValueError("at least one Depth Pro 2 model must be selected")
    return selected


def fetch_json(url: str) -> list[dict]:
    req = urllib.request.Request(url, headers={"User-Agent": "XDRemux/1.0"})
    with urllib.request.urlopen(req, timeout=120) as response:
        return json.load(response)


def download_file(repo_path: str, dest: Path, expected_size: int | None = None) -> None:
    encoded = urllib.parse.quote(repo_path, safe="/")
    url = f"https://huggingface.co/{HF_REPO}/resolve/{HF_REVISION}/{encoded}"
    dest.parent.mkdir(parents=True, exist_ok=True)
    if (
        expected_size is not None
        and expected_size > 0
        and dest.is_file()
        and dest.stat().st_size == expected_size
    ):
        return

    req = urllib.request.Request(url, headers={"User-Agent": "XDRemux/1.0"})
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="wb",
            dir=dest.parent,
            prefix=f".{dest.name}.part-",
            delete=False,
        ) as out:
            temporary = Path(out.name)
            with urllib.request.urlopen(req, timeout=600) as response:
                while True:
                    chunk = response.read(1024 * 1024)
                    if not chunk:
                        break
                    out.write(chunk)
        if expected_size is not None and expected_size > 0:
            actual_size = temporary.stat().st_size
            if actual_size != expected_size:
                raise ValueError(
                    f"size mismatch for {repo_path}: expected {expected_size}, got {actual_size}"
                )
        temporary.replace(dest)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def verify_weight_digest(package_dir: Path, expected_sha256: str) -> None:
    weight_file = package_dir / "Data" / "com.apple.CoreML" / "weights" / "weight.bin"
    if not weight_file.is_file():
        raise ValueError(f"missing weights binary: {weight_file}")
    with weight_file.open("rb") as stream:
        actual = hashlib.file_digest(stream, "sha256").hexdigest()
    if actual != expected_sha256:
        raise ValueError(
            f"{package_dir.name} weight SHA-256 mismatch: expected {expected_sha256}, got {actual}"
        )


def write_license_receipt(models_dir: Path, model_ids: list[str]) -> None:
    payload = {
        "licenseName": AMLR_LICENSE_NAME,
        "licenseURL": AMLR_LICENSE_URL,
        "huggingFaceRepo": HF_REPO,
        "revision": HF_REVISION,
        "acceptedAtISO8601": datetime.now(timezone.utc).isoformat(),
        "modelIDs": model_ids,
    }
    receipt = models_dir / LICENSE_RECEIPT_NAME
    encoded = json.dumps(payload, indent=2) + "\n"
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            dir=models_dir,
            prefix=".license-receipt-",
            delete=False,
        ) as stream:
            temporary = Path(stream.name)
            stream.write(encoded)
        temporary.replace(receipt)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def download_models(
    models_dir: Path,
    model_keys: list[str],
    *,
    accept_license: bool,
) -> list[str]:
    accepted = accept_license or os.environ.get("XDREMUX_ACCEPT_APPLE_AMLR_LICENSE") == "1"
    if not accepted:
        raise ValueError(
            f"downloading Apple Depth Pro 2 weights requires --accept-apple-amlr-license ({AMLR_LICENSE_URL})"
        )

    models_dir.mkdir(parents=True, exist_ok=True)
    for doc_name in ("LICENSE", "README.md"):
        doc_path = models_dir / doc_name
        if not doc_path.is_file():
            download_file(doc_name, doc_path)

    installed_ids: list[str] = []
    for key in model_keys:
        spec = MODEL_CATALOG[key]
        pkg_name = spec["package"]
        pkg_dir = models_dir / pkg_name
        weight_path = pkg_dir / "Data" / "com.apple.CoreML" / "weights" / "weight.bin"
        if not ((pkg_dir / "Manifest.json").is_file() and weight_path.is_file()):
            tree_url = (
                f"https://huggingface.co/api/models/{HF_REPO}/tree/{HF_REVISION}"
                f"/{urllib.parse.quote(pkg_name)}?recursive=true"
            )
            entries = fetch_json(tree_url)
            files = [entry for entry in entries if entry.get("type") == "file"]
            if not files:
                raise ValueError(f"no files returned for {pkg_name} from Hugging Face")
            for entry in files:
                download_file(entry["path"], models_dir / entry["path"], entry.get("size"))
        verify_weight_digest(pkg_dir, spec["weight_sha256"])
        installed_ids.append(spec["id"])

    write_license_receipt(models_dir, installed_ids)
    return installed_ids


def install_launcher(
    bin_dir: Path,
    models_dir: Path,
    *,
    python_path: Path | None = None,
    adapter_path: Path | None = None,
) -> Path:
    bin_dir = bin_dir.resolve(strict=True)
    models_dir = models_dir.resolve(strict=True)
    py = (python_path or Path("/usr/bin/python3")).absolute()
    if not py.is_file() or not os.access(py, os.X_OK):
        raise ValueError(f"python interpreter must be executable: {py}")

    source = (ROOT / "model_adapters/depthpro2/estimate.py").read_text(encoding="utf-8")
    delimiter = "XDREMUX_DEPTHPRO2_MODEL_SOURCE"
    if delimiter in source.splitlines():
        raise ValueError("unexpected model source delimiter")

    adapter_arg = (
        f" --adapter {shlex.quote(str(adapter_path.resolve(strict=True)))}"
        if adapter_path is not None
        else ""
    )
    launcher = (
        f"#!/bin/sh\n"
        f"exec {shlex.quote(str(py))} -I - --models-dir {shlex.quote(str(models_dir))}"
        f"{adapter_arg} \"$@\" <<'{delimiter}'\n"
        f"{source}\n"
        f"{delimiter}\n"
    )
    output = bin_dir / "xdremux-depthpro2"
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            dir=bin_dir,
            prefix=".depthpro2-install-",
            delete=False,
        ) as stream:
            temporary = Path(stream.name)
            stream.write(launcher)
        temporary.chmod(0o700)
        os.link(temporary, output)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)
    return output


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--models-dir", type=Path, default=default_models_dir())
    parser.add_argument("--models", default="s,b,fov", help="Comma-separated models: s, b, fov")
    parser.add_argument("--accept-apple-amlr-license", action="store_true")
    parser.add_argument("--bin-dir", type=Path, default=None)
    parser.add_argument("--python", type=Path, default=None)
    parser.add_argument("--adapter", type=Path, default=None)
    parser.add_argument("--skip-download", action="store_true")
    args = parser.parse_args()
    try:
        if not args.skip_download:
            keys = parse_models_selector(args.models)
            installed = download_models(
                args.models_dir,
                keys,
                accept_license=args.accept_apple_amlr_license,
            )
            print(f"models ready in {args.models_dir}: {', '.join(installed)}")
        if args.bin_dir is not None:
            launcher = install_launcher(
                args.bin_dir,
                args.models_dir,
                python_path=args.python,
                adapter_path=args.adapter,
            )
            print(launcher)
    except (OSError, ValueError) as error:
        parser.exit(1, f"Depth Pro 2 installation failed: {error}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
