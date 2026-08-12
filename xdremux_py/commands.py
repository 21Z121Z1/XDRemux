"""Typed command models built from parsed CLI arguments."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path

from .pipeline import ConversionConfiguration

DEFAULT_BATCH_GLOB = "*.heic"


@dataclass(frozen=True)
class ConvertCommand:
    input_path: Path
    output_path: Path
    output_explicit: bool
    configuration: ConversionConfiguration

    @classmethod
    def from_namespace(cls, args: argparse.Namespace) -> "ConvertCommand":
        input_path = Path(args.input)
        return cls(
            input_path=input_path,
            output_path=Path(args.output) if args.output else input_path,
            output_explicit=bool(args.output),
            configuration=conversion_configuration(args),
        )


@dataclass(frozen=True)
class BatchCommand:
    input_dir: Path
    output_dir: Path
    glob: str
    glob_explicit: bool
    categorize_output: bool
    configuration: ConversionConfiguration

    @classmethod
    def from_namespace(cls, args: argparse.Namespace) -> "BatchCommand":
        input_dir = Path(args.input_dir)
        return cls(
            input_dir=input_dir,
            output_dir=Path(args.output_dir) if args.output_dir else input_dir,
            glob=args.glob or DEFAULT_BATCH_GLOB,
            glob_explicit=bool(args.glob),
            categorize_output=bool(getattr(args, "categorize_output", False)),
            configuration=conversion_configuration(args),
        )


@dataclass(frozen=True)
class CategorizeCommand:
    input_paths: list[Path]
    output_dir: Path
    jobs: int
    dry_run: bool

    @classmethod
    def from_namespace(cls, args: argparse.Namespace) -> "CategorizeCommand":
        return cls(
            input_paths=[Path(value) for value in args.input],
            output_dir=Path(args.output_dir),
            jobs=args.jobs,
            dry_run=args.dry_run,
        )


def conversion_configuration(args: argparse.Namespace) -> ConversionConfiguration:
    return ConversionConfiguration(
        oppo_compat=bool(args.oppo_compat),
        passthrough=bool(getattr(args, "passthrough", True)),
        reencode=bool(getattr(args, "reencode", False)),
        debug_dir=Path(args.debug_dir) if args.debug_dir else None,
    )
