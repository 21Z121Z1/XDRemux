# LibLivePhoto integration

[简体中文](liblivephoto-integration.md)

LibLivePhoto owns the portable Motion/Live Photo on-disk format contract.
XDRemux consumes its resource facade for source inspection, classification,
primary-resource selection, exact presentation time and pair validation.
Both dependency entries pin one full Git revision in Cargo manifests and lockfile.
The library builds outside the XDRemux workspace.

`xdremux-format` re-exports `liblivephoto-format` cursor, error, FourCC, JPEG,
raw EXIF and BMFF primitives. Product codec profile inspection remains local.
`xdremux-motion-photo` retains product geometry policy, payload file copy and
atomic pair publication. Its legacy names re-export an unstable migration feature.
New product format operations must use `MotionPhoto`, `Input`, resources and
`MediaTime`. Do not add an independent parser to the consumer crate.

The runtime preserves a known integer/timescale presentation time. It does not
round a known XMP timestamp to a video frame. When no source time exists, the
existing product fallback selects a position and marks the receipt as inferred.
Apple on-disk geometry accepts explicit caller values. The existing OPPO matrix
and stabilization policy remains in XDRemux; it is not a format-library default.

JPEG and Gain Map encoding remain product operations. The Live Photo receipt
reports exact presentation, encoded movie payload comparisons, metadata rewrites,
reencoding requirements, generated identity and resources omitted from the pair.
The CLI reports omissions. The original source remains intact. The product pair
conversion does not claim complete vendor-resource preservation or losslessness.
Use the library's source-copy or explicit sidecar policy for that requirement.

Verification uses the existing parser/payload conformance script and all 14 pinned
real-fixture CLI conversion/validation cases. Runtime tests also compare known
source time with written time and check the product conversion report. The
completion gate binds local checks to committed HEAD. GitHub runs the consumer
build and gates in a separate environment.

Original Gallery/Photos acceptance remains a separate requirement. Reader, writer,
fixture, CI and device status must not be combined into one support claim. See
[LibLivePhoto support](https://github.com/21Z121Z1/LibLivePhoto/blob/f314fe85c7c9acfc2e7e97a6e78f32f68e6aa6ad/FORMAT_SUPPORT.md)
and [vendor evidence](https://github.com/21Z121Z1/LibLivePhoto/blob/f314fe85c7c9acfc2e7e97a6e78f32f68e6aa6ad/evidence/VENDORS.md).
