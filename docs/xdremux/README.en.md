# XDRemux Technical Implementation Index

This directory contains public, relatively stable documentation for HDR, HEIF, and ISO container behavior. Start with the [project README](../../README.en.md) or [CLI reference](../cli.en.md) for normal use.

## Current public documents

- [ISO conformance audit](iso-conformance-audit-20260511.md): ISO 21496-1, HEIF items and references, tmap behavior, and Apple ImageIO compatibility.
- [Validation guide](../validation/README.md): boundaries between structural, renderer, regression, and device evidence.
- [Apple features](../apple-features.en.md): user capabilities, input requirements, and acceptance scope for Styles and Portrait.
- [Development guide](../development.en.md): module boundaries, helpers, Swift Package APIs, and build workflows.

## Documentation boundary

Public technical documentation describes current implementation constraints and repeatable validation. Dated single-sample experiments, firmware fields, reverse-engineering work, open hypotheses, and temporary UI acceptance logs belong under `docs/research/` or `docs/experiments/`; they are not current product commitments.

When a research result becomes stable product behavior, encode it in code and regression tests first, then summarize the user-facing or developer-facing conclusion in the appropriate document. Do not append raw research logs to the README.
