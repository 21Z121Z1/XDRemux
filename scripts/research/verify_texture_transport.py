#!/usr/bin/env python3
"""Reproduce the offline transport contract. No native activation is asserted.

All inputs are synthetic container fixtures with opaque non-image payloads.
Use native_images.swift in the isolated PSR branch for actual Apple codecs.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess

BASE = "f8fcf2067c506e87665e9ea113324db1a57d4cf7"
TEXTURE_URI = b"tag:apple.com,2026:photo:metadata:texture_styles"


def box(kind, data):
    return struct.pack(">I4s", len(data) + 8, kind) + data


def full(kind, version, data, flags=0):
    return box(kind, bytes([version]) + flags.to_bytes(3, "big") + data)


def fixture(texture, escape=False):
    """Independent Python constructor; not a HEVC encoder or native positive."""
    kinds = [b"hvc1", b"tmap", b"hvc1", b"mime", b"uri ", b"Exif"]
    if texture:
        kinds.append(b"uri ")
    infos = []
    for i, kind in enumerate(kinds, 1):
        tail = b"metadata\0" if kind == b"uri " else b"\0"
        if i == 5:
            tail += b"tag:apple.com,2023:photo:metadata:styles\0"
        if i == 7:
            tail += TEXTURE_URI + b"\0"
        infos.append(full(b"infe", 2, struct.pack(">HH4s", i, 0, kind) + tail, 1 if kind == b"uri " else 0))
    payloads = [b"compressed-primary-sentinel", b"gain-map-sentinel", b"own-skin-matte",
                b"<rdf>own-descriptor</rdf>", b"bplist00opaque-styles",
                b"\0\0\0\0II\x2a\0\x08\0\0\0\0\0\0\0\0\0", b"bplist00opaque-texture"][:len(kinds)]
    locations, offset = [], 0
    for i, payload in enumerate(payloads, 1):
        length = len(payload) + (10 if escape and i == len(kinds) else 0)
        locations.append(struct.pack(">HHHHII", i, 1, 0, 1, offset, length))
        offset += len(payload)
    properties = full(b"auxC", 0, b"urn:com:apple:photo:2019:aux:semanticskinmatte\0")
    properties += box(b"irot", bytes([3])) + box(b"colr", b"profkeep-ICC-exact") + box(b"imir", bytes([1]))
    associations = full(b"ipma", 0, struct.pack(">IHB BHB3B", 2, 3, 1, 1, 1, 3, 2, 3, 4))
    references = [(5, [1, 2]), (4, [3])]
    if texture:
        references.append((7, [1]))
    refs = b"".join(box(b"cdsc", struct.pack(">HH", source, len(targets)) + b"".join(struct.pack(">H", i) for i in targets)) for source, targets in references)
    refs += box(b"auxl", struct.pack(">HHH", 3, 1, 1))
    meta = full(b"pitm", 0, struct.pack(">H", 1))
    meta += full(b"iinf", 0, struct.pack(">H", len(infos)) + b"".join(infos))
    meta += full(b"iloc", 1, b"\x44\0" + struct.pack(">H", len(locations)) + b"".join(locations))
    meta += box(b"iprp", box(b"ipco", properties) + associations)
    meta += full(b"iref", 0, refs) + box(b"idat", b"".join(payloads))
    return box(b"ftyp", b"heic\0\0\0\0mif1heic") + full(b"meta", 0, meta) + box(b"mdat", b"compressed-media-tail-do-not-touch") + box(b"uuid", b"unrelated-metadata-exact") + b"TAIL"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--out", type=Path, required=True, help="new evidence directory")
    p.add_argument("--baseline-repo", type=Path, help="optional clean checkout of the recorded original base")
    a = p.parse_args()
    root = Path(__file__).resolve().parents[2]
    out = a.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    commands = []

    def run(command, label, cwd=root, env=None, expected=0):
        r = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True)
        (out / (label + ".log")).write_text(r.stdout + r.stderr)
        commands.append({"command": list(map(str, command)), "cwd": str(cwd), "exit_code": r.returncode, "expected_exit_code": expected, "log": label + ".log"})
        if r.returncode != expected:
            raise RuntimeError(f"{label}: exit {r.returncode}, expected {expected}; see evidence log")
        return r

    cargo = os.environ.get("CARGO", "cargo")
    run([cargo, "test", "--locked", "-p", "xdremux-heif"], "rust-tests")
    run([cargo, "clippy", "--locked", "-p", "xdremux-heif", "--all-targets", "--", "-D", "warnings"], "clippy")
    run([cargo, "build", "--locked", "-p", "xdremux-heif", "--examples"], "build-examples")
    target = Path(os.environ.get("CARGO_TARGET_DIR", root / "target"))
    if not target.is_absolute():
        target = root / target
    client = target / "debug/examples/texture_transport"
    source, absent, corrupt = out / "opaque-control.heic", out / "absent-control.heic", out / "escape-control.heic"
    source.write_bytes(fixture(True)); absent.write_bytes(fixture(False)); corrupt.write_bytes(fixture(True, True))
    output = out / "preserved.heic"
    run([str(client), str(source), str(output)], "opaque-transport")
    assert output.read_bytes() == source.read_bytes()
    rejected = out / "rejected.heic"
    run([str(client), str(absent), str(rejected)], "missing-producer", expected=2)
    assert not rejected.exists()
    run([str(client), str(corrupt), str(rejected)], "out-of-bounds", expected=2)
    assert not rejected.exists()
    run([str(client), str(source), str(output)], "existing-output-protection", expected=2)
    assert output.read_bytes() == source.read_bytes()
    baseline = {"status": "not-requested"}
    if a.baseline_repo:
        original = a.baseline_repo.resolve()
        head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=original, text=True).strip()
        assert head == BASE, "baseline must be the exact audited original commit"
        assert not subprocess.check_output(["git", "status", "--porcelain"], cwd=original, text=True).strip(), "baseline checkout must be clean"
        probe = original / "crates/xdremux-heif/tests/xdremux_texture_original_regression.rs"
        code = '''#[test]
fn refuse_unbacked_texture_generation() {
    let input = std::fs::read(std::env::var_os("XDREMUX_TEXTURE_BASELINE_INPUT").unwrap()).unwrap();
    let result = xdremux_heif::augment_texture_style_heif(&input);
    if let Ok(ref bytes) = result {
        std::fs::write(std::env::var_os("XDREMUX_TEXTURE_BASELINE_OUTPUT").unwrap(), bytes).unwrap();
    }
    assert!(result.is_err(), "OLD_PROBE_FABRICATED_UNBACKED_TEXTURE_CARRIER");
}
'''
        with probe.open("x") as f:
            f.write(code)
        baseline_output = out / "original-probe-generated.heic"
        env = dict(os.environ, XDREMUX_TEXTURE_BASELINE_INPUT=str(absent), XDREMUX_TEXTURE_BASELINE_OUTPUT=str(baseline_output))
        env["CARGO_TARGET_DIR"] = str(out / "baseline-target")
        try:
            r = run([cargo, "test", "--locked", "-p", "xdremux-heif", "--test", "xdremux_texture_original_regression"], "original-regression-fails", cwd=original, env=env, expected=101)
            assert "OLD_PROBE_FABRICATED_UNBACKED_TEXTURE_CARRIER" in r.stdout + r.stderr
            data = baseline_output.read_bytes()
            assert TEXTURE_URI + b"\0" in data and b"iPhone19,2" in data and b"Apple iOS" in data
            baseline = {"status": "original-defect-reproduced", "head_sha": head, "test_source_sha256": sha(code.encode()), "input_sha256": sha(absent.read_bytes()), "output_sha256": sha(data), "output_bytes": len(data), "invented_texture_uri": True, "invented_model": "iPhone19,2", "invented_apple_makernote": True}
        finally:
            probe.unlink()  # Only this invocation's exclusively-created file.
    summary = {"schema": "xdremux-texture-transport-verification-v1",
               "head_sha": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
               "tracked_worktree_dirty": bool(subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=no"], cwd=root, text=True).strip()),
               "source_sha256": {str(p.relative_to(root)): sha(p.read_bytes()) for p in [root / "crates/xdremux-heif/src/texture_styles.rs", Path(__file__).resolve()]},
               "scope": "offline opaque transport and rejection only; fixtures are not decodable photographs",
               "native_consumer_executed": False, "photos_texture_activation": "unverified",
               "media_gain_color_orientation_unknown_bytes_preserved": True,
               "input_sha256": sha(source.read_bytes()), "output_sha256": sha(output.read_bytes()),
               "baseline": baseline, "commands": commands}
    (out / "verification.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps({"status": "offline-transport-passed", "head_sha": summary["head_sha"], "native_activation": "unverified", "baseline": baseline["status"]}))


if __name__ == "__main__":
    main()
