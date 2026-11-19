#!/usr/bin/env python3
"""Exercise the OPPO Live Photo Styles conversion in the macOS Photos renderer."""

import argparse
import json
import os
import statistics
import subprocess
import tempfile
from fractions import Fraction
from math import sqrt
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "fixtures/motion-photo/oppo/coloros16-dualstream-ultrahdr-01.jpg"


def run(*args, env=None):
    process = subprocess.run(args, text=True, capture_output=True, env=env, timeout=180)
    if process.returncode:
        raise RuntimeError(f"{Path(args[0]).name} returned {process.returncode}\n{process.stderr}")
    return process.stdout


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def probe_movie(ffprobe, path, *options):
    return json.loads(run(
        ffprobe, "-v", "error", *options, "-show_packets", "-show_streams",
        "-show_entries",
        "stream=index,codec_name,codec_type,codec_tag_string,profile,color_transfer,color_primaries,time_base"
        ":packet=stream_index,pts,dts,duration,data_hash",
        "-show_data_hash", "sha256", "-of", "json", path,
    ))


def track_packets(movie, track):
    index = track["index"]
    return [packet for packet in movie["packets"] if packet["stream_index"] == index]


def pts(movie, track):
    scale = Fraction(track["time_base"])
    return sorted(packet["pts"] * scale for packet in track_packets(movie, track))


def compressed_packets(movie, track):
    scale = Fraction(track["time_base"])
    return [
        (p["pts"] * scale, p["dts"] * scale, p["duration"] * scale, p["data_hash"])
        for p in track_packets(movie, track)
    ]


def main_track(movie, kind):
    return next(track for track in movie["streams"] if track["codec_type"] == kind)


def hlg(track):
    return (track["codec_name"], track["profile"], track["color_transfer"], track["color_primaries"]) == (
        "hevc", "Main 10", "arib-std-b67", "bt2020",
    )


def rmse(first, second, plane):
    differences = []
    for a, b in zip(first, second, strict=True):
        if plane in a:
            differences.extend(x - y for x, y in zip(a[plane], b[plane], strict=True))
    return sqrt(statistics.mean(delta * delta for delta in differences))


def preview(probe, cover, movie, timeline, tone=0.0, color=0.0, enabled=True, texture="-"):
    result = json.loads(run(probe, cover, movie, str(tone), str(color), str(int(enabled)), texture))
    require(result["photographicStyles"] and result["textureStyles"], "Photos did not admit both Styles resources")
    require(result["compositor"] == "NUVideoPlaybackCompositorHDR", "Photos did not select the HDR compositor")
    frames = result["frames"]
    require([Fraction(*frame["pts"]) for frame in frames] == timeline, "preview PTS differ from the source video")
    require(all((f["transfer"], f["primaries"]) == ("ITU_R_2100_HLG", "ITU_R_2020") for f in frames),
            "a rendered preview frame lost HLG or BT.2020")
    return frames


def compile_probe(temp):
    probe = temp / "live-style-headless"
    run(
        "xcrun", "clang", "-O2", "-fobjc-arc", "-Wno-deprecated-declarations",
        Path(__file__).with_name("live_style_headless.m"), "-o", probe, "-lz",
        "-framework", "Foundation", "-framework", "AVFoundation", "-framework", "CoreGraphics",
        "-framework", "CoreMedia", "-framework", "CoreVideo", "-framework", "ImageIO",
    )
    return probe


def convert(options, temp, env, name, target, styled):
    extension = ".pvt" if target == "apple-pvt" else ".heic"
    output = temp / (name + extension)
    flags = ["--apple-styles"] if styled else []
    run(options.cli, "convert", "--input", FIXTURE, "--motion-target", target, "--output", output, *flags, env=env)
    cover = output / (name + ".heic") if extension == ".pvt" else output
    report = json.loads(run(options.cli, "validate", "--json", cover, env=env))
    require(report["valid"] and report["kind"] == "live-photo", f"{target} did not validate as a Live Photo")
    return cover, cover.with_suffix(".mov")


