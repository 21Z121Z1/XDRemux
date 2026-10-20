#!/usr/bin/env python3
"""Recover public native JSON from actual Actions logs and verify its manifest.

This verifies logged text payloads. It cannot certify downloaded ZIP bytes.
"""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re


def extract(log, output):
    output.mkdir(parents=True, exist_ok=False)
    active = None
    content = []
    payloads = {}
    for line in log.splitlines():
        line = re.sub(r"^\ufeff?\d{4}-\d\d-\d\dT\S+Z ?", "", line)
        if line.startswith("TEXTURE_JSON_BEGIN "):
            if active is not None:
                raise ValueError("nested native JSON block")
            active = line.removeprefix("TEXTURE_JSON_BEGIN ")
            path = PurePosixPath(active)
            if path.is_absolute() or ".." in path.parts or path.suffix != ".json":
                raise ValueError("unsafe native JSON block path")
            content = []
        elif line.startswith("TEXTURE_JSON_END "):
            if active is None or line.removeprefix("TEXTURE_JSON_END ") != active:
                raise ValueError("unmatched native JSON block")
            raw = ("\n".join(content) + "\n").encode()
            json.loads(raw)
            payloads[active] = raw  # Retain the last complete native checkpoint.
            active = None
        elif active is not None:
            content.append(line)
    if active is not None:
        raise ValueError("truncated native JSON block")
    if "manifest.json" not in payloads:
        raise ValueError("native log does not contain the complete manifest")
    manifest = json.loads(payloads["manifest.json"])
    checks = []
    for name, raw in payloads.items():
        if name == "manifest.json":
            continue
        matches = [r for r in manifest["files"] if r["path"] == name or r["path"].endswith("/"+name)]
        if len(matches) != 1:
            raise ValueError(f"native JSON block has ambiguous or absent manifest row: {name}")
        expected = matches[0]
        actual = hashlib.sha256(raw).hexdigest()
        if len(raw) != expected["size"] or actual != expected["sha256"]:
            raise ValueError(f"native JSON hash does not match runner manifest: {name}")
        checks.append({"path": expected["path"], "size": len(raw), "sha256": actual})
    for name, raw in payloads.items():
        p = output / name
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(raw)
    digests = re.findall(r"SHA256 digest of uploaded artifact zip is ([0-9a-f]{64})", log)
    summary = {"schema": "xdremux-texture-native-log-readback-v1",
               "head_sha": manifest.get("head_sha"), "xdremux_head_sha": manifest.get("xdremux_head_sha"),
               "run_id": manifest.get("run_id"), "log_sha256": hashlib.sha256(log.encode()).hexdigest(),
               "verified_native_json_payloads": checks, "verified_payload_count": len(checks),
               "archive_bytes_verified": False,
               "github_reported_zip_sha256": digests[-1] if digests else None,
               "scope": "hash-checked actual native JSON emitted in job logs; not ZIP byte readback or Photos activation"}
    (output/"log-readback.json").write_text(json.dumps(summary, indent=2)+"\n")
    return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("log", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    # Preserve raw CR bytes when binding the whole source job log.
    summary = extract(args.log.read_bytes().decode("utf-8"), args.out)
    print(json.dumps({"head_sha": summary["head_sha"], "run_id": summary["run_id"],
                      "verified_native_json_payloads": summary["verified_payload_count"],
                      "zip_bytes_verified": False}))


if __name__ == "__main__":
    main()
