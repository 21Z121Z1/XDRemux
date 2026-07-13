# Research

This directory is for research notes that are not yet production design decisions.

Use it for investigations such as:

- OPPO/OnePlus/realme ProXDR container behavior.
- ISO 21496-1 gain-map interpretation.
- Apple ImageIO and Photos recognition behavior.
- Android framework or Gallery rendering observations.
- Compatibility matrices and reverse-engineering notes that should remain separate from user-facing README material.

When a research conclusion becomes a stable engineering decision, summarize it in `docs/design/` or in the relevant production README and keep the detailed evidence here.

Current portrait-depth reverse engineering:

- `oppo-x9-ultra-portrait-depth-consumption-20260713.md` traces the X9 Ultra
  Gallery -> IPU/APS -> native Zstd package decode and nonlinear bokeh render
  path.
- `oppo-apple-portrait-information-coverage-20260713.md` audits the 80-original
  batch, maps OPPO resources to Apple auxiliaries, and records the remaining
  config/semantic/rendering gaps.
