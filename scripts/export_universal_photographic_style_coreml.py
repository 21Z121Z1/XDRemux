#!/usr/bin/env python3
"""Export the universal Photographic Style state network as a Core ML program."""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path
from typing import Any

import numpy as np

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from xdremux_py.apple_reverse_key1_training import _atomic_json, _require_torch, sha256_file
from xdremux_py.universal_photographic_style import load_universal_image, load_universal_model
from xdremux_py.universal_photographic_style_training import METADATA_FIELDS, PRIMARY_CHANNELS
from xdremux_py.universal_photographic_style_training import (
    GAIN_MAP_CHANNELS,
    INPUT_SIZE,
    LINEAR_CHANNELS,
    MODALITY_FIELDS,
)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkpoint", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--fixture", type=Path)
    parser.add_argument("--linear-rgb-sidecar", type=Path)
    parser.add_argument("--gain-map-sidecar", type=Path)
    args = parser.parse_args()
    output = args.output.resolve()
    if output.exists():
        raise FileExistsError(f"refusing to overwrite Core ML package: {output}")

    try:
        import coremltools as ct
    except ImportError as error:
        raise RuntimeError("coremltools is required for the isolated export step") from error
    torch, nn = _require_torch()
    model, checkpoint, _ = load_universal_model(args.checkpoint, "cpu")

    class TraceableSelfAttention(nn.Module):
        def __init__(self, source: Any) -> None:
            super().__init__()
            self.embed_dim = source.embed_dim
            self.num_heads = source.num_heads
            self.head_dim = self.embed_dim // self.num_heads
            self.in_proj_weight = nn.Parameter(source.in_proj_weight.detach().clone())
            self.in_proj_bias = nn.Parameter(source.in_proj_bias.detach().clone())
            self.out_proj = nn.Linear(self.embed_dim, self.embed_dim)
            self.out_proj.load_state_dict(source.out_proj.state_dict())

        def forward(self, query: Any, _key: Any, _value: Any, **_kwargs: Any) -> tuple[Any, None]:
            qkv = torch.nn.functional.linear(query, self.in_proj_weight, self.in_proj_bias)
            q, k, v = torch.chunk(qkv, 3, dim=-1)
            batch, tokens, _ = q.shape
            q = q.reshape(batch, tokens, self.num_heads, self.head_dim).transpose(1, 2)
            k = k.reshape(batch, tokens, self.num_heads, self.head_dim).transpose(1, 2)
            v = v.reshape(batch, tokens, self.num_heads, self.head_dim).transpose(1, 2)
            weights = torch.softmax(
                torch.matmul(q, k.transpose(-2, -1)) / self.head_dim**0.5,
                dim=-1,
            )
            attended = torch.matmul(weights, v).transpose(1, 2).reshape(
                batch, tokens, self.embed_dim
            )
            return self.out_proj(attended), None

    for block in model.context:
        block.self_attn = TraceableSelfAttention(block.self_attn)

    class FlattenedState(nn.Module):
        def __init__(self, source: Any) -> None:
            super().__init__()
            self.source = source
            self.register_buffer(
                "identity_flat", source.identity.detach().reshape(1, 12, 12, 8, 30)
            )
            self.register_buffer(
                "key1_scale_flat",
                source.key1_scale.detach().reshape(1, 1, 1, 8, 30),
            )

        def forward(
            self,
            features: Any,
            metadata: Any,
            metadata_mask: Any,
            linear_features: Any | None = None,
            gain_map_features: Any | None = None,
            modality_mask: Any | None = None,
        ) -> tuple[Any, ...]:
            value = features
            if self.source.supports_optional_modalities:
                linear_mask = modality_mask[:, 0, None, None, None]
                gain_mask = modality_mask[:, 1, None, None, None]
                mask_planes = modality_mask[:, :, None, None].expand(
                    -1, -1, INPUT_SIZE, INPUT_SIZE
                )
                value = torch.cat(
                    (
                        features,
                        linear_features * linear_mask,
                        gain_map_features * gain_mask,
                        mask_planes,
                    ),
                    dim=1,
                )
            pyramid = []
            for stage, projection in zip(
                self.source.encoder_stages, self.source.scale_projections
            ):
                value = stage(value)
                pyramid.append(
                    torch.nn.functional.interpolate(
                        projection(value),
                        size=(12, 12),
                        mode="bilinear",
                        align_corners=False,
                    )
                )
            spatial = self.source.spatial_fusion(torch.cat(pyramid, dim=1))
            active_mask = metadata_mask * self.source.metadata_active
            normalized_metadata = (
                (metadata - self.source.metadata_center) / self.source.metadata_scale
            ).clamp(-8.0, 8.0) * active_mask
            gamma, beta = self.source.metadata(
                torch.cat((normalized_metadata, active_mask), dim=-1)
            ).chunk(2, dim=-1)
            spatial = spatial * (1.0 + gamma[:, :, None, None])
            spatial = spatial + beta[:, :, None, None]
            tokens = spatial.flatten(2).transpose(1, 2)
            for block in self.source.context:
                tokens = block(tokens)
            spatial = tokens.transpose(1, 2).reshape(1, 256, 12, 12)
            global_feature = tokens.mean(dim=1)
            node = spatial.permute(0, 2, 3, 1)[:, :, :, None, :].expand(
                -1, -1, -1, 8, -1
            )
            plane = self.source.plane_embedding.weight.reshape(1, 1, 1, 8, 64)
            plane = plane.expand(1, 12, 12, -1, -1)
            key_residual = self.source.key_head(torch.cat((node, plane), dim=-1))
            key1 = self.identity_flat + key_residual * self.key1_scale_flat
            key1_log_variance = self.source.uncertainty_head(global_feature).clamp(
                -8.0, 6.0
            )
            gtc = self.source.gtc_center + self.source.gtc_head(
                global_feature
            ) * self.source.gtc_scale
            light_residual = self.source.light_head(
                torch.nn.functional.interpolate(
                    spatial, size=(32, 32), mode="bilinear", align_corners=False
                )
            )
            light = self.source.light_center[None] + light_residual * self.source.light_scale[
                None, :, None, None
            ]
            scalars = self.source.scalar_center + self.source.scalar_head(
                global_feature
            ) * self.source.scalar_scale
            scalars = torch.maximum(
                torch.minimum(scalars, self.source.scalar_high), self.source.scalar_low
            )
            return (
                key1.reshape(1, -1),
                key1_log_variance.reshape(1, -1),
                gtc.reshape(1, -1),
                light.reshape(1, -1),
                scalars.reshape(1, -1),
            )

    wrapper = FlattenedState(model).eval()
    if hasattr(torch.backends, "mha"):
        torch.backends.mha.set_fastpath_enabled(False)
    if args.fixture is not None:
        fixture = load_universal_image(
            args.fixture,
            linear_rgb_sidecar=args.linear_rgb_sidecar,
            gain_map_sidecar=args.gain_map_sidecar,
        )
        examples = (
            torch.from_numpy(fixture.primary).unsqueeze(0),
            torch.from_numpy(fixture.metadata).unsqueeze(0),
            torch.from_numpy(fixture.metadata_mask).unsqueeze(0),
        )
    else:
        fixture = None
        examples = (
            torch.zeros((1, PRIMARY_CHANNELS, 256, 256), dtype=torch.float32),
            torch.zeros((1, len(METADATA_FIELDS)), dtype=torch.float32),
            torch.zeros((1, len(METADATA_FIELDS)), dtype=torch.float32),
        )
    if model.supports_optional_modalities:
        linear = (
            fixture.linear_rgb_features
            if fixture is not None and fixture.linear_rgb_features is not None
            else np.zeros((LINEAR_CHANNELS, INPUT_SIZE, INPUT_SIZE), dtype=np.float32)
        )
        gain = (
            fixture.gain_map_features
            if fixture is not None and fixture.gain_map_features is not None
            else np.zeros((GAIN_MAP_CHANNELS, INPUT_SIZE, INPUT_SIZE), dtype=np.float32)
        )
        modality_mask = np.asarray(
            [
                fixture is not None and fixture.linear_rgb_features is not None,
                fixture is not None and fixture.gain_map_features is not None,
            ],
            dtype=np.float32,
        )
        examples = examples + (
            torch.from_numpy(linear).unsqueeze(0),
            torch.from_numpy(gain).unsqueeze(0),
            torch.from_numpy(modality_mask).unsqueeze(0),
        )
    with torch.no_grad():
        expected = [value.numpy() for value in wrapper(*examples)]
        traced = torch.jit.trace(wrapper, examples, strict=True)
    converted = ct.convert(
        traced,
        convert_to="mlprogram",
        inputs=[
            ct.TensorType(name="features", shape=examples[0].shape, dtype=np.float32),
            ct.TensorType(name="metadata", shape=examples[1].shape, dtype=np.float32),
            ct.TensorType(
                name="metadata_mask", shape=examples[2].shape, dtype=np.float32
            ),
        ]
        + (
            [
                ct.TensorType(
                    name="linear_features", shape=examples[3].shape, dtype=np.float32
                ),
                ct.TensorType(
                    name="gain_map_features", shape=examples[4].shape, dtype=np.float32
                ),
                ct.TensorType(
                    name="modality_mask", shape=examples[5].shape, dtype=np.float32
                ),
            ]
            if model.supports_optional_modalities
            else []
        ),
        outputs=[
            ct.TensorType(name="key1"),
            ct.TensorType(name="key1_log_variance"),
            ct.TensorType(name="gtc"),
            ct.TensorType(name="light_maps"),
            ct.TensorType(name="scalars"),
        ],
        compute_precision=ct.precision.FLOAT16,
        minimum_deployment_target=ct.target.macOS15,
    )
    converted.author = "XDRemux universal Photographic Style experiment"
    converted.short_description = "Single-image Apple Photographic Style state candidate"
    converted.user_defined_metadata["checkpointSHA256"] = sha256_file(
        args.checkpoint.resolve()
    )
    converted.user_defined_metadata["architecture"] = str(checkpoint["architecture"])
    converted.user_defined_metadata["inputContract"] = (
        "required primary features plus masked metadata and optional linear/gain tensors"
        if model.supports_optional_modalities
        else "primary image features plus masked metadata"
    )
    output.parent.mkdir(parents=True, exist_ok=True)
    converted.save(output)

    runtime = ct.models.MLModel(str(output), compute_units=ct.ComputeUnit.ALL)
    feed = {
        "features": examples[0].numpy(),
        "metadata": examples[1].numpy(),
        "metadata_mask": examples[2].numpy(),
    }
    if model.supports_optional_modalities:
        feed.update(
            {
                "linear_features": examples[3].numpy(),
                "gain_map_features": examples[4].numpy(),
                "modality_mask": examples[5].numpy(),
            }
        )
    names = (
        "key1",
        "key1_log_variance",
        "gtc",
        "light_maps",
        "scalars",
    )
    for _ in range(3):
        runtime.predict(feed)
    durations = []
    actual = None
    for _ in range(20):
        started = time.perf_counter()
        actual = runtime.predict(feed)
        durations.append(time.perf_counter() - started)
    parity = {}
    for name, reference in zip(names, expected):
        prediction = np.asarray(actual[name]).reshape(reference.shape)
        absolute = np.abs(prediction - reference)
        parity[name] = {
            "maximumAbsoluteError": float(absolute.max()),
            "meanAbsoluteError": float(absolute.mean()),
            "finite": bool(np.isfinite(prediction).all()),
        }
    report = {
        "schema": "xdremux-universal-photographic-style-coreml-export-v1",
        "checkpoint": {
            "path": str(args.checkpoint.resolve()),
            "sha256": sha256_file(args.checkpoint.resolve()),
            "architecture": checkpoint["architecture"],
            "epoch": checkpoint["epoch"],
        },
        "output": str(output),
        "outputBytes": sum(path.stat().st_size for path in output.rglob("*") if path.is_file()),
        "runtime": {
            "computeUnits": "ALL",
            "warmIterations": len(durations),
            "medianMilliseconds": float(np.median(durations) * 1000),
            "p95Milliseconds": float(np.quantile(durations, 0.95) * 1000),
            "maximumMilliseconds": float(np.max(durations) * 1000),
        },
        "parity": parity,
        "parityFixture": (
            {
                "path": str(fixture.path),
                "sha256": fixture.source_sha256,
                "make": fixture.source_make,
                "model": fixture.source_model,
            }
            if fixture is not None
            else None
        ),
        "claimBoundary": (
            "Core ML computeUnits=ALL permits but does not prove Neural Engine placement; "
            "native response and Photos consumer behavior remain separate gates."
        ),
    }
    _atomic_json(args.report.resolve(), report)
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
