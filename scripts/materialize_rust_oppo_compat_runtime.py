#!/usr/bin/env python3
from pathlib import Path


path = Path("crates/xdremux-runtime/src/lib.rs")
text = path.read_text()

if "mod oppo_compat;" not in text:
    anchor = "mod live_photo;\nmod validation;\n"
    if anchor not in text:
        raise SystemExit("runtime module anchor not found")
    text = text.replace(anchor, "mod live_photo;\nmod oppo_compat;\nmod validation;\n", 1)

old_body = """        let body = standard_heif_body(
            self.source,
            self.prepared.extracted.manifest_info.extension_start,
        )?;
"""
new_body = """        let body = standard_heif_body(
            self.source,
            self.prepared.extracted.manifest_info.extension_start,
        )?;
        let patched_body = if plan.oppo_compatibility == OppoCompatibility::Off {
            None
        } else {
            Some(oppo_compat::patch_source_metadata(
                body,
                plan.oppo_compatibility,
            )?)
        };
        let assembly_body = patched_body.as_deref().unwrap_or(body);
"""
body_marker = "let assembly_body = patched_body.as_deref().unwrap_or(body);"
if body_marker not in text:
    if old_body not in text:
        raise SystemExit("runtime source-body anchor not found")
    text = text.replace(old_body, new_body, 1)

old_call = """        let mut output = assemble_iso_gain_map_heif(
            body,
"""
new_call = """        let mut output = assemble_iso_gain_map_heif(
            assembly_body,
"""
if new_call not in text:
    if old_call not in text:
        raise SystemExit("native assembly call anchor not found")
    text = text.replace(old_call, new_call, 1)

unsupported = """        if plan.oppo_compatibility != OppoCompatibility::Off {
            return Err(RuntimeError::new(
                "portable runtime",
                "OPPO-compatible output is not wired into the Rust runtime yet",
            ));
        }
"""
text = text.replace(unsupported, "", 1)

for marker in (
    "mod oppo_compat;",
    "oppo_compat::patch_source_metadata(",
    body_marker,
    "assemble_iso_gain_map_heif(\n            assembly_body,",
):
    if marker not in text:
        raise SystemExit(f"portable OPPO runtime marker missing: {marker}")
if text.count(body_marker) != 1:
    raise SystemExit("OPPO runtime source patch was materialized more than once")
if "OPPO-compatible output is not wired into the Rust runtime yet" in text:
    raise SystemExit("legacy blanket OPPO compatibility rejection remains")
if old_call in text:
    raise SystemExit("native assembly still consumes the unpatched source body")

path.write_text(text)
