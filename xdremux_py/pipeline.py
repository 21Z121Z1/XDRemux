"""ProXDR to ISO 21496-1 conversion pipeline shared by the Python CLI.

This module owns conversion behavior the way ``XDRemuxCore`` does on the Swift
side: it returns structured results and raises :class:`ConversionError` instead
of printing progress or exit codes. Terminal output belongs to the CLI layer.
"""

from __future__ import annotations

import io
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable

from . import categorize, container, edr, iso21496


class ConversionError(Exception):
    """A conversion failed for a reason the caller is expected to report."""


@dataclass(frozen=True)
class ConversionConfiguration:
    """Conversion switches resolved from CLI arguments."""

    oppo_compat: bool = False
    passthrough: bool = True
    reencode: bool = False
    debug_dir: Path | None = None

    @property
    def preserves_source_base_image(self) -> bool:
        """Remux the source HEVC base image unless a re-encode was requested."""
        return False if self.reencode else self.passthrough


@dataclass(frozen=True)
class ConversionAnalysis:
    """Container metadata and HDR parameters read from one input file."""

    input_path: Path
    lhdr: container.ExtractedLHDR
    edr_scale: float
    iso_meta: dict[str, Any]

    @property
    def mode(self) -> str:
        return self.lhdr.mode

    @property
    def gain_map_max(self) -> float:
        return self.iso_meta["gainMapMax"][0]

    @property
    def hdr_capacity_max(self) -> float:
        return self.iso_meta["hdrCapacityMax"]


class MissingImagingDependencies(ConversionError):
    """pillow-heif, Pillow, or numpy is unavailable, so nothing was written.

    Carries the completed analysis: the parameters are known even though no
    output file exists, and the CLI reports them before failing.
    """

    def __init__(self, message: str, analysis: ConversionAnalysis) -> None:
        super().__init__(message)
        self.analysis = analysis


@dataclass(frozen=True)
class ConversionResult:
    """Outcome of one successful conversion."""

    analysis: ConversionAnalysis
    output_path: Path

    @property
    def input_path(self) -> Path:
        return self.analysis.input_path

    @property
    def overwritten_in_place(self) -> bool:
        return self.output_path == self.analysis.input_path


@dataclass(frozen=True)
class BatchEntry:
    """One planned input/output pair of a batch run."""

    input_path: Path
    output_path: Path


def analyze(input_path: Path) -> ConversionAnalysis:
    """Extract container metadata and resolve the ISO 21496-1 parameters."""
    if not input_path.exists():
        raise ConversionError(f"input not found: {input_path}")

    try:
        lhdr = container.extract_lhdr(str(input_path))
    except Exception as exc:
        raise ConversionError(str(exc)) from exc

    try:
        if lhdr.mode == "uhdr":
            iso_meta = iso21496.build_iso21496_metadata_from_uhdr(lhdr.meta_floats)
            edr_scale = iso_meta.get("scale", 1.0)
        else:
            edr_scale = edr.edr_scale_calculator(list(lhdr.meta_floats))
            iso_meta = iso21496.build_iso21496_metadata(edr_scale)
    except (ValueError, OverflowError) as exc:
        raise ConversionError(
            f"invalid HDR metadata in {input_path.name}: {exc}"
        ) from exc

    return ConversionAnalysis(
        input_path=input_path,
        lhdr=lhdr,
        edr_scale=edr_scale,
        iso_meta=iso_meta,
    )


