#!/usr/bin/env python3
"""Export the fixed ReverseKey1Net ensemble as a Core ML program."""

from __future__ import annotations

import argparse
import json
import shutil
import sys
import time
from pathlib import Path
from typing import Any

import numpy as np

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from scripts.predict_reverse_key1 import _load_model
from xdremux_py.apple_reverse_key1_training import (
    INPUT_CHANNELS,
    INPUT_SIZE,
    _require_torch,
    sha256_file,
)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--candidate-weight", required=True, type=float)
    parser.add_argument("--profile", default="__unknown__")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    args = parser.parse_args()
    if not 0.0 <= args.candidate_weight <= 1.0:
        raise ValueError("candidate weight must be between zero and one")

    try:
        import coremltools as ct
    except ImportError as error:
        raise RuntimeError(
            "coremltools is required; provide it through the isolated export environment"
        ) from error
    torch, nn = _require_torch()
    baseline, baseline_checkpoint = _load_model(torch, args.baseline.resolve(), "cpu")
    candidate, candidate_checkpoint = _load_model(torch, args.candidate.resolve(), "cpu")

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

        def forward(
            self,
            query: Any,
            _key: Any,
            _value: Any,
            need_weights: bool = False,
        ) -> tuple[Any, None]:
            qkv = torch.nn.functional.linear(
                query, self.in_proj_weight, self.in_proj_bias
            )
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

    for block in candidate.context:
        block.attention = TraceableSelfAttention(block.attention)

    class SmallFlattened(nn.Module):
        def __init__(self, source: Any) -> None:
            super().__init__()
            self.source = source

        def forward(self, value: Any, profile_ids: Any) -> Any:
            spatial = torch.nn.functional.interpolate(
                self.source.encoder(value),
                size=(12, 12),
                mode="bilinear",
                align_corners=False,
            ).permute(0, 2, 3, 1)
            profile = self.source.profile_embedding(profile_ids)
            gamma, beta, profile_plane = torch.split(
                profile, (spatial.shape[-1], spatial.shape[-1], 8 * 32), dim=-1
            )
            spatial = spatial * (1.0 + gamma[:, None, None, :])
            spatial = spatial + beta[:, None, None, :]
            spatial = spatial[:, :, :, None, :].expand(-1, -1, -1, 8, -1)
            plane = self.source.plane_embedding.weight.reshape(1, 1, 1, 8, 32)
            plane = plane + profile_plane.reshape(-1, 1, 1, 8, 32)
            plane = plane.expand(spatial.shape[0], 12, 12, -1, -1)
            residual = self.source.head(torch.cat((spatial, plane), dim=-1))
            identity = self.source.identity.reshape(1, 12, 12, 8, 30)
            scales = self.source.coefficient_scales.reshape(1, 1, 1, 8, 30)
            return identity + residual * scales

    class LargeFlattened(nn.Module):
        def __init__(self, source: Any) -> None:
            super().__init__()
            self.source = source

        def forward(self, value: Any) -> Any:
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
            tokens = spatial.flatten(2).transpose(1, 2)
            for block in self.source.context:
                tokens = block(tokens)
            spatial = tokens.reshape(-1, 12, 12, 256)
            spatial = spatial[:, :, :, None, :].expand(-1, -1, -1, 8, -1)
            plane = self.source.plane_embedding.weight.reshape(1, 1, 1, 8, 64)
            plane = plane.expand(spatial.shape[0], 12, 12, -1, -1)
            residual = self.source.head(torch.cat((spatial, plane), dim=-1))
            identity = self.source.identity.reshape(1, 12, 12, 8, 30)
            scales = self.source.coefficient_scales.reshape(1, 1, 1, 8, 30)
            return identity + residual * scales

    baseline_flat = SmallFlattened(baseline)
    candidate_flat = LargeFlattened(candidate)
    vocabulary = tuple(baseline_checkpoint.get("profileVocabulary", ()))
    fallback = vocabulary.index("__unknown__") if "__unknown__" in vocabulary else 0
    profile_id = vocabulary.index(args.profile) if args.profile in vocabulary else fallback

    class FixedEnsemble(nn.Module):
        def __init__(self) -> None:
            super().__init__()
            self.baseline = baseline_flat
            self.candidate = candidate_flat
            self.register_buffer("profile_id", torch.tensor([profile_id], dtype=torch.long))

        def forward(self, features: Any) -> Any:
            baseline_value = self.baseline(features, self.profile_id)
            candidate_value = self.candidate(features)
            blended = (
                (1.0 - args.candidate_weight) * baseline_value
                + args.candidate_weight * candidate_value
            )
            return blended.reshape(1, -1)

    wrapper = FixedEnsemble().eval()
    if hasattr(torch.backends, "mha"):
        torch.backends.mha.set_fastpath_enabled(False)
    example = torch.zeros(
        (1, INPUT_CHANNELS, INPUT_SIZE, INPUT_SIZE), dtype=torch.float32
    )
    with torch.no_grad():
        expected = wrapper(example).numpy()
        traced = torch.jit.trace(wrapper, example, strict=True)
    converted = ct.convert(
        traced,
        convert_to="mlprogram",
        inputs=[
            ct.TensorType(
                name="features",
                shape=example.shape,
                dtype=np.float32,
            )
        ],
        outputs=[ct.TensorType(name="key1")],
        compute_precision=ct.precision.FLOAT16,
        minimum_deployment_target=ct.target.macOS15,
    )
    converted.author = "XDRemux ReverseKey1Net experiment"
    converted.short_description = "Fixed iPhone reverse key1 ensemble"
    converted.user_defined_metadata["baselineSHA256"] = sha256_file(args.baseline.resolve())
    converted.user_defined_metadata["candidateSHA256"] = sha256_file(args.candidate.resolve())
    converted.user_defined_metadata["candidateWeight"] = str(args.candidate_weight)
    converted.user_defined_metadata["profile"] = args.profile
    output = args.output.resolve()
    if output.exists():
        shutil.rmtree(output)
    output.parent.mkdir(parents=True, exist_ok=True)
    converted.save(output)

    runtime = ct.models.MLModel(str(output), compute_units=ct.ComputeUnit.ALL)
    for _ in range(3):
        runtime.predict({"features": example.numpy()})
    durations = []
    predicted = None
    for _ in range(20):
        started = time.perf_counter()
        predicted = np.asarray(runtime.predict({"features": example.numpy()})["key1"])
        durations.append(time.perf_counter() - started)
    absolute = np.abs(predicted.reshape(expected.shape) - expected)
    report = {
        "schema": "xdremux-reverse-key1-coreml-export-v1",
        "output": str(output),
        "outputBytes": sum(path.stat().st_size for path in output.rglob("*") if path.is_file()),
        "ensemble": {
            "baselineSHA256": sha256_file(args.baseline.resolve()),
            "candidateSHA256": sha256_file(args.candidate.resolve()),
            "candidateWeight": args.candidate_weight,
            "profile": args.profile,
        },
        "runtime": {
            "computeUnits": "ALL",
            "warmIterations": len(durations),
            "medianMilliseconds": float(np.median(durations) * 1000),
            "p95Milliseconds": float(np.quantile(durations, 0.95) * 1000),
            "maximumMilliseconds": float(np.max(durations) * 1000),
        },
        "parity": {
            "maximumAbsoluteError": float(absolute.max()),
            "meanAbsoluteError": float(absolute.mean()),
            "finite": bool(np.isfinite(predicted).all()),
        },
        "claimBoundary": "Core ML computeUnits=ALL permits but does not prove Neural Engine placement.",
    }
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
