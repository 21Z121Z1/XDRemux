#!/usr/bin/env python3
"""Generate half- or full-resolution depth maps with local Depth Pro 2 PyTorch models."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import sys
import tempfile
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    import numpy as np
    import torch
    from PIL import Image


LICENSE_RECEIPT = ".apple-amlr-license-accepted.json"
DECODER_STRIPE_PIXELS = 2 * 1024**2
ATTENTION_SCORE_BUDGET = 32 * 1024**2


def default_models_dir() -> Path:
    override = os.environ.get("XDREMUX_MODEL_DIR", "").strip()
    if override:
        return Path(override).expanduser()
    if sys.platform == "darwin":
        return Path.home() / "Library" / "Application Support" / "XDRemux" / "Models"
    return Path.home() / ".local" / "share" / "XDRemux" / "Models"


def half_size(size: tuple[int, int]) -> tuple[int, int]:
    width, height = size
    return (width + 1) // 2, (height + 1) // 2


def inference_size(size: tuple[int, int], max_side: int) -> tuple[int, int]:
    width, height = size
    longest = max(size)
    if max_side == 0 or longest <= max_side:
        return size
    return (
        max(1, (width * max_side + longest // 2) // longest),
        max(1, (height * max_side + longest // 2) // longest),
    )


def sample_rgb(image: Image.Image, size: tuple[int, int], orientation: int) -> np.ndarray:
    import numpy as np
    from PIL import Image

    if image.size != size:
        # JPEG decodes directly at 1/2, 1/4 or 1/8 when the target permits it.
        if image.format == "JPEG":
            image.draft("RGB", size)
        if image.mode.startswith("I;16"):
            image = image.convert("F")
            image = image.resize(size, Image.Resampling.LANCZOS)
            scaled = np.clip(np.asarray(image) / 256.0, 0, 255).astype(np.uint8)
            image = Image.fromarray(scaled)
        elif image.size != size:
            if image.mode in ("1", "P"):
                image = image.convert("RGB")
            image = image.resize(size, Image.Resampling.LANCZOS, reducing_gap=3.0)
    elif image.mode.startswith("I;16"):
        image = Image.fromarray((np.asarray(image) >> 8).astype(np.uint8))
    transpose = {
        2: Image.Transpose.FLIP_LEFT_RIGHT,
        3: Image.Transpose.ROTATE_180,
        4: Image.Transpose.FLIP_TOP_BOTTOM,
        5: Image.Transpose.TRANSPOSE,
        6: Image.Transpose.ROTATE_270,
        7: Image.Transpose.TRANSVERSE,
        8: Image.Transpose.ROTATE_90,
    }.get(orientation)
    if transpose is not None:
        image = image.transpose(transpose)
    return np.array(image.convert("RGB") if image.mode != "RGB" else image)


def get_focal_length_px(image: Image.Image) -> float | None:
    from PIL import ExifTags

    exif = image.getexif()
    focal_35mm = exif.get_ifd(ExifTags.IFD.Exif).get(ExifTags.Base.FocalLengthIn35mmFilm)
    if isinstance(focal_35mm, (int, float)) and math.isfinite(focal_35mm) and focal_35mm > 0:
        return focal_35mm * math.hypot(*image.size) / math.hypot(36, 24)
    return None


def select_device(requested: str) -> torch.device:
    import torch

    if requested == "auto":
        if torch.cuda.is_available():
            return torch.device("cuda")
        if torch.backends.mps.is_available():
            return torch.device("mps")
        return torch.device("cpu")
    if requested == "cuda" and not torch.cuda.is_available():
        raise ValueError("CUDA was requested but is unavailable")
    if requested == "mps" and not torch.backends.mps.is_available():
        raise ValueError("MPS was requested but is unavailable")
    return torch.device(requested)


def select_precision(device: torch.device, requested: str) -> torch.dtype:
    import torch

    if requested != "auto":
        return {"fp32": torch.float32, "fp16": torch.float16, "bf16": torch.bfloat16}[requested]
    if device.type == "cuda":
        return (
            torch.bfloat16
            if torch.cuda.is_bf16_supported(including_emulation=False)
            else torch.float16
        )
    return torch.bfloat16 if device.type == "mps" else torch.float32


def query_chunked_attention(query, key, value, *, score_budget=ATTENTION_SCORE_BUDGET, **options):
    from torch.nn.functional import scaled_dot_product_attention

    # Every query still sees every key and value. Bound the scores a math
    # backend can materialize in one operation.
    scores_per_query = math.prod(query.shape[:-2]) * key.shape[-2]
    rows = max(1, score_budget // scores_per_query)
    output = query.new_empty((*query.shape[:-1], value.shape[-1]))
    for start in range(0, query.shape[-2], rows):
        part = slice(start, start + rows)
        output[..., part, :].copy_(
            scaled_dot_product_attention(query[..., part, :], key, value, **options)
        )
    return output


def limit_attention_memory():
    """Use fused attention where available and bound the dense fallback."""
    import torch
    from torch.nn.attention import SDPBackend, sdpa_kernel
    from torch.nn.functional import scaled_dot_product_attention
    from torch.overrides import TorchFunctionMode

    class BoundedAttention(TorchFunctionMode):
        def __torch_function__(self, func, types, args=(), kwargs=None):
            options = kwargs or {}
            if func is not scaled_dot_product_attention or len(args) != 3:
                return func(*args, **options)
            query, key, value = args
            score_count = math.prod(query.shape[:-1]) * key.shape[-2]
            if (
                score_count <= ATTENTION_SCORE_BUDGET
                or options.get("attn_mask") is not None
                or options.get("dropout_p")
                or options.get("is_causal")
            ):
                return func(*args, **options)

            if query.device.type == "mps":
                # PyTorch 2.13 added fused long-sequence attention for the
                # 64-dimensional heads used by the Depth Pro 2 S/B models.
                if (
                    torch.__version__ >= "2.13.0"
                    and query.shape[-1] == key.shape[-1] == value.shape[-1] == 64
                    and os.environ.get("PYTORCH_MPS_DISABLE_PREFILL_ATTENTION", "0") == "0"
                ):
                    return func(*args, **options)
            else:
                with sdpa_kernel([SDPBackend.FLASH_ATTENTION, SDPBackend.EFFICIENT_ATTENTION]):
                    try:
                        return func(*args, **options)
                    except RuntimeError as error:
                        if not str(error).startswith(("No available kernel", "No viable backend")):
                            raise

            return query_chunked_attention(query, key, value, **options)

    return BoundedAttention()


class FusedInverseDepthHead:
    """Combine the FP32 transposed convolution and its following 3×3 convolution."""

    def __init__(self, head):
        import torch

        self.first = head[0]
        self.finish = head[3:]
        upsampling, convolution = head[1:3]
        out_channels = convolution.weight.shape[0]
        in_channels = upsampling.weight.shape[0]

        with (
            torch.inference_mode(),
            torch.autocast(device_type=convolution.weight.device.type, enabled=False),
        ):
            # A stride-2 transpose convolution picks a different kernel for
            # each output phase. Shift the output by one pixel so all four
            # phases use the same 2×2 neighborhood of input cells.
            weights = convolution.weight.new_zeros((out_channels, 4, in_channels, 2, 2))
            for phase_y in range(2):
                for phase_x in range(2):
                    phase = 2 * phase_y + phase_x
                    for row in range(3):
                        for col in range(3):
                            y, iy = divmod(phase_y + row - 2, 2)
                            x, ix = divmod(phase_x + col - 2, 2)
                            weights[:, phase, :, y + 1, x + 1].add_(
                                convolution.weight[:, :, row, col]
                                @ upsampling.weight[:, :, iy, ix].T
                            )
            self.weights = weights.flatten(0, 1).contiguous(memory_format=torch.channels_last)

            # The transpose convolution's bias exists only inside the image.
            # Remove its contribution where the next convolution sees padding.
            edge = torch.einsum("ocij,c->oij", convolution.weight, upsampling.bias)
            self.bias = (convolution.bias + edge.sum(dim=(1, 2))).repeat_interleave(4)
            self.top = edge[:, 0, :].sum(1).view(1, -1, 1)
            self.bottom = edge[:, 2, :].sum(1).view(1, -1, 1)
            self.left = edge[:, :, 0].sum(1).view(1, -1, 1)
            self.right = edge[:, :, 2].sum(1).view(1, -1, 1)
            self.edge = edge

    def __call__(self, decoded):
        from torch.nn.functional import conv2d, pixel_shuffle

        phases = conv2d(self.first(decoded), self.weights, self.bias, padding=1)
        output = pixel_shuffle(phases, 2)[..., 1:-1, 1:-1]
        del phases
        output[..., 0, :].sub_(self.top)
        output[..., -1, :].sub_(self.bottom)
        output[..., :, 0].sub_(self.left)
        output[..., :, -1].sub_(self.right)
        for row, kernel_row in ((0, 0), (-1, 2)):
            for col, kernel_col in ((0, 0), (-1, 2)):
                output[..., row, col].add_(self.edge[:, kernel_row, kernel_col])
        return self.finish(output)


def decode_in_stripes(model, encodings, size, stripe_pixels, *, stripe_head=None):
    """Run the highest-resolution fusion and head with the original receptive field."""
    import torch
    from depth_pro_2.network.encoder import autocast_for

    height, width = size
    decoder = model.decoder
    device = encodings[-1].device
    head = model.head if stripe_head is None else stripe_head
    # The last fusion consumes the quarter-resolution features. Keep the
    # lower levels whole and release each skip as soon as it is used.
    with autocast_for(device, model.precision)():
        fused = decoder.fusions[-1](decoder.convs[-1](encodings.pop()))
        for level in range(len(decoder.fusions) - 2, 0, -1):
            fused = decoder.fusions[level](fused, decoder.convs[level](encodings.pop()))
        highest_skip = encodings.pop()
        output = fused.new_empty((1, 1, height, width), dtype=torch.float32)
        axis = -1 if width > 4 * height else -2
        pixel_extent = size[axis]
        quarter_extent = (pixel_extent + 3) // 4
        across = fused.shape[-2 if axis == -1 else -1] * 4
        step = max(1, stripe_pixels // (4 * across))
        for start in range(0, quarter_extent, step):
            end = min(start + step, quarter_extent)
            # Quarter-scale halo: skip projection (1), two residual blocks
            # (2 each), and the half/full-resolution depth head (1 total).
            left = max(0, start - 6)
            right = min(fused.shape[axis], end + 6)
            skip = decoder.convs[0](highest_skip.narrow(axis, left, right - left))
            decoded = decoder.fusions[0](fused.narrow(axis, left, right - left), skip)
            del skip
            with torch.autocast(device_type=device.type, enabled=False):
                prediction = head(decoded.float())
            del decoded
            offset = (start - left) * 4
            count = min(end * 4, pixel_extent) - start * 4
            output.narrow(axis, start * 4, count).copy_(
                prediction.narrow(axis, offset, count)[..., :height, :width]
            )
            del prediction
    return output[:, 0]


def predict_inverse_depth(model, x, *, stripe_pixels=DECODER_STRIPE_PIXELS, stripe_head=None):
    from depth_pro_2.model import INPUT_MULTIPLE
    from depth_pro_2.network.encoder import autocast_for
    from torch.nn.functional import pad

    height, width = x.shape[-2:]
    pad_h, pad_w = -height % INPUT_MULTIPLE, -width % INPUT_MULTIPLE
    if (height + pad_h) * (width + pad_w) <= stripe_pixels:
        with limit_attention_memory():
            return model.predict_inverse_depth(x)
    if pad_h or pad_w:
        x = pad(x, (0, pad_w, 0, pad_h), mode="replicate")
    with limit_attention_memory(), autocast_for(x.device, model.precision)():
        features, grid = model.encoder.encode(x)
    with autocast_for(x.device, model.precision)():
        encodings = model.encoder.reassemble_tokens(features, grid)
    del x, features
    return decode_in_stripes(
        model, encodings, (height, width), stripe_pixels, stripe_head=stripe_head
    )


class DepthEstimator:
    def __init__(self, args: argparse.Namespace):
        models_dir = args.models_dir / "pytorch"
        if not (
            (models_dir / LICENSE_RECEIPT).is_file()
            or os.environ.get("XDREMUX_ACCEPT_APPLE_AMLR_LICENSE") == "1"
        ):
            raise ValueError(
                "install PyTorch weights with install_depthpro2.py --backend pytorch --accept-apple-amlr-license"
            )
        variant = args.model or os.environ.get("XDREMUX_DEPTHPRO2_PYTORCH_MODEL", "auto")
        if variant == "auto":
            variant = "s" if (models_dir / "depthpro2-s.safetensors").is_file() else "b"
        if variant not in ("s", "b"):
            raise ValueError("choose PyTorch model s or b")
        self.model_id = f"depthpro2-{variant}"
        self.weight_path = models_dir / f"{self.model_id}.safetensors"
        self.fov_path = models_dir / "depthpro2-fov.pt"
        if not self.weight_path.is_file():
            raise FileNotFoundError(f"PyTorch weights missing at {self.weight_path}")

        import torch
        from depth_pro_2 import PRESETS, DepthPro2
        from depth_pro_2.model import IMAGENET_MEAN, IMAGENET_STD
        from safetensors.torch import load_file
        from torch.nn.utils.memory_format import convert_conv2d_weight_memory_format

        self.device = select_device(args.device)
        self.precision = select_precision(self.device, args.precision)
        self.memory_format = (
            torch.contiguous_format if self.device.type == "mps" else torch.channels_last
        )
        self.mean = torch.tensor(IMAGENET_MEAN, device=self.device).view(1, 3, 1, 1)
        self.std = torch.tensor(IMAGENET_STD, device=self.device).view(1, 3, 1, 1)
        config = PRESETS[self.model_id]
        model = DepthPro2(
            backbone_preset=config.backbone_preset,
            reassembled_blocks=config.reassembled_blocks,
            cross_frame_blocks=config.cross_frame_blocks,
            dims_reassemble=config.dims_reassemble,
            dim_decoder=config.dim_decoder,
            fov=False,
            precision=self.precision,
        )
        model.load_state_dict(load_file(self.weight_path), strict=True, assign=True)
        if self.memory_format == torch.channels_last:
            convert_conv2d_weight_memory_format(model, torch.channels_last)
        self.model = model.to(self.device).eval()
        self.stripe_head = None
        if (
            self.device.type == "cpu"
            and self.precision == torch.float32
            and torch.backends.mkldnn.is_available()
            and torch.backends.mkldnn.enabled
        ):
            self.stripe_head = FusedInverseDepthHead(self.model.head)
        self.fov = None
        self.weights_sha256 = None
        self.native = args.native
        self.max_side = args.max_side

    def estimate_focal_length_px(self, rgb: torch.Tensor) -> float:
        from depth_pro_2.model import load_checkpoint
        from depth_pro_2.network.fov import FOVEstimator

        if self.fov is None:
            if not self.fov_path.is_file():
                raise FileNotFoundError(
                    f"PyTorch FOV weights missing at {self.fov_path}. Install fov or supply --focal-length-px."
                )
            fov = FOVEstimator(self.precision)
            fov.load_state_dict(load_checkpoint(self.fov_path), strict=True, assign=True)
            self.fov = fov.to(self.device).eval()

        # Bound the FOV model's pre-resize for narrow panoramas. A centered
        # crop preserves the focal length in pixels of the working image.
        height, width = rgb.shape[-2:]
        crop_width = min(width, height * 16 // 9)
        crop_height = min(height, width * 16 // 9)
        x = (width - crop_width) // 2
        y = (height - crop_height) // 2
        return float(self.fov.focal_length_px(rgb[..., y : y + crop_height, x : x + crop_width]))

    def estimate(
        self,
        path: Path,
        *,
        focal_px: float | None = None,
        orientation: int | None = None,
        focal_35mm: float | None = None,
        full_size: bool = False,
    ) -> tuple[np.ndarray, float]:
        import numpy as np
        import torch
        from PIL import Image
        from torch.nn.functional import interpolate

        with Image.open(path) as image:
            if image.format != "JPEG":
                image.load()
            size = image.size
            if orientation is None:
                orientation = image.info.get("original_orientation") or image.getexif().get(274, 1)
                if orientation not in range(1, 9):
                    orientation = 1
                if focal_px is None:
                    focal_px = get_focal_length_px(image)
            elif focal_35mm is not None:
                focal_px = focal_35mm * max(size) / 36.0
            output_size = size if full_size else half_size(size)
            working_size = inference_size(size if self.native else output_size, self.max_side)
            rgb = sample_rgb(image, working_size, orientation)
        del image

        display_size = size[::-1] if orientation in (5, 6, 7, 8) else size
        display_output = output_size[::-1] if orientation in (5, 6, 7, 8) else output_size
        with torch.inference_mode():
            x = (
                torch.from_numpy(rgb)
                .to(self.device, non_blocking=True)
                .permute(2, 0, 1)
                .unsqueeze(0)
            )
            x = x.to(dtype=torch.float32, memory_format=self.memory_format)
            x.div_(255.0).sub_(self.mean).div_(self.std)
            del rgb
            ratio = (
                self.estimate_focal_length_px(x) / x.shape[-1]
                if focal_px is None
                else focal_px / display_size[0]
            )
            if not math.isfinite(ratio) or ratio <= 0.0:
                raise ValueError("invalid focal length for depth estimation")
            inverse_depth = predict_inverse_depth(
                self.model, x, stripe_head=self.stripe_head
            ).unsqueeze(1)
            del x
            target = display_output[::-1]
            if target != inverse_depth.shape[-2:]:
                # Reduce a native prediction before copying it to the host.
                # Expand a capped prediction on the host to bound GPU memory.
                if any(out > actual for out, actual in zip(target, inverse_depth.shape[-2:])):
                    inverse_depth = inverse_depth.cpu()
                inverse_depth = interpolate(
                    inverse_depth, size=target, mode="bilinear", align_corners=False
                )
            disparity = inverse_depth[0, 0].div_(ratio).cpu().numpy()
        if not (0.0 <= disparity.min() <= disparity.max() <= np.finfo(np.float32).max):
            raise ValueError("Depth Pro 2 produced invalid inverse depth")
        return disparity, 36.0 * ratio * display_size[0] / max(display_size)


def write_array(path: Path, data: np.ndarray, *, sidecar: bool = False) -> None:
    import numpy as np

    temporary = None
    try:
        with tempfile.NamedTemporaryFile(
            dir=path.parent, prefix=".depthpro2-", delete=False
        ) as stream:
            temporary = Path(stream.name)
            if sidecar:
                data.tofile(stream)
            else:
                np.save(stream, data, allow_pickle=False)
        os.link(temporary, path)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def portrait_disparity(estimator: DepthEstimator, request: dict) -> dict:
    import numpy as np

    if request.get("schema_version") != 2 or request.get("operation") != "depthpro2-disparity":
        raise ValueError("expected a schema version 2 Depth Pro 2 disparity request")
    config = request["depthpro2_disparity"]
    if config.get("variant") not in (None, "auto", estimator.model_id, estimator.model_id[-1]):
        raise ValueError(f"the PyTorch adapter is configured for {estimator.model_id}")
    orientation = config.get("orientation", 1)
    if orientation not in range(1, 9):
        raise ValueError("expected EXIF orientation 1 through 8")
    disparity, focal_35mm = estimator.estimate(
        Path(request["input_path"]),
        orientation=orientation,
        focal_35mm=config.get("focal_length_35mm"),
    )
    # Undo display orientation for ImageIO, which associates the map with the
    # stored primary pixels and carries orientation separately.
    if orientation >= 5:
        disparity = disparity.T
    if orientation in (3, 4, 6, 7):
        disparity = disparity[::-1, :]
    if orientation in (2, 3, 7, 8):
        disparity = disparity[:, ::-1]
    height, width = disparity.shape
    if (width, height) != (config["target_width"], config["target_height"]):
        raise ValueError(
            f"decoded image produced {width}x{height} disparity, which differs from the requested geometry"
        )
    if disparity.max() > np.finfo(np.float16).max:
        raise ValueError("disparity exceeds Float16 range")
    pixels = np.asarray(disparity, dtype="<f2", order="C")
    sample_stride = max(1, pixels.size // 16384)
    p05, p50, p95 = np.percentile(pixels.ravel()[::sample_stride].astype(np.float32), (5, 50, 95))
    if estimator.weights_sha256 is None:
        with estimator.weight_path.open("rb") as stream:
            estimator.weights_sha256 = hashlib.file_digest(stream, "sha256").hexdigest()
    write_array(Path(request["output_path"]), pixels, sidecar=True)
    return {
        "schema_version": 2,
        "depthpro2_disparity": {
            "width": width,
            "height": height,
            "pixel_format": int.from_bytes(b"hdis", "big"),
            "min_disparity": float(pixels.min()),
            "max_disparity": float(pixels.max()),
            "p05_disparity": float(p05),
            "p50_disparity": float(p50),
            "p95_disparity": float(p95),
            "model_id": estimator.model_id,
            "weights_sha256": estimator.weights_sha256,
            "focal_length_35mm": focal_35mm,
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--input", type=Path, action="append", help="Input image. Repeat for a batch."
    )
    parser.add_argument(
        "--output",
        type=Path,
        action="append",
        help="Float32 depth in meters (.npy). Repeat in input order.",
    )
    parser.add_argument(
        "--model", choices=("s", "b"), help="Prefer S by default, or B when only B is installed."
    )
    parser.add_argument("--models-dir", type=Path, default=default_models_dir())
    parser.add_argument("--device", choices=("auto", "cuda", "mps", "cpu"), default="auto")
    parser.add_argument("--precision", choices=("auto", "bf16", "fp16", "fp32"), default="auto")
    parser.add_argument(
        "--max-side",
        type=int,
        default=0,
        help="Optionally limit the longer inference edge (default: 0, unlimited).",
    )
    parser.add_argument(
        "--native",
        action="store_true",
        help="Use input resolution, subject to --max-side, before reducing the output.",
    )
    parser.add_argument(
        "--full-size",
        action="store_true",
        help="Produce a full-resolution depth map. Inference uses the original size by default.",
    )
    parser.add_argument(
        "--focal-length-px",
        type=float,
        help="Known focal length in the original oriented image's pixels.",
    )
    parser.add_argument("--persistent-json-lines", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()

    if args.max_side < 0:
        parser.error("--max-side must be nonnegative")
    if args.focal_length_px is not None and (
        not math.isfinite(args.focal_length_px) or args.focal_length_px <= 0
    ):
        parser.error("--focal-length-px must be finite and positive")
    if not args.persistent_json_lines:
        if not args.input or not args.output or len(args.input) != len(args.output):
            parser.error("supply one --output per --input")
        outputs = {path.resolve() for path in args.output}
        if len(outputs) != len(args.output) or outputs.intersection(
            path.resolve() for path in args.input
        ):
            parser.error("outputs must be distinct from one another and from all inputs")
        for path in args.output:
            if path.suffix.lower() != ".npy":
                parser.error("--output must have an .npy extension")
            if not path.parent.is_dir() or os.path.lexists(path):
                parser.error(f"output must be a new file in an existing directory: {path}")

    try:
        estimator = DepthEstimator(args)
        if args.persistent_json_lines:
            for line in sys.stdin:
                print(
                    json.dumps(portrait_disparity(estimator, json.loads(line)), allow_nan=False),
                    flush=True,
                )
        else:
            import numpy as np

            for source, output in zip(args.input, args.output):
                disparity, _ = estimator.estimate(
                    source, focal_px=args.focal_length_px, full_size=args.full_size
                )
                with np.errstate(divide="ignore", over="ignore"):
                    np.reciprocal(disparity, out=disparity)
                write_array(output, disparity)
                del disparity
    except (ImportError, KeyError, OSError, RuntimeError, TypeError, ValueError) as error:
        parser.exit(1, f"Depth Pro 2 PyTorch inference failed: {error}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