def check(options, temp):
    probe = compile_probe(temp)
    env = dict(os.environ, XDREMUX_APPLE_ADAPTER=str(options.adapter))
    _, unstyled_path = convert(options, temp, env, "base", "apple-pair", False)
    cover, movie_path = convert(options, temp, env, "styled", "apple-pvt", True)
    _, pair_path = convert(options, temp, env, "pair", "apple-pair", True)
    base = probe_movie(options.ffprobe, unstyled_path)
    movie = probe_movie(options.ffprobe, movie_path)
    pair = probe_movie(options.ffprobe, pair_path)
    video = main_track(movie, "video")
    timeline = pts(movie, video)
    require(hlg(video) and len(timeline) == 89, "converted primary video lost HLG or frames")
    for result in (movie, pair):
        for kind in ("video", "audio"):
            require(
                compressed_packets(result, main_track(result, kind)) == compressed_packets(base, main_track(base, kind)),
                f"{kind} bytes or timestamps changed when generating Styles",
            )
        auxiliary_tracks = [track for track in result["streams"] if pts(result, track) == timeline]
        require([track["codec_tag_string"] for track in auxiliary_tracks] == ["hvc1", "hvc1", "mebx", "mebx"],
                "Linear Thumbnail and Style tracks must cover every presentation timestamp")
        for reference in auxiliary_tracks[1:3]:
            require(len({p["data_hash"] for p in track_packets(result, reference)}) > 1,
                    "the frame-derived thumbnail or tone curve did not vary over the clip")

    def render(**settings):
        return preview(probe, cover, movie_path, timeline, **settings)

    tone_up, tone_down = render(tone=0.25), render(tone=-0.25)
    color_up, color_down = render(color=0.35), render(color=-0.35)
    tone_rmse = rmse(tone_up, tone_down, "y")
    color_rmse = rmse(color_up, color_down, "uv")
    require(tone_rmse > 2 and color_rmse > 2, "Tone or Color did not affect the rendered video")
    # The dark mountain region is nearly static in this fixture. A 15 Hz
    # alternation in a 30 Hz clip reappears as a large odd/even difference.
    shadows = [frame["shadow"] for frame in tone_up[:72]]
    parity_delta = abs(statistics.mean(shadows[::2]) - statistics.mean(shadows[1::2]))
    require(parity_delta < 1, "the fixture shows alternating shadow brightness with Styles enabled")

    off, neutral = render(enabled=False), render()
    glowy = render(enabled=False, texture="Glowy")
    preview(probe, pair_path.with_suffix(".heic"), pair_path, timeline, tone=0.25)
    export = temp / "edited.mov"
    run(probe, cover, movie_path, "0.25", "0.1", "1", "-", export)
    saved = probe_movie(options.ffprobe, export)
    saved_video = main_track(saved, "video")
    require(hlg(saved_video) and pts(saved, saved_video) == timeline, "Photos export lost HLG or video timestamps")
    # The edit list can trim visible audio. Compare its stored compressed
    # samples before applying that edit list.
    raw_audio = ("-ignore_editlist", "1", "-select_streams", "a:0")
    raw_original = probe_movie(options.ffprobe, movie_path, *raw_audio)
    raw_export = probe_movie(options.ffprobe, export, *raw_audio)
    require([p["data_hash"] for p in raw_export["packets"]] == [p["data_hash"] for p in raw_original["packets"]],
            "Photos export did not pass through the original AAC packets")
    print(json.dumps({
        "frames": len(timeline), "exportedAACPackets": len(raw_export["packets"]),
        "toneLumaRMSE": tone_rmse, "colorChromaRMSE": color_rmse,
        "shadowOddEvenDifference": parity_delta,
        "maxLumaCodeOff": statistics.median(f["max"] for f in off),
        "maxLumaCodeNeutral": statistics.median(f["max"] for f in neutral),
        "fractionAbove721Off": statistics.median(f["above721"] for f in off),
        "fractionAbove721Neutral": statistics.median(f["above721"] for f in neutral),
        "glowyOnlyLumaRMSE": rmse(glowy, off, "y"), "glowyOnlyChromaRMSE": rmse(glowy, off, "uv"),
    }, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", required=True, type=Path)
    parser.add_argument("--adapter", required=True, type=Path)
    parser.add_argument("--ffprobe", default="ffprobe")
    options = parser.parse_args()
    options.cli = options.cli.resolve()
    options.adapter = options.adapter.resolve()
    with tempfile.TemporaryDirectory(prefix="xdremux-live-style-check-") as directory:
        check(options, Path(directory))


if __name__ == "__main__":
    main()
