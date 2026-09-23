"""Compare bounded synthetic lowering with the pinned public Core ML oracle."""
from __future__ import annotations
import hashlib
import json
import numpy as np
from .sparse import materialize_sparse_shift_scale


def main() -> None:
    import coremltools as ct
    from coremltools.converters.mil.mil.ops.defs.iOS18.compression import (
        constexpr_sparse_blockwise_shift_scale as SBSS,
        constexpr_sparse_to_dense as S2D,
    )
    if ct.__version__ != "9.0":
        raise RuntimeError("this evidence recipe is pinned to coremltools 9.0")
    rng = np.random.default_rng(0x20190)
    results = []
    for dtype in (np.float16, np.float32):
        for shape, scales in (((8, 12), (2, 3)), ((4, 6, 8), (1, 3, 2)), ((16, 16), (4, 1))):
            for with_offset in (False, True):
                mask = (rng.random(shape) > .68).astype(np.uint8)
                nonzero = rng.integers(-8, 8, size=np.count_nonzero(mask), dtype=np.int8)
                scale = (rng.random(scales) * 1.75 + .01).astype(dtype)
                offset = rng.integers(-3, 4, size=scales, dtype=np.int8) if with_offset else None
                output_mask, output_nonzero = SBSS.decompress(mask, nonzero, scale, offset)
                expected = S2D.decompress(output_nonzero, output_mask)
                actual = materialize_sparse_shift_scale(mask, nonzero, scale, offset)
                np.testing.assert_array_equal(actual, expected)
                if actual.dtype != expected.dtype:
                    raise AssertionError("materialized dtype differs from the reference")
                results.append({"dtype": str(actual.dtype), "shape": shape, "scaleShape": scales,
                                "offset": with_offset, "sha256": hashlib.sha256(actual.tobytes()).hexdigest()})
    print(json.dumps({"schema": 1, "coremltools": ct.__version__, "numpy": np.__version__,
                      "evidenceLevel": "synthetic-operator-equivalence-only", "cases": results,
                      "nativeModelExecuted": False, "productAccepted": False}, indent=2))


if __name__ == "__main__":
    main()
