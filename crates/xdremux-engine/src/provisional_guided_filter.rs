//! Provisional reconstruction of the consumer guided filter.
//!
//! `iterations/20261018-v3` / `guided.py` is not in the public research
//! checkout. This follows the published description of that reconstruction:
//! coefficients are computed on a half-size primary/guide grid, box smoothing
//! uses radius 1 in guide pixels, epsilon is 0.05, the smoothed coefficients
//! pass through half precision, and the coefficients and guide are bilinearly
//! sampled back to the output grid.
//!
//! The guide is the primary (self-guided), one channel at a time. That is the
//! provisional reading of the half RGB textures. It is not a calibration
//! against private `CMIGuidedFilter`, and the radius, epsilon, and local
//! weights are not Apple-tuned constants.

use std::error::Error;
use std::fmt;

use half::f16;
use serde::{Deserialize, Serialize};

pub const PROVISIONAL_GUIDED_RADIUS: usize = 1;
pub const PROVISIONAL_GUIDED_EPSILON: f32 = 0.05;
/// HDR look: keep the guided result and drop the local residual.
pub const PROVISIONAL_HDR_LOCAL_WEIGHT: f32 = 0.0;
/// SDR look: local weight 1 is the identity, so local detail stays.
pub const PROVISIONAL_SDR_LOCAL_WEIGHT: f32 = 1.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionalGuidedFilterError {
    context: &'static str,
    detail: String,
}

impl ProvisionalGuidedFilterError {
    fn new(context: &'static str, detail: impl Into<String>) -> Self {
        Self {
            context,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for ProvisionalGuidedFilterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.context, self.detail)
    }
}

impl Error for ProvisionalGuidedFilterError {}

/// `output = guided + local_weight * (primary - guided)`.
///
/// `local_weight == 1` returns the primary unchanged. Radius and epsilon are
/// the provisional consumer values above.
pub fn provisional_guided_filter(
    primary: &[f32],
    width: usize,
    height: usize,
    local_weight: f32,
) -> Result<Vec<f32>, ProvisionalGuidedFilterError> {
    let pixel_count = checked_pixel_count(width, height)?;
    if primary.len() != pixel_count.saturating_mul(3) {
        return Err(ProvisionalGuidedFilterError::new(
            "provisional guided filter",
            format!(
                "expected {} RGB samples, found {}",
                pixel_count.saturating_mul(3),
                primary.len()
            ),
        ));
    }
    if !local_weight.is_finite() {
        return Err(ProvisionalGuidedFilterError::new(
            "provisional guided filter",
            "local weight must be finite",
        ));
    }
    if primary.iter().any(|sample| !sample.is_finite()) {
        return Err(ProvisionalGuidedFilterError::new(
            "provisional guided filter",
            "primary contains a non-finite sample",
        ));
    }
    let mut output = vec![0.0_f32; primary.len()];
    for channel in 0..3 {
        let planar = channel_plane(primary, channel);
        let filtered = filter_plane(&planar, width, height, local_weight)?;
        for (pixel, sample) in filtered.iter().enumerate() {
            output[pixel * 3 + channel] = *sample;
        }
    }
    Ok(output)
}

/// Tightly packed RGBA8 in Display P3. Display P3 uses the IEC sRGB transfer,
/// the same transfer `srgb_decode` inverts for Styles.
pub fn encode_display_p3_rgba8(
    linear_rgb: &[f32],
) -> Result<Vec<u8>, ProvisionalGuidedFilterError> {
    if !linear_rgb.len().is_multiple_of(3) {
        return Err(ProvisionalGuidedFilterError::new(
            "Display P3 RGBA8",
            "linear RGB length is not a multiple of 3",
        ));
    }
    let mut packed = Vec::with_capacity(linear_rgb.len() / 3 * 4);
    for pixel in linear_rgb.as_chunks::<3>().0 {
        for component in pixel {
            if !component.is_finite() {
                return Err(ProvisionalGuidedFilterError::new(
                    "Display P3 RGBA8",
                    "linear RGB contains a non-finite sample",
                ));
            }
            packed.push(srgb_encode_channel(*component));
        }
        packed.push(255);
    }
    Ok(packed)
}

/// Tightly packed little-endian RGBAh. Alpha is 1.
pub fn pack_linear_rgba_f16(linear_rgb: &[f32]) -> Result<Vec<u8>, ProvisionalGuidedFilterError> {
    if !linear_rgb.len().is_multiple_of(3) {
        return Err(ProvisionalGuidedFilterError::new(
            "linear RGBAh",
            "linear RGB length is not a multiple of 3",
        ));
    }
    let alpha = f16::from_f32(1.0).to_le_bytes();
    let mut packed = Vec::with_capacity(linear_rgb.len() / 3 * 8);
    for pixel in linear_rgb.as_chunks::<3>().0 {
        for component in pixel {
            if !component.is_finite() {
                return Err(ProvisionalGuidedFilterError::new(
                    "linear RGBAh",
                    "linear RGB contains a non-finite sample",
                ));
            }
            packed.extend_from_slice(&f16::from_f32(*component).to_le_bytes());
        }
        packed.extend_from_slice(&alpha);
    }
    Ok(packed)
}

