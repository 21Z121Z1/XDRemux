"""Cross-platform Motion Photo -> Apple Live Photo conversion orchestration."""

from __future__ import annotations

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
from .live_photo_publish import publish_pair, reconcile_pair
from .live_photo_still_portable import (
    LivePhotoStillError,
    read_apple_content_identifier,
    write_live_photo_still,
)
from .motion_photo import MotionPhotoError, copy_range, parse_motion_photo, primary_video_range
from .motion_video import MotionVideoError, strip_trailing_vendor_data


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
    """Preserve the basename while never claiming an existing HEIC/MOV namespace."""
    input_path = Path(input_path)
    base = input_path.with_suffix(".heic")
    sequence = 1
    while True:
        candidate = (
            base
            if sequence == 1
            else base.with_name(f"{base.stem} ({sequence}){base.suffix}")
        )
        video = companion_video_path(candidate)
        if (
            candidate.resolve() != input_path.resolve()
            and not candidate.exists()
            and not video.exists()
        ):
            return candidate
        sequence += 1


def companion_video_path(image_path: Path) -> Path:
    return image_path.with_suffix(".mov")


def validate_pair(image: Path, video: Path, content_identifier: str, still_time_seconds: float) -> None:
    image_identifier = read_apple_content_identifier(image)
    if image_identifier is None or image_identifier.upper() != content_identifier.upper():
        raise LivePhotoConversionError("HEIC Apple MakerNote ContentIdentifier mismatch")
    movie_identifier = read_movie_identifier(video)
    if movie_identifier is None or movie_identifier.upper() != content_identifier.upper():
        raise LivePhotoConversionError("MOV QuickTime ContentIdentifier mismatch")
    validate_live_photo_movie(video, movie_identifier, still_time_seconds)


def existing_pair_matches_identifier(image: Path, video: Path, content_identifier: str) -> bool:
    image, video = Path(image), Path(video)
    if not image.is_file() or not video.is_file():
        return False
    try:
        image_identifier = read_apple_content_identifier(image)
        movie_identifier = read_movie_identifier(video)
        still_time = read_still_time(video)
        expected = content_identifier.upper()
        if not image_identifier or not movie_identifier or still_time is None:
            return False
        if image_identifier.upper() != expected or movie_identifier.upper() != expected:
            return False
        validate_live_photo_movie(video, movie_identifier, still_time)
        return True
    except (OSError, ValueError, LivePhotoMovieError, LivePhotoStillError):
        return False


def existing_pair_is_valid(image: Path, video: Path) -> bool:
    image, video = Path(image), Path(video)
    if not image.is_file() or not video.is_file():
        return False
    try:
        image_identifier = read_apple_content_identifier(image)
        if not image_identifier:
            return False
        return existing_pair_matches_identifier(image, video, image_identifier)
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

    output_was_explicit = output_image is not None
    output_image = Path(output_image) if output_was_explicit else default_output_image(input_path)
    if output_image.suffix.lower() not in {".heic", ".heif"}:
        raise LivePhotoConversionError("Live Photo still output must use .heic or .heif")
    if output_image.resolve() == input_path.resolve():
        raise LivePhotoConversionError("Motion Photo conversion never overwrites the source image")
    output_video = companion_video_path(output_image)
    if output_was_explicit and (output_image.exists() or output_video.exists()):
        raise LivePhotoConversionError(
            "explicit Live Photo output HEIC/MOV already exists; "
            "refusing to overwrite an output pair with unknown provenance"
        )
    output_directory = output_image.parent
    output_directory.mkdir(parents=True, exist_ok=True)

    try:
        reconcile_pair(output_image, output_video, existing_pair_is_valid)
    except (OSError, ValueError) as exc:
        raise LivePhotoConversionError(f"could not reconcile prior Live Photo output: {exc}") from exc

    content_identifier = str(uuid.uuid4()).upper()
    publication_id = uuid.uuid4().hex
    stem = output_image.stem
    # Final-pair temporaries live beside the destination so each publication rename stays on the
    # same filesystem. Interrupted outputs are discarded and rebuilt on the next conversion.
    temp_image = output_directory / f".{stem}.{publication_id}.tmp.heic"
    temp_video = output_directory / f".{stem}.{publication_id}.tmp.mov"

    scratch = Path(tempfile.mkdtemp(prefix="xdremux-py-livephoto-"))
    try:
        video_source = scratch / "motion.mp4"
        copy_range(input_path, primary_video_range(asset), video_source)
        try:
            removed_vendor_bytes = strip_trailing_vendor_data(video_source)
            still_time = resolve_still_time(video_source, asset.presentation_timestamp_us)
        except (LivePhotoMovieError, MotionVideoError) as exc:
            raise LivePhotoConversionError(str(exc)) from exc

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
        if removed_vendor_bytes:
            diagnostics.append(
                f"removed {removed_vendor_bytes} trailing OPPO vendor bytes after the complete Stream 1 BMFF container"
            )
        if asset.source_kind == "androidHeifMotionPhotoV1":
            diagnostics.append("HEIF mpvd video extracted without trailing vendor boxes")

        try:
            publish_pair(temp_image, temp_video, output_image, output_video)
        except (OSError, ValueError) as exc:
            raise LivePhotoConversionError(f"Live Photo pair publication failed: {exc}") from exc

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
        try:
            temp_image.unlink()
        except FileNotFoundError:
            pass
        try:
            temp_video.unlink()
        except FileNotFoundError:
            pass
        shutil.rmtree(scratch, ignore_errors=True)
