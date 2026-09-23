"""Public synthetic media gate for CarrierProbe; no Photos/device claim."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from research.video_styles.probe import VARIANTS, digest, failure_evidence, run

CODECS = (
    ("h264", ["-c:v", "libx264", "-pix_fmt", "yuv420p", "-bf", "2"]),
    ("hevc10", ["-c:v", "libx265", "-pix_fmt", "yuv420p10le", "-tag:v", "hvc1",
                "-x265-params", "pools=1:frame-threads=1:log-level=error",
                "-color_primaries", "bt2020", "-color_trc", "smpte2084", "-colorspace", "bt2020nc"]),
)


def command(*arguments: str) -> None:
    subprocess.run(list(arguments), check=True, timeout=120)


def prepare_source(root: Path, codec: str, extra: list[str]) -> Path:
    source = root / f"{codec}.mov"
    command("ffmpeg", "-v", "error", "-f", "lavfi", "-i", "testsrc2=size=96x64:rate=12:duration=1",
            "-f", "lavfi", "-i", "sine=frequency=523:sample_rate=48000:duration=1",
            *extra, "-c:a", "aac", "-metadata", "title=XDRemux synthetic probe", "-shortest", str(source))
    rotated = root / f"{codec}-rotated.mov"
    command("ffmpeg", "-v", "error", "-i", str(source), "-map", "0", "-c", "copy",
            "-metadata:s:v:0", "rotate=90", str(rotated))
    return rotated


def run_matrix(helper: Path, root: Path) -> dict:
    """Exercise every mandatory case even after a failure; no skipped-green cases."""
    receipts: list[dict] = []
    prerequisites: dict = {"passed": True}
    try:
        helper = helper.resolve(strict=True)
        if not helper.is_file() or not os.access(helper, os.X_OK):
            raise ValueError("compiled helper is not executable")
        for executable in ("ffmpeg", "ffprobe"):
            if shutil.which(executable) is None:
                raise ValueError(f"missing prerequisite: {executable}")
    except (OSError, ValueError) as error:
        prerequisites = {"passed": False, "error": str(error)}

    for codec, extra in CODECS:
        try:
            if not prerequisites["passed"]:
                raise ValueError(prerequisites["error"])
            source = prepare_source(root, codec, extra)
            source_hash = digest(source)
        except (OSError, ValueError, subprocess.SubprocessError) as error:
            for variant in VARIANTS:
                receipts.append({"fixture": codec, "phase": "prerequisite-or-fixture",
                                 **failure_evidence(error, variant=variant)})
            continue
        for variant in VARIANTS:
            target = root / f"{codec}-{variant}.mov"
            receipt: dict = {}
            try:
                receipt = run(source, target, helper=helper, variant=variant)
                if receipt["native"].get("terminalEmptyMarkersSkipped", 0) <= 0:
                    raise ValueError("terminal empty-marker regression path was not exercised")
                original = digest(target)
                try:
                    run(source, target, helper=helper, variant=variant)
                except FileExistsError:
                    pass
                else:
                    raise ValueError("existing output was not rejected")
                if digest(target) != original:
                    raise ValueError("no-clobber control changed the existing output")
                if digest(source) != source_hash:
                    raise ValueError("source changed during no-clobber control")
                receipt = {**receipt, "passed": True, "noClobberValidated": True,
                           "terminalMarkerValidated": True}
            except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
                failure = getattr(error, "evidence", failure_evidence(
                    error, variant=variant, source_hash=source_hash,
                    candidate_hash=receipt.get("candidateSHA256"), native=receipt.get("native")))
                receipt = {**failure, "passed": False,
                           "published": failure.get("published", False) or receipt.get("published", False)}
            # Even failed trials must not change the source. A missing source is
            # a failure, not unavailable evidence that can be ignored.
            unchanged = source.is_file() and digest(source) == source_hash
            receipt["sourceUnchanged"] = unchanged
            if not unchanged:
                receipt["passed"] = False
                receipt["sourceIntegrityError"] = "source changed or disappeared"
            receipts.append({"fixture": codec, **receipt})

    malformed: dict = {"passed": False}
    if prerequisites["passed"]:
        invalid, target = root / "malformed.mov", root / "must-not-publish.mov"
        invalid.write_bytes(b"not a movie")
        try:
            run(invalid, target, helper=helper, variant="static")
        except ValueError as error:
            # A missing helper/tool must never satisfy a malformed-input control.
            evidence = getattr(error, "evidence", {})
            rejected = "ffprobe failed" in str(error)
            malformed = {"passed": rejected and not target.exists()
                         and invalid.read_bytes() == b"not a movie",
                         "rejection": evidence or {"message": str(error)}}
        except (OSError, KeyError, TypeError, subprocess.SubprocessError) as error:
            malformed = {"passed": False, "error": str(error)}
    else:
        malformed["error"] = "prerequisites failed; malformed control was not executed"
    expected = {(codec, variant) for codec, _ in CODECS for variant in VARIANTS}
    observed = {(receipt["fixture"], receipt["variant"]) for receipt in receipts}
    passed = (prerequisites["passed"] and len(receipts) == len(expected) and observed == expected
              and all(receipt["passed"] for receipt in receipts) and malformed["passed"])
    return {"schema": 1, "passed": passed, "requiredCases": len(expected),
            "prerequisites": prerequisites, "receipts": receipts,
            "malformedInput": malformed, "photosEditingValidated": False}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--helper", type=Path, required=True)
    args = parser.parse_args(argv)
    with tempfile.TemporaryDirectory(prefix="video-style-native-test-") as temporary:
        report = run_matrix(args.helper, Path(temporary))
    print(json.dumps(report, indent=2, allow_nan=False))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
