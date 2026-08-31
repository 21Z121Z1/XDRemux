#!/usr/bin/env python3
"""Collect licensed public photos and run synthetic key1 pretraining."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from xdremux_py.public_style_pretraining import (
    DEFAULT_CATEGORIES,
    PublicPretrainingConfig,
    collect_public_corpus,
    pretrain_public_synthetic_style,
)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    collect = commands.add_parser("collect")
    collect.add_argument("--output", required=True, type=Path)
    collect.add_argument("--image-directory", required=True, type=Path)
    collect.add_argument("--category", action="append", dest="categories")
    collect.add_argument("--per-category", type=int, default=3)
    collect.add_argument("--seed", type=int, default=260829)
    train = commands.add_parser("train")
    train.add_argument("--manifest", required=True, type=Path)
    train.add_argument("--output", required=True, type=Path)
    train.add_argument("--epochs", type=int, default=1)
    train.add_argument("--batch-size", type=int, default=2)
    train.add_argument("--learning-rate", type=float, default=2e-4)
    train.add_argument("--transforms-per-image", type=int, default=2)
    train.add_argument("--key-loss-weight", type=float, default=8.0)
    train.add_argument("--unstyled-loss-weight", type=float, default=0.1)
    train.add_argument("--device", choices=("cpu", "mps"), default="cpu")
    train.add_argument("--seed", type=int, default=260829)
    args = parser.parse_args()
    if args.command == "collect":
        result = collect_public_corpus(
            args.output,
            args.image_directory,
            categories=args.categories or DEFAULT_CATEGORIES,
            per_category=args.per_category,
            seed=args.seed,
        )
    else:
        result = pretrain_public_synthetic_style(
            PublicPretrainingConfig(
                manifest=args.manifest,
                output=args.output,
                epochs=args.epochs,
                batch_size=args.batch_size,
                learning_rate=args.learning_rate,
                transforms_per_image=args.transforms_per_image,
                key_loss_weight=args.key_loss_weight,
                unstyled_loss_weight=args.unstyled_loss_weight,
                device=args.device,
                seed=args.seed,
            )
        )
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
