"""Pure, frozen low-dimensional research corrections; never a product provider.

Fitting and application have deliberately different interfaces. Application
cannot receive labels or a split: heldout observations cannot refit coefficients.
Native-render evidence remains a separate, provenance-bound experiment.
"""
from __future__ import annotations

from dataclasses import dataclass
import math
from typing import Any

import numpy as np


@dataclass(frozen=True)
class ChannelBias:
    """Thirty normalized coefficients shared across all spatial/plane positions."""

    coefficients: tuple[float, ...]
    ridge: float

    def __post_init__(self) -> None:
        if (not isinstance(self.coefficients, tuple) or len(self.coefficients) != 30
                or not all(type(x) in (int, float) and math.isfinite(x) for x in self.coefficients)
                or type(self.ridge) not in (int, float)
                or not math.isfinite(self.ridge) or self.ridge <= 0):
            raise ValueError("a frozen correction requires 30 finite coefficients and positive ridge")

    def to_dict(self) -> dict[str, Any]:
        return {"schema": 1, "kind": "shared-channel-bias", "ridge": self.ridge,
                "coefficients": list(self.coefficients)}

    @classmethod
    def from_dict(cls, record: dict[str, Any]) -> ChannelBias:
        if (not isinstance(record, dict)
                or set(record) != {"schema", "kind", "ridge", "coefficients"}
                or type(record["schema"]) is not int or record["schema"] != 1
                or record["kind"] != "shared-channel-bias"
                or not isinstance(record["coefficients"], list)):
            raise ValueError("invalid frozen channel correction")
        return cls(tuple(record["coefficients"]), record["ridge"])


def _array(value: Any, *, name: str) -> np.ndarray:
    result = np.asarray(value)
    if (result.dtype.kind not in "fiu" or not result.size or result.size > 16_000_000
            or not np.isfinite(result).all()):
        raise ValueError(f"{name} must be bounded, nonempty finite real data")
    return result.astype(np.float64)


def _inputs(prediction: Any, scales: Any) -> tuple[np.ndarray, np.ndarray]:
    prediction = _array(prediction, name="prediction")
    scales = _array(scales, name="scales")
    if prediction.ndim < 3 or prediction.shape[-2:] != (10, 3):
        raise ValueError("prediction requires sample/position axes followed by 10 x 3 coefficients")
    if scales.shape != (10, 3) or (scales <= 0).any():
        raise ValueError("scales must be positive 10 x 3 coefficients")
    return prediction, scales


def fit_channel_bias(prediction: Any, target: Any, mask: Any, scales: Any,
                     *, ridge: float = 10.0) -> ChannelBias:
    """Fit once from a designated training cohort, before calibration selection.

    The mask covers all leading axes; no missing spatial cell supplies a label.
    Persist this value and its input/cohort identities before revealing heldout.
    """
    prediction, scales = _inputs(prediction, scales)
    target = _array(target, name="target")
    mask = np.asarray(mask)
    if target.shape != prediction.shape or mask.dtype.kind != "b" or mask.shape != prediction.shape[:-2]:
        raise ValueError("target and Boolean mask must exactly match prediction")
    if not mask.any():
        raise ValueError("a correction needs at least one observed coefficient block")
    # Validate regularization before division, including bool/NaN/infinity.
    ChannelBias((0.0,) * 30, ridge)
    with np.errstate(over="raise", invalid="raise", divide="raise"):
        try:
            residual = ((target[mask] - prediction[mask]) / scales)
            coefficients = residual.sum(axis=0) / (int(mask.sum()) + ridge)
        except FloatingPointError as error:
            raise ValueError("correction fitting overflowed") from error
    return ChannelBias(tuple(float(x) for x in coefficients.flat), ridge)


def apply_channel_bias(prediction: Any, scales: Any, frozen: ChannelBias) -> np.ndarray:
    """Apply only stored coefficients. There is intentionally no target argument."""
    prediction, scales = _inputs(prediction, scales)
    if not isinstance(frozen, ChannelBias):
        raise ValueError("a parsed frozen correction is required")
    bias = np.asarray(frozen.coefficients, dtype=np.float64).reshape(10, 3)
    with np.errstate(over="raise", invalid="raise"):
        try:
            result = prediction + bias * scales
        except FloatingPointError as error:
            raise ValueError("correction application overflowed") from error
    if not np.isfinite(result).all():
        raise ValueError("correction application is non-finite")
    return result
