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
no-clobber pair publication. Its legacy names re-export an unstable migration feature.
New product format operations must use `MotionPhoto`, `Input`, resources and
`MediaTime`. Do not add an independent parser to the consumer crate.

The public pair publication helper keeps caller-owned staging files on success
and failure, including staging in a same-filesystem child directory. It never
replaces or unlinks final paths. A second-member failure exposes the partial
output path through `PairPublicationError::partial_output()`; that path is not
authority to delete an entry that another writer may have changed. The runtime
retains its private staging directory on this error. The reconciliation helper
preserves unknown partial/invalid pairs and all sidecars. See the
[publication and recovery contract](architecture.en.md#publication) for filesystem
requirements and the two-file transaction boundary.

The runtime validates the primary movie's video track and sample table even
when a presentation time is already known. It preserves a known integer/timescale presentation time. It does not
round a known XMP timestamp to a video frame. When no source time exists, the
existing product fallback selects a position and marks the receipt as inferred.
Apple on-disk geometry accepts explicit caller values. The existing OPPO matrix
and stabilization policy remains in XDRemux; it is not a format-library default.

Optional PVT is a local runtime container around an already validated Apple
pair. It writes one real directory with unchanged media and a bounded XML plist,
using exclusive directory publication. LibLivePhoto still owns media parsing,
pairing identity and exact presentation time; no second MOV/HEIF parser, second
identity or extra codec/remux operation is introduced. The default pair remains
unchanged. See [CLI packaging](cli.en.md#optional-pvt-packaging) and
[publication](architecture.en.md#publication) for no-clobber/recovery limits.
Package structure does not prove Photos import or dynamic wallpaper support.

JPEG and Gain Map encoding remain product operations. The Live Photo receipt
reports exact presentation, encoded movie payload comparisons, metadata rewrites,
reencoding requirements, generated identity and resources omitted from the pair.
Both `convert` and newly converted `batch` items report omissions. A resumed
batch item reports an unknown omission count because its checkpoint does not
store a conversion report. The original source remains intact. The product pair
conversion does not claim complete vendor-resource preservation or losslessness.
Use the library's source-copy or explicit sidecar policy for that requirement.

Verification uses the existing parser/payload conformance script and all 14 pinned
real-fixture CLI conversion/validation cases. Runtime tests also compare known
source time with written time and check the product conversion report. The
completion gate binds local checks to committed HEAD. GitHub runs the consumer
build and gates in a separate environment.

Original Gallery/Photos acceptance remains a separate requirement. Reader, writer,
fixture, CI and device status must not be combined into one support claim. See
[LibLivePhoto support](https://github.com/21Z121Z1/LibLivePhoto/blob/794ab552596db0f66095600d23cf7937a631b9ec/FORMAT_SUPPORT.md)
and [vendor evidence](https://github.com/21Z121Z1/LibLivePhoto/blob/794ab552596db0f66095600d23cf7937a631b9ec/evidence/VENDORS.md).

The public consumer contract uses an original ColorOS 16 dual-stream fixture.
The default Apple plan blocks its unmapped vendor resources. Explicit sidecar
policy retains every unmapped resource byte-for-byte. Re-reading the pair checks
exact presentation time and encoded media equality; a truncated movie is rejected.
This evidence does not assign a semantic role to stream 2 or prove Gallery support.

The consumer keeps the current product HDR application-colour-space flag,
Photographic Styles and Cinematic routes. Shared format parsing stays in
LibLivePhoto; HDR decisions and codec operations stay in XDRemux. The standalone
library converter and external workspace check exercise Android/Apple round trips
without this product stack. See the library compatibility document for the
integer-microsecond restriction on Apple-to-Android output.

The `xdremux-compat` feature remains a migration boundary for existing low-level
helpers. It is not a stable API. Remove each re-export after its consumer uses a
public resource or Apple primitive API and the associated conformance gate passes.
Keep payload copying, no-clobber publication and product geometry local. No removal
of the compatibility feature is scheduled before these callers are migrated.

The library reader now derives the supported primary movie MIME from `ftyp`. Android JPEG/HEIF directory output retains QuickTime MIME without changing movie bytes. Codec conformance and Gallery acceptance remain separate checks.
