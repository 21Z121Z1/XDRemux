//! Relative scene luminance from embedded Log-like thumbnail guidance.
//! This is an approximation, not inversion of Apple's entire capture ISP.
use crate::error::{HdrError, Result};
use crate::{SceneGuidanceMetrics, SceneGuidancePolicy};

pub type LinearPixel = [f32; 4];

#[derive(Debug)]
pub struct SceneReconstruction {
    pub pixels: Vec<LinearPixel>,
    pub exposure_ev: f32,
    pub guidance_valid_fraction: f32,
    pub limited_fraction: f32,
    pub guidance: SceneGuidanceMetrics,
}

#[derive(Debug)]
pub struct SceneGainMap {
    pub raster: crate::GainMapRaster,
    pub min_ev: f32,
    pub max_ev: f32,
    pub offset: f32,
    pub clipped_negative_fraction: f32,
}

/// RGB gain carries both the HDR-only style change and luminance restoration.
/// A shared nonzero offset keeps gains finite at encoded black. The SDR base is
/// never changed. Signed minimum gains permit HDR shadows darker than SDR.
pub fn make_scene_gain_map(
    sdr: &[LinearPixel],
    scene: &[LinearPixel],
    width: usize,
    height: usize,
) -> Result<SceneGainMap> {
    checked_raster(sdr, width, height)?;
    checked_raster(scene, width, height)?;
    let offset = 1. / 64.;
    let mut gains = Vec::with_capacity(sdr.len() * 3);
    let mut lo = 0f32;
    let mut hi = 0f32;
    let mut negative = 0usize;
    for (base, target) in sdr.iter().zip(scene) {
        for c in 0..3 {
            if target[c] < 0. {
                negative += 1;
            }
            let ev = ((target[c].max(0.) + offset) / (base[c].max(0.) + offset)).log2();
            lo = lo.min(ev);
            hi = hi.max(ev);
            gains.push(ev);
        }
    }
    if hi - lo > 24. || hi > 16. || lo < -16. {
        return Err(invalid("scene Gain Map exceeds the supported gain range"));
    }
    hi = hi.max(lo + 0.01);
    let data = gains
        .iter()
        .map(|v| (((v - lo) / (hi - lo)) * 255.).round().clamp(0., 255.) as u8)
        .collect();
    Ok(SceneGainMap {
        raster: crate::GainMapRaster {
            width,
            height,
            bytes_per_row: width * 3,
            channel_count: 3,
            data,
        },
        min_ev: lo,
        max_ev: hi,
        offset,
        clipped_negative_fraction: negative as f32 / (sdr.len() * 3) as f32,
    })
}

pub fn display_p3_luminance(p: &LinearPixel) -> f32 {
    p[0] * 0.22897456 + p[1] * 0.69173855 + p[2] * 0.07928691
}

/// Matches the measured native CIAppleLogToLinear exponential branch. In
/// particular, this does not extrapolate the standard piecewise toe below zero.
pub fn styles_thumbnail_to_linear(code: f32) -> f32 {
    ((code - 0.69336945) / 0.08550479).exp2() - 0.00964052
}

fn invalid(message: &str) -> HdrError {
    HdrError::invalid("scene reconstruction", message)
}

fn checked_raster(pixels: &[LinearPixel], w: usize, h: usize) -> Result<()> {
    if w == 0
        || h == 0
        || w.checked_mul(h) != Some(pixels.len())
        || pixels.iter().flatten().any(|x| !x.is_finite())
    {
        return Err(invalid("invalid scene reconstruction raster"));
    }
    Ok(())
}

// Area averages, including partial pixel coverage, avoid sampling aliases when
// the full-resolution image and auxiliary image do not have integer ratios.
pub(super) fn area_luminance(
    pixels: &[LinearPixel],
    w: usize,
    h: usize,
    tw: usize,
    th: usize,
) -> Vec<f32> {
    let mut values = vec![0.; tw * th];
    for ty in 0..th {
        let y0 = ty as f64 * h as f64 / th as f64;
        let y1 = (ty + 1) as f64 * h as f64 / th as f64;
        for tx in 0..tw {
            let x0 = tx as f64 * w as f64 / tw as f64;
            let x1 = (tx + 1) as f64 * w as f64 / tw as f64;
            let mut sum = 0.;
            let mut weight = 0.;
            for y in y0.floor() as usize..(y1.ceil() as usize).min(h) {
                let wy = (y1.min((y + 1) as f64) - y0.max(y as f64)).max(0.);
                for x in x0.floor() as usize..(x1.ceil() as usize).min(w) {
                    let q = wy * (x1.min((x + 1) as f64) - x0.max(x as f64)).max(0.);
                    sum += display_p3_luminance(&pixels[y * w + x]) as f64 * q;
                    weight += q;
                }
            }
            values[ty * tw + tx] = (sum / weight) as f32;
        }
    }
    values
}

