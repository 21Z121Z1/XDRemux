#!/usr/bin/env python3
"""Score native PISemanticStyleFilter tone renders.

The published harness compares its reconstructed Metal model with Apple's
output. This script treats the native half RGBA texture as the render under
test. Tone 0 on an Apple sample is the host gate: a non-zero differing-component
count means the later XDRemux numbers are not comparable to the 26A434 evidence.

The harness sets inputToneBias and does not expose a color control.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import signal
import subprocess
import sys
import time
from pathlib import Path

import numpy as np

# Display P3 luminance weights already used by the Styles runtime.
Y_WEIGHTS = np.array([0.22897456, 0.69173852, 0.07928691], dtype=np.float64)
TONES = (-1.0, -0.75, -0.5, -0.35, -0.15, 0.0, 0.15, 0.35, 0.5, 0.75, 1.0)
STEP_EPSILON = 1e-4


def _finite_number(value) -> bool:
    return not isinstance(value, bool) and isinstance(value, (int, float)) and math.isfinite(value)


def sweep_errors(rows: list[dict], photos: list[str]) -> list[str]:
    errors = []
    seen = set()
    for row in rows:
        photo, tone = row.get("photo"), row.get("tone")
        if row.get("error") or type(row.get("returncode")) is not int or row["returncode"] != 0:
            errors.append(f"{photo} tone {tone}: {row.get('error') or 'renderer returncode ' + str(row.get('returncode'))}")
        if photo not in photos or not _finite_number(tone) or tone not in TONES:
            errors.append(f"unexpected or missing photo/tone in consumer row: {photo}, {tone}")
            continue
        pair = (photo, tone)
        if pair in seen:
            errors.append(f"duplicate consumer row: {photo} tone {tone}")
        seen.add(pair)
        components, differing = row.get("components"), row.get("differing_components")
        if (type(components) is not int or components <= 0 or type(differing) is not int
                or not 0 <= differing <= components or not _finite_number(row.get("max_abs"))
                or row["max_abs"] < 0):
            errors.append(f"{photo} tone {tone}: native/model comparison metrics missing or invalid")
        psnr = row.get("psnr_peak1_db")
        if not _finite_number(psnr) and not (psnr == "inf" and row.get("max_abs") == 0):
            errors.append(f"{photo} tone {tone}: native/model PSNR missing or invalid")
    for photo in photos:
        for tone in TONES:
            if (photo, tone) not in seen:
                errors.append(f"missing consumer row: {photo} tone {tone}")
    return errors


def run_consumer(research_root: Path, photos: list[Path], out_dir: Path, *, timeout: float = 3600) -> dict:
    """Run the pinned consumer unchanged, retaining text diagnostics before scoring."""
    research_root, out_dir = research_root.resolve(), out_dir.resolve()
    out_dir.mkdir(parents=True, exist_ok=False)
    diagnostics = out_dir / "diagnostics"
    diagnostics.mkdir()
    script = research_root / "consumer-model/harness/run_tone_sweep.py"
    names = [photo.stem for photo in photos]
    command = [sys.executable, str(script), str(out_dir), *(str(photo) for photo in photos),
               "--tones", *(str(value) for value in TONES)]
    receipt = {"status": "running", "photos": names, "tones": list(TONES), "command": command,
               "returncode": None, "errors": [], "diagnostics": []}
    receipt_path = out_dir / "consumer-run.json"
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n")
    started = time.monotonic()
    try:
        if not names or len(set(names)) != len(names):
            raise ValueError("consumer photos must have distinct nonempty stems")
        receipt["consumer_script_sha256"] = hashlib.sha256(script.read_bytes()).hexdigest()
        with (diagnostics / "consumer.stdout.log").open("w") as stdout, (diagnostics / "consumer.stderr.log").open("w") as stderr:
            with subprocess.Popen(command, cwd=research_root, stdout=stdout, stderr=stderr,
                                  start_new_session=os.name == "posix") as child:
                try:
                    receipt["returncode"] = child.wait(timeout=timeout)
                except BaseException:
                    try:
                        if os.name == "posix":
                            os.killpg(child.pid, signal.SIGKILL)
                        else:
                            child.kill()
                    except ProcessLookupError:
                        pass
                    child.wait()
                    receipt["returncode"] = child.returncode
                    raise
        if receipt["returncode"] != 0:
            receipt["errors"].append(f"consumer process exited {receipt['returncode']}")
        receipt["errors"].extend(sweep_errors(read_sweep(out_dir / "tone-sweep.json"), names))
        receipt["status"] = "failed" if receipt["errors"] else "completed"
    except Exception as error:
        receipt["status"] = "failed"
        receipt["errors"].append(str(error))
    finally:
        receipt["wall_seconds"] = time.monotonic() - started
        if receipt["status"] == "running":
            receipt["status"] = "cancelled"
        # Only known text logs leave this boundary. Never copy trace/pixels
        # dictionaries, resource blobs, HEIF files or rendered images.
        logs = [("build", out_dir / "build.log")]
        for name in names:
            logs.append((f"{name}-extract", out_dir / name / "extract.log"))
            logs.extend((f"{name}-tone-{tone:g}", out_dir / name / f"tone-{tone:g}.log") for tone in TONES)
        for label, path in logs:
            if not path.is_file():
                continue
            raw = path.read_bytes()
            lines, omitted, omitted_facts = [], 0, 0
            in_native_facts = False
            for line in raw.decode("utf-8", errors="replace").splitlines():
                # native_filter_probe.m prints its final NSDictionary with
                # facts.description. Keep preceding stderr, not that multiline
                # stdout dictionary. These are the fixed probe's stderr tags.
                if path.name.startswith("tone-") and line.lstrip().startswith("{"):
                    in_native_facts = True
                if in_native_facts and not line.startswith(("Trace exception:", "NATIVE ASSERTION:")):
                    omitted_facts += 1
                    continue
                try:
                    value = json.loads(line)
                except ValueError:
                    value = None
                # The extractor prints its raw inventory to combined stdout;
                # retain stderr/traceback while excluding that dictionary.
                if isinstance(value, dict) and {"source_sha256", "primary_id", "items"} <= value.keys():
                    omitted += 1
                else:
                    lines.append(line)
            target = diagnostics / f"{label}.log"
            target.write_text("\n".join(lines) + ("\n" if lines else ""))
            receipt["diagnostics"].append({"log": target.name, "source_log_sha256": hashlib.sha256(raw).hexdigest(),
                                           "omitted_inventory_lines": omitted,
                                           "omitted_native_facts_lines": omitted_facts})
        receipt["native_failures"] = []
        for name in names:
            for tone in TONES:
                facts_path = out_dir / name / f"tone-{tone:g}" / "facts.json"
                if not facts_path.is_file():
                    continue
                try:
                    facts = json.loads(facts_path.read_text())
                    if isinstance(facts.get("exception"), str):
                        receipt["native_failures"].append({"photo": name, "tone": tone, "exception": facts["exception"]})
                    for trial in facts.get("trials", []):
                        if isinstance(trial.get("exception"), str):
                            receipt["native_failures"].append({"photo": name, "tone": tone,
                                                               "use_style_engine": trial.get("use_style_engine")
                                                               if type(trial.get("use_style_engine")) is bool else None,
                                                               "exception": trial["exception"]})
                except (OSError, ValueError, TypeError, AttributeError):
                    receipt["native_failures"].append({"photo": name, "tone": tone,
                                                       "error": "native exception diagnostic is unreadable"})
        receipt_path.write_text(json.dumps(receipt, indent=2) + "\n")
    stdout_path = diagnostics / "consumer.stdout.log"
    if stdout_path.is_file():
        print(stdout_path.read_text(), end="")
    if receipt["status"] != "completed":
        print("Tone consumer failed: " + "; ".join(receipt["errors"]), file=sys.stderr)
        for path in sorted(diagnostics.glob("*.log")):
            text = path.read_text()
            if text:
                print(f"--- {path.name} ---\n{text}", file=sys.stderr)
    return receipt


def luminance(rgb: np.ndarray) -> np.ndarray:
    return rgb.astype(np.float64) @ Y_WEIGHTS


def load_native_rgb(renderer: Path) -> np.ndarray:
    trace = json.loads((renderer / "trace.json").read_text())
    for field in ("process_status", "finish_status"):
        if type(trace.get(field)) is not int:
            raise ValueError(f"{renderer}: native {field} missing or malformed")
    texture = trace["textures"]["_outputImageTexture"]
    width, height = texture["width"], texture["height"]
    if type(width) is not int or type(height) is not int or width <= 0 or height <= 0:
        raise ValueError(f"{renderer}: invalid native render dimensions")
    path = renderer / "_outputImageTexture.bin"
    expected = height * width * 4
    if path.stat().st_size != expected * 2:
        raise ValueError(f"{renderer}: expected {expected * 2} native render bytes, got {path.stat().st_size}")
    raw = np.fromfile(path, dtype="<f2")
    rgb = raw.reshape(height, width, 4)[:, :, :3].astype(np.float32)
    if not np.isfinite(rgb).all():
        raise ValueError(f"{renderer}: native RGB contains nonfinite samples")
    return rgb


def _edge_ratio(base: np.ndarray, other: np.ndarray) -> float:
    y = luminance(base)
    magnitude = np.abs(np.diff(y, axis=0)[:, :-1]) + np.abs(np.diff(y, axis=1)[:-1, :])
    delta = np.abs(luminance(other) - y)[:-1, :-1]
    if magnitude.size == 0:
        return 1.0
    cutoff = np.quantile(magnitude, 0.95)
    edge = delta[magnitude >= cutoff]
    flat = delta[magnitude <= np.quantile(magnitude, 0.50)]
    flat_mean = float(flat.mean()) if flat.size else 0.0
    if flat_mean <= 1e-8:
        return float(edge.mean()) if edge.size else 1.0
    return float(edge.mean() / flat_mean)


def score_photo(images: dict[float, np.ndarray]) -> dict:
    base = images[0.0]
    rows = []
    for tone in sorted(images):
        image = images[tone]
        delta = image.astype(np.float64) - base.astype(np.float64)
        y = luminance(image)
        y0 = luminance(base)
        shown = np.clip(y, 0.0, 1.0)
        codes = np.rint(shown * 255.0).astype(np.uint8)
        rows.append(
            {
                "tone": tone,
                "width": int(image.shape[1]),
                "height": int(image.shape[0]),
                "mean_luminance": float(y.mean()),
                "mean_rgb": [float(value) for value in image.reshape(-1, 3).mean(axis=0)],
                "max_abs_vs_tone0": float(np.max(np.abs(delta))),
                "mean_abs_vs_tone0": float(np.mean(np.abs(delta))),
                "mean_delta_rgb": [float(value) for value in delta.reshape(-1, 3).mean(axis=0)],
                "mean_luminance_delta": float((y - y0).mean()),
                "unique_8bit_luma": int(np.unique(codes).size),
                "fraction_above_one": float(np.mean(image > 1.0)),
                "fraction_below_zero": float(np.mean(image < 0.0)),
            }
        )
    means = [row["mean_luminance"] for row in rows]
    steps = [
        {
            "from_tone": rows[index]["tone"],
            "to_tone": rows[index + 1]["tone"],
            "mean_luminance_step": means[index + 1] - means[index],
        }
        for index in range(len(rows) - 1)
    ]
    step_values = [step["mean_luminance_step"] for step in steps]
    positive = [value for value in step_values if value > STEP_EPSILON]
    negative = [value for value in step_values if value < -STEP_EPSILON]
    monotonic = not (positive and negative)
    by_tone = {row["tone"]: row["mean_luminance"] for row in rows}
    dark_abs = bright_abs = luminance_ratio = None
    signed_minus = signed_plus = None
    if -1.0 in by_tone and 0.0 in by_tone and 1.0 in by_tone:
        signed_minus = by_tone[-1.0] - by_tone[0.0]
        signed_plus = by_tone[1.0] - by_tone[0.0]
        dark_abs = abs(signed_minus)
        bright_abs = abs(signed_plus)
        luminance_ratio = None if bright_abs < 1e-8 else dark_abs / bright_abs
    inactive = all(row["max_abs_vs_tone0"] < 1e-3 for row in rows)
    concerns = []
    if inactive:
        concerns.append("tone bias leaves the native render unchanged")
    if not monotonic:
        concerns.append("mean luminance is not monotonic across the tone sweep")
    base_levels = next(row["unique_8bit_luma"] for row in rows if row["tone"] == 0.0)
    for row in rows:
        if row["tone"] == 0.0:
            continue
        if base_levels > 128 and row["unique_8bit_luma"] < 32:
            concerns.append(f"tone {row['tone']:g} collapses 8-bit luminance levels")
        deltas = np.array(row["mean_delta_rgb"])
        order = np.argsort(np.abs(deltas))
        if abs(deltas[order[-1]]) > 0.05 and abs(deltas[order[-1]]) > 3.0 * abs(deltas[order[-2]]):
            concerns.append(f"tone {row['tone']:g} shifts one channel much more than the others")
        edge_ratio = _edge_ratio(base, images[row["tone"]])
        row["edge_to_flat_delta_ratio"] = edge_ratio
        if edge_ratio > 8.0 and row["mean_abs_vs_tone0"] > 0.02:
            concerns.append(f"tone {row['tone']:g} concentrates its change on edges")
        if row["fraction_above_one"] > 0.25 or row["fraction_below_zero"] > 0.02:
            concerns.append(f"tone {row['tone']:g} clips a large share of samples")
    return {
        "tones": rows,
        "luminance_steps": steps,
        "monotonic_mean_luminance": monotonic,
        "darken_abs": dark_abs,
        "brighten_abs": bright_abs,
        "signed_luminance_at_minus_1": signed_minus,
        "signed_luminance_at_plus_1": signed_plus,
        "darken_brighten_ratio": luminance_ratio,
        "tone_changes_render": not inactive,
        "concerns": concerns,
    }


def tone_dirs(photo_dir: Path) -> dict[float, Path]:
    found = {}
    for child in photo_dir.iterdir():
        if not child.is_dir() or not child.name.startswith("tone-"):
            continue
        try:
            tone = float(child.name.removeprefix("tone-"))
        except ValueError as error:
            raise ValueError(f"invalid tone directory: {child.name}") from error
        if tone not in TONES or tone in found:
            raise ValueError(f"unexpected or duplicate tone directory: {child.name}")
        renderer = child / "renderer-0"
        if (renderer / "_outputImageTexture.bin").is_file():
            found[tone] = renderer
    return found


def read_sweep(path: Path) -> list[dict]:
    if not path.is_file():
        raise ValueError(f"consumer sweep receipt missing: {path}")
    rows = json.loads(path.read_text())
    if not isinstance(rows, list) or not rows or not all(isinstance(row, dict) for row in rows):
        raise ValueError(f"consumer sweep receipt is empty or malformed: {path}")
    return rows


def sanity_gate(rows: list[dict]) -> dict:
    tone0 = [row for row in rows if row.get("tone") == 0 or row.get("tone") == 0.0]
    named = [row for row in tone0 if str(row.get("photo", "")).startswith("IMG_0032")]
    chosen = named
    if len(chosen) != 1:
        return {"trusted": False, "reason": "IMG_0032 tone 0 row missing", "rows": chosen}
    row = chosen[0]
    differing = row.get("differing_components")
    trusted = (type(row.get("returncode")) is int and row["returncode"] == 0 and type(row.get("components")) is int
               and row["components"] > 0 and type(differing) is int and differing == 0
               and _finite_number(row.get("max_abs")) and row["max_abs"] == 0 and not row.get("error"))
    return {
        "trusted": trusted,
        "photo": row.get("photo"),
        "differing_components": differing,
        "max_abs": row.get("max_abs"),
        "psnr_peak1_db": row.get("psnr_peak1_db"),
        "returncode": row.get("returncode"),
        "error": row.get("error"),
        "reason": None if trusted else "tone 0 model and native output differ, or the render failed",
    }


def _srgb(linear: np.ndarray) -> np.ndarray:
    clipped = np.clip(linear, 0.0, 1.0)
    encoded = np.where(
        clipped <= 0.0031308,
        clipped * 12.92,
        1.055 * np.power(clipped, 1.0 / 2.4) - 0.055,
    )
    return np.clip(np.rint(encoded * 255.0), 0, 255).astype(np.uint8)


def _box(image: np.ndarray, long_edge: int) -> np.ndarray:
    height, width = image.shape[:2]
    scale = long_edge / max(height, width)
    if scale >= 1:
        return image
    out_h = max(1, int(round(height * scale)))
    out_w = max(1, int(round(width * scale)))
    ys = (np.arange(out_h) * height / out_h).astype(int)
    xs = (np.arange(out_w) * width / out_w).astype(int)
    y2 = np.clip(ys + np.maximum(1, height // out_h), 0, height)
    x2 = np.clip(xs + np.maximum(1, width // out_w), 0, width)
    output = np.empty((out_h, out_w, image.shape[2]), dtype=np.float32)
    for row, (y0, y1) in enumerate(zip(ys, y2)):
        for column, (x0, x1) in enumerate(zip(xs, x2)):
            output[row, column] = image[y0:y1, x0:x1].mean(axis=(0, 1))
    return output


def write_comparison(images: dict[float, np.ndarray], path: Path) -> None:
    try:
        from PIL import Image
    except ImportError:
        return
    tones = [tone for tone in TONES if tone in images]
    tiles = [_box(images[tone], 160) for tone in tones]
    base = _box(images[0.0], 160) if 0.0 in images else tiles[0]
    height, width = tiles[0].shape[:2]
    canvas = np.zeros((height * 2, width * len(tiles), 3), dtype=np.uint8)
    for index, (tone, tile) in enumerate(zip(tones, tiles)):
        canvas[:height, index * width : (index + 1) * width] = _srgb(tile)
        delta = np.clip(0.5 + 4.0 * (tile.astype(np.float64) - base.astype(np.float64)), 0.0, 1.0)
        canvas[height:, index * width : (index + 1) * width] = _srgb(delta)
    path.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(canvas, mode="RGB").save(path)


def input_keys(photo_dir: Path) -> list[str]:
    for child in sorted(photo_dir.glob("tone-*/checkpoint.json")):
        payload = json.loads(child.read_text())
        defaults = payload.get("defaults")
        if isinstance(defaults, dict):
            return sorted(defaults)
    return []


def analyze(sanity_path: Path, render_root: Path, out_dir: Path) -> dict:
    errors = []
    sanity_rows, agreement, expected = [], [], []
    try:
        sanity_rows = read_sweep(sanity_path)
    except (OSError, ValueError) as error:
        errors.append(str(error))
    gate = sanity_gate(sanity_rows)
    try:
        receipt = json.loads((render_root / "consumer-run.json").read_text())
        expected = receipt["photos"]
        if (not isinstance(expected, list) or not expected or not all(isinstance(name, str) and name for name in expected)
                or len(set(expected)) != len(expected) or receipt.get("tones") != list(TONES)):
            raise ValueError("consumer request identity has missing/duplicate photos or tones")
        if (receipt.get("status") != "completed" or type(receipt.get("returncode")) is not int
                or receipt["returncode"] != 0):
            errors.append(f"consumer execution failed: {receipt.get('errors', receipt.get('status'))}")
    except (OSError, ValueError, KeyError, TypeError) as error:
        errors.append(f"consumer execution receipt invalid: {error}")
        expected = []
    try:
        agreement = read_sweep(render_root / "tone-sweep.json")
        errors.extend(sweep_errors(agreement, expected))
    except (OSError, ValueError) as error:
        errors.append(str(error))
    photos = []
    for name in expected:
        photo_dir = render_root / name
        entry = {"photo": name, "rendered_tones": [], "input_keys": []}
        try:
            renders = tone_dirs(photo_dir)
            entry.update(rendered_tones=sorted(renders), input_keys=input_keys(photo_dir))
            if set(renders) != set(TONES):
                raise ValueError("native render missing for a requested comparison tone")
            images = {tone: load_native_rgb(path) for tone, path in renders.items()}
            entry["native_statuses"] = {
                tone: {key: value for key, value in json.loads((path / "trace.json").read_text()).items()
                       if key in ("process_status", "finish_status")}
                for tone, path in renders.items()
            }
            shapes = {image.shape for image in images.values()}
            if len(shapes) != 1:
                raise ValueError("native render geometry changes across the tone sweep")
            components = images[0.0].size
            if any(row.get("components") != components for row in agreement if row.get("photo") == name):
                raise ValueError("native/model comparison components differ from the native texture")
            entry.update(score_photo(images))
            write_comparison(images, out_dir / "comparisons" / f"{photo_dir.name}.png")
        except (OSError, ValueError, KeyError, TypeError, IndexError) as error:
            entry["errors"] = [str(error)]
            entry["concerns"] = [str(error)]
            errors.append(f"{name}: {error}")
        photos.append(entry)
    if not expected:
        errors.append("no requested photos with complete native measurements")
    report = {
        "sanity": gate,
        "sanity_rows": sanity_rows,
        "model_agreement": agreement,
        "photos": photos,
        "color_control": "The harness sets inputToneBias only. It has no color-bias argument.",
        "evidence_scope": "Complete native Tone measurements do not establish Photos Tone/Color editing admission or private status-code domains.",
        "status": "failed" if errors or not gate.get("trusted") else "completed",
        "measurement_complete": not errors,
        "errors": errors,
        "trusted": bool(gate.get("trusted")) and not errors,
    }
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "tone-response.json").write_text(json.dumps(report, indent=2) + "\n")
    (out_dir / "tone-response.md").write_text(render_markdown(report))
    return report


def render_markdown(report: dict) -> str:
    lines = ["# Photographic Styles tone response", ""]
    gate = report["sanity"]
    lines.append(
        f"Host gate trusted: {gate.get('trusted')}. "
        f"Tone 0 differing components: {gate.get('differing_components')}."
    )
    lines.append(f"Measurement status: {report.get('status')}. Product measurements complete: {report.get('measurement_complete')}.")
    for error in report.get("errors", []):
        lines.append(f"- {error}")
    if gate.get("reason"):
        lines.append(gate["reason"])
    lines.append("")
    lines.append(report["color_control"])
    lines.append(report["evidence_scope"])
    lines.append("")
    for photo in report["photos"]:
        lines.append(f"## {photo['photo']}")
        lines.append("")
        ratio = photo.get("darken_brighten_ratio")
        ratio_text = "n/a" if ratio is None else f"{ratio:.4f}"
        lines.append(
            f"Monotonic mean luminance: {photo.get('monotonic_mean_luminance')}. "
            f"L(-1)-L(0) = {photo.get('signed_luminance_at_minus_1')}. "
            f"L(+1)-L(0) = {photo.get('signed_luminance_at_plus_1')}. "
            f"|L(-1)-L(0)| / |L(+1)-L(0)| = {ratio_text}."
        )
        lines.append("")
        lines.append("| tone | mean Y | mean R | mean G | mean B |")
        lines.append("|---:|---:|---:|---:|---:|")
        for row in photo.get("tones", []):
            red, green, blue = row["mean_rgb"]
            lines.append(
                f"| {row['tone']:g} | {row['mean_luminance']:.5f} | "
                f"{red:.5f} | {green:.5f} | {blue:.5f} |"
            )
        if photo.get("concerns"):
            lines.append("")
            for concern in photo["concerns"]:
                lines.append(f"- {concern}")
        lines.append("")
    lines.append("## Model versus native")
    lines.append("")
    lines.append("| photo | tone | differing | max abs | PSNR |")
    lines.append("|---|---:|---:|---:|---:|")
    for row in report.get("model_agreement", []):
        lines.append(
            f"| {row.get('photo')} | {row.get('tone')} | {row.get('differing_components', row.get('error'))} | "
            f"{row.get('max_abs', '')} | {row.get('psnr_peak1_db', '')} |"
        )
    lines.append("")
    return "\n".join(lines)


def _scalar_leaves(value, prefix: str = ""):
    if isinstance(value, dict):
        if set(value) == {"bytes", "sha256"}:
            yield prefix, value
            return
        for key, item in value.items():
            path = f"{prefix}.{key}" if prefix else str(key)
            yield from _scalar_leaves(item, path)
        return
    if isinstance(value, list):
        yield prefix, value
        return
    yield prefix, value


def _gtc_summary(path: Path) -> dict:
    raw = np.fromfile(path, dtype="<u2")
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if path.stat().st_size % 2 or raw.size < 258:
        raise ValueError(f"{path}: short or damaged curve ({raw.size} values)")
    nodes = raw[1:257].astype(np.float64) / 65534.0
    return {
        "count": int(raw[0]),
        "sha256": digest,
        "node0": float(nodes[0]),
        "node64": float(nodes[64]),
        "node128": float(nodes[128]),
        "node192": float(nodes[192]),
        "node255": float(nodes[255]),
        "mean": float(nodes.mean()),
        "minimum": float(nodes.min()),
        "maximum": float(nodes.max()),
        "monotonic_nodes": bool(np.all(np.diff(nodes) >= -1e-6)),
    }


def _half_summary(path: Path) -> dict:
    raw = np.fromfile(path, dtype="<f2").astype(np.float64)
    finite = raw[np.isfinite(raw)]
    if path.stat().st_size % 2 or not raw.size or finite.size != raw.size:
        raise ValueError(f"{path}: empty, damaged or nonfinite light map")
    return {
        "count": int(raw.size),
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "mean": None if finite.size == 0 else float(finite.mean()),
        "minimum": None if finite.size == 0 else float(finite.min()),
        "maximum": None if finite.size == 0 else float(finite.max()),
    }


def load_style_metadata(resources: Path) -> list[dict]:
    if not resources.is_dir():
        return []
    records = []
    for photo in sorted(path for path in resources.iterdir() if path.is_dir()):
        record = {"photo": photo.name, "scalars": {}}
        summary_path = photo / "styles-summary.json"
        if summary_path.is_file():
            summary = json.loads(summary_path.read_text())
            if not isinstance(summary, dict) or not summary:
                raise ValueError(f"{summary_path}: style metadata dictionary is empty or malformed")
            record["scalars"] = dict(_scalar_leaves(summary))
        else:
            record["missing_summary"] = True
        curve = photo / "key3.bin"
        if curve.is_file():
            record["gtc"] = _gtc_summary(curve)
        for key, name in (("c", "light_c"), ("d", "light_d")):
            blob = photo / f"key{key}.bin"
            if blob.is_file():
                record[name] = _half_summary(blob)
        records.append(record)
    return records


def _flat_metadata(record: dict) -> dict:
    flat = {}
    for key, value in record.get("scalars", {}).items():
        flat[key] = value
    for group in ("gtc", "light_c", "light_d"):
        payload = record.get(group)
        if isinstance(payload, dict):
            for key, value in payload.items():
                flat[f"{group}.{key}"] = value
    return flat


def _as_float(value):
    if isinstance(value, bool) or isinstance(value, (dict, list)) or value is None:
        return None
    if _finite_number(value):
        return float(value)
    return None


def metadata_ranges(records: list[dict]) -> dict:
    apple = [record for record in records if record.get("group") == "apple"]
    xdremux = [record for record in records if record.get("group") == "xdremux"]
    paths = sorted({key for record in records for key in _flat_metadata(record)})
    fields = []
    for path in paths:
        apple_values = []
        for record in apple:
            number = _as_float(_flat_metadata(record).get(path))
            if number is not None:
                apple_values.append(number)
        if not apple_values:
            continue
        low, high = min(apple_values), max(apple_values)
        outside = []
        for record in xdremux:
            number = _as_float(_flat_metadata(record).get(path))
            if number is None:
                continue
            if number < low - 1e-9 or number > high + 1e-9:
                outside.append({"photo": record["photo"], "value": number})
        fields.append(
            {
                "field": path,
                "apple_min": low,
                "apple_max": high,
                "xdremux_outside": outside,
            }
        )
    return {"fields": fields}


def curve_span(photos: list[dict]) -> dict:
    apple = [
        photo["darken_brighten_ratio"]
        for photo in photos
        if photo.get("group") == "apple" and isinstance(photo.get("darken_brighten_ratio"), (int, float))
    ]
    span = None if not apple else {"min": min(apple), "max": max(apple)}
    verdicts = []
    for photo in photos:
        if photo.get("group") != "xdremux":
            continue
        ratio = photo.get("darken_brighten_ratio")
        inside = None
        if span is not None and isinstance(ratio, (int, float)):
            inside = span["min"] - 1e-9 <= ratio <= span["max"] + 1e-9
        verdicts.append({"photo": photo["photo"], "ratio": ratio, "inside_apple_range": inside})
    return {"apple_ratio_range": span, "xdremux": verdicts}


def _group_for(name: str) -> str:
    if "xdremux" in name:
        return "xdremux"
    if name.startswith("IMG_"):
        return "apple"
    return "other"


def write_asymmetry(apple_report: dict, xdremux_report: dict, resources: Path | None, out_dir: Path) -> dict:
    if not isinstance(apple_report, dict) or not isinstance(xdremux_report, dict):
        raise ValueError("Apple/product tone reports must be dictionaries")
    errors = []
    photos = []
    for report, group in ((apple_report, "apple"), (xdremux_report, "xdremux")):
        if report.get("trusted") is not True or report.get("status") != "completed" or not report.get("measurement_complete"):
            errors.append(f"{group} native measurement report is incomplete or untrusted")
        report_photos = report.get("photos")
        if not isinstance(report_photos, list) or not report_photos:
            errors.append(f"{group} photo measurements missing")
            report_photos = []
        for photo in report_photos:
            if not isinstance(photo, dict) or not isinstance(photo.get("photo"), str):
                errors.append(f"{group} photo measurement identity malformed")
                continue
            item = dict(photo)
            item["group"] = group
            photos.append(item)
            rows = photo.get("tones", [])
            if (not isinstance(rows, list) or len(rows) != len(TONES)
                    or not all(isinstance(row, dict) for row in rows)
                    or {row.get("tone") for row in rows} != set(TONES)
                    or any(not _finite_number(row.get("mean_luminance"))
                           or not isinstance(row.get("mean_rgb"), list) or len(row["mean_rgb"]) != 3
                           or not all(_finite_number(value) for value in row["mean_rgb"]) for row in rows)):
                errors.append(f"{group} {photo.get('photo')}: complete finite native tone statistics missing")
            fields = ("darken_abs", "brighten_abs", "signed_luminance_at_minus_1", "signed_luminance_at_plus_1",
                      "darken_brighten_ratio")
            if (any(not _finite_number(photo.get(field)) for field in fields)
                    or type(photo.get("monotonic_mean_luminance")) is not bool):
                errors.append(f"{group} {photo.get('photo')}: required asymmetry comparison values missing")
    metadata = []
    try:
        metadata = load_style_metadata(resources) if resources else []
    except (OSError, ValueError, TypeError) as error:
        errors.append(f"metadata comparison unavailable: {error}")
    for record in metadata:
        record["group"] = _group_for(record["photo"])
    by_name = {record["photo"]: record for record in metadata}
    for photo in photos:
        record = by_name.get(photo["photo"], {})
        if not record.get("scalars") or any(key not in record for key in ("gtc", "light_c", "light_d")):
            errors.append(f"{photo['photo']}: required style metadata/curve/light-map comparison data missing")
    ranges = metadata_ranges(metadata)
    if not ranges["fields"]:
        errors.append("no comparable Apple/product numeric metadata fields")
    payload = {
        "status": "failed" if errors else "completed",
        "trusted": not errors,
        "errors": errors,
        "curves": curve_span(photos),
        "photos": [
            {
                "photo": photo["photo"],
                "group": photo["group"],
                "monotonic_mean_luminance": photo.get("monotonic_mean_luminance"),
                "darken_abs": photo.get("darken_abs"),
                "brighten_abs": photo.get("brighten_abs"),
                "signed_luminance_at_minus_1": photo.get("signed_luminance_at_minus_1"),
                "signed_luminance_at_plus_1": photo.get("signed_luminance_at_plus_1"),
                "darken_brighten_ratio": photo.get("darken_brighten_ratio"),
            }
            for photo in photos
        ],
        "metadata": metadata,
        "metadata_ranges": ranges,
    }
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "asymmetry.json").write_text(json.dumps(payload, indent=2) + "\n")
    (out_dir / "asymmetry.md").write_text(_asymmetry_markdown(payload))
    return payload


def _asymmetry_markdown(payload: dict) -> str:
    lines = ["# Tone asymmetry", ""]
    lines.append(f"Comparison status: {payload.get('status')}.")
    for error in payload.get("errors", []):
        lines.append(f"- {error}")
    lines.append("")
    span = payload["curves"]["apple_ratio_range"]
    if span is None:
        lines.append("Apple captures produced no darken/brighten ratio.")
    else:
        lines.append(
            f"Apple |L(-1)-L(0)| / |L(+1)-L(0)| ranges from {span['min']:.4f} to {span['max']:.4f}."
        )
    lines.append("")
    lines.append("| photo | group | monotonic | L(-1)-L(0) | L(+1)-L(0) | ratio | inside Apple range |")
    lines.append("|---|---|---|---:|---:|---:|---|")
    inside = {item["photo"]: item["inside_apple_range"] for item in payload["curves"]["xdremux"]}
    for photo in payload["photos"]:
        ratio = photo["darken_brighten_ratio"]
        ratio_text = "n/a" if ratio is None else f"{ratio:.4f}"
        lines.append(
            f"| {photo['photo']} | {photo['group']} | {photo['monotonic_mean_luminance']} | "
            f"{photo.get('signed_luminance_at_minus_1', photo.get('darken_abs'))} | "
            f"{photo.get('signed_luminance_at_plus_1', photo.get('brighten_abs'))} | {ratio_text} | "
            f"{inside.get(photo['photo'], '')} |"
        )
    lines.append("")
    lines.append("## Plist fields outside the Apple numeric range")
    lines.append("")
    outside_fields = [field for field in payload["metadata_ranges"]["fields"] if field["xdremux_outside"]]
    if payload.get("errors"):
        lines.append("Metadata comparison is incomplete; no range-admission conclusion is available.")
    elif not outside_fields:
        lines.append("No numeric plist field on an XDRemux output falls outside the Apple captures.")
    else:
        lines.append("| field | Apple min | Apple max | XDRemux outside |")
        lines.append("|---|---:|---:|---|")
        for field in outside_fields:
            rendered = ", ".join(
                f"{item['photo']}={item['value']:.6g}" for item in field["xdremux_outside"]
            )
            lines.append(
                f"| {field['field']} | {field['apple_min']:.6g} | {field['apple_max']:.6g} | {rendered} |"
            )
    lines.append("")
    lines.append("## Tone-related plist values")
    lines.append("")
    interesting = (
        "0",
        "4",
        "5",
        "h",
        "j",
        "k",
        "7.PeopleRatio",
        "7.PersonMasksValidHint",
        "7.SkinRatio",
        "i.Gain",
        "i.OriginalRangeMin",
        "i.OriginalRangeMax",
        "gtc.node0",
        "gtc.node64",
        "gtc.node128",
        "gtc.node192",
        "gtc.node255",
        "gtc.mean",
        "gtc.monotonic_nodes",
        "light_c.mean",
        "light_d.mean",
    )
    header = "| field | " + " | ".join(record["photo"] for record in payload["metadata"]) + " |"
    if payload["metadata"]:
        lines.append(header)
        lines.append("|---|" + "|".join("---:" for _ in payload["metadata"]) + "|")
        flats = [_flat_metadata(record) for record in payload["metadata"]]
        for field in interesting:
            cells = []
            for flat in flats:
                value = flat.get(field, "")
                if isinstance(value, float):
                    cells.append(f"{value:.6g}")
                else:
                    cells.append(str(value))
            lines.append("| " + field + " | " + " | ".join(cells) + " |")
    lines.append("")
    return "\n".join(lines)


def self_test() -> None:
    height, width = 24, 32
    base = np.full((height, width, 3), 0.4, dtype=np.float32)
    images = {}
    for tone in TONES:
        shift = 0.05 * tone
        images[tone] = np.clip(base + shift, 0.0, 1.0)
    scored = score_photo(images)
    if not scored["monotonic_mean_luminance"] or not scored["tone_changes_render"]:
        raise SystemExit(f"monotonic sweep was not recognized: {scored}")
    if abs(scored["darken_brighten_ratio"] - 1.0) > 1e-6:
        raise SystemExit(f"symmetric sweep ratio was not 1: {scored['darken_brighten_ratio']}")
    if abs(scored["darken_brighten_ratio"] - scored["darken_abs"] / scored["brighten_abs"]) > 1e-9:
        raise SystemExit("luminance ratio was overwritten")
    asymmetric = {}
    for tone in TONES:
        shift = 0.01 * min(tone, 0.0) + 0.20 * max(tone, 0.0)
        asymmetric[tone] = np.clip(base + shift, 0.0, 1.0)
    uneven = score_photo(asymmetric)
    if abs(uneven["darken_brighten_ratio"] - 0.05) > 1e-6:
        raise SystemExit(f"uneven luminance ratio was {uneven['darken_brighten_ratio']}")
    if uneven["signed_luminance_at_minus_1"] >= 0 or uneven["signed_luminance_at_plus_1"] <= 0:
        raise SystemExit(uneven)
    flat = {tone: base.copy() for tone in TONES}
    inactive = score_photo(flat)
    if inactive["tone_changes_render"] or "tone bias leaves the native render unchanged" not in inactive["concerns"]:
        raise SystemExit(f"inactive sweep was not recognized: {inactive}")
    cast = {tone: base.copy() for tone in TONES}
    cast[1.0] = base + np.array([0.2, 0.0, 0.0], dtype=np.float32)
    cast[-1.0] = base + np.array([-0.08, 0.0, 0.0], dtype=np.float32)
    colored = score_photo(cast)
    if not any("one channel" in concern for concern in colored["concerns"]):
        raise SystemExit(f"channel imbalance was not recognized: {colored['concerns']}")
    gate = sanity_gate([{"photo": "IMG_0032", "tone": 0, "returncode": 0, "components": 12,
                         "differing_components": 0, "max_abs": 0}])
    if not gate["trusted"]:
        raise SystemExit(gate)
    bad = sanity_gate([{"photo": "IMG_0032", "tone": 0, "differing_components": 12}])
    if bad["trusted"]:
        raise SystemExit(bad)
    mixed = sanity_gate(
        [
            {"photo": "IMG_0033", "tone": 0, "differing_components": 9},
            {"photo": "IMG_0032", "tone": 0, "returncode": 0, "components": 12,
             "differing_components": 0, "max_abs": 0},
        ]
    )
    if not mixed["trusted"] or mixed["photo"] != "IMG_0032":
        raise SystemExit(mixed)
    span = curve_span(
        [
            {"photo": "IMG_0032", "group": "apple", "darken_brighten_ratio": 0.2},
            {"photo": "IMG_0033", "group": "apple", "darken_brighten_ratio": 0.9},
            {"photo": "sample.xdremux", "group": "xdremux", "darken_brighten_ratio": 0.05},
        ]
    )
    if span["xdremux"][0]["inside_apple_range"] is not False:
        raise SystemExit(span)
    ranges = metadata_ranges(
        [
            {"photo": "IMG_0032", "group": "apple", "scalars": {"h": 1.2}},
            {"photo": "IMG_0033", "group": "apple", "scalars": {"h": 1.8}},
            {"photo": "sample.xdremux", "group": "xdremux", "scalars": {"h": 4.0}},
        ]
    )
    outside = ranges["fields"][0]["xdremux_outside"]
    if not outside or outside[0]["photo"] != "sample.xdremux":
        raise SystemExit(ranges)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--sanity", type=Path)
    parser.add_argument("--renders", type=Path)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--compare", nargs=2, type=Path, metavar=("APPLE_REPORT", "XDREMUX_REPORT"))
    parser.add_argument("--metadata", type=Path)
    parser.add_argument("--run-consumer", type=Path, metavar="RESEARCH_ROOT")
    parser.add_argument("--photos", nargs="+", type=Path)
    args = parser.parse_args()
    if args.self_test:
        self_test()
        print("style tone response self-test passed")
        return 0
    if args.run_consumer:
        if not args.out or not args.photos:
            parser.error("--out and --photos are required with --run-consumer")
        receipt = run_consumer(args.run_consumer, args.photos, args.out)
        print(json.dumps(receipt, indent=2))
        return 0 if receipt["status"] == "completed" else 3
    if args.compare:
        if not args.out:
            parser.error("--out is required with --compare")
        try:
            apple_report = json.loads(args.compare[0].read_text())
            xdremux_report = json.loads(args.compare[1].read_text())
            report = write_asymmetry(apple_report, xdremux_report, args.metadata, args.out)
        except (OSError, ValueError, KeyError, TypeError) as error:
            report = {"status": "failed", "trusted": False, "errors": [str(error)]}
            args.out.mkdir(parents=True, exist_ok=True)
            (args.out / "asymmetry.json").write_text(json.dumps(report, indent=2) + "\n")
            (args.out / "asymmetry.md").write_text("Tone comparison failed: " + str(error) + "\n")
        print((args.out / "asymmetry.md").read_text())
        return 0 if report["trusted"] else 3
    if not args.sanity or not args.renders or not args.out:
        parser.error("--sanity, --renders, and --out are required")
    report = analyze(args.sanity, args.renders, args.out)
    print((args.out / "tone-response.md").read_text())
    return 0 if report["trusted"] else 3


if __name__ == "__main__":
    sys.exit(main())
