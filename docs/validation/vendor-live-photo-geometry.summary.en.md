# Vendor Live Photo Geometry Evidence Summary

English | [简体中文](vendor-live-photo-geometry.summary.md)

This is a current-language summary of the evidence record [vendor-live-photo-geometry.md](vendor-live-photo-geometry.md).

The original file records the evidence boundary for vendor-specific Live Photo geometry work. Keep its detailed observations as evidence.

## Current stable conclusion

The production Live Photo path preserves the paired motion-video bitstream instead of rendering a geometry correction into video pixels.

For inputs covered by the vendor geometry policy, the converter can use source metadata and analysis-only auxiliary resources to select Live Photo transform metadata.

A geometry-analysis failure does not automatically require the whole Live Photo conversion to fail. The production path can use its supported metadata fallback when the implementation allows it.

## Evidence boundary

Private per-frame Apple payloads and other unproven private metadata must not be written only because their binary shape looks plausible.

Use current code and tests to determine which geometry metadata is writable now. Use the original record to understand why some private payloads were intentionally excluded.
