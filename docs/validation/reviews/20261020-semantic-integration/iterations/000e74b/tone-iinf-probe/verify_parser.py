#!/usr/bin/env python3
"""Read-only verification of the exact parser against real Rust writer output."""
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import types

sys.path.insert(0, "/workspace/XDRemux-integration")
from scripts.apple import check_style_tone_response as tone

root = Path(__file__).resolve().parent
source = Path("/workspace/xdremux-audit/tone-pinned-heif-dump.py")
before_source = source.read_bytes()
spec = importlib.util.spec_from_file_location("fixed_original", source)
original = importlib.util.module_from_spec(spec)
spec.loader.exec_module(original)
patched_source, receipt = tone.patched_iinf_parser(source)
patched = types.ModuleType("derived_parser")
exec(compile(patched_source, str(source), "exec"), patched.__dict__)
report = {"original_parser": str(source), "compatibility": receipt, "cases": {}}
for name in ("before", "after"):
    path = root / "outputs" / (name + ".heic")
    data = path.read_bytes()
    rows = {"path": str(path), "sha256": hashlib.sha256(data).hexdigest()}
    for label, module in (("fixed_original", original), ("production_adapter", patched)):
        graph = module.parse_heif(data)
        matches = [item for item in graph["items"].values()
                   if item.get("uri_type") == "tag:apple.com,2023:photo:metadata:styles"
                   and any(r["from_item_id"] == item["item_id"]
                           and graph["primary_item_id"] in r["to_item_ids"]
                           and r["type"] == "cdsc" for r in graph["references"])]
        meta = next(box for box in module.iter_boxes(data) if box["type"] == "meta")
        iinf = next(box for box in module.iter_boxes(data, meta["payload_offset"] + 4, meta["offset"] + meta["size"])
                    if box["type"] == "iinf")
        rows[label] = {
            "iinf_version": data[iinf["payload_offset"]],
            "infe_items": sum("item_type" in item for item in graph["items"].values()),
            "linked_styles": len(matches),
            "styles_payload_sha256": hashlib.sha256(module._payload_from_item_extents(data, matches[0])).hexdigest()
            if matches else None,
            "original_cdsc_present": any(r["type"] == "cdsc" and r["from_item_id"] == 2
                                         and r["to_item_ids"] == [1] for r in graph["references"]),
        }
    assert path.read_bytes() == data
    report["cases"][name] = rows
assert report["cases"]["before"]["fixed_original"]["linked_styles"] == 1
assert report["cases"]["after"]["fixed_original"]["linked_styles"] == 0
assert report["cases"]["after"]["fixed_original"]["original_cdsc_present"]
assert report["cases"]["after"]["production_adapter"]["infe_items"] == 3
assert report["cases"]["after"]["production_adapter"]["linked_styles"] == 1
assert (report["cases"]["before"]["fixed_original"]["styles_payload_sha256"]
        == report["cases"]["after"]["production_adapter"]["styles_payload_sha256"])
assert source.read_bytes() == before_source
report["original_source_unchanged"] = True
report["original_source_sha256_after"] = hashlib.sha256(source.read_bytes()).hexdigest()
(root / "complete-parser-proof.json").write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report, indent=2))