pub fn reconstruct_relative_scene(
    unstyled: &[LinearPixel],
    original_hdr: &[LinearPixel],
    base_sdr: &[LinearPixel],
    geometry: (usize, usize),
    thumbnail_codes: &[LinearPixel],
    thumbnail_geometry: (usize, usize),
    guidance_policy: SceneGuidancePolicy,
) -> Result<SceneReconstruction> {
    let (width, height) = geometry;
    let (thumb_width, thumb_height) = thumbnail_geometry;
    crate::scene_guidance::validate(guidance_policy)?;
    checked_raster(unstyled, width, height)?;
    checked_raster(original_hdr, width, height)?;
    checked_raster(base_sdr, width, height)?;
    checked_raster(thumbnail_codes, thumb_width, thumb_height)?;
    if ((width as f64 / height as f64) / (thumb_width as f64 / thumb_height as f64) - 1.).abs()
        > 0.015
    {
        return Err(invalid(
            "thumbnail aspect ratio does not match the oriented primary",
        ));
    }
    if thumbnail_codes
        .iter()
        .any(|p| p[..3].iter().any(|x| *x < -0.25 || *x > 1.25))
    {
        return Err(invalid("thumbnail is outside the supported encoded domain"));
    }
    let base = area_luminance(original_hdr, width, height, thumb_width, thumb_height);
    let scene: Vec<f32> = thumbnail_codes
        .iter()
        .map(|p| {
            display_p3_luminance(&[
                styles_thumbnail_to_linear(p[0]),
                styles_thumbnail_to_linear(p[1]),
                styles_thumbnail_to_linear(p[2]),
                1.,
            ])
        })
        .collect();
    let mut anchors: Vec<(f32, f32)> = base
        .iter()
        .zip(&scene)
        .filter(|(b, s)| **b > 1e-5 && **s > 1e-5)
        .map(|(b, s)| (*s, (*b / *s).log2()))
        .collect();
    let valid_fraction = anchors.len() as f32 / base.len() as f32;
    if anchors.len() < 16 || valid_fraction < 0.5 {
        return Err(invalid(
            "insufficient positive thumbnail luminance for scene guidance",
        ));
    }
    anchors.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
    let mut offsets: Vec<f32> = anchors[anchors.len() / 5..anchors.len() * 4 / 5]
        .iter()
        .map(|x| x.1)
        .collect();
    offsets.sort_unstable_by(f32::total_cmp);
    // A per-image exposure convention preserves the midtone brightness of its
    // existing HDR. No RAW, filename, camera-specific fit or nit anchor is used.
    let exposure_ev = offsets[offsets.len() / 2];
    let mut limited = 0;
    let gains: Vec<Option<f32>> = base
        .iter()
        .zip(&scene)
        .map(|(b, s)| {
            if *b <= 1e-5 || *s <= 1e-5 {
                return None;
            }
            let ev = (*s / *b).log2() + exposure_ev;
            if ev.abs() > 6. {
                limited += 1;
            }
            Some(ev.clamp(-6., 6.))
        })
        .collect();
    if limited as f32 / anchors.len() as f32 > 0.05 {
        return Err(invalid(
            "scene guidance requires excessive gain; unsupported or misaligned resources",
        ));
    }
    let adjustment = crate::scene_guidance::adjustment(
        &base,
        &scene,
        &area_luminance(base_sdr, width, height, thumb_width, thumb_height),
        (thumb_width, thumb_height),
        exposure_ev,
        guidance_policy,
    )?;
    let mut output = Vec::with_capacity(unstyled.len());
    let mut adjustment_square = 0f64;
    let mut maximum_adjustment = 0f32;
    for y in 0..height {
        let fy = (y as f32 + 0.5) * thumb_height as f32 / height as f32 - 0.5;
        let iy = fy.floor() as isize;
        for x in 0..width {
            let n = y * width + x;
            let hdr_y = display_p3_luminance(&original_hdr[n]).max(1e-8);
            let fx = (x as f32 + 0.5) * thumb_width as f32 / width as f32 - 0.5;
            let ix = fx.floor() as isize;
            let mut sum = 0.;
            let mut weight = 0.;
            // Joint bilateral interpolation follows primary luminance edges and
            // keeps the gain bounded. The high-frequency primary detail remains.
            for dy in -1..=2 {
                for dx in -1..=2 {
                    let sx = (ix + dx).clamp(0, thumb_width as isize - 1) as usize;
                    let sy = (iy + dy).clamp(0, thumb_height as isize - 1) as usize;
                    let k = sy * thumb_width + sx;
                    if let Some(ev) = gains[k] {
                        let distance = (sx as f32 - fx).powi(2) + (sy as f32 - fy).powi(2);
                        let range = (base[k].max(1e-8) / hdr_y).log2();
                        let q = (-0.5 * distance - 2. * range * range).exp();
                        sum += q * ev;
                        weight += q;
                    }
                }
            }
            let original_gain = if weight > 1e-10 { sum / weight } else { 0. };
            let delta = interpolate_adjustment(
                &adjustment,
                (x, y),
                (width, height),
                display_p3_luminance(&base_sdr[n]).max(1e-8),
                original_gain,
                guidance_policy.limit_ev,
            );
            adjustment_square += (delta as f64).powi(2);
            maximum_adjustment = maximum_adjustment.max(delta.abs());
            let gain = (original_gain + delta).exp2();
            let uy = display_p3_luminance(&unstyled[n]);
            let color = if uy > 1e-7 {
                unstyled[n]
            } else {
                original_hdr[n]
            };
            let lum = display_p3_luminance(&color).max(1e-8);
            let factor = hdr_y / lum * gain;
            output.push([color[0] * factor, color[1] * factor, color[2] * factor, 1.]);
        }
    }
    if output.iter().flatten().any(|v| !v.is_finite()) {
        return Err(invalid("scene reconstruction generated non-finite pixels"));
    }
    let mut metrics = adjustment.metrics;
    metrics.adjustment_rms_ev = (adjustment_square / output.len() as f64).sqrt() as f32;
    metrics.maximum_adjustment_ev = maximum_adjustment;
    Ok(SceneReconstruction {
        pixels: output,
        exposure_ev,
        guidance_valid_fraction: valid_fraction,
        limited_fraction: limited as f32 / base.len() as f32,
        guidance: metrics,
    })
}

