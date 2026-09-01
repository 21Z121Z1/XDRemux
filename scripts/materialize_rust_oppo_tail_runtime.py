#!/usr/bin/env python3
from pathlib import Path


path = Path("crates/xdremux-runtime/src/lib.rs")
text = path.read_text()

if "mod oppo_tail;" not in text:
    anchor = "mod oppo_compat;\nmod validation;\n" if "mod oppo_compat;" in text else "mod live_photo;\nmod validation;\n"
    if anchor not in text:
        raise SystemExit("runtime module anchor not found")
    replacement = anchor.replace("mod validation;", "mod oppo_tail;\nmod validation;")
    text = text.replace(anchor, replacement, 1)

old_append = """        if plan.oppo_camera_tail == OppoCameraTail::PreserveWithoutPrivateHdr {
            let tail = pack_filtered_oppo_camera_tail(
                self.source,
                &self.prepared.extracted.manifest_info,
                self.prepared.extracted.data_base,
                |entry| !is_oppo_private_hdr_tail_entry(&entry.name),
            )
            .map_err(|error| RuntimeError::external("OPPO camera tail", error))?;
            output.extend_from_slice(&tail);
        }
"""
new_append = """        let tail = oppo_tail::build_tail(
            self.source,
            &self.prepared.extracted,
            plan.oppo_camera_tail,
        )?;
        output.extend_from_slice(&tail);
"""
if new_append not in text:
    if old_append not in text:
        raise SystemExit("legacy OPPO tail append anchor not found")
    text = text.replace(old_append, new_append, 1)

old_guard = """        if !matches!(
            plan.oppo_camera_tail,
            OppoCameraTail::Off | OppoCameraTail::PreserveWithoutPrivateHdr
        ) {
            return Err(RuntimeError::new(
                "portable runtime",
                "requested OPPO camera-tail policy is not wired into the Rust runtime yet",
            ));
        }
"""
text = text.replace(old_guard, "", 1)

for marker in (
    "mod oppo_tail;",
    "let tail = oppo_tail::build_tail(",
    "plan.oppo_camera_tail,",
):
    if marker not in text:
        raise SystemExit(f"OPPO tail runtime marker missing: {marker}")
if "requested OPPO camera-tail policy is not wired into the Rust runtime yet" in text:
    raise SystemExit("legacy OPPO camera-tail rejection remains")
if "if plan.oppo_camera_tail == OppoCameraTail::PreserveWithoutPrivateHdr" in text:
    raise SystemExit("legacy one-mode camera-tail append remains")

# Runtime policy moved behind dedicated modules; clean imports that were only
# needed by the old inline implementation.
text = text.replace(
    "    extract, is_oppo_private_hdr_tail_entry, pack_filtered_oppo_camera_tail, ExtractedLhdr,\n",
    "    extract, ExtractedLhdr,\n",
    1,
)
text = text.replace(
    "    InputProcessingBranch, OperationCapability, OppoCameraTail, OppoCompatibility, RasterDecoder,\n",
    "    InputProcessingBranch, OperationCapability, OppoCompatibility, RasterDecoder,\n",
    1,
)

if "is_oppo_private_hdr_tail_entry, pack_filtered_oppo_camera_tail" in text:
    raise SystemExit("legacy container tail imports remain")
if "OperationCapability, OppoCameraTail, OppoCompatibility" in text:
    raise SystemExit("legacy engine camera-tail import remains")

path.write_text(text)