def encode(
    analysis: ConversionAnalysis,
    output_path: Path,
    config: ConversionConfiguration,
) -> ConversionResult:
    """Write the ISO 21496-1 output for an analyzed input.

    Raises :class:`MissingImagingDependencies` when the imaging stack is
    unavailable, and :class:`ConversionError` when the source lacks the data
    the conversion needs.
    """
    try:
        from . import heif_io

        base_image = None
        exif_data = None
        passthrough = config.preserves_source_base_image

        if not passthrough:
            from pillow_heif import open_heif

            base_image = heif_io.read_heic(str(analysis.input_path))["base_image"]

            # Extract source EXIF for normal-mode re-encode.
            # Passthrough copies the original EXIF item at the ISOBMFF layer.
            try:
                src_heif = open_heif(str(analysis.input_path))
                exif_data = (
                    src_heif[0].info.get("exif")
                    if hasattr(src_heif, "__getitem__")
                    else src_heif.info.get("exif")
                )
            except Exception:
                pass

            if base_image is None:
                raise ConversionError(
                    "HEIC decode failed — install pillow-heif for full conversion"
                )

        gain_map = _resolve_gain_map(analysis)

        if passthrough:
            heif_io.write_heic_passthrough(
                str(analysis.input_path),
                str(output_path),
                gain_map,
                analysis.iso_meta,
                lhdr=analysis.lhdr,
                oppo_compat=config.oppo_compat,
            )
        else:
            heif_io.write_heic(
                str(output_path),
                base_image,
                gain_map,
                analysis.iso_meta,
                oppo_compat=config.oppo_compat,
                lhdr=analysis.lhdr,
                exif_data=exif_data,
            )

        if config.debug_dir is not None:
            _write_debug_manifest(analysis, config.debug_dir)
    except ImportError as exc:
        raise MissingImagingDependencies(
            f"conversion requires pillow-heif + Pillow + numpy ({exc}); "
            "no output was written",
            analysis,
        ) from exc

    return ConversionResult(analysis=analysis, output_path=output_path)


def convert_file(
    input_path: Path,
    output_path: Path,
    config: ConversionConfiguration,
    *,
    on_analysis: Callable[[ConversionAnalysis], None] | None = None,
) -> ConversionResult:
    """Analyze and convert one file.

    ``on_analysis`` is invoked once the HDR parameters are known and before
    encoding starts, so a caller can report them even if encoding then fails.
    """
    if not input_path.exists():
        raise ConversionError(f"input not found: {input_path}")

    if output_path != input_path:
        output_path.parent.mkdir(parents=True, exist_ok=True)

    analysis = analyze(input_path)
    if on_analysis is not None:
        on_analysis(analysis)
    return encode(analysis, output_path, config)


def plan_batch(
    input_dir: Path,
    output_dir: Path,
    glob: str = "*.heic",
    categorize_output: bool = False,
) -> list[BatchEntry]:
    """Resolve the input/output pairs of a batch run.

    Creates ``output_dir`` when it differs from the input directory so the
    planned destinations are writable.
    """
    if not input_dir.is_dir():
        raise ConversionError(f"input dir not found: {input_dir}")

    if output_dir != input_dir:
        output_dir.mkdir(parents=True, exist_ok=True)

    files = sorted(input_dir.glob(glob))
    destinations = (
        categorize.batch_destinations(files, output_dir) if categorize_output else {}
    )
    return [
        BatchEntry(
            input_path=path,
            output_path=destinations.get(path, output_dir / path.name),
        )
        for path in files
    ]


def _resolve_gain_map(analysis: ConversionAnalysis):
    """Decode the UHDR gain map, or reconstruct the LHDR one from its mask."""
    from PIL import Image

    lhdr = analysis.lhdr
    if lhdr.mode == "uhdr":
        gain_map = None
        if lhdr.gainmap_data:
            try:
                gain_map = Image.open(io.BytesIO(lhdr.gainmap_data))
            except Exception:
                gain_map = None
        if gain_map is None:
            raise ConversionError("UHDR gain map JPEG is missing or undecodable")
        return gain_map

    if lhdr.mask_data is None:
        raise ConversionError("no mask data found")

    import numpy as np

    from . import gainmap

    mask = np.array(Image.open(io.BytesIO(lhdr.mask_data)).convert("L"))
    return gainmap.reconstruct(mask, analysis.edr_scale, lhdr.meta_floats[0])


def _write_debug_manifest(analysis: ConversionAnalysis, debug_root: Path) -> None:
    debug_dir = debug_root / analysis.input_path.stem
    debug_dir.mkdir(parents=True, exist_ok=True)
    debug = {
        "input": str(analysis.input_path),
        "mode": analysis.mode,
        "edr_scale": analysis.edr_scale,
        "iso_meta": analysis.iso_meta,
        "floats": list(analysis.lhdr.meta_floats),
    }
    (debug_dir / "meta.json").write_text(json.dumps(debug, indent=2))
