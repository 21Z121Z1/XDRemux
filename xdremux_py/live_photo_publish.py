"""Publish validated Apple Live Photo resource pairs with simple retry semantics.

The source Motion Photo is immutable. Final HEIC/MOV resources are derived outputs: ordinary
publication errors roll back in-process, while an interrupted process is handled on the next run by
removing stale publication files and rebuilding any incomplete or invalid pair.
"""

from __future__ import annotations

import os
import shutil
import uuid
from pathlib import Path
from typing import Callable

PairValidator = Callable[[Path, Path], bool]


def reconcile_pair(image: Path, video: Path, pair_validator: PairValidator) -> None:
    image = Path(image)
    video = Path(video)
    directory = image.parent
    if video.parent != directory:
        raise ValueError("Live Photo resources must share one destination directory")
    directory.mkdir(parents=True, exist_ok=True)

    _remove_stale_artifacts(image, video)

    image_exists = image.is_file()
    video_exists = video.is_file()
    if not image_exists and not video_exists:
        return
    if image_exists and video_exists and pair_validator(image, video):
        return
    _unlink_if_exists(image)
    _unlink_if_exists(video)


def publish_pair(
    temporary_image: Path,
    temporary_video: Path,
    final_image: Path,
    final_video: Path,
) -> None:
    temporary_image = Path(temporary_image)
    temporary_video = Path(temporary_video)
    final_image = Path(final_image)
    final_video = Path(final_video)
    directory = final_image.parent
    if (
        final_video.parent != directory
        or temporary_image.parent != directory
        or temporary_video.parent != directory
    ):
        raise ValueError("Live Photo publication resources must be on the destination directory/filesystem")
    if not temporary_image.is_file() or not temporary_video.is_file():
        raise FileNotFoundError("validated Live Photo temporary pair is incomplete")

    backup_id = uuid.uuid4().hex
    image_backup = directory / f".{final_image.name}.{backup_id}.backup"
    video_backup = directory / f".{final_video.name}.{backup_id}.backup"
    had_image = final_image.exists()
    had_video = final_video.exists()
    image_installed = False
    video_installed = False

    try:
        if had_image:
            os.replace(final_image, image_backup)
        if had_video:
            os.replace(final_video, video_backup)
        os.replace(temporary_image, final_image)
        image_installed = True
        os.replace(temporary_video, final_video)
        video_installed = True
    except BaseException:
        if image_installed:
            _unlink_if_exists(final_image)
        if video_installed:
            _unlink_if_exists(final_video)
        if had_image and image_backup.exists():
            os.replace(image_backup, final_image)
        if had_video and video_backup.exists():
            os.replace(video_backup, final_video)
        raise

    # Both final names are installed. Cleanup is best-effort; reconcile_pair() removes leftovers on
    # the next run, so a cleanup failure must not invalidate a successfully published pair.
    for backup in (image_backup, video_backup):
        try:
            _unlink_if_exists(backup)
        except OSError:
            pass


def _remove_stale_artifacts(image: Path, video: Path) -> None:
    directory = image.parent

    # One-time cleanup when upgrading from the former journal-based publisher. The journal is not
    # interpreted; final outputs are validated below and rebuilt from the immutable source if needed.
    shutil.rmtree(directory / ".xdremux-live-photo-transactions", ignore_errors=True)
    _unlink_if_exists(directory / ".xdremux-live-photo-transactions.lock")

    stem = image.stem
    for entry in directory.iterdir():
        name = entry.name
        is_backup = (
            (name.startswith(f".{image.name}.") or name.startswith(f".{video.name}."))
            and name.endswith(".backup")
        )
        is_temporary = name.startswith(f".{stem}.") and (
            name.endswith(".tmp.heic") or name.endswith(".tmp.mov")
        )
        if is_backup or is_temporary:
            if entry.is_dir():
                shutil.rmtree(entry)
            else:
                _unlink_if_exists(entry)


def _unlink_if_exists(path: Path) -> None:
    try:
        path.unlink()
    except FileNotFoundError:
        pass
