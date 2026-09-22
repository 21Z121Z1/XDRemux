"""Public synthetic media gate for CarrierProbe; no Photos/device claim."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import tempfile

from research.video_styles.probe import VARIANTS, run


def command(*arguments: str) -> None:
    subprocess.run(list(arguments), check=True, timeout=120)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--helper", type=Path, required=True)
    args = parser.parse_args()
    receipts = []
    with tempfile.TemporaryDirectory(prefix="video-style-native-test-") as temporary:
        root = Path(temporary)
        for codec, extra in (
            ("h264", ["-c:v", "libx264", "-pix_fmt", "yuv420p", "-bf", "2"]),
            ("hevc10", ["-c:v", "libx265", "-pix_fmt", "yuv420p10le", "-tag:v", "hvc1",
                        "-x265-params", "pools=1:frame-threads=1:log-level=error",
                        "-color_primaries", "bt2020", "-color_trc", "smpte2084", "-colorspace", "bt2020nc"]),
        ):
            source = root / f"{codec}.mov"
            command("ffmpeg", "-v", "error", "-f", "lavfi", "-i", "testsrc2=size=96x64:rate=12:duration=1",
                    "-f", "lavfi", "-i", "sine=frequency=523:sample_rate=48000:duration=1",
                    *extra, "-c:a", "aac", "-metadata", "title=XDRemux synthetic probe", "-shortest", str(source))
            rotated = root / f"{codec}-rotated.mov"
            command("ffmpeg", "-v", "error", "-i", str(source), "-map", "0", "-c", "copy",
                    "-metadata:s:v:0", "rotate=90", str(rotated))
            for variant in VARIANTS:
                target = root / f"{codec}-{variant}.mov"
                receipt = run(rotated, target, helper=args.helper, variant=variant)
                receipts.append({"fixture": codec, **receipt})
                original = target.read_bytes()
                try:
                    run(rotated, target, helper=args.helper, variant=variant)
                except FileExistsError:
                    pass
                else:
                    raise AssertionError("existing output was not rejected")
                assert target.read_bytes() == original
        invalid = root / "malformed.mov"
        invalid.write_bytes(b"not a movie")
        target = root / "must-not-publish.mov"
        try:
            run(invalid, target, helper=args.helper, variant="static")
        except ValueError:
            pass
        else:
            raise AssertionError("malformed source was accepted")
        assert not target.exists() and invalid.read_bytes() == b"not a movie"
    print(json.dumps({"schema": 1, "receipts": receipts, "photosEditingValidated": False}, indent=2))


if __name__ == "__main__":
    main()
