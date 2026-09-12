#!/usr/bin/env python3
"""Fail-closed structural validation for the native Standard TextureStyle matrix."""

import argparse
import hashlib
import importlib.util
import json
import plistlib
from pathlib import Path


def load_v3():
    path = Path(__file__).with_name("patch_texture_style_heif_v3.py")
    spec = importlib.util.spec_from_file_location("texture_v3", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


V3 = load_v3()
STYLE_URI = b"tag:apple.com,2023:photo:metadata:styles\0"
TEXTURE_URI = "tag:apple.com,2026:photo:metadata:texture_styles"


def parse_uri_infe(raw):
    size = int.from_bytes(raw[:4], "big")
    if raw[4:8] != b"infe" or size != len(raw):
        raise ValueError("not an infe box")
    p = 8
    version = raw[p]
    flags = int.from_bytes(raw[p + 1:p + 4], "big")
    p += 4
    if version != 2:
        raise ValueError(f"expected infe v2, got {version}")
    item_id = int.from_bytes(raw[p:p + 2], "big")
    p += 2
    protection = int.from_bytes(raw[p:p + 2], "big")
    p += 2
    item_type = raw[p:p + 4]
    p += 4
    if item_type != b"uri ":
        raise ValueError(f"expected uri item, got {item_type!r}")
    end = raw.index(b"\0", p)
    name = raw[p:end].decode("utf-8")
    p = end + 1
    end = raw.index(b"\0", p)
    uri = raw[p:end].decode("utf-8")
    return {
        "version": version,
        "flags": flags,
        "itemID": item_id,
        "protection": protection,
        "itemType": "uri ",
        "name": name,
        "uri": uri,
    }


def inspect(path):
    data = Path(path).read_bytes()
    top = list(V3.boxes(data, 0, len(data)))
    meta = next(box for box in top if box["type"] == "meta")
    children = list(V3.boxes(data, meta["data_start"] + 4, meta["data_end"]))
    by_type = {box["type"]: box for box in children}
    _, infos = V3.parse_iinf(data, by_type["iinf"])
    _, refs = V3.parse_iref(data, by_type["iref"])
    iloc = V3.parse_iloc(data, by_type["iloc"])
    idat = by_type.get("idat")
    entries = {entry["id"]: entry for entry in iloc["entries"]}

    exif_id = next(info["id"] for info in infos if info["type"] == "Exif")
    exif = V3.item_payload(data, entries[exif_id], idat)
    note, _ = V3.extract_makernote(exif)
    _, note_entries = V3.parse_apple_makernote(note)
    tag84_raw = next(raw for tag, _, raw in note_entries if tag == 84)
    tag84 = plistlib.loads(tag84_raw)

    style = next(
        info for info in infos
        if info["type"] == "uri " and STYLE_URI in info["raw"]
    )
    style_targets = next(
        ref["to"] for ref in refs
        if ref["type"] == "cdsc" and ref["from"] == style["id"]
    )
    texture_infos = [
        info for info in infos
        if info["type"] == "uri "
        and (TEXTURE_URI + "\0").encode() in info["raw"]
    ]
    texture = None
    if texture_infos:
        if len(texture_infos) != 1:
            raise AssertionError(f"{path}: expected one texture item")
        info = texture_infos[0]
        carrier = parse_uri_infe(info["raw"])
        entry = entries[info["id"]]
        targets = next(
            ref["to"] for ref in refs
            if ref["type"] == "cdsc" and ref["from"] == info["id"]
        )
        payload = V3.item_payload(data, entry, idat)
        texture = {
            "carrier": carrier,
            "constructionMethod": entry["method"],
            "targets": targets,
            "payloadSize": len(payload),
            "payloadSHA256": hashlib.sha256(payload).hexdigest(),
            "object": plistlib.loads(payload),
        }
    return {
        "file": Path(path).name,
        "size": len(data),
        "sha256": hashlib.sha256(data).hexdigest(),
        "tag84Size": len(tag84_raw),
        "tag84SHA256": hashlib.sha256(tag84_raw).hexdigest(),
        "tag84": tag84,
        "styleTargets": style_targets,
        "texture": texture,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("contract")
    parser.add_argument("candidate_dir")
    parser.add_argument("report")
    args = parser.parse_args()

    contract = json.loads(Path(args.contract).read_text())
    candidate_dir = Path(args.candidate_dir)
    maker_variants = contract["maker84Variants"]
    texture_variants = contract["textureInfoVariants"]
    results = []

    for combination in contract["combinations"]:
        result = inspect(candidate_dir / f"{combination['name']}.heic")
        expected_maker = maker_variants[combination["maker84"]]
        if result["tag84"] != expected_maker:
            raise AssertionError(f"{combination['name']}: tag84 mismatch")
        for key, expected in {
            "8": 1, "9": 1.0, "10": 0.0, "11": False, "12": 1
        }.items():
            if result["tag84"].get(key) != expected:
                raise AssertionError(f"{combination['name']}: native slot {key} mismatch")

        texture_name = combination.get("textureInfo")
        if texture_name is None:
            if result["texture"] is not None:
                raise AssertionError(f"{combination['name']}: unexpected texture item")
        else:
            texture = result["texture"]
            if texture is None:
                raise AssertionError(f"{combination['name']}: missing texture item")
            if texture["carrier"]["name"] != contract["textureItemName"]:
                raise AssertionError(f"{combination['name']}: item name mismatch")
            if texture["carrier"]["uri"] != contract["textureURI"]:
                raise AssertionError(f"{combination['name']}: URI mismatch")
            if texture["constructionMethod"] != 0:
                raise AssertionError(f"{combination['name']}: expected construction method 0")
            if texture["targets"] != result["styleTargets"]:
                raise AssertionError(f"{combination['name']}: cdsc targets diverge from 2023 styles")
            expected_texture = texture_variants[texture_name]
            if texture["object"] != expected_texture:
                raise AssertionError(f"{combination['name']}: texture plist mismatch")
            if "Version" in texture["object"]:
                raise AssertionError(f"{combination['name']}: invalid literal Version key")
            if "TextureStylePostProcessedPeopleData" in texture["object"]:
                raise AssertionError(f"{combination['name']}: no-person variant must omit people data")

        results.append(result)

    report = {
        "schema": "xdremux-texture-style-native-standard-validation-v1",
        "contract": contract["schema"],
        "candidateCount": len(results),
        "allPassed": True,
        "results": results,
    }
    Path(args.report).write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()

