# Vendor Live Photo Geometry: ColorOS 16 and Samsung

This document records the evidence boundary for XDRemux's metadata-only Live Photo geometry work.
The production goal is to preserve the source motion-video bitstream and improve Apple Live Photo
geometry metadata without baking blur, scaling, or other raster changes into the video.

## Scope

The new geometry policy is intentionally limited to two fixture-backed vendor families:

- **ColorOS 16 / OPPO dual-stream Motion Photo**: current strict fixtures contain two concatenated
  ISO-BMFF streams. Stream 1 is the paired motion video. Stream 2 is exposed as an
  `auxiliaryGeometry` resource for analysis only and is never copied into the Apple paired MOV.
- **Samsung Motion Photo**: current JPEG and HEIF strict fixtures contain one semantic motion-video
  resource. BMFF-looking bytes in Samsung JPEG static resources and trailing `sefd` data in Samsung
  HEIF resources are vendor data, not an inferred second video stream. They must not be promoted to
  `auxiliaryGeometry` without new fixture evidence.

Other Android Motion Photo inputs remain outside this geometry policy. The generic
`MotionPhotoVideoStreamLayout` API allows another vendor to add an evidence-backed auxiliary stream
later without changing downstream geometry consumers.

## Implemented contracts

### Compressed video stays passthrough

`AppleLivePhotoVideoWriter` reads and writes compressed video/audio with nil output settings. The
existing `AppleLivePhotoValidator` hashes exact compressed sample storage ranges before and after
remuxing. A conversion that changes encoded video or audio payload bytes fails validation.

No blur border, canvas expansion, EIS warp, or Vision transform is rendered into video pixels by
this feature.

### ColorOS 16 EIS FOV normalization

The parser now preserves `photoEisCropFactor` separately from the older `eisCropFactor` field.
This matters because the two observed conventions are not interchangeable:

- a crop factor greater than one, such as `1.11`, normalizes to an Apple-facing FOV scale of
  `1 / 1.11 = 0.9009009009...`;
- a direct scale below or equal to one, such as `0.90`, remains `0.90`.

`photoEisCropFactor` has priority when both fields are present. Invalid, non-finite, or non-positive
values are rejected. The existing `0.90` ColorOS 16 compatibility fallback remains only for samples
that have no usable factor.

### Track 5 reference dimensions use the still raster

Core Media defines the Live Photo still-image-transform reference dimensions as the dimensions of
the Live Photo still image. Vendor-scoped conversion therefore passes the actual extracted still
raster dimensions to `AppleLivePhotoVideoWriter` instead of treating the OPPO video raster as the
production reference dimensions. The old OPPO video-size path remains only as a direct-call
compatibility fallback when no still dimensions were supplied.

The actual Track 5 transform source is otherwise unchanged by this branch: XDRemux only writes an
OPPO transform that the existing alignment implementation can derive from vendor metadata.

### Vision registration is available as an analysis primitive

`VendorLivePhotoVisionHomographyEstimator` exposes the same public Vision homographic registration
primitive needed by ColorOS Stream 1/Stream 2/still analysis and by future auxiliary-stream vendors.
It returns a finite, normalized row-major `floatingToReference` matrix and deliberately retains
Vision's mapping direction in the API name. A synthetic high-texture identity-registration test
checks both the Vision call and the SIMD-to-row-major conversion.

This estimator is not connected to Apple metadata writing. A caller may use it for diagnostics or
for offline trajectory research without silently crossing the still-unverified Apple coordinate
boundary.

## Deliberately not written yet

The following data is useful research evidence but is **not production-writable on this branch**:

1. `mdta/com.apple.quicktime.live-photo-info` Track 4 generated from Vision. The recovered Apple v3
   payload contains a per-frame stabilized trajectory homography, but the Vision-to-Apple coordinate
   convention, mapping direction, reference dimensions, orientation handling, and normalization have
   not been closed across native Apple samples.
2. `AVAppleMakerNote_StillImageProcessingHomography`. Firmware analysis connects this class of still
   processing metadata to Photos vitality transforms, but its third-party writable MakerNote layout
   and coordinate contract are private and not yet proven.
3. Vision homographies as a replacement for Track 5. Existing Codex experiments successfully
   estimate ColorOS Stream 1/Stream 2/still relationships, but those experiments explicitly treat the
   matrices as diagnostics until the Apple metadata convention is independently validated.
4. Synthetic motion-blur vectors or blur radii. Photos owns its horizontal-scroll/vitality blur
   state; XDRemux does not inflate motion metadata to force a visual effect.

This is intentional: writing a structurally plausible private payload is not evidence that Photos
interprets it correctly.

## Fixture gates

The strict real-fixture lane now additionally asserts:

- both ColorOS 16 fixtures resolve one primary stream plus one `auxiliaryGeometry` stream;
- current Samsung JPEG/HEIF fixtures enter the Samsung geometry scope but expose zero auxiliary
  video streams;
- representative ColorOS 15 fixtures do not enter the new vendor geometry policy;
- vendor-scoped plans can read the actual still raster dimensions.

Synthetic tests additionally cover:

- semantic primary/auxiliary byte ranges;
- generic Android inputs never invent an auxiliary stream;
- `1.11 -> 1/1.11` ColorOS normalization;
- direct sub-unity legacy factors;
- `photoEisCropFactor` precedence;
- still-image dimensions taking priority over video dimensions for Track 5 metadata;
- Track 5 reference dimensions surviving an AVFoundation write/read round trip;
- Vision identity registration producing a normalized near-identity homography.

## Device validation still required

The intended Photos.app experiment remains an A/B matrix rather than an offline acceptance claim:

- baseline current conversion;
- corrected public Track 5 geometry;
- future Track 4 candidate only after its coordinate mapping is closed;
- future still-processing-homography candidate only after its writable MakerNote contract is closed;
- combined candidate.

Horizontal swipe/vitality blur, geometry movement, and cover settling are device-dependent Photos
behavior. Offline parsing, PhotoKit construction, and compressed-payload equality cannot prove those
visual effects. Device evidence must remain a separate acceptance gate.
