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

### Track 5 reference dimensions describe the transform analysis space

The macOS 26.5 Core Media header defines
`kCMMetadataIdentifier_QuickTimeMetadataLivePhotoStillImageTransformReferenceDimensions` as the
dimensions of the image used to generate the transform. It does not require the full still raster.
For the validated ColorOS 16 path, Vision analyzes Stream 1, Stream 2, and the still in a common
`1920×1440` coordinate space declared by OPPO metadata, so Track 5 records `1920×1440`. The full
still raster is still measured for validation, but is not substituted for this transform-specific
reference space.

Direct writer callers may continue to supply an explicit reference size. The production converter
keeps matrix and dimensions together in `AppleLivePhotoStillTransform` so a pixel translation can
never silently change coordinate systems.

### Vision cover alignment is connected to Track 5 behind a trajectory gate

`VendorLivePhotoVisionHomographyEstimator` exposes the same public Vision homographic registration
primitive needed by ColorOS Stream 1/Stream 2/still analysis and by future auxiliary-stream vendors.
It returns a finite, normalized row-major `floatingToReference` matrix and deliberately retains
Vision's mapping direction in the API name. A synthetic high-texture identity-registration test
checks both the Vision call and the SIMD-to-row-major conversion.

For a fixture-backed ColorOS 16 dual-stream input, the production converter now:

1. decodes analysis frames only; the paired MOV still copies compressed Stream 1 samples exactly;
2. anchors the end of Stream 2 to the resolved cover PTS, matching the observed ColorOS layout;
3. estimates Stream 1-to-Stream 2 matrices across the overlapping trajectory;
4. estimates Stream 1-to-still matrices in a `±0.12 s` cover window;
5. accepts the median cover matrix only when at least 60 percent of paired trajectory frames pass
   the geometry gate and the Stream 1-to-Stream 2 cover median agrees with the still median;
6. writes that accepted median as Track 5 `live-photo-still-image-transform` with the same analysis
   reference dimensions.
7. writes movie-level `com.apple.quicktime.limit-still-image-transform = 1` only for that accepted
   Vision result. iOS 26.5.2 Photos firmware reads this through
   `PFMetadataMovie.livePhotoVitalityLimitingAllowed`, forwards it to
   `PUBrowsingIrisPlayer` as `limitingAllowed`, and uses the larger allowed-inset budget before
   deciding whether to disable the Track 5 vitality transform.

If Vision, timing, or trajectory agreement fails, conversion falls back to the existing
vendor-metadata transform instead of failing the Live Photo conversion. The selected matrix and
reference dimensions are read back from the output MOV and compared numerically by the production
validator.

This is a static cover/settling transform. The Stream 1-to-Stream 2 trajectory is confidence
evidence for selecting it; it is not serialized as a private per-frame Track 4 payload.

## Deliberately not written yet

The following data is useful research evidence but is **not production-writable on this branch**:

1. `mdta/com.apple.quicktime.live-photo-info` Track 4 generated from Vision. The recovered Apple v3
   payload contains a per-frame stabilized trajectory homography, but the Vision-to-Apple coordinate
   convention, mapping direction, reference dimensions, orientation handling, and normalization have
   not been closed across native Apple samples.
2. `AVAppleMakerNote_StillImageProcessingHomography`. Firmware analysis connects this class of still
   processing metadata to Photos vitality transforms, but its third-party writable MakerNote layout
   and coordinate contract are private and not yet proven.
3. Synthetic motion-blur vectors or blur radii. Photos owns its horizontal-scroll/vitality blur
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
- an explicit transform reference space taking priority over the legacy metadata fallback;
- Track 5 reference dimensions surviving an AVFoundation write/read round trip;
- Vision identity registration producing a normalized near-identity homography.

The local verified ColorOS 16 functional gate additionally uses
`IMG20260509190128.jpg` and checks the Photos-validated contract:

- cover time approximately `1.36612 s`;
- matrix approximately
  `[0.901964, -0.004643, 108.713, 0.004598, 0.900466, 81.331, 0, 0, 1]`;
- reference dimensions `1920×1440`;
- PhotoKit pair construction succeeds;
- compressed Stream 1 video and audio samples are byte-identical after remuxing.
- the Vision-selected output reads back `limit-still-image-transform = 1`, while metadata-only
  fallback transforms do not opt into that private vitality behavior.

## Device validation still required

The intended Photos.app experiment remains an A/B matrix rather than an offline acceptance claim:

- baseline current conversion;
- corrected public Track 5 geometry selected from the Vision trajectory gate;
- future Track 4 candidate only after its coordinate mapping is closed;
- future still-processing-homography candidate only after its writable MakerNote contract is closed;
- combined candidate.

Horizontal swipe/vitality blur, geometry movement, and cover settling are device-dependent Photos
behavior. Offline parsing, PhotoKit construction, and compressed-payload equality cannot prove those
visual effects. Device evidence must remain a separate acceptance gate.
