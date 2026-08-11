"""Cross-platform Motion Photo -> Apple Live Photo conversion orchestration."""

from __future__ import annotations

import os
import shutil
import tempfile
import uuid
from dataclasses import dataclass
from pathlib import Path

from .live_photo_mov import (
    LivePhotoMovieError,
    media_payload_sha256,
    read_content_identifier as read_movie_identifier,
    read_still_time,
    resolve_still_time,
    validate_live_photo_movie,
    write_live_photo_movie,
)
from .live_photo_still import LivePhotoStillError, read_apple_content_identifier, write_live_photo_still
from .motion_photo import MotionPhotoError, copy_range, parse_motion_photo, primary_video_range


class LivePhotoConversionError(ValueError):
    pass


@dataclass(frozen=True)
class LivePhotoResult:
    input_path: Path
    image_path: Path
    video_path: Path
    content_identifier: str
    still_time_seconds: float
    source_kind: str
    source_had_gain_map: bool
    diagnostics: tuple[str, ...] = ()


def is_motion_photo(path: Path) -> bool:
    try:
        return parse_motion_photo(path) is not None
    except (OSError, MotionPhotoError):
        return False


def default_output_image(input_path: Path) -> Path:
    if input_path.suffix.lower() in {".heic", ".heif"}:
        return input_path.with_name(input_path.stem + ".live.heic")
    return input_path.with_suffix(".heic")


def companion_video_path(image_path: Path) -> Path:
    return image_path.with_suffix(".mov")


def _transactional_commit(temp_image: Path, temp_video: Path, image: Path, video: Path) -> None:
    image.parent.mkdir(parents=True, exist_ok=True)
    backup_id = uuid.uuid4().hex
    image_backup = image.with_name(f".{image.name}.{backup_id}.backup")
    video_backup = video.with_name(f".{video.name}.{backup_id}.backup")
    had_image, had_video = image.exists(), video.exists()
    image_installed = video_installed = False
    try:
        if had_image:
            os.replace(image, image_backup)
        if had_video:
            os.replace(video, video_backup)
        os.replace(temp_image, image)
        image_installed = True
        os.replace(temp_video, video)
        video_installed = True
        if image_backup.exists():
            image_backup.unlink()
        if video_backup.exists():
            video_backup.unlink()
    except Exception:
        if image_installed and image.exists():
            image.unlink()
        if video_installed and video.exists():
            video.unlink()
        if image_backup.exists():
            os.replace(image_backup, image)
        if video_backup.exists():
            os.replace(video_backup, video)
        raise


def validate_pair(image: Path, video: Path, content_identifier: str, still_time_seconds: float) -> None:
    image_identifier = read_apple_content_identifier(image)
    if image_identifier is None or image_identifier.upper() != content_identifier.upper():
        raise LivePhotoConversionError("HEIC Apple MakerNote ContentIdentifier mismatch")
    movie_identifier = read_movie_identifier(video)
    if movie_identifier is None or movie_identifier.upper() != content_identifier.upper():
        raise LivePhotoConversionError("MOV QuickTime ContentIdentifier mismatch")
    validate_live_photo_movie(video, movie_identifier, still_time_seconds)


def existing_pair_is_valid(image: Path, video: Path) -> bool:
    image, video = Path(image), Path(video)
    if not image.is_file() or not video.is_file():
        return False
    try:
        image_identifier = read_apple_content_identifier(image)
        movie_identifier = read_movie_identifier(video)
        still_time = read_still_time(video)
        if not image_identifier or not movie_identifier or still_time is None:
            return False
        if image_identifier.upper() != movie_identifier.upper():
            return False
        validate_live_photo_movie(video, movie_identifier, still_time)
        return True
    except (OSError, ValueError, LivePhotoMovieError, LivePhotoStillError):
        return False


def convert_motion_photo(input_path: Path, output_image: Path | None = None) -> LivePhotoResult:
    input_path = Path(input_path)
    if not input_path.is_file():
        raise LivePhotoConversionError(f"input not found: {input_path}")
    try:
        asset = parse_motion_photo(input_path)
    except (OSError, MotionPhotoError) as exc:
        raise LivePhotoConversionError(str(exc)) from exc
    if asset is None:
        raise LivePhotoConversionError("input is not a supported Motion Photo")

    output_image = Path(output_image) if output_image is not None else default_output_image(input_path)
    if output_image.suffix.lower() not in {".heic", ".heif"}:
        raise LivePhotoConversionError("Live Photo still output must use .heic or .heif")
    if output_image.resolve() == input_path.resolve():
        raise LivePhotoConversionError("Motion Photo conversion never overwrites the source image")
    output_video = companion_video_path(output_image)
    content_identifier = str(uuid.uuid4()).upper()

    scratch = Path(tempfile.mkdtemp(prefix="xdremux-py-livephoto-"))
    try:
        video_source = scratch / "motion.mp4"
        copy_range(input_path, primary_video_range(asset), video_source)
        try:
            still_time = resolve_still_time(video_source, asset.presentation_timestamp_us)
        except LivePhotoMovieError as exc:
            raise LivePhotoConversionError(str(exc)) from exc

        temp_image = scratch / "pair.heic"
        temp_video = scratch / "pair.mov"
        try:
            had_gain_map = write_live_photo_still(asset, temp_image, content_identifier)
            source_media_hashes = media_payload_sha256(video_source)
            if not source_media_hashes:
                raise LivePhotoConversionError("Motion Photo video contains no media-data payload")
            write_live_photo_movie(
                video_source,
                temp_video,
                content_identifier,
                still_time,
                oppo_metadata=asset.vendor_metadata,
            )
            validate_pair(temp_image, temp_video, content_identifier, still_time)
            if media_payload_sha256(temp_video) != source_media_hashes:
                raise LivePhotoConversionError("compressed video/audio media payload changed during MOV write")
        except (LivePhotoStillError, LivePhotoMovieError) as exc:
            raise LivePhotoConversionError(str(exc)) from exc

        diagnostics: list[str] = []
        metadata = asset.vendor_metadata
        if (
            asset.presentation_timestamp_us is not None
            and metadata is not None
            and metadata.cover_frame_pts_us is not None
            and asset.presentation_timestamp_us != metadata.cover_frame_pts_us
        ):
            diagnostics.append(
                "Android XMP still time and OPPO coverFramePts differ; "
                f"selected {asset.presentation_source or 'timeline'}"
            )
        if metadata is not None and metadata.stream_count >= 2:
            diagnostics.append("OPPO dual-stream input detected; selected Stream 1 for Apple paired video")
        if asset.source_kind == "androidHeifMotionPhotoV1":
            diagnostics.append("HEIF mpvd video extracted without trailing vendor boxes")

        _transactional_commit(temp_image, temp_video, output_image, output_video)
        return LivePhotoResult(
            input_path=input_path,
            image_path=output_image,
            video_path=output_video,
            content_identifier=content_identifier,
            still_time_seconds=still_time,
            source_kind=asset.source_kind,
            source_had_gain_map=had_gain_map,
            diagnostics=tuple(diagnostics),
        )
    finally:
        shutil.rmtree(scratch, ignore_errors=True)
