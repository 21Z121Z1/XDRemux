"""Synthetic FSINC lowering evidence, not an Apple model parser or runtime.

A masked-out quantized entry stays zero after sparse shift/scale. Applying
shift/scale to the entire zero-filled dense tensor is incorrect when offset
is nonzero. See the primary Core ML operator reference in README.en.md.
"""
from __future__ import annotations

import numpy as np


def materialize_sparse_shift_scale(
    mask: np.ndarray,
    nonzero: np.ndarray,
    scale: np.ndarray,
    offset: np.ndarray | None = None,
) -> np.ndarray:
    """Materialize an explicitly unpacked sparse tensor with bounded validation.

    Nonzero entries use C-order mask traversal. This does not decode a compiled
    Apple model, packed uint1/int4 weights, or guess an unsupported MIL opset.
    """
    mask, nonzero, scale = (np.asarray(value) for value in (mask, nonzero, scale))
    floats = (np.dtype("float16"), np.dtype("float32"))
    numbers = (*floats, np.dtype("int8"), np.dtype("uint8"))
    if mask.dtype not in (np.dtype("bool"), np.dtype("uint8")):
        raise ValueError("mask must be an explicitly unpacked binary bool/uint8 tensor")
    if mask.ndim < 1 or any(d <= 0 for d in mask.shape) or mask.size > 16_777_216:
        raise ValueError("mask dimensions must be positive and bounded")
    if np.any((mask != 0) & (mask != 1)):
        raise ValueError("mask values must be 0 or 1")
    if nonzero.ndim != 1 or nonzero.dtype not in numbers:
        raise ValueError("nonzero data must be a rank-one int8/uint8/fp16/fp32 tensor")
    if nonzero.size != np.count_nonzero(mask):
        raise ValueError("nonzero count does not match the mask")
    if scale.dtype not in floats or scale.ndim != mask.ndim or any(
        s <= 0 or d % s for d, s in zip(mask.shape, scale.shape)
    ):
        raise ValueError("fp16/fp32 scale must have the same rank and divide each mask dimension")
    if offset is not None:
        offset = np.asarray(offset)
        if offset.shape != scale.shape or offset.dtype not in numbers:
            raise ValueError("offset must have the scale shape and a supported numeric dtype")
        if offset.dtype not in floats and offset.dtype != nonzero.dtype:
            raise ValueError("integer offset dtype must equal the nonzero dtype")
    for value in (nonzero, scale, offset):
        if value is not None and not np.isfinite(value).all():
            raise ValueError("sparse inputs must be finite")
    scale_expanded = scale
    offset_expanded = offset
    for axis, (size, blocks) in enumerate(zip(mask.shape, scale.shape)):
        scale_expanded = np.repeat(scale_expanded, size // blocks, axis=axis)
        if offset_expanded is not None:
            offset_expanded = np.repeat(offset_expanded, size // blocks, axis=axis)
    selected = mask.astype(bool, copy=False)
    with np.errstate(over="ignore", invalid="ignore"):
        values = nonzero.astype(scale.dtype)
        if offset_expanded is not None:
            values = values - offset_expanded[selected]
        values = values * scale_expanded[selected]
    if not np.isfinite(values).all():
        raise ValueError("sparse materialization overflowed the output dtype")
    dense = np.zeros(mask.shape, dtype=values.dtype)
    dense[selected] = values
    return dense
