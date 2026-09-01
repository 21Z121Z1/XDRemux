#!/usr/bin/env python3
from pathlib import Path

path = Path("crates/xdremux-engine/src/lib.rs")
text = path.read_text()

family_preference = '''#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FamilyPreference {
    #[default]
    Auto,
    X6,
    X7,
}

'''
text = text.replace(family_preference, "", 1)

text = text.replace("    pub family: FamilyPreference,\n", "", 1)
text = text.replace("            family: FamilyPreference::Auto,\n", "", 1)

legacy_resolution = '''    let effective_family = match request.family {
        FamilyPreference::Auto => analysis.source_family,
        FamilyPreference::X6 => SourceFamily::X6,
        FamilyPreference::X7 => SourceFamily::X7,
    };
'''
text = text.replace(legacy_resolution, "    let effective_family = analysis.source_family;\n", 1)

text = text.replace(
    "        assert_eq!(request.family, FamilyPreference::Auto);\n",
    "",
    1,
)

if "FamilyPreference" in text:
    raise SystemExit("FamilyPreference remains in Rust engine after materialization")
if "request.family" in text:
    raise SystemExit("request.family remains in Rust engine after materialization")
if "pub family:" in text:
    raise SystemExit("family remains part of ConversionRequest")
if "let effective_family = analysis.source_family;" not in text:
    raise SystemExit("planner no longer derives source generation from analysis")

path.write_text(text)
