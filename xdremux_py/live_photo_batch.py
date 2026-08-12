"""Stable output planning and provenance for Python Motion Photo batch conversion."""

from __future__ import annotations

import hashlib
import json
import os
import unicodedata
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Callable


SCHEMA_VERSION = 2
DEFAULT_STATE_NAME = ".xdremux-motion-photo-checkpoint.jsonl"


@dataclass(frozen=True)
class SourceSignature:
    size: int
    mtime_ns: int
    sha256: str


@dataclass(frozen=True)
class BatchStateItem:
    kind: str
    input_path: str
    relative_source_path: str
    output_image_path: str
    output_video_path: str
    status: str
    input_size: int
    input_mtime_ns: int
    input_sha256: str
    asset_identifier: str | None
    error: str | None = None

    def matches_source(self, signature: SourceSignature) -> bool:
        # Size is a cheap corruption guard; SHA-256 is the source identity. mtime is retained for
        # diagnostics but touching/copying an unchanged source does not invalidate provenance.
        return self.input_size == signature.size and self.input_sha256 == signature.sha256

    def matches_outputs(self, image: Path, video: Path) -> bool:
        return (
            self.output_image_path == str(Path(image).resolve())
            and self.output_video_path == str(Path(video).resolve())
        )

    def reusable(self, signature: SourceSignature, image: Path, video: Path) -> bool:
        return (
            self.status in {"success", "skipped_existing"}
            and bool(self.asset_identifier)
            and self.matches_source(signature)
            and self.matches_outputs(image, video)
        )


def source_signature(path: Path, chunk_size: int = 1024 * 1024) -> SourceSignature:
    path = Path(path)
    stat = path.stat()
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while True:
            chunk = handle.read(chunk_size)
            if not chunk:
                break
            digest.update(chunk)
    return SourceSignature(size=stat.st_size, mtime_ns=stat.st_mtime_ns, sha256=digest.hexdigest())


def relative_source_path(source: Path, input_root: Path) -> str:
    source = Path(source).resolve()
    input_root = Path(input_root).resolve()
    try:
        relative = source.relative_to(input_root).as_posix()
    except ValueError:
        # Explicit globs should normally stay below input_root. If a caller supplies an unusual
        # path, hashing the absolute path is still deterministic and cannot alias another source.
        relative = source.as_posix()
    return unicodedata.normalize("NFC", relative)


def stable_source_token(source: Path, input_root: Path, length: int = 16) -> str:
    relative = relative_source_path(source, input_root)
    return hashlib.sha256(relative.encode("utf-8")).hexdigest()[:length]


def planned_output_image(source: Path, input_root: Path, output_directory: Path) -> Path:
    """Return a stable Motion Photo output independent of the current batch membership/order."""
    source = Path(source)
    output_directory = Path(output_directory)
    token = stable_source_token(source, input_root)
    stem = source.stem + (".live" if source.suffix.lower() in {".heic", ".heif"} else "")
    return output_directory / f"{stem}~{token}.heic"


def validate_unique_plan(outputs: list[tuple[Path, Path]]) -> None:
    """Fail closed on the practically-impossible deterministic-token/path collision."""
    seen: dict[str, Path] = {}
    for source, output in outputs:
        key = str(Path(output).resolve())
        prior = seen.get(key)
        if prior is not None and prior.resolve() != Path(source).resolve():
            raise ValueError(f"stable Motion Photo output collision: {prior} and {source} -> {output}")
        seen[key] = Path(source)


def state_path(output_dir: Path, requested: Path | None = None) -> Path:
    return Path(requested) if requested is not None else Path(output_dir) / DEFAULT_STATE_NAME


def load_state(path: Path) -> dict[str, BatchStateItem]:
    path = Path(path)
    if not path.is_file():
        return {}
    state: dict[str, BatchStateItem] = {}
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            try:
                raw = json.loads(line)
            except json.JSONDecodeError:
                continue
            if raw.get("kind") != "item":
                continue
            # Schema-1 entries did not contain a content digest/asset identifier and therefore are
            # deliberately not trusted as provenance for --skip-existing.
            if not raw.get("input_sha256") or not raw.get("asset_identifier"):
                continue
            try:
                item = BatchStateItem(**raw)
            except TypeError:
                continue
            state[item.input_path] = item
    return state


class StateWriter:
    def __init__(self, path: Path):
        self.path = Path(path)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        new_file = not self.path.exists() or self.path.stat().st_size == 0
        self._handle = self.path.open("a", encoding="utf-8", newline="\n")
        if new_file:
            self._append({"kind": "header", "schema_version": SCHEMA_VERSION})

    def _append(self, value: dict[str, object]) -> None:
        self._handle.write(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n")
        self._handle.flush()
        os.fsync(self._handle.fileno())

    def append(
        self,
        *,
        source: Path,
        input_root: Path,
        image: Path,
        video: Path,
        status: str,
        signature: SourceSignature,
        asset_identifier: str | None,
        error: str | None = None,
    ) -> None:
        item = BatchStateItem(
            kind="item",
            input_path=str(Path(source).resolve()),
            relative_source_path=relative_source_path(source, input_root),
            output_image_path=str(Path(image).resolve()),
            output_video_path=str(Path(video).resolve()),
            status=status,
            input_size=signature.size,
            input_mtime_ns=signature.mtime_ns,
            input_sha256=signature.sha256,
            asset_identifier=asset_identifier,
            error=error,
        )
        self._append(asdict(item))

    def close(self) -> None:
        if not self._handle.closed:
            self._handle.flush()
            os.fsync(self._handle.fileno())
            self._handle.close()

    def __enter__(self) -> "StateWriter":
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        self.close()


def provenance_allows_reuse(
    prior: BatchStateItem | None,
    signature: SourceSignature,
    image: Path,
    video: Path,
    pair_matches_identifier: Callable[[Path, Path, str], bool],
) -> bool:
    if prior is None or not prior.reusable(signature, image, video) or prior.asset_identifier is None:
        return False
    return pair_matches_identifier(image, video, prior.asset_identifier)
