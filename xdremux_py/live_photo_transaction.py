"""Crash-recoverable two-file commit protocol for Apple Live Photo pairs.

A Live Photo is a HEIC/HEIF still plus a MOV resource. No filesystem primitive can
atomically rename both files together, so this module uses a small, durable journal
and same-directory renames. The journal wire schema and POSIX record lock intentionally
match the Swift implementation so either runtime can recover the other's interrupted
transaction.
"""

from __future__ import annotations

import json
import os
import uuid
from contextlib import contextmanager
from dataclasses import dataclass, replace
from pathlib import Path
from typing import Callable, Iterator

try:  # POSIX/macOS/Linux
    import fcntl  # type: ignore
except ImportError:  # pragma: no cover - exercised on Windows only
    fcntl = None

try:  # Windows
    import msvcrt  # type: ignore
except ImportError:  # pragma: no cover - exercised on POSIX only
    msvcrt = None


JOURNAL_DIRECTORY = ".xdremux-live-photo-transactions"
LOCK_FILE = ".xdremux-live-photo-transactions.lock"
SCHEMA_VERSION = 1

PairValidator = Callable[[Path, Path], bool]


@dataclass(frozen=True)
class TransactionManifest:
    schema_version: int
    transaction_id: str
    state: str
    final_image: str
    final_video: str
    temporary_image: str
    temporary_video: str
    backup_image: str
    backup_video: str
    had_image: bool
    had_video: bool

    def to_wire(self) -> dict[str, object]:
        """Canonical camelCase schema shared with Swift LivePhotoPairTransaction.Manifest."""
        return {
            "schemaVersion": self.schema_version,
            "transactionID": self.transaction_id,
            "state": self.state,
            "finalImage": self.final_image,
            "finalVideo": self.final_video,
            "temporaryImage": self.temporary_image,
            "temporaryVideo": self.temporary_video,
            "backupImage": self.backup_image,
            "backupVideo": self.backup_video,
            "hadImage": self.had_image,
            "hadVideo": self.had_video,
        }

    @classmethod
    def from_wire(cls, raw: dict[str, object]) -> "TransactionManifest":
        """Read canonical Swift/Python schema and the short-lived Python snake_case schema."""
        def required(camel: str, snake: str):
            if camel in raw:
                return raw[camel]
            if snake in raw:
                return raw[snake]
            raise ValueError(f"missing Live Photo transaction field: {camel}")

        try:
            return cls(
                schema_version=int(required("schemaVersion", "schema_version")),
                transaction_id=str(required("transactionID", "transaction_id")),
                state=str(required("state", "state")),
                final_image=str(required("finalImage", "final_image")),
                final_video=str(required("finalVideo", "final_video")),
                temporary_image=str(required("temporaryImage", "temporary_image")),
                temporary_video=str(required("temporaryVideo", "temporary_video")),
                backup_image=str(required("backupImage", "backup_image")),
                backup_video=str(required("backupVideo", "backup_video")),
                had_image=bool(required("hadImage", "had_image")),
                had_video=bool(required("hadVideo", "had_video")),
            )
        except (TypeError, ValueError) as exc:
            raise ValueError(f"invalid Live Photo transaction manifest: {exc}") from exc


_STATE_ORDER = {
    "prepared": 0,
    "originals_backed_up": 1,
    "image_installed": 2,
    "pair_installed": 3,
    "committed": 4,
}


def _safe_child(directory: Path, name: str) -> Path:
    candidate = Path(name)
    if not name or candidate.name != name or candidate.is_absolute() or name in {".", ".."}:
        raise ValueError(f"unsafe Live Photo transaction path: {name!r}")
    return directory / name


def _fsync_file(path: Path) -> None:
    with path.open("rb") as handle:
        os.fsync(handle.fileno())


def _fsync_directory(directory: Path) -> None:
    """Best-effort directory sync; some platforms/filesystems reject directory fsync."""
    flags = os.O_RDONLY
    if hasattr(os, "O_DIRECTORY"):
        flags |= os.O_DIRECTORY
    try:
        fd = os.open(directory, flags)
    except OSError:
        return
    try:
        try:
            os.fsync(fd)
        except OSError:
            pass
    finally:
        os.close(fd)


