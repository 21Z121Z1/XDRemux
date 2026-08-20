#!/usr/bin/env python3
"""Prepare and train the universal-image Photographic Style state model."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from xdremux_py.universal_photographic_style_training import (
    UniversalPreparationConfig,
    UniversalTrainingConfig,
    prepare_universal_dataset,
    train_universal_model,
)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    prepare = commands.add_parser("prepare")
    prepare.add_argument("--native-manifest", required=True, type=Path)
    prepare.add_argument("--output", required=True, type=Path)
    prepare.add_argument("--exiftool", default="exiftool")
    train = commands.add_parser("train")
    train.add_argument("--manifest", required=True, type=Path)
    train.add_argument("--output", required=True, type=Path)
    train.add_argument("--epochs", type=int, default=40)
    train.add_argument("--batch-size", type=int, default=6)
    train.add_argument("--learning-rate", type=float, default=2e-4)
    train.add_argument("--device", choices=("auto", "mps", "cpu"), default="auto")
    train.add_argument("--seed", type=int, default=260820)
    train.add_argument("--metadata-dropout", type=float, default=0.25)
    train.add_argument(
        "--architecture",
        choices=("base", "multiscale_large"),
        default="base",
    )
    train.add_argument("--consumer-weight", type=float, default=0.0)
    train.add_argument("--resume", type=Path)
    args = parser.parse_args()
    if args.command == "prepare":
        result = prepare_universal_dataset(
            UniversalPreparationConfig(
                native_manifest=args.native_manifest,
                output=args.output,
                exiftool=args.exiftool,
            )
        )
    else:
        result = train_universal_model(
            UniversalTrainingConfig(
                manifest=args.manifest,
                output=args.output,
                epochs=args.epochs,
                batch_size=args.batch_size,
                learning_rate=args.learning_rate,
                device=args.device,
                seed=args.seed,
                metadata_dropout=args.metadata_dropout,
                architecture=args.architecture,
                consumer_weight=args.consumer_weight,
                resume=args.resume,
            )
        )
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
