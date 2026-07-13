# OPPO-Derived Apple Camera Calibration Plan

Date: 2026-07-13

## Goal

Replace the fixed iPhone disparity calibration currently written by
`--apple-portrait` with a per-capture model derived from OPPO EXIF and the
stored `src.image` geometry.

Also replace the fixed Apple simulated aperture with the OPPO portrait edit
f-number while preserving the device-validated rendering-parameter graph.

## Model

1. Read physical focal length, 35mm-equivalent focal length, digital zoom
   ratio, and lens model from the OPPO primary EXIF.
2. Resolve the optical anchor from the lens model (`23mm`, `70mm`, etc.), with
   `equivalent / digitalZoomRatio` as a fallback.
3. Estimate the anchor focal length in pixels from the reference-image
   diagonal and the 35mm-frame diagonal.
4. Match Apple's observed crop representation: keep the anchor focal length in
   the intrinsic matrix while shrinking the reference dimensions by the
   continuous digital zoom ratio.
5. Derive effective pixel size from physical focal length and the anchor pixel
   focal length, then scale it by the zoom ratio.
6. Until real OPPO lens distortion tables are available, describe the
   registered OPPO source/depth pair as rectified with zero forward/inverse
   distortion instead of copying an unrelated iPhone lens profile.
7. Read the portrait f-number from `rear.depth.config` v4 offset 292, fall back
   to EXIF `FNumber`, and reserve `f/1.4` for missing/invalid metadata only.

## Validation

- Swift CLI type-checks with warnings as errors.
- A 7.73mm/23mm-anchor portrait and a 20.1mm/70mm-anchor portrait convert.
- ExifTool reports different, EXIF-derived calibration matrices for the two
  physical lenses and zoom ratios.
- ImageIO still exposes ISO gain map, disparity, Portrait Effects Matte, and
  Focus XMP.
- ExifTool reports the OPPO portrait f-number as `SimulatedAperture`.
- Portrait/landscape selection follows the outer image's displayed aspect and
  the `src.image` EXIF orientation; it must not depend on exact swapped pixel
  dimensions.
- Final iPhone import remains the device acceptance gate.