/// RGB samples from tightly packed little-endian RGBAh. Alpha is discarded.
pub fn unpack_linear_rgb_f16(packed: &[u8]) -> Result<Vec<f32>, ProvisionalGuidedFilterError> {
    if !packed.len().is_multiple_of(8) {
        return Err(ProvisionalGuidedFilterError::new(
            "linear RGBAh",
            "packed length is not a multiple of 8",
        ));
    }
    let mut rgb = Vec::with_capacity(packed.len() / 8 * 3);
    for pixel in packed.as_chunks::<8>().0 {
        for component in 0..3 {
            let bytes = [pixel[component * 2], pixel[component * 2 + 1]];
            let value = f16::from_le_bytes(bytes).to_f32();
            if !value.is_finite() {
                return Err(ProvisionalGuidedFilterError::new(
                    "linear RGBAh",
                    "packed RGBAh contains a non-finite sample",
                ));
            }
            rgb.push(value);
        }
    }
    Ok(rgb)
}

/// Headroom handed to Core Image. The header accepts 0 (unknown) or a value
/// greater than or equal to 1. The measured peak is raised to 1 so a value in
/// `(0, 1)` is never sent. This is not a nit calibration.
pub fn content_headroom_from_linear(samples: &[f32]) -> Result<f32, ProvisionalGuidedFilterError> {
    let mut peak = 0.0_f32;
    for sample in samples {
        if !sample.is_finite() {
            return Err(ProvisionalGuidedFilterError::new(
                "content headroom",
                "linear sample is non-finite",
            ));
        }
        peak = peak.max(*sample);
    }
    Ok(peak.max(1.0))
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LinearDifference {
    pub mean_abs: f64,
    pub max_abs: f64,
    /// Peak is the maximum absolute reference sample, and at least 1.
    /// Exact agreement has no finite PSNR.
    pub psnr_db: Option<f64>,
}

pub fn linear_difference(
    reference: &[f32],
    candidate: &[f32],
) -> Result<LinearDifference, ProvisionalGuidedFilterError> {
    if reference.len() != candidate.len() {
        return Err(ProvisionalGuidedFilterError::new(
            "linear difference",
            format!(
                "reference has {} samples and candidate has {}",
                reference.len(),
                candidate.len()
            ),
        ));
    }
    if reference.is_empty() {
        return Err(ProvisionalGuidedFilterError::new(
            "linear difference",
            "sample list is empty",
        ));
    }
    let mut abs_sum = 0.0_f64;
    let mut square_sum = 0.0_f64;
    let mut max_abs = 0.0_f64;
    let mut peak = 0.0_f64;
    for (reference_sample, candidate_sample) in reference.iter().zip(candidate) {
        if !reference_sample.is_finite() || !candidate_sample.is_finite() {
            return Err(ProvisionalGuidedFilterError::new(
                "linear difference",
                "difference contains a non-finite sample",
            ));
        }
        let delta = f64::from(*candidate_sample) - f64::from(*reference_sample);
        let abs_delta = delta.abs();
        abs_sum += abs_delta;
        square_sum += delta * delta;
        max_abs = max_abs.max(abs_delta);
        peak = peak.max(f64::from(reference_sample.abs()));
    }
    let count = reference.len() as f64;
    let mse = square_sum / count;
    let psnr_db = if mse == 0.0 {
        None
    } else {
        let peak = peak.max(1.0);
        Some(10.0 * (peak * peak / mse).log10())
    };
    Ok(LinearDifference {
        mean_abs: abs_sum / count,
        max_abs,
        psnr_db,
    })
}

fn filter_plane(
    primary: &[f32],
    width: usize,
    height: usize,
    local_weight: f32,
) -> Result<Vec<f32>, ProvisionalGuidedFilterError> {
    if (local_weight - 1.0).abs() <= f32::EPSILON {
        return Ok(primary.to_vec());
    }
    let half_width = half_grid(width);
    let half_height = half_grid(height);
    let guide = area_reduce(primary, width, height, half_width, half_height)?;
    let mean_i = box_mean(&guide, half_width, half_height, PROVISIONAL_GUIDED_RADIUS);
    let mean_ii = box_mean(
        &product(&guide, &guide),
        half_width,
        half_height,
        PROVISIONAL_GUIDED_RADIUS,
    );
    let mut a = Vec::with_capacity(guide.len());
    let mut b = Vec::with_capacity(guide.len());
    for index in 0..guide.len() {
        let variance = mean_ii[index] - mean_i[index] * mean_i[index];
        // Self-guided: p is the guide, so cov/var collapses to var/(var+eps)
        // and b is (1-a) times the guide mean.
        let coefficient = variance / (variance + PROVISIONAL_GUIDED_EPSILON);
        a.push(coefficient);
        b.push(mean_i[index] - coefficient * mean_i[index]);
    }
    let a = through_f16_plane(&box_mean(
        &a,
        half_width,
        half_height,
        PROVISIONAL_GUIDED_RADIUS,
    ));
    let b = through_f16_plane(&box_mean(
        &b,
        half_width,
        half_height,
        PROVISIONAL_GUIDED_RADIUS,
    ));
    let mut output = Vec::with_capacity(primary.len());
    for y in 0..height {
        for x in 0..width {
            let guide_sample = bilinear(&guide, half_width, half_height, x, y, width, height);
            let a_sample = bilinear(&a, half_width, half_height, x, y, width, height);
            let b_sample = bilinear(&b, half_width, half_height, x, y, width, height);
            let guided = a_sample * guide_sample + b_sample;
            let sample = primary[y * width + x];
            output.push(guided + local_weight * (sample - guided));
        }
    }
    Ok(output)
}

fn half_grid(length: usize) -> usize {
    if length <= 1 {
        1
    } else {
        length.div_ceil(2)
    }
}

fn through_f16_plane(values: &[f32]) -> Vec<f32> {
    values
        .iter()
        .copied()
        .map(|value| f16::from_f32(value).to_f32())
        .collect()
}

fn channel_plane(rgb: &[f32], channel: usize) -> Vec<f32> {
    rgb.as_chunks::<3>()
        .0
        .iter()
        .map(|pixel| pixel[channel])
        .collect()
}

fn product(left: &[f32], right: &[f32]) -> Vec<f32> {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .collect()
}

fn area_reduce(
    source: &[f32],
    width: usize,
    height: usize,
    target_width: usize,
    target_height: usize,
) -> Result<Vec<f32>, ProvisionalGuidedFilterError> {
    if target_width == width && target_height == height {
        return Ok(source.to_vec());
    }
    let mut samples = vec![0.0_f32; target_width * target_height];
    for y in 0..target_height {
        let y0 = y as f64 * height as f64 / target_height as f64;
        let y1 = (y + 1) as f64 * height as f64 / target_height as f64;
        for x in 0..target_width {
            let x0 = x as f64 * width as f64 / target_width as f64;
            let x1 = (x + 1) as f64 * width as f64 / target_width as f64;
            let mut accumulated = 0.0_f64;
            let mut coverage = 0.0_f64;
            let row_start = y0.floor() as usize;
            let row_end = (y1.ceil() as usize).min(height);
            let column_start = x0.floor() as usize;
            let column_end = (x1.ceil() as usize).min(width);
            for source_y in row_start..row_end {
                let y_coverage = ((source_y + 1) as f64).min(y1) - (source_y as f64).max(y0);
                if y_coverage <= 0.0 {
                    continue;
                }
                for source_x in column_start..column_end {
                    let x_coverage = ((source_x + 1) as f64).min(x1) - (source_x as f64).max(x0);
                    if x_coverage <= 0.0 {
                        continue;
                    }
                    let weight = x_coverage * y_coverage;
                    accumulated += f64::from(source[source_y * width + source_x]) * weight;
                    coverage += weight;
                }
            }
            if coverage <= 0.0 {
                return Err(ProvisionalGuidedFilterError::new(
                    "provisional guided filter",
                    "half-size bin has no source coverage",
                ));
            }
            samples[y * target_width + x] = (accumulated / coverage) as f32;
        }
    }
    Ok(samples)
}

fn box_mean(source: &[f32], width: usize, height: usize, radius: usize) -> Vec<f32> {
    let stride = width + 1;
    let mut integral = vec![0.0_f64; stride * (height + 1)];
    for y in 0..height {
        let mut row_sum = 0.0_f64;
        for x in 0..width {
            row_sum += f64::from(source[y * width + x]);
            integral[(y + 1) * stride + (x + 1)] = integral[y * stride + (x + 1)] + row_sum;
        }
    }
    let mut output = vec![0.0_f32; width * height];
    for y in 0..height {
        let y0 = y.saturating_sub(radius);
        let y1 = (y + radius).min(height - 1);
        for x in 0..width {
            let x0 = x.saturating_sub(radius);
            let x1 = (x + radius).min(width - 1);
            let sum = integral[(y1 + 1) * stride + (x1 + 1)]
                - integral[y0 * stride + (x1 + 1)]
                - integral[(y1 + 1) * stride + x0]
                + integral[y0 * stride + x0];
            let count = (x1 - x0 + 1) * (y1 - y0 + 1);
            output[y * width + x] = (sum / count as f64) as f32;
        }
    }
    output
}

fn bilinear(
    grid: &[f32],
    grid_width: usize,
    grid_height: usize,
    x: usize,
    y: usize,
    full_width: usize,
    full_height: usize,
) -> f32 {
    if grid_width == 1 && grid_height == 1 {
        return grid[0];
    }
    let fx = (x as f64 + 0.5) * grid_width as f64 / full_width as f64 - 0.5;
    let fy = (y as f64 + 0.5) * grid_height as f64 / full_height as f64 - 0.5;
    let x0 = fx.floor().clamp(0.0, (grid_width - 1) as f64) as usize;
    let y0 = fy.floor().clamp(0.0, (grid_height - 1) as f64) as usize;
    let x1 = (x0 + 1).min(grid_width - 1);
    let y1 = (y0 + 1).min(grid_height - 1);
    let tx = (fx - x0 as f64).clamp(0.0, 1.0);
    let ty = (fy - y0 as f64).clamp(0.0, 1.0);
    let sample = |sx: usize, sy: usize| f64::from(grid[sy * grid_width + sx]);
    let top = sample(x0, y0) * (1.0 - tx) + sample(x1, y0) * tx;
    let bottom = sample(x0, y1) * (1.0 - tx) + sample(x1, y1) * tx;
    (top * (1.0 - ty) + bottom * ty) as f32
}

fn checked_pixel_count(width: usize, height: usize) -> Result<usize, ProvisionalGuidedFilterError> {
    if width == 0 || height == 0 {
        return Err(ProvisionalGuidedFilterError::new(
            "provisional guided filter",
            "width and height must be positive",
        ));
    }
    width.checked_mul(height).ok_or_else(|| {
        ProvisionalGuidedFilterError::new("provisional guided filter", "pixel count overflows")
    })
}

fn srgb_encode_channel(linear: f32) -> u8 {
    let linear = linear.clamp(0.0, 1.0);
    let encoded = if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    let code = (encoded * 255.0 + 0.5).floor();
    code.clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_image_stays_constant_without_local_detail() {
        let primary = vec![0.5_f32; 6 * 4 * 3];
        let filtered =
            provisional_guided_filter(&primary, 6, 4, PROVISIONAL_HDR_LOCAL_WEIGHT).unwrap();
        assert_eq!(filtered, primary);
    }

    #[test]
    fn unit_local_weight_is_the_primary() {
        let mut primary = Vec::new();
        for value in 0..48 {
            primary.push(value as f32 / 47.0);
        }
        let filtered =
            provisional_guided_filter(&primary, 4, 4, PROVISIONAL_SDR_LOCAL_WEIGHT).unwrap();
        assert_eq!(filtered, primary);
    }

    #[test]
    fn checkerboard_changes_when_local_detail_is_removed() {
        let mut primary = Vec::with_capacity(4 * 4 * 3);
        for y in 0..4_usize {
            for x in 0..4_usize {
                let value = if (x + y).is_multiple_of(2) { 0.0 } else { 1.0 };
                primary.extend_from_slice(&[value, value, value]);
            }
        }
        let filtered =
            provisional_guided_filter(&primary, 4, 4, PROVISIONAL_HDR_LOCAL_WEIGHT).unwrap();
        let max_abs = primary
            .iter()
            .zip(&filtered)
            .map(|(before, after)| (before - after).abs())
            .fold(0.0_f32, f32::max);
        assert!(
            max_abs > 0.2,
            "guided look should leave the checkerboard, max abs {max_abs}"
        );
    }

    #[test]
    fn half_packing_round_trips_exact_values_and_sets_alpha() {
        let packed = pack_linear_rgba_f16(&[1.0, 0.0, 0.5]).unwrap();
        assert_eq!(packed.len(), 8);
        assert_eq!(&packed[6..8], &f16::from_f32(1.0).to_le_bytes());
        let unpacked = unpack_linear_rgb_f16(&packed).unwrap();
        assert_eq!(unpacked, vec![1.0, 0.0, 0.5]);
    }

    #[test]
    fn headroom_is_at_least_sdr_white() {
        assert_eq!(content_headroom_from_linear(&[0.2, 0.4, 4.0]).unwrap(), 4.0);
        assert_eq!(content_headroom_from_linear(&[0.2, 0.4]).unwrap(), 1.0);
    }

    #[test]
    fn exact_buffers_have_no_finite_psnr() {
        let difference = linear_difference(&[0.25, 0.5], &[0.25, 0.5]).unwrap();
        assert_eq!(difference.mean_abs, 0.0);
        assert_eq!(difference.max_abs, 0.0);
        assert_eq!(difference.psnr_db, None);
    }
}