// The bounded adjustment uses the full-resolution SDR base as the edge guide.
// Interpolation authors a constant regional gain before bounding the change.
fn interpolate_adjustment(
    field: &crate::scene_guidance::GuidanceAdjustment,
    point: (usize, usize),
    geometry: (usize, usize),
    primary_y: f32,
    original_gain: f32,
    limit_ev: f32,
) -> f32 {
    let (w, h) = (field.metrics.width, field.metrics.height);
    let fx = (point.0 as f32 + 0.5) * w as f32 / geometry.0 as f32 - 0.5;
    let fy = (point.1 as f32 + 0.5) * h as f32 / geometry.1 as f32 - 0.5;
    let mut sum = 0.;
    let mut weight = 0.;
    for dy in -1..=2 {
        for dx in -1..=2 {
            let x = (fx.floor() as isize + dx).clamp(0, w as isize - 1) as usize;
            let y = (fy.floor() as isize + dy).clamp(0, h as isize - 1) as usize;
            let i = y * w + x;
            if let Some(delta) = field.values[i] {
                let range = (field.guide[i].max(1e-8) / primary_y).log2();
                let distance = (x as f32 - fx).powi(2) + (y as f32 - fy).powi(2);
                let q = (-0.5 * distance - 2. * range * range).exp();
                sum += q * field.strength[i] * (delta - original_gain);
                weight += q;
            }
        }
    }
    if weight > 1e-10 {
        (sum / weight).clamp(-limit_ev, limit_ev)
    } else {
        0.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn policy() -> SceneGuidancePolicy {
        SceneGuidancePolicy {
            analysis_edge: 512,
            primary_span_ev: 0.4,
            minimum_region_pixels: 64,
            feather_radius: 4,
            edge_ev: 0.1,
            flat_ev: 0.075,
            limit_ev: 0.25,
        }
    }
    fn encode(x: f32) -> f32 {
        0.08550479 * (x + 0.00964052).log2() + 0.69336945
    }
    #[test]
    fn restores_luminance_relationship_without_raw_or_image_specific_parameters() {
        let codes: Vec<_> = (0..256)
            .map(|i| {
                let v = encode(0.002 * 2f32.powf(i as f32 / 25.));
                [v, v, v, 1.]
            })
            .collect();
        let processed: Vec<_> = (0..256)
            .map(|i| {
                let v = (0.002 * 2f32.powf(i as f32 / 25.)).sqrt();
                [v * 1.1, v, v * 0.8, 1.]
            })
            .collect();
        let result = reconstruct_relative_scene(
            &processed,
            &processed,
            &processed,
            (16, 16),
            &codes,
            (16, 16),
            policy(),
        )
        .unwrap();
        let before =
            (display_p3_luminance(&processed[240]) / display_p3_luminance(&processed[16])).log2();
        let after = (display_p3_luminance(&result.pixels[240])
            / display_p3_luminance(&result.pixels[16]))
        .log2();
        assert!((after - 224. / 25.).abs() < 0.08, "{before} -> {after}");
        assert!((before - 224. / 50.).abs() < 0.001);
        assert!((result.pixels[128][0] / result.pixels[128][1] - 1.1).abs() < 1e-5);
    }
    #[test]
    fn rejects_mismatched_orientation_and_nonfinite_or_empty_guidance() {
        let p = vec![[0.2, 0.2, 0.2, 1.]; 128];
        assert!(reconstruct_relative_scene(&p, &p, &p, (16, 8), &p, (8, 16), policy()).is_err());
        let bad = vec![[f32::NAN, 0., 0., 1.]; 128];
        assert!(reconstruct_relative_scene(&p, &p, &p, (16, 8), &bad, (16, 8), policy()).is_err());
        let black = vec![[0., 0., 0., 1.]; 128];
        assert!(
            reconstruct_relative_scene(&p, &p, &p, (16, 8), &black, (16, 8), policy()).is_err()
        );
    }
    #[test]
    fn weak_primary_structure_does_not_inherit_all_thumbnail_blotches_or_lose_detail() {
        let (w, h, tw, th) = (512, 256, 256, 128);
        let primary = (0..w * h)
            .map(|i| {
                let v = 0.2 * 2f32.powf(if (i / w) % 2 == 0 { 0.03 } else { -0.03 });
                [v, v, v, 1.]
            })
            .collect::<Vec<_>>();
        let thumbnail = (0..tw * th)
            .map(|i| {
                let blotch = 0.12 * (std::f32::consts::TAU * (i % tw) as f32 / 64.).sin();
                let v = encode(0.1 * blotch.exp2());
                [v, v, v, 1.]
            })
            .collect::<Vec<_>>();
        let result = reconstruct_relative_scene(
            &primary,
            &primary,
            &primary,
            (w, h),
            &thumbnail,
            (tw, th),
            policy(),
        )
        .unwrap();
        let gains = (64..448)
            .map(|x| {
                let i = 120 * w + x;
                (display_p3_luminance(&result.pixels[i]) / display_p3_luminance(&primary[i])).log2()
            })
            .collect::<Vec<_>>();
        let spread = gains.iter().copied().fold(f32::NEG_INFINITY, f32::max)
            - gains.iter().copied().fold(f32::INFINITY, f32::min);
        // The old pointwise guidance transfers almost the full 0.24 EV wave.
        assert!(spread < 0.025, "transferred low-frequency spread {spread}");
        for x in 64..448 {
            let a = 120 * w + x;
            let b = a + w;
            let detail = (display_p3_luminance(&result.pixels[a])
                / display_p3_luminance(&result.pixels[b]))
            .log2();
            assert!((detail - 0.06).abs() < 1e-5, "detail {detail}");
        }
        assert!(result.guidance.maximum_adjustment_ev <= policy().limit_ev);
        assert!(result.guidance.regions > 0);
    }
    #[test]
    fn stabilization_keeps_true_range_and_does_not_bleed_across_a_strong_edge() {
        let (w, h) = (128, 32);
        let primary = (0..w * h)
            .map(|i| {
                let v = if i % w < w / 2 { 0.1 } else { 0.5 };
                [v, v, v, 1.]
            })
            .collect::<Vec<_>>();
        let thumbnail = (0..w * h)
            .map(|i| {
                let v = encode(if i % w < w / 2 { 0.03 } else { 2. });
                [v, v, v, 1.]
            })
            .collect::<Vec<_>>();
        let result = reconstruct_relative_scene(
            &primary,
            &primary,
            &primary,
            (w, h),
            &thumbnail,
            (w, h),
            policy(),
        )
        .unwrap();
        let row = &result.pixels[16 * w..17 * w];
        let lo = display_p3_luminance(&row[16]);
        let hi = display_p3_luminance(&row[112]);
        assert!(((hi / lo).log2() - (2f32 / 0.03).log2()).abs() < 0.005);
        for (x, p) in row.iter().enumerate() {
            let expected = if x < w / 2 { lo } else { hi };
            assert!((display_p3_luminance(p) / expected).log2().abs() < 0.005);
        }
    }
    #[test]
    fn full_resolution_base_guides_flatness_without_flattening_hdr_range() {
        let (w, h) = (256, 64);
        let primary = (0..w * h)
            .map(|i| {
                let v = 0.1 * 2f32.powf((i % w) as f32 / 128.);
                [v, v, v, 1.]
            })
            .collect::<Vec<_>>();
        let base = vec![[0.15, 0.15, 0.15, 1.]; w * h];
        let thumbnail = primary
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let wave = 0.12 * (std::f32::consts::TAU * (i % w) as f32 / 64.).sin();
                let c = encode(p[0] * wave.exp2());
                [c, c, c, 1.]
            })
            .collect::<Vec<_>>();
        let result = reconstruct_relative_scene(
            &primary,
            &primary,
            &base,
            (w, h),
            &thumbnail,
            (w, h),
            policy(),
        )
        .unwrap();
        let gains = (16..240)
            .map(|x| {
                let i = 32 * w + x;
                (display_p3_luminance(&result.pixels[i]) / display_p3_luminance(&primary[i])).log2()
            })
            .collect::<Vec<_>>();
        let spread = gains.iter().copied().fold(f32::NEG_INFINITY, f32::max)
            - gains.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(spread < 0.025, "base-guided gain spread {spread}");
        let ratio = (display_p3_luminance(&result.pixels[32 * w + 224])
            / display_p3_luminance(&result.pixels[32 * w + 32]))
        .log2();
        assert!((ratio - 1.5).abs() < 0.005, "HDR range {ratio}");
    }
    #[test]
    fn large_thumbnail_scene_differences_are_not_declared_flat_surface_artifacts() {
        let (w, h) = (256, 64);
        let primary = vec![[0.2, 0.2, 0.2, 1.]; w * h];
        let thumbnail = (0..w * h)
            .map(|i| {
                let wave = 0.75 * (std::f32::consts::TAU * (i % w) as f32 / 64.).sin();
                let c = encode(0.1 * wave.exp2());
                [c, c, c, 1.]
            })
            .collect::<Vec<_>>();
        let result = reconstruct_relative_scene(
            &primary,
            &primary,
            &primary,
            (w, h),
            &thumbnail,
            (w, h),
            policy(),
        )
        .unwrap();
        assert_eq!(result.guidance.regions, 0);
        assert_eq!(result.guidance.maximum_adjustment_ev, 0.);
        let hi = display_p3_luminance(&result.pixels[32 * w + 16]);
        let lo = display_p3_luminance(&result.pixels[32 * w + 48]);
        assert!(((hi / lo).log2() - 1.5).abs() < 0.02);
    }
    #[test]
    fn gain_map_can_darken_hdr_shadows_without_touching_sdr() {
        let base = vec![[0.3, 0.2, 0.1, 1.]; 32];
        let original = base.clone();
        let target: Vec<_> = (0..32)
            .map(|i| {
                if i < 16 {
                    [0.03, 0.04, 0.06, 1.]
                } else {
                    [2., 3., 4., 1.]
                }
            })
            .collect();
        let map = make_scene_gain_map(&base, &target, 8, 4).unwrap();
        assert!(map.min_ev < 0. && map.max_ev > 0.);
        for (i, p) in target.iter().enumerate() {
            for c in 0..3 {
                let ev = map.min_ev
                    + map.raster.data[i * 3 + c] as f32 / 255. * (map.max_ev - map.min_ev);
                let value = (base[i][c] + map.offset) * ev.exp2() - map.offset;
                assert!((value - p[c]).abs() < 0.04);
            }
        }
        assert_eq!(base, original);
    }
}