@contextmanager
def _directory_lock(directory: Path) -> Iterator[None]:
    """Serialize recovery/commit across Python and Swift XDRemux processes."""
    directory.mkdir(parents=True, exist_ok=True)
    lock_path = directory / LOCK_FILE
    fd = os.open(lock_path, os.O_CREAT | os.O_RDWR, 0o600)
    try:
        if fcntl is not None:
            # Swift uses POSIX lockf(). Python lockf() is the same fcntl record-lock family; using
            # flock() here would not be guaranteed to interoperate with it on every Unix platform.
            fcntl.lockf(fd, fcntl.LOCK_EX)
        elif msvcrt is not None:  # pragma: no cover - Windows-only path
            if os.fstat(fd).st_size == 0:
                os.write(fd, b"\0")
            os.lseek(fd, 0, os.SEEK_SET)
            msvcrt.locking(fd, msvcrt.LK_LOCK, 1)
        yield
    finally:
        if fcntl is not None:
            fcntl.lockf(fd, fcntl.LOCK_UN)
        elif msvcrt is not None:  # pragma: no cover - Windows-only path
            os.lseek(fd, 0, os.SEEK_SET)
            try:
                msvcrt.locking(fd, msvcrt.LK_UNLCK, 1)
            except OSError:
                pass
        os.close(fd)


def _journal_directory(directory: Path) -> Path:
    return directory / JOURNAL_DIRECTORY


def _journal_path(directory: Path, transaction_id: str) -> Path:
    if not transaction_id or any(ch not in "0123456789abcdefABCDEF-" for ch in transaction_id):
        raise ValueError("invalid Live Photo transaction identifier")
    return _journal_directory(directory) / f"{transaction_id}.json"


