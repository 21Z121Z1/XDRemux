"""Readable Motion Photo batch output planning and local resume state."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Callable


SCHEMA_VERSION = 1
DEFAULT_STATE_NAME = ".xdremux-motion-photo-checkpoint.jsonl"


@dataclass(frozen=True)
class SourceSignature:
    size: int
    mtime_ns: int


@dataclass(frozen=True)
class BatchStateItem:
    kind: str
    input_path: str
    output_image_path: str
    output_video_path: str
    status: str
    input_size: int | None
    input_mtime_ns: int | None
    asset_identifier: str | None
    error: str | None = None

    def matches_source(self, signature: SourceSignature) -> bool:
        return self.input_size == signature.size and self.input_mtime_ns == signature.mtime_ns

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

    def to_wire(self) -> dict[str, object]:
        return {
            "kind": self.kind,
            "input_path": self.input_path,
            "output_image_path": self.output_image_path,
            "output_video_path": self.output_video_path,
            "status": self.status,
            "input_size": self.input_size,
            "input_mtime_ns": self.input_mtime_ns,
            "asset_identifier": self.asset_identifier,
            "error": self.error,
        }

    @classmethod
    def from_wire(cls, raw: dict[str, object]) -> "BatchStateItem" | None:
        # Accept PR #18's camelCase records as a one-way migration path, but new state is intentionally
        # runtime-local and uses ordinary Python field names.
        def value(snake: str, camel: str):
            return raw[snake] if snake in raw else raw.get(camel)

        try:
            input_path = value("input_path", "inputPath")
            image_path = value("output_image_path", "outputImagePath")
            video_path = value("output_video_path", "outputVideoPath")
            status = raw.get("status")
            if not all(isinstance(item, str) and item for item in (input_path, image_path, video_path, status)):
                return None
            input_size = value("input_size", "inputSize")
            input_mtime_ns = value("input_mtime_ns", "inputMtimeNs")
            return cls(
                kind="item",
                input_path=input_path,
                output_image_path=image_path,
                output_video_path=video_path,
                status=status,
                input_size=int(input_size) if input_size is not None else None,
                input_mtime_ns=int(input_mtime_ns) if input_mtime_ns is not None else None,
                asset_identifier=value("asset_identifier", "assetIdentifier") if isinstance(value("asset_identifier", "assetIdentifier"), str) else None,
                error=value("error", "error") if isinstance(value("error", "error"), str) else None,
            )
        except (TypeError, ValueError):
            return None


def source_signature(path: Path) -> SourceSignature:
    stat = Path(path).stat()
    return SourceSignature(size=stat.st_size, mtime_ns=stat.st_mtime_ns)


def relative_source_path(source: Path, input_root: Path) -> Path:
    source = Path(source).resolve()
    input_root = Path(input_root).resolve()
    try:
        return source.relative_to(input_root)
    except ValueError:
        # Discovery normally keeps inputs below input_root. A readable basename fallback keeps this
        # helper total; validate_unique_plan() still rejects collisions within one invocation.
        return Path(source.name)


def planned_output_image(source: Path, input_root: Path, output_directory: Path) -> Path:
    source = Path(source)
    relative = relative_source_path(source, input_root)
    kind_suffix = "live" if source.suffix.lower() in {".heic", ".heif"} else "motion"
    return Path(output_directory) / relative.parent / f"{source.stem}.{kind_suffix}.heic"


def validate_unique_plan(outputs: list[tuple[Path, Path]]) -> None:
    seen: dict[str, Path] = {}
    for source, output in outputs:
        key = str(Path(output).resolve())
        prior = seen.get(key)
        if prior is not None and prior.resolve() != Path(source).resolve():
            raise ValueError(f"Motion Photo output collision: {prior} and {source} -> {output}")
        seen[key] = Path(source)


def state_path(output_dir: Path, requested: Path | None = None) -> Path:
    if requested is not None:
        requested = Path(requested)
        return requested.parent / f"{requested.name}.motion-photo"
    return Path(output_dir) / DEFAULT_STATE_NAME


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
            if not isinstance(raw, dict) or raw.get("kind") != "item":
                continue
            item = BatchStateItem.from_wire(raw)
            if item is not None:
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
        del input_root  # Kept in the call signature to avoid coupling the CLI to checkpoint internals.
        item = BatchStateItem(
            kind="item",
            input_path=str(Path(source).resolve()),
            output_image_path=str(Path(image).resolve()),
            output_video_path=str(Path(video).resolve()),
            status=status,
            input_size=signature.size,
            input_mtime_ns=signature.mtime_ns,
            asset_identifier=asset_identifier,
            error=error,
        )
        self._append(item.to_wire())

    def close(self) -> None:
        if not self._handle.closed:
            self._handle.flush()
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
    # The function name stays for CLI source compatibility in this focused refactor. Reuse is now
    # driven by cheap file metadata + the saved output paths; the asset identifier only confirms that
    # the existing pair is the pair recorded by this local checkpoint.
    if prior is None or not prior.reusable(signature, image, video) or prior.asset_identifier is None:
        return False
    return pair_matches_identifier(image, video, prior.asset_identifier)
