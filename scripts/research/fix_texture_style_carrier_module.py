#!/usr/bin/env python3
from pathlib import Path

p = Path("crates/xdremux-heif/src/texture_styles.rs")
s = p.read_text()
old = "value.as_bytes().chunks_exact(2)"
if s.count(old) != 1:
    raise SystemExit(f"expected one clippy anchor, got {s.count(old)}")
s = s.replace(old, "value.as_bytes().as_chunks::<2>().0", 1)

old_import = "    ILOC, IPCO, IPMA, IPRP, IREF, MDAT, META, PITM,\n"
if s.count(old_import) != 1:
    raise SystemExit("PITM import anchor changed")
s = s.replace(old_import, "    ILOC, IPCO, IPMA, IPRP, IREF, MDAT, META,\n", 1)

constants = (
    'const HDLR: FourCC = FourCC::new(*b"hdlr");\n'
    'const DINF: FourCC = FourCC::new(*b"dinf");\n'
    'const GRPL: FourCC = FourCC::new(*b"grpl");\n'
)
if s.count(constants) != 1:
    raise SystemExit("meta-order constants anchor changed")
s = s.replace(constants, "", 1)

old_order = '''    let canonical_order = [HDLR, DINF, ILOC, IINF, PITM, IPRP, IDAT, IREF, GRPL];
    let mut ordered = Vec::with_capacity(graph.meta_children.len());
    for kind in canonical_order {
        ordered.extend(graph.meta_children.iter().filter(|header| header.kind == kind));
    }
    ordered.extend(
        graph
            .meta_children
            .iter()
            .filter(|header| !canonical_order.contains(&header.kind)),
    );
    let mut saw_iref = false;
    for header in ordered {
'''
new_order = '''    // The physical-device-positive H carrier preserves the source meta-child
    // order. Replacing boxes in place keeps this postprocessor from adding a
    // second, unverified container-layout transformation.
    let mut saw_iref = false;
    for header in &graph.meta_children {
'''
if s.count(old_order) != 1:
    raise SystemExit("meta-order anchor changed")
s = s.replace(old_order, new_order, 1)
p.write_text(s)