def _write_manifest(directory: Path, manifest: TransactionManifest) -> Path:
    journal_dir = _journal_directory(directory)
    journal_dir.mkdir(parents=True, exist_ok=True)
    destination = _journal_path(directory, manifest.transaction_id)
    temporary = journal_dir / f".{manifest.transaction_id}.{uuid.uuid4().hex}.tmp"
    payload = (json.dumps(manifest.to_wire(), sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")
    try:
        with temporary.open("xb") as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, destination)
        _fsync_directory(journal_dir)
        return destination
    finally:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass


def _load_manifest(path: Path) -> TransactionManifest:
    raw = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(raw, dict):
        raise ValueError("Live Photo transaction manifest must be an object")
    manifest = TransactionManifest.from_wire(raw)
    if manifest.schema_version != SCHEMA_VERSION:
        raise ValueError(f"unsupported Live Photo transaction schema: {manifest.schema_version}")
    if manifest.state not in _STATE_ORDER:
        raise ValueError(f"invalid Live Photo transaction state: {manifest.state}")
    return manifest


def _remove_if_exists(path: Path) -> None:
    try:
        path.unlink()
    except FileNotFoundError:
        pass


def _rollback(directory: Path, manifest: TransactionManifest, journal_path: Path) -> None:
    final_image = _safe_child(directory, manifest.final_image)
    final_video = _safe_child(directory, manifest.final_video)
    temp_image = _safe_child(directory, manifest.temporary_image)
    temp_video = _safe_child(directory, manifest.temporary_video)
    image_backup = _safe_child(directory, manifest.backup_image)
    video_backup = _safe_child(directory, manifest.backup_video)

    # Backup existence is stronger evidence than the persisted state because a crash can occur
    # between a rename and the following journal update.
    if image_backup.exists():
        _remove_if_exists(final_image)
        os.replace(image_backup, final_image)
    elif not manifest.had_image and not temp_image.exists() and final_image.exists():
        _remove_if_exists(final_image)

    if video_backup.exists():
        _remove_if_exists(final_video)
        os.replace(video_backup, final_video)
    elif not manifest.had_video and not temp_video.exists() and final_video.exists():
        _remove_if_exists(final_video)

    _remove_if_exists(temp_image)
    _remove_if_exists(temp_video)
    _remove_if_exists(image_backup)
    _remove_if_exists(video_backup)
    _fsync_directory(directory)
    _remove_if_exists(journal_path)
    _fsync_directory(journal_path.parent)


def _cleanup_committed(directory: Path, manifest: TransactionManifest, journal_path: Path) -> None:
    _remove_if_exists(_safe_child(directory, manifest.backup_image))
    _remove_if_exists(_safe_child(directory, manifest.backup_video))
    _remove_if_exists(_safe_child(directory, manifest.temporary_image))
    _remove_if_exists(_safe_child(directory, manifest.temporary_video))
    _fsync_directory(directory)
    # Journal removal is deliberately last. Until then, a crash simply re-enters this cleanup path.
    _remove_if_exists(journal_path)
    _fsync_directory(journal_path.parent)


def _mark_committed(directory: Path, manifest: TransactionManifest) -> tuple[TransactionManifest, Path]:
    committed = replace(manifest, state="committed")
    journal_path = _write_manifest(directory, committed)
    return committed, journal_path


def _recover_locked(directory: Path, pair_validator: PairValidator | None) -> None:
    journal_dir = _journal_directory(directory)
    if not journal_dir.is_dir():
        return
    for journal_path in sorted(journal_dir.glob("*.json")):
        manifest = _load_manifest(journal_path)
        if manifest.state == "committed":
            _cleanup_committed(directory, manifest, journal_path)
            continue

        final_image = _safe_child(directory, manifest.final_image)
        final_video = _safe_child(directory, manifest.final_video)
        if (
            manifest.state == "pair_installed"
            and pair_validator is not None
            and final_image.is_file()
            and final_video.is_file()
            and pair_validator(final_image, final_video)
        ):
            manifest, journal_path = _mark_committed(directory, manifest)
            _cleanup_committed(directory, manifest, journal_path)
        else:
            _rollback(directory, manifest, journal_path)


def recover_transactions(directory: Path, pair_validator: PairValidator | None = None) -> None:
    directory = Path(directory)
    if not directory.exists():
        return
    with _directory_lock(directory):
        _recover_locked(directory, pair_validator)


def commit_pair(
    temporary_image: Path,
    temporary_video: Path,
    final_image: Path,
    final_video: Path,
    *,
    pair_validator: PairValidator | None = None,
) -> None:
    """Install a validated Live Photo pair with durable crash recovery."""
    temporary_image = Path(temporary_image)
    temporary_video = Path(temporary_video)
    final_image = Path(final_image)
    final_video = Path(final_video)
    directory = final_image.parent
    if final_video.parent != directory or temporary_image.parent != directory or temporary_video.parent != directory:
        raise ValueError("Live Photo transaction resources must be on the destination directory/filesystem")
    if not temporary_image.is_file() or not temporary_video.is_file():
        raise FileNotFoundError("validated Live Photo temporary pair is incomplete")

    directory.mkdir(parents=True, exist_ok=True)
    with _directory_lock(directory):
        _recover_locked(directory, pair_validator)
        transaction_id = uuid.uuid4().hex
        image_backup = f".{final_image.name}.{transaction_id}.backup"
        video_backup = f".{final_video.name}.{transaction_id}.backup"
        manifest = TransactionManifest(
            schema_version=SCHEMA_VERSION,
            transaction_id=transaction_id,
            state="prepared",
            final_image=final_image.name,
            final_video=final_video.name,
            temporary_image=temporary_image.name,
            temporary_video=temporary_video.name,
            backup_image=image_backup,
            backup_video=video_backup,
            had_image=final_image.exists(),
            had_video=final_video.exists(),
        )
        _fsync_file(temporary_image)
        _fsync_file(temporary_video)
        journal_path = _write_manifest(directory, manifest)
        try:
            if manifest.had_image:
                os.replace(final_image, directory / image_backup)
            if manifest.had_video:
                os.replace(final_video, directory / video_backup)
            manifest = replace(manifest, state="originals_backed_up")
            _write_manifest(directory, manifest)

            os.replace(temporary_image, final_image)
            manifest = replace(manifest, state="image_installed")
            _write_manifest(directory, manifest)

            os.replace(temporary_video, final_video)
            manifest = replace(manifest, state="pair_installed")
            journal_path = _write_manifest(directory, manifest)
            _fsync_directory(directory)

            if pair_validator is not None and not pair_validator(final_image, final_video):
                raise ValueError("installed Live Photo pair failed final validation")

            manifest, journal_path = _mark_committed(directory, manifest)
            _cleanup_committed(directory, manifest, journal_path)
        except BaseException:
            try:
                if manifest.state == "committed":
                    _cleanup_committed(directory, manifest, journal_path)
                else:
                    _rollback(directory, manifest, journal_path)
            except Exception:
                pass
            raise
