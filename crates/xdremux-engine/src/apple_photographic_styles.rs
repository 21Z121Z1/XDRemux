use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use half::f16;
use sha2::{Digest, Sha256};

pub const APPLE_STYLE_POLYNOMIAL_COUNT: usize = 10;
pub const APPLE_STYLE_CHANNEL_COUNT: usize = 3;
pub const APPLE_STYLE_BLOCK_VALUE_COUNT: usize =
    APPLE_STYLE_POLYNOMIAL_COUNT * APPLE_STYLE_CHANNEL_COUNT;
pub const APPLE_STYLE_GRID_WIDTH: usize = 12;
pub const APPLE_STYLE_GRID_HEIGHT: usize = 9;
pub const APPLE_STYLE_PLANE_COUNT: usize = 8;
pub const APPLE_STYLE_TILE_COUNT: usize =
    APPLE_STYLE_GRID_WIDTH * APPLE_STYLE_GRID_HEIGHT * APPLE_STYLE_PLANE_COUNT;
pub const APPLE_STYLE_VALUE_BYTE_COUNT: usize = 2;
pub const APPLE_STYLE_BYTE_COUNT: usize =
    APPLE_STYLE_BLOCK_VALUE_COUNT * APPLE_STYLE_TILE_COUNT * APPLE_STYLE_VALUE_BYTE_COUNT;
pub const APPLE_STYLE_IDENTITY_SHA256: &str =
    "43e0ae73508cc10684d4be708fa1d19f3b55b8de15cb8e3544ef16300db91dbe";
pub const APPLE_STYLE_REFINEMENT_PARAMETER_COUNT: usize = 12;
pub const APPLE_STYLE_REFINEMENT_MAX_PIXELS: usize = 50_000;
pub const APPLE_STYLE_REFINEMENT_EPSILON: f64 = 1.0 / 32.0;
pub const APPLE_STYLE_LIGHT_MAP_SIDE: usize = 32;
pub const APPLE_STYLE_LIGHT_MAP_BYTE_COUNT: usize =
    APPLE_STYLE_LIGHT_MAP_SIDE * APPLE_STYLE_LIGHT_MAP_SIDE * 2;

/// SmartStyle state for the Standard, reversible Styles resources we generate.
///
/// This is a separate consumer input from the Styles coefficient plist. These
/// eight integer fields match the former Swift writer and its working carrier;
/// they describe our generated edit state, not an Apple camera or capture UUID.
pub fn apple_style_maker_apple_dictionary() -> BTreeMap<String, BTreeMap<String, i64>> {
    BTreeMap::from([(
        "84".to_owned(),
        [1, 0, 0, 1, 1, 1, 4, 0]
            .into_iter()
            .enumerate()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    )])
}

const IDENTITY_INDICES: [usize; 3] = [3, 7, 11];
const DIRECT_PARAMETER_INDICES: [usize; 6] = [0, 1, 2, 3, 7, 11];
const LINEAR_BOUND: f64 = 1.0 / 8.0;
const QUADRATIC_BOUND: f64 = 1.0 / 16.0;

#[derive(Debug, Clone, PartialEq)]
pub struct AppleStyleDataFacts {
    pub value_count: usize,
    pub minimum: f32,
    pub maximum: f32,
    pub identity_residual_rmse: f64,
    pub identity_residual_maximum_absolute: f32,
    pub complete_identity: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppleStyleLinearMetadata {
    pub baseline_exposure: f32,
    pub baseline_exposure_unclamped: f32,
    pub highlight_compression_ratio: f32,
    pub base_gain: f32,
    pub encoding_gain: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppleStyleGlobalToneCurve {
    pub data: Vec<u8>,
    pub linear_samples: Vec<f32>,
    pub fit_rmse: f64,
    pub populated_bins: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppleStyleDataError {
    InvalidLength {
        actual: usize,
        expected: usize,
    },
    InvalidCoefficientCount {
        actual: usize,
        expected: usize,
    },
    InvalidRasterPair,
    InvalidBasisInput,
    NonFiniteCoefficient {
        index: usize,
    },
    CoefficientOutOfBounds {
        index: usize,
        value: f64,
        bound: f64,
    },
    NonFiniteValue {
        index: usize,
    },
    InvalidRgbRaster {
        actual: usize,
    },
    InvalidThumbnailExposure,
    InvalidRefinementParameter {
        actual: usize,
        expected: usize,
    },
    InvalidRefinementStep,
    InvalidScalarRow {
        index: usize,
    },
    InvalidJacobianSampleCount {
        actual: usize,
        expected: usize,
    },
    InvalidSceneScore {
        class: AppleStyleSceneClass,
        value: f64,
    },
    InvalidLightMapInput,
    InvalidLightMapSample {
        index: usize,
    },
    InvalidOrientation(u8),
    NonFiniteLightMapValue {
        index: usize,
    },
    InvalidPropertyListInput,
    PropertyListOverflow,
    SingularSystem,
}

impl fmt::Display for AppleStyleDataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength { actual, expected } => write!(
                formatter,
                "Apple Photographic Styles data has {actual} bytes; expected {expected}"
            ),
            Self::InvalidCoefficientCount { actual, expected } => write!(
                formatter,
                "Apple Photographic Styles coefficient vector has {actual} values; expected {expected}"
            ),
            Self::InvalidRasterPair => formatter.write_str(
                "Apple Photographic Styles polynomial fit requires matching finite RGB rasters",
            ),
            Self::InvalidBasisInput => {
                formatter.write_str("Apple Photographic Styles polynomial basis input is not finite")
            }
            Self::NonFiniteCoefficient { index } => write!(
                formatter,
                "Apple Photographic Styles coefficient {index} is not finite"
            ),
            Self::CoefficientOutOfBounds { index, value, bound } => write!(
                formatter,
                "Apple Photographic Styles coefficient {index} value {value} exceeds bound {bound}"
            ),
            Self::NonFiniteValue { index } => write!(
                formatter,
                "Apple Photographic Styles Float16 value {index} is not finite"
            ),
            Self::InvalidRgbRaster { actual } => write!(
                formatter,
                "Apple Photographic Styles RGB raster has {actual} values; expected a non-empty multiple of 3"
            ),
            Self::InvalidThumbnailExposure => formatter.write_str(
                "Apple Photographic Styles thumbnail exposure must be finite and positive",
            ),
            Self::InvalidRefinementParameter { actual, expected } => write!(
                formatter,
                "Apple Photographic Styles refinement parameter {actual} is outside 0..{expected}"
            ),
            Self::InvalidRefinementStep => formatter.write_str(
                "Apple Photographic Styles refinement Jacobian step must be finite and non-zero",
            ),
            Self::InvalidScalarRow { index } => write!(
                formatter,
                "Apple Photographic Styles scalar constraint row {index} is invalid"
            ),
            Self::InvalidJacobianSampleCount { actual, expected } => write!(
                formatter,
                "Apple Photographic Styles sampled Jacobian has {actual} values; expected {expected}"
            ),
            Self::InvalidSceneScore { class, value } => write!(
                formatter,
                "Apple Photographic Styles {class:?} scene score {value} is outside 0 through 1"
            ),
            Self::InvalidLightMapInput => formatter.write_str(
                "Apple Photographic Styles light-map input dimensions or numeric bounds are invalid",
            ),
            Self::InvalidLightMapSample { index } => write!(
                formatter,
                "Apple Photographic Styles light-map sample {index} is not finite"
            ),
            Self::InvalidOrientation(value) => {
                write!(formatter, "Apple Photographic Styles orientation {value} is invalid")
            }
            Self::NonFiniteLightMapValue { index } => write!(
                formatter,
                "Apple Photographic Styles serialized light-map value {index} is not finite"
            ),
            Self::InvalidPropertyListInput => {
                formatter.write_str("Apple Photographic Styles property-list input is invalid")
            }
            Self::PropertyListOverflow => {
                formatter.write_str("Apple Photographic Styles property-list size overflows")
            }
            Self::SingularSystem => {
                formatter.write_str("Apple Photographic Styles polynomial system is singular")
            }
        }
    }
}

impl Error for AppleStyleDataError {}

/// Apple Log constants from the macOS 26A434 exponential decoder.
///
/// `_appleLogToLinear` and `sw_appleLogToLinear` always take this branch.
/// The second compare in the CI kernel tests the constant positive toe
/// threshold, so the quadratic toe is never selected. `G` in
/// `L = (2^((code - C) / A) - B) / G` is 1 for the transfer itself. Styles
/// still divides by `baseline_exposure` before encoding; that division is not
/// a second `G`.
#[allow(clippy::excessive_precision)]
pub const APPLE_LOG_A: f64 = 0.085_504_792_630_672_45;
#[allow(clippy::excessive_precision)]
pub const APPLE_LOG_B: f64 = 0.009_640_520_438_551_903;
#[allow(clippy::excessive_precision)]
pub const APPLE_LOG_C: f64 = 0.693_369_448_184_967;

/// Encode one linear sample as an Apple Log code in \[0, 1\].
///
/// This is the exact inverse of [`apple_log_decode`]. Values that would fall
/// below code 0, including a non-positive `linear + B`, clamp to 0. Values
/// above the code-1 linear value clamp to 1. The camera-side encoder toe is
/// unverified and is not applied: the macOS consumer decoder does not invert
/// it.
pub fn apple_log_encode(linear: f64) -> f64 {
    let lifted = linear + APPLE_LOG_B;
    if lifted.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        return 0.0;
    }
    (APPLE_LOG_A * lifted.log2() + APPLE_LOG_C).clamp(0.0, 1.0)
}

/// Decode one Apple Log code with the macOS exponential branch.
///
/// `L = (2^((code - C) / A) - B) / G` with `G = 1`. Codes outside the stored
/// \[0, 1\] range are clamped before the exponential, which is the code-range
/// limit of this transfer. The CI kernel also clamps the linear result to
/// caller-supplied bounds; those bounds are not separate numeric immediates
/// of this decoder, and clamping the code keeps the result inside the range
/// the encoder can store.
pub fn apple_log_decode(code: f64) -> f64 {
    let code = code.clamp(0.0, 1.0);
    2.0_f64.powf((code - APPLE_LOG_C) / APPLE_LOG_A) - APPLE_LOG_B
}

/// Encode the reconstructed HDR reference for the Styles Linear Thumbnail.
///
/// Divide each linear Display P3 channel by plist key 4, then apply Apple Log.
/// The result is a 16-bit code in \[0, 65535\]. The input remains in linear
/// Display P3; this does not convert it to the BT.2020 gamut of Apple Log
/// video, and it does not replace Apple Log with sRGB, PQ, HLG, or linear.
/// `encoding_gain` belongs to the renderer metadata and must not be applied a
/// second time here.
///
/// Quantize to 16-bit only after the log curve so the Main10 encoder receives
/// the Apple Log codes directly. An 8-bit quantization would leave most of the
/// 10-bit HEVC range unused. HDR values above SDR white still survive.
pub fn apple_style_linear_thumbnail_codes(
    hdr_rgb: &[f32],
    baseline_exposure: f32,
) -> Result<Vec<u16>, AppleStyleDataError> {
    if hdr_rgb.is_empty() || !hdr_rgb.len().is_multiple_of(3) {
        return Err(AppleStyleDataError::InvalidRgbRaster {
            actual: hdr_rgb.len(),
        });
    }
    if !baseline_exposure.is_finite() || baseline_exposure <= 0.0 {
        return Err(AppleStyleDataError::InvalidThumbnailExposure);
    }
    hdr_rgb
        .iter()
        .enumerate()
        .map(|(index, &value)| {
            if !value.is_finite() {
                return Err(AppleStyleDataError::NonFiniteValue { index });
            }
            // Float64 division also prevents finite f32 HDR/exposure pairs
            // from overflowing before the log compresses them.
            let linear = f64::from(value) / f64::from(baseline_exposure);
            let code = apple_log_encode(linear);
            Ok((code * 65_535.0).round() as u16)
        })
        .collect()
}

/// Build the verified identity key-1 payload used by CMImaging.
pub fn apple_style_identity_data() -> Vec<u8> {
    let mut block =
        Vec::with_capacity(APPLE_STYLE_BLOCK_VALUE_COUNT * APPLE_STYLE_VALUE_BYTE_COUNT);
    for index in 0..APPLE_STYLE_BLOCK_VALUE_COUNT {
        let value = if IDENTITY_INDICES.contains(&index) {
            1.0
        } else {
            0.0
        };
        block.extend_from_slice(&f16::from_f32(value).to_le_bytes());
    }

    let mut result = Vec::with_capacity(APPLE_STYLE_BYTE_COUNT);
    for _ in 0..APPLE_STYLE_TILE_COUNT {
        result.extend_from_slice(&block);
    }
    result
}

/// Build the protocol identity global-tone-curve resource.
///
/// The curve is the 257-entry little-endian table used by the established
/// Apple producer: a two-byte count followed by 256 sRGB-encoded samples and
/// one terminal sample. It is a portable resource construction rule, so it
/// belongs beside the rest of the Rust-owned Styles metadata policy rather
/// than in the Apple adapter.
pub fn apple_style_identity_global_tone_curve() -> Vec<u8> {
    let mut output = Vec::with_capacity(516);
    output.extend_from_slice(&257_u16.to_le_bytes());
    for index in 0..256 {
        let linear = f64::from(index) / 255.0;
        let encoded = if linear <= 0.003_130_8 {
            linear * 12.92
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        };
        let value = (encoded.clamp(0.0, 1.0) * 65_534.0).round() as u16;
        let value = match index {
            0 => 0,
            255 => 65_534,
            _ => value,
        };
        output.extend_from_slice(&value.to_le_bytes());
    }
    output.extend_from_slice(&65_534_u16.to_le_bytes());
    output
}

/// Derive the per-photo Linear Thumbnail metadata from the same-photo Base
/// and reconstructed HDR luminance distributions.
///
/// The constants are the bounded behavior-equivalent calibration already
/// documented by the producer contract. They describe a source-derived
/// proxy for the unavailable camera-time LTMDigitalGain; they are not a donor
/// image or a scene-independent identity fallback.
#[allow(clippy::excessive_precision)]
pub fn apple_style_linear_metadata(
    base_luminance: &[f32],
    hdr_luminance: &[f32],
    gain_map_maximum_stops: f32,
) -> Result<AppleStyleLinearMetadata, AppleStyleDataError> {
    if base_luminance.is_empty()
        || base_luminance.len() != hdr_luminance.len()
        || !base_luminance.iter().all(|value| value.is_finite())
        || !hdr_luminance.iter().all(|value| value.is_finite())
        || !gain_map_maximum_stops.is_finite()
    {
        return Err(AppleStyleDataError::InvalidRasterPair);
    }
    let base_p98 = apple_style_distribution(base_luminance).p98 as f32;
    let hdr_p98 = apple_style_distribution(hdr_luminance).p98 as f32;
    if !base_p98.is_finite() || !hdr_p98.is_finite() || base_p98 <= 1.0 / 4096.0 || hdr_p98 <= 0.0 {
        return Err(AppleStyleDataError::InvalidLightMapInput);
    }
    let highlight_compression_ratio = hdr_p98 / base_p98;
    let baseline_unclamped = highlight_compression_ratio / 0.401_264_06;
    let baseline_clamped = baseline_unclamped.clamp(4.0, 10.4);
    let baseline_exposure = (baseline_clamped * 65.0).round() / 65.0;
    let base_gain = (1.512_718_43
        + 0.156_706_32 * gain_map_maximum_stops
        + 0.147_667_24 * highlight_compression_ratio)
        .clamp(0.5, 2.5);
    Ok(AppleStyleLinearMetadata {
        baseline_exposure,
        baseline_exposure_unclamped: baseline_unclamped,
        highlight_compression_ratio,
        base_gain,
        encoding_gain: 4.0 * base_gain,
    })
}

/// Build the shared per-channel global tone curve used by Styles key 3.
///
/// `PISemanticStyleFilter` samples one 256-node curve independently on each
/// channel. The x value is `min(E · S, 1)`, where `S` is the linear thumbnail
/// after its base gain. These arrays are that code and the displayed primary
/// channel, interleaved RGB, with no luminance reduction and no percentile
/// normalization on either axis. `E` is the consumer's extra exposure and is
/// not a source fact; callers pass the `S` they will hand to the consumer.
///
/// Empty bins are filled from neighboring observed means. The monotone
/// projection is pool-adjacent-violators on those means. There is no parametric
/// family and no forced `(0, 0)` or `(1, 1)` endpoint. The 516-byte wire layout
/// is unchanged: a 257 count, 256 inverse-sRGB nodes on the existing 65534
/// scale, and one repeated final node.
pub fn apple_style_monotonic_global_tone_curve(
    thumbnail_rgb: &[f32],
    primary_rgb: &[f32],
) -> Result<AppleStyleGlobalToneCurve, AppleStyleDataError> {
    if thumbnail_rgb.is_empty()
        || thumbnail_rgb.len() != primary_rgb.len()
        || !thumbnail_rgb
            .len()
            .is_multiple_of(APPLE_STYLE_CHANNEL_COUNT)
        || thumbnail_rgb.iter().any(|value| !value.is_finite())
        || primary_rgb.iter().any(|value| !value.is_finite())
    {
        return Err(AppleStyleDataError::InvalidRasterPair);
    }

    const SAMPLE_COUNT: usize = 256;
    let mut sums = vec![0.0_f64; SAMPLE_COUNT];
    let mut counts = vec![0.0_f64; SAMPLE_COUNT];
    for (&input, &output) in thumbnail_rgb.iter().zip(primary_rgb) {
        let code = input.clamp(0.0, 1.0);
        let target = output.clamp(0.0, 1.0);
        let index = gtc_node_index(code);
        sums[index] += f64::from(target);
        counts[index] += 1.0;
    }
    let populated_bins = counts.iter().filter(|count| **count > 0.0).count();
    if populated_bins == 0 {
        return Err(AppleStyleDataError::InvalidRasterPair);
    }
    let values = interpolate_observed_bins(&sums, &counts);
    let fitted = isotonic_curve(&values, &counts)?;

    let mut squared_error = 0.0_f64;
    for (&input, &output) in thumbnail_rgb.iter().zip(primary_rgb) {
        let predicted = f64::from(sample_gtc_linear(&fitted, input));
        let expected = f64::from(output.clamp(0.0, 1.0));
        let error = predicted - expected;
        squared_error += error * error;
    }
    let finite_count = thumbnail_rgb.len();

    let mut data = Vec::with_capacity(516);
    data.extend_from_slice(&257_u16.to_le_bytes());
    for linear in &fitted {
        data.extend_from_slice(&encode_gtc_node(f64::from(*linear)).to_le_bytes());
    }
    let last = *fitted.last().unwrap_or(&0.0);
    data.extend_from_slice(&encode_gtc_node(f64::from(last)).to_le_bytes());
    Ok(AppleStyleGlobalToneCurve {
        data,
        linear_samples: fitted,
        fit_rmse: (squared_error / finite_count.max(1) as f64).sqrt(),
        populated_bins,
    })
}

fn gtc_node_index(code: f32) -> usize {
    let scaled = f16::from_f32(code.clamp(0.0, 1.0) * 255.0).to_f32();
    scaled.round().clamp(0.0, 255.0) as usize
}

fn encode_gtc_node(linear: f64) -> u16 {
    let encoded = srgb_encode(linear.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    (encoded * 65_534.0).round().clamp(0.0, 65_534.0) as u16
}

fn interpolate_observed_bins(sums: &[f64], counts: &[f64]) -> Vec<f64> {
    let populated = counts
        .iter()
        .enumerate()
        .filter_map(|(index, count)| (*count > 0.0).then_some(index))
        .collect::<Vec<_>>();
    let mut values = vec![0.0_f64; sums.len()];
    for (index, value) in values.iter_mut().enumerate() {
        if counts[index] > 0.0 {
            *value = sums[index] / counts[index];
            continue;
        }
        let lower = populated
            .iter()
            .copied()
            .rev()
            .find(|candidate| *candidate < index);
        let upper = populated
            .iter()
            .copied()
            .find(|candidate| *candidate > index);
        *value = match (lower, upper) {
            (Some(left), Some(right)) => {
                let fraction = (index - left) as f64 / (right - left) as f64;
                let left_value = sums[left] / counts[left];
                let right_value = sums[right] / counts[right];
                left_value * (1.0 - fraction) + right_value * fraction
            }
            (Some(left), None) => sums[left] / counts[left],
            (None, Some(right)) => sums[right] / counts[right],
            (None, None) => 0.0,
        };
    }
    values
}

fn isotonic_curve(values: &[f64], counts: &[f64]) -> Result<Vec<f32>, AppleStyleDataError> {
    #[derive(Clone, Copy)]
    struct Block {
        lower: usize,
        upper: usize,
        weight: f64,
        weighted_value: f64,
    }
    impl Block {
        fn mean(self) -> f64 {
            self.weighted_value / self.weight.max(f64::MIN_POSITIVE)
        }
    }
    let mut blocks = Vec::with_capacity(values.len());
    for (index, &value) in values.iter().enumerate() {
        let weight = if counts[index] > 0.0 {
            counts[index]
        } else {
            1.0
        };
        blocks.push(Block {
            lower: index,
            upper: index,
            weight,
            weighted_value: weight * value.clamp(0.0, 1.0),
        });
        while blocks.len() >= 2 {
            let right = blocks[blocks.len() - 1];
            let left = blocks[blocks.len() - 2];
            if left.mean() <= right.mean() {
                break;
            }
            blocks.pop();
            let mut merged = blocks.pop().ok_or(AppleStyleDataError::SingularSystem)?;
            merged.upper = right.upper;
            merged.weight += right.weight;
            merged.weighted_value += right.weighted_value;
            blocks.push(merged);
        }
    }
    let mut fitted = vec![0.0_f32; values.len()];
    for block in blocks {
        let value = block.mean().clamp(0.0, 1.0) as f32;
        fitted[block.lower..=block.upper].fill(value);
    }
    for index in 1..fitted.len() {
        fitted[index] = fitted[index].max(fitted[index - 1]);
    }
    Ok(fitted)
}

/// Sample the 256 linear GTC nodes with the 26A434 half coordinate.
///
/// The consumer converts `code * 255` to half, promotes it back to float, and
/// adds `0.5` before a clamp-to-edge texture fetch. Texel centers sit on that
/// half-pixel grid, so the interpolation coordinate is the half-quantized code.
pub fn sample_gtc_linear(samples: &[f32], code: f32) -> f32 {
    if samples.len() != 256 {
        return 0.0;
    }
    let scaled = f16::from_f32(code.clamp(0.0, 1.0) * 255.0)
        .to_f32()
        .clamp(0.0, 255.0);
    let lower = scaled.floor() as usize;
    let upper = (lower + 1).min(255);
    let fraction = scaled - lower as f32;
    samples[lower] * (1.0 - fraction) + samples[upper] * fraction
}

/// Apply the shared GTC to each channel code with the consumer's half coordinate.
pub fn apple_style_apply_global_tone_curve(
    codes: &[f32],
    samples: &[f32],
) -> Result<Vec<f32>, AppleStyleDataError> {
    if samples.len() != 256
        || !samples.iter().all(|value| value.is_finite())
        || !codes.iter().all(|value| value.is_finite())
    {
        return Err(AppleStyleDataError::InvalidRasterPair);
    }
    Ok(codes
        .iter()
        .map(|value| sample_gtc_linear(samples, *value))
        .collect())
}

fn srgb_encode(linear: f64) -> f64 {
    if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

/// Return a stable lowercase SHA-256 digest for a style payload.
pub fn apple_style_data_sha256(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Validate the fixed Float16 key-1 layout and report identity residuals.
pub fn validate_apple_style_data(data: &[u8]) -> Result<AppleStyleDataFacts, AppleStyleDataError> {
    if data.len() != APPLE_STYLE_BYTE_COUNT {
        return Err(AppleStyleDataError::InvalidLength {
            actual: data.len(),
            expected: APPLE_STYLE_BYTE_COUNT,
        });
    }

    let mut minimum = f32::INFINITY;
    let mut maximum = f32::NEG_INFINITY;
    let mut identity_squared_error = 0.0_f64;
    let mut identity_maximum_absolute = 0.0_f32;
    for value_index in 0..(data.len() / APPLE_STYLE_VALUE_BYTE_COUNT) {
        let offset = value_index * APPLE_STYLE_VALUE_BYTE_COUNT;
        let value = f16::from_le_bytes([data[offset], data[offset + 1]]).to_f32();
        if !value.is_finite() {
            return Err(AppleStyleDataError::NonFiniteValue { index: value_index });
        }
        minimum = minimum.min(value);
        maximum = maximum.max(value);
        let expected = if IDENTITY_INDICES.contains(&(value_index % APPLE_STYLE_BLOCK_VALUE_COUNT))
        {
            1.0_f32
        } else {
            0.0_f32
        };
        let error = value - expected;
        identity_squared_error += f64::from(error) * f64::from(error);
        identity_maximum_absolute = identity_maximum_absolute.max(error.abs());
    }

    Ok(AppleStyleDataFacts {
        value_count: data.len() / APPLE_STYLE_VALUE_BYTE_COUNT,
        minimum,
        maximum,
        identity_residual_rmse: (identity_squared_error
            / (data.len() / APPLE_STYLE_VALUE_BYTE_COUNT) as f64)
            .sqrt(),
        identity_residual_maximum_absolute: identity_maximum_absolute,
        complete_identity: data == apple_style_identity_data(),
    })
}

/// Build the polynomial basis used by the portable style solver.
pub fn apple_style_polynomial_basis(
    red: f32,
    green: f32,
    blue: f32,
) -> Result<[f32; APPLE_STYLE_POLYNOMIAL_COUNT], AppleStyleDataError> {
    if !red.is_finite() || !green.is_finite() || !blue.is_finite() {
        return Err(AppleStyleDataError::InvalidBasisInput);
    }
    Ok([
        1.0,
        red,
        green,
        blue,
        red * red,
        red * green,
        red * blue,
        green * green,
        green * blue,
        blue * blue,
    ])
}

/// Fit the bounded global quadratic style transform used to initialize the
/// constrained solver.
///
/// The renderer remains a platform primitive, but this policy is pure numeric
/// work: sample the RGB pair deterministically, run three Huber-weighted IRLS
/// passes, add the same trace-scaled ridge, and clamp each coefficient to the
/// verified key-1 bounds. Keeping it here means a future Apple adapter can
/// provide observations without owning a second solver.
pub fn apple_style_fit_global_polynomial(
    source_rgb8: &[f32],
    target_rgb8: &[f32],
) -> Result<Vec<f64>, AppleStyleDataError> {
    if source_rgb8.len() != target_rgb8.len()
        || source_rgb8.len() < APPLE_STYLE_CHANNEL_COUNT
        || !source_rgb8.len().is_multiple_of(APPLE_STYLE_CHANNEL_COUNT)
        || !source_rgb8.iter().all(|value| value.is_finite())
        || !target_rgb8.iter().all(|value| value.is_finite())
    {
        return Err(AppleStyleDataError::InvalidRasterPair);
    }

    let pixel_count = source_rgb8.len() / APPLE_STYLE_CHANNEL_COUNT;
    let pixel_stride = (pixel_count / 100_000).max(1);
    let sampled_pixel_count = (pixel_count - 1) / pixel_stride + 1;
    let basis_values = (0..sampled_pixel_count)
        .map(|sampled_pixel| {
            let source_offset = sampled_pixel * pixel_stride * APPLE_STYLE_CHANNEL_COUNT;
            let red = f64::from(source_rgb8[source_offset]) / 255.0;
            let green = f64::from(source_rgb8[source_offset + 1]) / 255.0;
            let blue = f64::from(source_rgb8[source_offset + 2]) / 255.0;
            [
                1.0,
                red,
                green,
                blue,
                red * red,
                red * green,
                red * blue,
                green * green,
                green * blue,
                blue * blue,
            ]
        })
        .collect::<Vec<_>>();

    let mut coefficients = vec![0.0_f64; APPLE_STYLE_BLOCK_VALUE_COUNT];
    for _ in 0..3 {
        for output in 0..APPLE_STYLE_CHANNEL_COUNT {
            let mut normal =
                vec![0.0_f64; APPLE_STYLE_POLYNOMIAL_COUNT * APPLE_STYLE_POLYNOMIAL_COUNT];
            let mut right_hand_side = vec![0.0_f64; APPLE_STYLE_POLYNOMIAL_COUNT];
            for (sampled_pixel, basis) in basis_values.iter().enumerate() {
                let source_offset = sampled_pixel * pixel_stride * APPLE_STYLE_CHANNEL_COUNT;
                let source_value = source_rgb8[source_offset + output];
                let target_value = target_rgb8[source_offset + output];
                let observed = f64::from(target_value - source_value) / 255.0;
                let predicted = basis
                    .iter()
                    .enumerate()
                    .map(|(term, value)| {
                        value * coefficients[term * APPLE_STYLE_CHANNEL_COUNT + output]
                    })
                    .sum::<f64>();
                let residual = observed - predicted;
                let huber_threshold = 4.0_f64 / 255.0;
                let mut weight = (huber_threshold / huber_threshold.max(residual.abs())).min(1.0);
                if source_value <= 2.0 || source_value >= 253.0 {
                    weight *= 0.25;
                }
                for row in 0..APPLE_STYLE_POLYNOMIAL_COUNT {
                    let row_value = basis[row];
                    right_hand_side[row] += weight * row_value * observed;
                    for column in row..APPLE_STYLE_POLYNOMIAL_COUNT {
                        normal[row * APPLE_STYLE_POLYNOMIAL_COUNT + column] +=
                            weight * row_value * basis[column];
                    }
                }
            }

            for row in 0..APPLE_STYLE_POLYNOMIAL_COUNT {
                for column in 0..row {
                    normal[row * APPLE_STYLE_POLYNOMIAL_COUNT + column] =
                        normal[column * APPLE_STYLE_POLYNOMIAL_COUNT + row];
                }
            }
            let trace = (0..APPLE_STYLE_POLYNOMIAL_COUNT)
                .map(|term| normal[term * APPLE_STYLE_POLYNOMIAL_COUNT + term])
                .sum::<f64>();
            let ridge = (trace / APPLE_STYLE_POLYNOMIAL_COUNT as f64 * 1e-5).max(1e-9);
            for term in 0..APPLE_STYLE_POLYNOMIAL_COUNT {
                normal[term * APPLE_STYLE_POLYNOMIAL_COUNT + term] +=
                    ridge * f64::from((term >= 4) as u8 * 9 + 1);
            }
            let solution = solve_linear_system(&normal, &right_hand_side)?;
            for (term, value) in solution.into_iter().enumerate() {
                let coefficient_index = term * APPLE_STYLE_CHANNEL_COUNT + output;
                let bound = coefficient_bound(coefficient_index);
                coefficients[coefficient_index] = value.clamp(-bound, bound);
            }
        }
    }
    Ok(coefficients)
}

/// Parameter-major finite-difference observations consumed by the constrained
/// style refinement solve.
///
/// The Apple renderer is allowed to produce the perturbed rasters, but it does
/// not own the sampling or solve policy. Keeping the sampled storage here also
/// preserves the bounded-memory contract: at most
/// `APPLE_STYLE_REFINEMENT_MAX_PIXELS` pixels are retained by the Jacobian.
#[derive(Debug, Clone, PartialEq)]
pub struct AppleStyleSampledJacobian {
    rgb_value_count: usize,
    pub pixel_stride: usize,
    pub sample_count: usize,
    values: Vec<f32>,
}

impl AppleStyleSampledJacobian {
    /// Allocate the exact sample grid used by the refinement solver.
    pub fn new(rgb_value_count: usize) -> Result<Self, AppleStyleDataError> {
        if rgb_value_count < 3 || !rgb_value_count.is_multiple_of(3) {
            return Err(AppleStyleDataError::InvalidRgbRaster {
                actual: rgb_value_count,
            });
        }
        let pixel_count = rgb_value_count / 3;
        let pixel_stride = (pixel_count / APPLE_STYLE_REFINEMENT_MAX_PIXELS).max(1);
        let sampled_pixel_count = (pixel_count - 1) / pixel_stride + 1;
        let sample_count = sampled_pixel_count * 3;
        Ok(Self {
            rgb_value_count,
            pixel_stride,
            sample_count,
            values: vec![0.0; sample_count * APPLE_STYLE_REFINEMENT_PARAMETER_COUNT],
        })
    }

    /// Populate one parameter column from a renderer perturbation and return
    /// its sampled derivative RMS in the renderer's native 8-bit domain.
    pub fn populate_parameter(
        &mut self,
        parameter: usize,
        rendered_rgb: &[f32],
        current_rgb: &[f32],
        step: f64,
    ) -> Result<f64, AppleStyleDataError> {
        if parameter >= APPLE_STYLE_REFINEMENT_PARAMETER_COUNT {
            return Err(AppleStyleDataError::InvalidRefinementParameter {
                actual: parameter,
                expected: APPLE_STYLE_REFINEMENT_PARAMETER_COUNT,
            });
        }
        if rendered_rgb.len() != current_rgb.len()
            || rendered_rgb.len() != self.rgb_value_count
            || rendered_rgb.len() < 3
            || !rendered_rgb.len().is_multiple_of(3)
            || !rendered_rgb.iter().all(|value| value.is_finite())
            || !current_rgb.iter().all(|value| value.is_finite())
        {
            return Err(AppleStyleDataError::InvalidRasterPair);
        }
        if !step.is_finite() || step == 0.0 {
            return Err(AppleStyleDataError::InvalidRefinementStep);
        }
        let float_step = step as f32;
        if !float_step.is_finite() || float_step == 0.0 {
            return Err(AppleStyleDataError::InvalidRefinementStep);
        }

        let pixel_count = current_rgb.len() / 3;
        let parameter_base = parameter * self.sample_count;
        let mut sampled = 0;
        let mut squared = 0.0_f64;
        for pixel in (0..pixel_count).step_by(self.pixel_stride) {
            let base = pixel * 3;
            for channel in 0..3 {
                let derivative =
                    (rendered_rgb[base + channel] - current_rgb[base + channel]) / float_step;
                self.values[parameter_base + sampled] = derivative;
                squared += f64::from(derivative) * f64::from(derivative);
                sampled += 1;
            }
        }
        if sampled != self.sample_count {
            return Err(AppleStyleDataError::InvalidJacobianSampleCount {
                actual: sampled,
                expected: self.sample_count,
            });
        }
        Ok((squared / self.sample_count.max(1) as f64).sqrt())
    }

    fn derivative(&self, parameter: usize, sample: usize) -> f32 {
        self.values[parameter * self.sample_count + sample]
    }
}

/// One optional scalar response constraint added to the sampled RGB normal
/// equations. The fixed-size derivative prevents a caller from supplying a
/// second, differently-sized solver parameter vector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppleStyleScalarRow {
    pub derivative: [f64; APPLE_STYLE_REFINEMENT_PARAMETER_COUNT],
    pub residual: f64,
    pub weight: f64,
}

/// Solve one bounded constrained-polynomial refinement update.
///
/// This is the policy half of the constrained refinement contract. A
/// platform adapter may render the finite-difference candidates and report
/// scalar observations, while Rust owns deterministic sampling, Huber weights,
/// normalization, ridge regularization, linear solving, and update bounds.
pub fn apple_style_solve_refinement_update(
    current_rgb: &[f32],
    target_rgb: &[f32],
    jacobian: &AppleStyleSampledJacobian,
    scalar_rows: &[AppleStyleScalarRow],
) -> Result<Vec<f64>, AppleStyleDataError> {
    if current_rgb.len() != target_rgb.len()
        || current_rgb.len() < 3
        || !current_rgb.len().is_multiple_of(3)
        || !current_rgb.iter().all(|value| value.is_finite())
        || !target_rgb.iter().all(|value| value.is_finite())
    {
        return Err(AppleStyleDataError::InvalidRasterPair);
    }
    if jacobian.rgb_value_count != current_rgb.len() {
        return Err(AppleStyleDataError::InvalidRasterPair);
    }
    let sampled_pixel_count = (current_rgb.len() / 3 - 1) / jacobian.pixel_stride + 1;
    let expected_sample_count = sampled_pixel_count * 3;
    if jacobian.sample_count != expected_sample_count {
        return Err(AppleStyleDataError::InvalidJacobianSampleCount {
            actual: jacobian.sample_count,
            expected: expected_sample_count,
        });
    }
    for (index, row) in scalar_rows.iter().enumerate() {
        if !row.derivative.iter().all(|value| value.is_finite())
            || !row.residual.is_finite()
            || !row.weight.is_finite()
            || row.weight < 0.0
        {
            return Err(AppleStyleDataError::InvalidScalarRow { index });
        }
    }

    let count = APPLE_STYLE_REFINEMENT_PARAMETER_COUNT;
    let mut normal = vec![0.0_f64; count * count];
    let mut gradient = vec![0.0_f64; count];
    let mut sample_count = 0_usize;
    let mut sample_ordinal = 0_usize;
    for pixel in (0..current_rgb.len() / 3).step_by(jacobian.pixel_stride) {
        for channel in 0..3 {
            let sample = pixel * 3 + channel;
            let residual = f64::from(target_rgb[sample] - current_rgb[sample]);
            let huber_weight = (12.0 / 12.0_f64.max(residual.abs())).min(1.0);
            sample_count += 1;
            for row in 0..count {
                let row_value = f64::from(jacobian.derivative(row, sample_ordinal));
                gradient[row] += huber_weight * row_value * residual;
                for column in row..count {
                    normal[row * count + column] += huber_weight
                        * row_value
                        * f64::from(jacobian.derivative(column, sample_ordinal));
                }
            }
            sample_ordinal += 1;
        }
    }
    if sample_ordinal != jacobian.sample_count {
        return Err(AppleStyleDataError::InvalidJacobianSampleCount {
            actual: sample_ordinal,
            expected: jacobian.sample_count,
        });
    }

    // Mean-scale the pixel block so scalar-response rows have
    // sample-count-independent weights. The pure pixel solution is unchanged
    // because the linear system is scale invariant.
    if sample_count > 0 {
        let normalization = 1.0 / sample_count as f64;
        for row in 0..count {
            gradient[row] *= normalization;
            for column in row..count {
                normal[row * count + column] *= normalization;
            }
        }
    }
    for scalar in scalar_rows {
        for row in 0..count {
            gradient[row] += scalar.weight * scalar.derivative[row] * scalar.residual;
            for column in row..count {
                normal[row * count + column] +=
                    scalar.weight * scalar.derivative[row] * scalar.derivative[column];
            }
        }
    }
    for row in 0..count {
        for column in 0..row {
            normal[row * count + column] = normal[column * count + row];
        }
    }
    let trace = (0..count)
        .map(|index| normal[index * count + index])
        .sum::<f64>();
    let ridge = (trace / count as f64 * 1e-6).max(1e-9);
    for index in 0..count {
        normal[index * count + index] += ridge;
    }
    let mut solution = solve_linear_system(&normal, &gradient)?;
    for value in &mut solution {
        *value = value.clamp(
            -APPLE_STYLE_REFINEMENT_EPSILON,
            APPLE_STYLE_REFINEMENT_EPSILON,
        );
    }
    Ok(solution)
}

/// The four public Vision score buckets consumed by Apple's scene-type policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppleStyleSceneClass {
    Food,
    Sunset,
    Indoor,
    Outdoor,
}

/// Vision observations passed from a platform primitive to the Rust scene
/// policy. Scores are confidence values and therefore must already be in 0..1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppleStyleSceneScores {
    pub food: f64,
    pub sunset: f64,
    pub indoor: f64,
    pub outdoor: f64,
}

/// The product-level scene choice used in the style property-list contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppleStyleSceneDecision {
    pub scene_type: u8,
    pub selected_class: Option<AppleStyleSceneClass>,
    pub native_default_applied: bool,
}

/// Resolve the current iPhone scene-type priority in Rust.
///
/// Vision classification is a platform observation. The thresholds and
/// priority are product policy and remain portable here: food, sunset, indoor,
/// outdoor, then the native default scene type 0.
pub fn resolve_apple_style_scene_type(
    scores: AppleStyleSceneScores,
) -> Result<AppleStyleSceneDecision, AppleStyleDataError> {
    for (class, value) in [
        (AppleStyleSceneClass::Food, scores.food),
        (AppleStyleSceneClass::Sunset, scores.sunset),
        (AppleStyleSceneClass::Indoor, scores.indoor),
        (AppleStyleSceneClass::Outdoor, scores.outdoor),
    ] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(AppleStyleDataError::InvalidSceneScore { class, value });
        }
    }

    let (scene_type, selected_class) = if scores.food >= 0.08 {
        (1, Some(AppleStyleSceneClass::Food))
    } else if scores.sunset >= 0.08 {
        (3, Some(AppleStyleSceneClass::Sunset))
    } else if scores.indoor >= 0.15 {
        (0, Some(AppleStyleSceneClass::Indoor))
    } else if scores.outdoor >= 0.15 {
        (2, Some(AppleStyleSceneClass::Outdoor))
    } else {
        (0, None)
    };
    Ok(AppleStyleSceneDecision {
        scene_type,
        selected_class,
        native_default_applied: selected_class.is_none(),
    })
}

/// Derive the source-dependent face exposure boost used by the style plist.
/// Missing or non-positive medians deliberately resolve to the neutral value;
/// the caller still has to provide a credible person observation.
pub fn apple_style_face_exposure_boost(
    global_median: f64,
    person_median: f64,
    has_credible_person: bool,
) -> Result<f64, AppleStyleDataError> {
    if !global_median.is_finite()
        || !person_median.is_finite()
        || global_median < 0.0
        || person_median < 0.0
    {
        return Err(AppleStyleDataError::InvalidLightMapInput);
    }
    if has_credible_person && global_median > 0.0 && person_median > 0.0 {
        Ok((global_median / person_median).sqrt().clamp(1.0, 2.5))
    } else {
        Ok(1.0)
    }
}

/// Typed source for one 32×32 style control map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppleStyleLightMapRequest<'a> {
    pub luma: &'a [f32],
    pub width: usize,
    pub height: usize,
    pub storage_orientation: u8,
}

/// Build one 32×32 Float16 control map in primary-item storage order.
///
/// The stored value is the area mean of the guide in that cell. The 26A434
/// consumer upsamples `c` and `d` with its own guided filter (`r = 1`,
/// `eps ≈ 0.05`) and then forms `q = mix(mix(d, c, l_b), mix(d, c, l_f), m)`.
/// This map is not that upsample and it is not multiplied onto luminance.
/// No scale or offset is applied: those constants are not source facts, and
/// the later functions that consume `q` are not inverted here.
pub fn apple_style_light_map(
    request: &AppleStyleLightMapRequest<'_>,
) -> Result<Vec<u8>, AppleStyleDataError> {
    let expected_count = request
        .width
        .checked_mul(request.height)
        .ok_or(AppleStyleDataError::InvalidLightMapInput)?;
    if request.width == 0
        || request.height == 0
        || request.luma.len() != expected_count
        || !(1..=8).contains(&request.storage_orientation)
    {
        if !(1..=8).contains(&request.storage_orientation) {
            return Err(AppleStyleDataError::InvalidOrientation(
                request.storage_orientation,
            ));
        }
        return Err(AppleStyleDataError::InvalidLightMapInput);
    }
    if let Some(index) = request.luma.iter().position(|value| !value.is_finite()) {
        return Err(AppleStyleDataError::InvalidLightMapSample { index });
    }

    let side = APPLE_STYLE_LIGHT_MAP_SIDE;
    let mut presentation = vec![f16::from_f32(0.0); side * side];
    for target_y in 0..side {
        let y0 = target_y * request.height / side;
        let y1 = (target_y + 1)
            .checked_mul(request.height)
            .ok_or(AppleStyleDataError::InvalidLightMapInput)?
            / side;
        let y1 = y1.max(y0 + 1).min(request.height);
        for target_x in 0..side {
            let x0 = target_x * request.width / side;
            let x1 = (target_x + 1)
                .checked_mul(request.width)
                .ok_or(AppleStyleDataError::InvalidLightMapInput)?
                / side;
            let x1 = x1.max(x0 + 1).min(request.width);
            let mut sum = 0.0_f64;
            let mut count = 0_usize;
            for y in y0..y1 {
                for x in x0..x1 {
                    sum += f64::from(request.luma[y * request.width + x]);
                    count += 1;
                }
            }
            let average = (sum / count as f64) as f32;
            let encoded = f16::from_f32(average);
            if !encoded.to_f32().is_finite() {
                return Err(AppleStyleDataError::NonFiniteLightMapValue {
                    index: target_y * side + target_x,
                });
            }
            presentation[target_y * side + target_x] = encoded;
        }
    }

    let mut output = Vec::with_capacity(APPLE_STYLE_LIGHT_MAP_BYTE_COUNT);
    for storage_y in 0..side {
        for storage_x in 0..side {
            let (display_x, display_y) = match request.storage_orientation {
                1 => (storage_x, storage_y),
                2 => (side - 1 - storage_x, storage_y),
                3 => (side - 1 - storage_x, side - 1 - storage_y),
                4 => (storage_x, side - 1 - storage_y),
                5 => (storage_y, storage_x),
                6 => (side - 1 - storage_y, storage_x),
                7 => (side - 1 - storage_y, side - 1 - storage_x),
                8 => (storage_y, side - 1 - storage_x),
                _ => unreachable!("orientation was validated above"),
            };
            output.extend_from_slice(&presentation[display_y * side + display_x].to_le_bytes());
        }
    }
    Ok(output)
}

/// The nine-number distribution stored for each style statistics channel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppleStyleDistribution {
    pub black_point: f64,
    pub high_key: f64,
    pub p02: f64,
    pub p10: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p98: f64,
    pub white_point: f64,
}

/// Compute the nine-number distribution stored by the Styles producer.
///
/// The empty-input values intentionally match the established Apple
/// producer: `highKey` is one while all other fields are zero. Keeping this
/// behavior in the engine means every statistics consumer uses one portable
/// percentile implementation rather than a runtime-local approximation.
pub fn apple_style_distribution(values: &[f32]) -> AppleStyleDistribution {
    let mut sorted = values
        .iter()
        .filter_map(|value| value.is_finite().then_some(f64::from(*value)))
        .collect::<Vec<_>>();
    if sorted.is_empty() {
        return AppleStyleDistribution {
            black_point: 0.0,
            high_key: 1.0,
            p02: 0.0,
            p10: 0.0,
            p25: 0.0,
            p50: 0.0,
            p75: 0.0,
            p98: 0.0,
            white_point: 0.0,
        };
    }
    sorted.sort_by(f64::total_cmp);
    let percentile = |percent: f64| {
        let position = (percent.clamp(0.0, 100.0) / 100.0) * (sorted.len() - 1) as f64;
        let lower = position.floor() as usize;
        let upper = position.ceil() as usize;
        if lower == upper {
            sorted[lower]
        } else {
            let fraction = position - lower as f64;
            sorted[lower] * (1.0 - fraction) + sorted[upper] * fraction
        }
    };
    AppleStyleDistribution {
        black_point: percentile(0.5),
        high_key: percentile(95.0),
        p02: percentile(2.0),
        p10: percentile(10.0),
        p25: percentile(25.0),
        p50: percentile(50.0),
        p75: percentile(75.0),
        p98: percentile(98.0),
        white_point: percentile(99.5),
    }
}

/// The fixed statistics dictionary consumed by the Apple style property-list
/// contract. An explicit field for every producer channel makes omissions
/// impossible at the call site and keeps the metadata schema in Rust.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppleStyleStatistics {
    pub linear_gtc_image: AppleStyleDistribution,
    pub linear_image: AppleStyleDistribution,
    pub linear_image_person_segment_based: AppleStyleDistribution,
    pub linear_image_skin_based: AppleStyleDistribution,
    pub tone_mapped_image: AppleStyleDistribution,
    pub tone_mapped_image_blue_channel_skin_based: AppleStyleDistribution,
    pub tone_mapped_image_green_channel_skin_based: AppleStyleDistribution,
    pub tone_mapped_image_person_segment_based: AppleStyleDistribution,
    pub tone_mapped_image_red_channel_skin_based: AppleStyleDistribution,
    pub tone_mapped_image_skin_based: AppleStyleDistribution,
}

/// Typed input for the version-15 Apple Photographic Styles metadata object.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppleStylePropertyListRequest<'a> {
    pub style_data: &'a [u8],
    pub global_tone_curve: &'a [u8],
    pub baseline_exposure: f64,
    pub scene_type: u8,
    pub statistics: &'a AppleStyleStatistics,
    pub people_ratio: f64,
    pub person_masks_valid_hint: f64,
    pub skin_ratio: f64,
    pub tone_light_map: &'a [u8],
    pub linear_light_map: &'a [u8],
    pub base_gain: f64,
    pub linear_gain: f64,
    pub original_range_min: f64,
    pub original_range_max: f64,
    pub face_exposure_boost: f64,
}

/// Synthesize the binary property list carried by the Styles MIME item.
///
/// `PropertyListSerialization` is a platform primitive for consuming this
/// object, not a product-policy boundary. Rust validates every fixed-size
/// resource, constructs the complete key schema, and emits a deterministic
/// binary plist without delegating metadata synthesis to the platform adapter.
pub fn apple_style_property_list(
    request: &AppleStylePropertyListRequest<'_>,
) -> Result<Vec<u8>, AppleStyleDataError> {
    validate_apple_style_data(request.style_data)?;
    if request.global_tone_curve.len() != 516
        || request.tone_light_map.len() != APPLE_STYLE_LIGHT_MAP_BYTE_COUNT
        || request.linear_light_map.len() != APPLE_STYLE_LIGHT_MAP_BYTE_COUNT
        || request.scene_type > 3
        || !(0.0..=1.0).contains(&request.people_ratio)
        || !(-1.0..=1.0).contains(&request.person_masks_valid_hint)
        || !(0.0..=1.0).contains(&request.skin_ratio)
        || !request.baseline_exposure.is_finite()
        || !request.base_gain.is_finite()
        || !request.linear_gain.is_finite()
        || !request.original_range_min.is_finite()
        || !request.original_range_max.is_finite()
        || request.original_range_min > request.original_range_max
        || !request.face_exposure_boost.is_finite()
    {
        return Err(AppleStyleDataError::InvalidPropertyListInput);
    }
    validate_style_statistics(request.statistics)?;

    let statistics = style_statistics_value(request.statistics);
    let root = plist_dictionary([
        ("0", PlistValue::Integer(15)),
        ("1", PlistValue::Data(request.style_data.to_vec())),
        ("2", PlistValue::Bool(true)),
        ("3", PlistValue::Data(request.global_tone_curve.to_vec())),
        ("4", PlistValue::Real(request.baseline_exposure)),
        ("5", PlistValue::Integer(u64::from(request.scene_type))),
        ("6", statistics),
        (
            "7",
            plist_dictionary([
                ("PeopleRatio", PlistValue::Real(request.people_ratio)),
                (
                    "PersonMasksValidHint",
                    PlistValue::Real(request.person_masks_valid_hint),
                ),
                ("SkinRatio", PlistValue::Real(request.skin_ratio)),
            ]),
        ),
        ("c", PlistValue::Data(request.tone_light_map.to_vec())),
        ("d", PlistValue::Data(request.linear_light_map.to_vec())),
        ("e", PlistValue::Integer(32)),
        ("f", PlistValue::Integer(32)),
        ("g", PlistValue::Integer(0x4C30_3068)),
        ("h", PlistValue::Real(request.base_gain)),
        (
            "i",
            plist_dictionary([
                ("Gain", PlistValue::Real(request.linear_gain)),
                (
                    "OriginalRangeMin",
                    PlistValue::Real(request.original_range_min),
                ),
                (
                    "OriginalRangeMax",
                    PlistValue::Real(request.original_range_max),
                ),
            ]),
        ),
        ("j", PlistValue::Real(request.face_exposure_boost)),
        ("k", PlistValue::Bool(false)),
    ]);
    serialize_binary_plist(root)
}

fn validate_style_statistics(statistics: &AppleStyleStatistics) -> Result<(), AppleStyleDataError> {
    let distributions = [
        statistics.linear_gtc_image,
        statistics.linear_image,
        statistics.linear_image_person_segment_based,
        statistics.linear_image_skin_based,
        statistics.tone_mapped_image,
        statistics.tone_mapped_image_blue_channel_skin_based,
        statistics.tone_mapped_image_green_channel_skin_based,
        statistics.tone_mapped_image_person_segment_based,
        statistics.tone_mapped_image_red_channel_skin_based,
        statistics.tone_mapped_image_skin_based,
    ];
    for distribution in distributions {
        let values = [
            distribution.black_point,
            distribution.high_key,
            distribution.p02,
            distribution.p10,
            distribution.p25,
            distribution.p50,
            distribution.p75,
            distribution.p98,
            distribution.white_point,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(AppleStyleDataError::InvalidPropertyListInput);
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
enum PlistValue {
    Bool(bool),
    Integer(u64),
    Real(f64),
    String(String),
    Data(Vec<u8>),
    Dictionary(BTreeMap<String, PlistValue>),
}

fn plist_dictionary<const N: usize>(entries: [(&str, PlistValue); N]) -> PlistValue {
    PlistValue::Dictionary(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

fn distribution_value(distribution: AppleStyleDistribution) -> PlistValue {
    plist_dictionary([
        ("blackPoint", PlistValue::Real(distribution.black_point)),
        ("highKey", PlistValue::Real(distribution.high_key)),
        ("p02", PlistValue::Real(distribution.p02)),
        ("p10", PlistValue::Real(distribution.p10)),
        ("p25", PlistValue::Real(distribution.p25)),
        ("p50", PlistValue::Real(distribution.p50)),
        ("p75", PlistValue::Real(distribution.p75)),
        ("p98", PlistValue::Real(distribution.p98)),
        ("whitePoint", PlistValue::Real(distribution.white_point)),
    ])
}

fn style_statistics_value(statistics: &AppleStyleStatistics) -> PlistValue {
    plist_dictionary([
        (
            "LinearGTCImage",
            distribution_value(statistics.linear_gtc_image),
        ),
        ("LinearImage", distribution_value(statistics.linear_image)),
        (
            "LinearImagePersonSegmentBased",
            distribution_value(statistics.linear_image_person_segment_based),
        ),
        (
            "LinearImageSkinBased",
            distribution_value(statistics.linear_image_skin_based),
        ),
        (
            "ToneMappedImage",
            distribution_value(statistics.tone_mapped_image),
        ),
        (
            "ToneMappedImageBlueChannelSkinBased",
            distribution_value(statistics.tone_mapped_image_blue_channel_skin_based),
        ),
        (
            "ToneMappedImageGreenChannelSkinBased",
            distribution_value(statistics.tone_mapped_image_green_channel_skin_based),
        ),
        (
            "ToneMappedImagePersonSegmentBased",
            distribution_value(statistics.tone_mapped_image_person_segment_based),
        ),
        (
            "ToneMappedImageRedChannelSkinBased",
            distribution_value(statistics.tone_mapped_image_red_channel_skin_based),
        ),
        (
            "ToneMappedImageSkinBased",
            distribution_value(statistics.tone_mapped_image_skin_based),
        ),
    ])
}

#[derive(Debug, Clone, PartialEq)]
enum PlistObject {
    Bool(bool),
    Integer(u64),
    Real(f64),
    String(String),
    Data(Vec<u8>),
    Dictionary(Vec<(usize, usize)>),
}

fn serialize_binary_plist(root: PlistValue) -> Result<Vec<u8>, AppleStyleDataError> {
    let mut objects = Vec::new();
    let top_object = collect_plist_object(root, &mut objects)?;
    let object_ref_size = integer_width(
        u64::try_from(objects.len().saturating_sub(1))
            .map_err(|_| AppleStyleDataError::PropertyListOverflow)?,
    );
    let mut output = b"bplist00".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for object in &objects {
        offsets.push(
            u64::try_from(output.len()).map_err(|_| AppleStyleDataError::PropertyListOverflow)?,
        );
        encode_plist_object(object, object_ref_size, &mut output)?;
    }
    let offset_table_offset =
        u64::try_from(output.len()).map_err(|_| AppleStyleDataError::PropertyListOverflow)?;
    let largest_offset = offset_table_offset.max(offsets.last().copied().unwrap_or(0));
    let offset_int_size = integer_width(largest_offset);
    for offset in offsets {
        write_fixed_width(&mut output, offset, offset_int_size);
    }

    output.extend_from_slice(&[0; 6]);
    output.push(offset_int_size as u8);
    output.push(object_ref_size as u8);
    output.extend_from_slice(
        &u64::try_from(objects.len())
            .map_err(|_| AppleStyleDataError::PropertyListOverflow)?
            .to_be_bytes(),
    );
    output.extend_from_slice(
        &u64::try_from(top_object)
            .map_err(|_| AppleStyleDataError::PropertyListOverflow)?
            .to_be_bytes(),
    );
    output.extend_from_slice(&offset_table_offset.to_be_bytes());
    Ok(output)
}

fn collect_plist_object(
    value: PlistValue,
    objects: &mut Vec<PlistObject>,
) -> Result<usize, AppleStyleDataError> {
    let object = match value {
        PlistValue::Bool(value) => PlistObject::Bool(value),
        PlistValue::Integer(value) => PlistObject::Integer(value),
        PlistValue::Real(value) => PlistObject::Real(value),
        PlistValue::String(value) => PlistObject::String(value),
        PlistValue::Data(value) => PlistObject::Data(value),
        PlistValue::Dictionary(entries) => {
            let mut references = Vec::with_capacity(entries.len());
            for (key, value) in entries {
                let key_index = collect_plist_object(PlistValue::String(key), objects)?;
                let value_index = collect_plist_object(value, objects)?;
                references.push((key_index, value_index));
            }
            PlistObject::Dictionary(references)
        }
    };
    let index = objects.len();
    objects.push(object);
    Ok(index)
}

fn encode_plist_object(
    object: &PlistObject,
    object_ref_size: usize,
    output: &mut Vec<u8>,
) -> Result<(), AppleStyleDataError> {
    match object {
        PlistObject::Bool(value) => output.push(if *value { 0x09 } else { 0x08 }),
        PlistObject::Integer(value) => encode_integer_object(*value, output),
        PlistObject::Real(value) => {
            output.push(0x23);
            output.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        PlistObject::String(value) => encode_string_object(value, output)?,
        PlistObject::Data(value) => {
            encode_count_marker(0x40, value.len(), output)?;
            output.extend_from_slice(value);
        }
        PlistObject::Dictionary(references) => {
            encode_count_marker(0xD0, references.len(), output)?;
            for (key, _) in references {
                write_fixed_width(
                    output,
                    u64::try_from(*key).map_err(|_| AppleStyleDataError::PropertyListOverflow)?,
                    object_ref_size,
                );
            }
            for (_, value) in references {
                write_fixed_width(
                    output,
                    u64::try_from(*value).map_err(|_| AppleStyleDataError::PropertyListOverflow)?,
                    object_ref_size,
                );
            }
        }
    }
    Ok(())
}

fn encode_integer_object(value: u64, output: &mut Vec<u8>) {
    let width = integer_width(value);
    output.push(0x10 | width.trailing_zeros() as u8);
    write_fixed_width(output, value, width);
}

fn encode_string_object(value: &str, output: &mut Vec<u8>) -> Result<(), AppleStyleDataError> {
    if value.is_ascii() {
        encode_count_marker(0x50, value.len(), output)?;
        output.extend_from_slice(value.as_bytes());
    } else {
        let utf16 = value.encode_utf16().collect::<Vec<_>>();
        encode_count_marker(0x60, utf16.len(), output)?;
        for unit in utf16 {
            output.extend_from_slice(&unit.to_be_bytes());
        }
    }
    Ok(())
}

fn encode_count_marker(
    marker: u8,
    count: usize,
    output: &mut Vec<u8>,
) -> Result<(), AppleStyleDataError> {
    if count < 15 {
        output.push(marker | count as u8);
    } else {
        output.push(marker | 0x0f);
        encode_integer_object(
            u64::try_from(count).map_err(|_| AppleStyleDataError::PropertyListOverflow)?,
            output,
        );
    }
    Ok(())
}

fn integer_width(value: u64) -> usize {
    if value <= u64::from(u8::MAX) {
        1
    } else if value <= u64::from(u16::MAX) {
        2
    } else if value <= u64::from(u32::MAX) {
        4
    } else {
        8
    }
}

fn write_fixed_width(output: &mut Vec<u8>, value: u64, width: usize) {
    output.extend_from_slice(&value.to_be_bytes()[8 - width..]);
}

/// Render parameters for the verified semantic-tone stage.
///
/// `tone_mix` is `g` in `T = mix(X, GTC(X), g)`. The LTM and highlight values
/// are `l` and `h` in the background and foreground branches. `person_matte`
/// is `m`. These are consumer render parameters. This struct does not map a
/// Photos Tone or Color slider position onto them; that mapping is not a
/// source fact.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppleStyleSemanticTone {
    pub exposure: f64,
    pub tone_mix: f64,
    pub background_ltm: f64,
    pub foreground_ltm: f64,
    pub background_highlight: f64,
    pub foreground_highlight: f64,
    pub person_matte: f64,
}

/// Evaluate one channel of the 26A434 semantic-tone stage.
///
/// `X = min(E · S, 1)`, `T = mix(X, GTC(X), g)`, then the ratio `T / A` is
/// mixed with the LTM and highlight parameters and multiplied back by `A`.
/// Below `1/255` the ratio is cleared before those mixes. The threshold is
/// evaluated in `f64`; the consumer's half subtraction is not reproduced.
/// Color, saturation, skin, and selective-color stages are later in the
/// shader and are not part of this result.
pub fn apple_style_semantic_tone_channel(
    primary: f64,
    thumbnail: f64,
    gtc_samples: &[f32],
    tone: AppleStyleSemanticTone,
) -> Result<f64, AppleStyleDataError> {
    if gtc_samples.len() != 256
        || gtc_samples.iter().any(|value| !value.is_finite())
        || !primary.is_finite()
        || !thumbnail.is_finite()
        || !tone.exposure.is_finite()
        || !tone.tone_mix.is_finite()
        || !tone.background_ltm.is_finite()
        || !tone.foreground_ltm.is_finite()
        || !tone.background_highlight.is_finite()
        || !tone.foreground_highlight.is_finite()
        || !tone.person_matte.is_finite()
    {
        return Err(AppleStyleDataError::InvalidRasterPair);
    }
    let x = (tone.exposure * thumbnail).min(1.0);
    let gtc = f64::from(sample_gtc_linear(gtc_samples, x.clamp(0.0, 1.0) as f32));
    let t = mix(x, gtc, tone.tone_mix);
    let background =
        semantic_tone_branch(primary, t, tone.background_ltm, tone.background_highlight);
    let foreground =
        semantic_tone_branch(primary, t, tone.foreground_ltm, tone.foreground_highlight);
    Ok(mix(background, foreground, tone.person_matte))
}

fn semantic_tone_branch(primary: f64, tone_mapped: f64, ltm: f64, highlight: f64) -> f64 {
    let ratio = if primary < (1.0 / 255.0) {
        0.0
    } else {
        tone_mapped / primary
    };
    let lifted = mix(ratio, 1.0, ltm);
    let recovered = mix(lifted, lifted.min(1.0), highlight);
    recovered * primary
}

fn mix(start: f64, end: f64, weight: f64) -> f64 {
    start * (1.0 - weight) + end * weight
}

/// Displayed primary and the image a remove-style revert should recover.
///
/// Both buffers are interleaved linear RGB at the same resolution. The revert
/// processor reads the displayed primary. The polynomial is fit so that
/// applying it to that primary approximates `recovered_rgb`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppleStyleRevertRequest<'a> {
    pub width: usize,
    pub height: usize,
    pub styled_rgb: &'a [f32],
    pub recovered_rgb: &'a [f32],
}

/// Fit key1 per 12×9 spotlight cell and per luminance plane.
///
/// The payload is x-major: `((x * 9 + y) * 8 + plane)`, then the existing
/// 10×3 quadratic block `[1, R, G, B, R², RG, RB, G², GB, B²]`. Plane weights
/// use `b = 0.5 * (mean + max)` and Gaussian centers `(k + 1) / 9` with
/// `σ = 2.5`, which is the current `inputIsLinear = false` consumer. Spatial
/// weights use the default spotlight coordinate. The solved value is the
/// residual added to identity, so remove-style of an unchanged image stays the
/// identity transform.
///
/// A plane whose samples do not identify a coefficient keeps that coefficient
/// at the identity residual. The existing numerical ridge is only a stabilizer
/// for that underdetermined case. It is not a scene calibration. Half rounding
/// of the plane weight and the optional 3×3 spatial filter are not reproduced.
pub fn apple_style_revert_local_data(
    request: &AppleStyleRevertRequest<'_>,
) -> Result<Vec<u8>, AppleStyleDataError> {
    let pixel_count = request
        .width
        .checked_mul(request.height)
        .ok_or(AppleStyleDataError::InvalidRasterPair)?;
    let value_count = pixel_count
        .checked_mul(APPLE_STYLE_CHANNEL_COUNT)
        .ok_or(AppleStyleDataError::InvalidRasterPair)?;
    if request.width == 0
        || request.height == 0
        || request.styled_rgb.len() != value_count
        || request.recovered_rgb.len() != value_count
        || request.styled_rgb.iter().any(|value| !value.is_finite())
        || request.recovered_rgb.iter().any(|value| !value.is_finite())
    {
        return Err(AppleStyleDataError::InvalidRasterPair);
    }

    const CELLS: usize = APPLE_STYLE_GRID_WIDTH * APPLE_STYLE_GRID_HEIGHT * APPLE_STYLE_PLANE_COUNT;
    let mut normal =
        vec![0.0_f64; CELLS * APPLE_STYLE_POLYNOMIAL_COUNT * APPLE_STYLE_POLYNOMIAL_COUNT];
    let mut rhs = vec![0.0_f64; CELLS * APPLE_STYLE_BLOCK_VALUE_COUNT];
    let mut plane_weights = [0.0_f64; APPLE_STYLE_PLANE_COUNT];
    for y in 0..request.height {
        let row_weights = spotlight_weights(y, request.height, APPLE_STYLE_GRID_HEIGHT);
        for x in 0..request.width {
            let offset = (y * request.width + x) * APPLE_STYLE_CHANNEL_COUNT;
            let styled = [
                f64::from(request.styled_rgb[offset]),
                f64::from(request.styled_rgb[offset + 1]),
                f64::from(request.styled_rgb[offset + 2]),
            ];
            let recovered = [
                f64::from(request.recovered_rgb[offset]),
                f64::from(request.recovered_rgb[offset + 1]),
                f64::from(request.recovered_rgb[offset + 2]),
            ];
            let basis = quadratic_basis(styled);
            let mean = (styled[0] + styled[1] + styled[2]) / 3.0;
            let peak = styled[0].max(styled[1]).max(styled[2]);
            let brightness = 0.5 * (mean + peak);
            let mut plane_sum = 0.0_f64;
            for (plane, weight) in plane_weights.iter_mut().enumerate() {
                let center = (plane as f64 + 1.0) / (APPLE_STYLE_PLANE_COUNT as f64 + 1.0);
                let distance = (brightness - center) * APPLE_STYLE_PLANE_COUNT as f64;
                *weight = (-0.5 * (distance / 2.5).powi(2)).exp();
                plane_sum += *weight;
            }
            if plane_sum <= 0.0 {
                continue;
            }
            let column_weights = spotlight_weights(x, request.width, APPLE_STYLE_GRID_WIDTH);
            for &(cell_y, y_weight) in &row_weights {
                for &(cell_x, x_weight) in &column_weights {
                    let spatial = x_weight * y_weight;
                    if spatial <= 0.0 {
                        continue;
                    }
                    for (plane, plane_weight) in plane_weights.iter().copied().enumerate() {
                        let weight = spatial * plane_weight / plane_sum;
                        if weight <= 0.0 {
                            continue;
                        }
                        let cell = ((cell_x * APPLE_STYLE_GRID_HEIGHT + cell_y)
                            * APPLE_STYLE_PLANE_COUNT
                            + plane)
                            * APPLE_STYLE_POLYNOMIAL_COUNT;
                        let rhs_cell = ((cell_x * APPLE_STYLE_GRID_HEIGHT + cell_y)
                            * APPLE_STYLE_PLANE_COUNT
                            + plane)
                            * APPLE_STYLE_BLOCK_VALUE_COUNT;
                        for row in 0..APPLE_STYLE_POLYNOMIAL_COUNT {
                            for column in row..APPLE_STYLE_POLYNOMIAL_COUNT {
                                normal[(cell + row) * APPLE_STYLE_POLYNOMIAL_COUNT + column] +=
                                    weight * basis[row] * basis[column];
                            }
                            for (channel, (&source, target)) in
                                styled.iter().zip(recovered).enumerate()
                            {
                                rhs[rhs_cell + row * APPLE_STYLE_CHANNEL_COUNT + channel] +=
                                    weight * basis[row] * (target - source);
                            }
                        }
                    }
                }
            }
        }
    }

    let mut deltas = vec![0.0_f64; APPLE_STYLE_BYTE_COUNT / APPLE_STYLE_VALUE_BYTE_COUNT];
    for cell_x in 0..APPLE_STYLE_GRID_WIDTH {
        for cell_y in 0..APPLE_STYLE_GRID_HEIGHT {
            for plane in 0..APPLE_STYLE_PLANE_COUNT {
                let normal_base =
                    ((cell_x * APPLE_STYLE_GRID_HEIGHT + cell_y) * APPLE_STYLE_PLANE_COUNT + plane)
                        * APPLE_STYLE_POLYNOMIAL_COUNT
                        * APPLE_STYLE_POLYNOMIAL_COUNT;
                let mut matrix =
                    vec![0.0_f64; APPLE_STYLE_POLYNOMIAL_COUNT * APPLE_STYLE_POLYNOMIAL_COUNT];
                for row in 0..APPLE_STYLE_POLYNOMIAL_COUNT {
                    for column in 0..row {
                        matrix[row * APPLE_STYLE_POLYNOMIAL_COUNT + column] =
                            normal[normal_base + column * APPLE_STYLE_POLYNOMIAL_COUNT + row];
                    }
                    for column in row..APPLE_STYLE_POLYNOMIAL_COUNT {
                        matrix[row * APPLE_STYLE_POLYNOMIAL_COUNT + column] =
                            normal[normal_base + row * APPLE_STYLE_POLYNOMIAL_COUNT + column];
                    }
                }
                let trace = (0..APPLE_STYLE_POLYNOMIAL_COUNT)
                    .map(|term| matrix[term * APPLE_STYLE_POLYNOMIAL_COUNT + term])
                    .sum::<f64>();
                let ridge = (trace / APPLE_STYLE_POLYNOMIAL_COUNT as f64 * 1e-5).max(1e-9);
                for term in 0..APPLE_STYLE_POLYNOMIAL_COUNT {
                    matrix[term * APPLE_STYLE_POLYNOMIAL_COUNT + term] +=
                        ridge * f64::from((term >= 4) as u8 * 9 + 1);
                }
                for channel in 0..APPLE_STYLE_CHANNEL_COUNT {
                    let rhs_base = ((cell_x * APPLE_STYLE_GRID_HEIGHT + cell_y)
                        * APPLE_STYLE_PLANE_COUNT
                        + plane)
                        * APPLE_STYLE_BLOCK_VALUE_COUNT;
                    let vector = (0..APPLE_STYLE_POLYNOMIAL_COUNT)
                        .map(|term| rhs[rhs_base + term * APPLE_STYLE_CHANNEL_COUNT + channel])
                        .collect::<Vec<_>>();
                    let solution = solve_linear_system(&matrix, &vector)
                        .unwrap_or_else(|_| vec![0.0; APPLE_STYLE_POLYNOMIAL_COUNT]);
                    for (basis, value) in solution.into_iter().enumerate() {
                        let bound = coefficient_bound(basis * APPLE_STYLE_CHANNEL_COUNT + channel);
                        let index = style_coefficient_index(cell_x, cell_y, plane, basis, channel);
                        deltas[index] = value.clamp(-bound, bound);
                    }
                }
            }
        }
    }

    let mut result = Vec::with_capacity(APPLE_STYLE_BYTE_COUNT);
    for x in 0..APPLE_STYLE_GRID_WIDTH {
        for y in 0..APPLE_STYLE_GRID_HEIGHT {
            for plane in 0..APPLE_STYLE_PLANE_COUNT {
                for basis in 0..APPLE_STYLE_POLYNOMIAL_COUNT {
                    for channel in 0..APPLE_STYLE_CHANNEL_COUNT {
                        let index = style_coefficient_index(x, y, plane, basis, channel);
                        let identity = if IDENTITY_INDICES
                            .contains(&((basis * APPLE_STYLE_CHANNEL_COUNT) + channel))
                        {
                            1.0
                        } else {
                            0.0
                        };
                        let value = identity + deltas[index] as f32;
                        result.extend_from_slice(&f16::from_f32(value).to_le_bytes());
                    }
                }
            }
        }
    }
    validate_apple_style_data(&result)?;
    Ok(result)
}

fn quadratic_basis(rgb: [f64; 3]) -> [f64; APPLE_STYLE_POLYNOMIAL_COUNT] {
    let [red, green, blue] = rgb;
    [
        1.0,
        red,
        green,
        blue,
        red * red,
        red * green,
        red * blue,
        green * green,
        green * blue,
        blue * blue,
    ]
}

fn spotlight_weights(pixel: usize, extent: usize, count: usize) -> [(usize, f64); 2] {
    let coordinate = spotlight_coordinate(pixel, extent, count) * count as f64;
    let lower = coordinate.floor().clamp(0.0, (count - 1) as f64) as usize;
    let upper = (lower + 1).min(count - 1);
    let fraction = if lower == upper {
        0.0
    } else {
        (coordinate - lower as f64).clamp(0.0, 1.0)
    };
    [(lower, 1.0 - fraction), (upper, fraction)]
}

fn spotlight_coordinate(pixel: usize, extent: usize, count: usize) -> f64 {
    if extent == 0 || count == 0 {
        return 0.0;
    }
    let cells = (extent as f64 / count as f64).ceil();
    let pitch = if (cells as usize).is_multiple_of(2) {
        cells
    } else {
        cells + 1.0
    };
    let first = pitch / 2.0 - 1.0;
    let last = first + pitch * (count as f64 - 1.0);
    let span = (pitch * (count as f64 - 1.0)).max(1.0);
    let position = (pixel as f64).clamp(first, last);
    (0.5 + (position - first) * (count as f64 - 1.0) / span) / count as f64
}

fn style_coefficient_index(
    x: usize,
    y: usize,
    plane: usize,
    basis: usize,
    channel: usize,
) -> usize {
    (((x * APPLE_STYLE_GRID_HEIGHT + y) * APPLE_STYLE_PLANE_COUNT + plane)
        * APPLE_STYLE_POLYNOMIAL_COUNT
        + basis)
        * APPLE_STYLE_CHANNEL_COUNT
        + channel
}

/// Serialize bounded coefficient deltas over the verified identity payload.
pub fn apple_style_data_from_coefficient_deltas(
    coefficient_deltas: &[f64],
) -> Result<Vec<u8>, AppleStyleDataError> {
    validate_coefficient_deltas(coefficient_deltas)?;
    let mut block = [0.0_f32; APPLE_STYLE_BLOCK_VALUE_COUNT];
    for index in IDENTITY_INDICES {
        block[index] = 1.0;
    }
    for (index, delta) in coefficient_deltas.iter().copied().enumerate() {
        block[index] += delta as f32;
    }

    let mut result = Vec::with_capacity(APPLE_STYLE_BYTE_COUNT);
    for _ in 0..APPLE_STYLE_TILE_COUNT {
        for value in block {
            result.extend_from_slice(&f16::from_f32(value).to_le_bytes());
        }
    }
    validate_apple_style_data(&result)?;
    Ok(result)
}

/// Serialize the six direct editor parameters into the full coefficient block.
pub fn apple_style_data_from_parameters(
    parameters: &[f64],
) -> Result<Vec<u8>, AppleStyleDataError> {
    if parameters.len() != DIRECT_PARAMETER_INDICES.len() {
        return Err(AppleStyleDataError::InvalidCoefficientCount {
            actual: parameters.len(),
            expected: DIRECT_PARAMETER_INDICES.len(),
        });
    }
    let mut deltas = [0.0_f64; APPLE_STYLE_BLOCK_VALUE_COUNT];
    for (parameter_index, coefficient_index) in DIRECT_PARAMETER_INDICES.iter().copied().enumerate()
    {
        deltas[coefficient_index] = parameters[parameter_index];
    }
    apple_style_data_from_coefficient_deltas(&deltas)
}

/// Add bounded coefficient deltas to an existing Float16 key-1 payload.
pub fn apple_style_data_apply_coefficient_deltas(
    base: &[u8],
    coefficient_deltas: &[f64],
) -> Result<Vec<u8>, AppleStyleDataError> {
    validate_apple_style_data(base)?;
    validate_coefficient_deltas(coefficient_deltas)?;

    let mut result = Vec::with_capacity(base.len());
    for value_index in 0..(base.len() / APPLE_STYLE_VALUE_BYTE_COUNT) {
        let offset = value_index * APPLE_STYLE_VALUE_BYTE_COUNT;
        let base_value = f16::from_le_bytes([base[offset], base[offset + 1]]).to_f32();
        let value =
            base_value + coefficient_deltas[value_index % APPLE_STYLE_BLOCK_VALUE_COUNT] as f32;
        result.extend_from_slice(&f16::from_f32(value).to_le_bytes());
    }
    validate_apple_style_data(&result)?;
    Ok(result)
}

fn validate_coefficient_deltas(values: &[f64]) -> Result<(), AppleStyleDataError> {
    if values.len() != APPLE_STYLE_BLOCK_VALUE_COUNT {
        return Err(AppleStyleDataError::InvalidCoefficientCount {
            actual: values.len(),
            expected: APPLE_STYLE_BLOCK_VALUE_COUNT,
        });
    }
    for (index, value) in values.iter().copied().enumerate() {
        if !value.is_finite() {
            return Err(AppleStyleDataError::NonFiniteCoefficient { index });
        }
        let bound = coefficient_bound(index);
        if value.abs() > bound + 1e-9 {
            return Err(AppleStyleDataError::CoefficientOutOfBounds {
                index,
                value,
                bound,
            });
        }
    }
    Ok(())
}

fn coefficient_bound(index: usize) -> f64 {
    if index / APPLE_STYLE_CHANNEL_COUNT >= 4 {
        QUADRATIC_BOUND
    } else {
        LINEAR_BOUND
    }
}

fn solve_linear_system(matrix: &[f64], vector: &[f64]) -> Result<Vec<f64>, AppleStyleDataError> {
    let count = vector.len();
    if matrix.len() != count * count {
        return Err(AppleStyleDataError::SingularSystem);
    }
    let mut augmented = (0..count)
        .map(|row| {
            let mut values = matrix[row * count..(row + 1) * count].to_vec();
            values.push(vector[row]);
            values
        })
        .collect::<Vec<_>>();
    for pivot in 0..count {
        let best = (pivot..count)
            .max_by(|left, right| {
                augmented[*left][pivot]
                    .abs()
                    .total_cmp(&augmented[*right][pivot].abs())
            })
            .ok_or(AppleStyleDataError::SingularSystem)?;
        if augmented[best][pivot].abs() <= 1e-12 {
            return Err(AppleStyleDataError::SingularSystem);
        }
        if best != pivot {
            augmented.swap(best, pivot);
        }
        let divisor = augmented[pivot][pivot];
        for value in augmented[pivot][pivot..=count].iter_mut() {
            *value /= divisor;
        }
        let pivot_values = augmented[pivot].clone();
        let (before, pivot_and_after) = augmented.split_at_mut(pivot);
        let (_, after) = pivot_and_after
            .split_first_mut()
            .ok_or(AppleStyleDataError::SingularSystem)?;
        for row_values in before.iter_mut().chain(after.iter_mut()) {
            let factor = row_values[pivot];
            for (value, pivot_value) in row_values[pivot..=count]
                .iter_mut()
                .zip(pivot_values[pivot..=count].iter())
            {
                *value -= factor * pivot_value;
            }
        }
    }
    Ok(augmented.into_iter().map(|row| row[count]).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_thumbnail_log_round_trips_the_exponential_decoder() {
        let mut worst = 0.0_f64;
        for step in 0..=4_000 {
            let linear = -0.005 + (11.5 - (-0.005)) * f64::from(step) / 4_000.0;
            let code = apple_log_encode(linear);
            if code <= 0.0 || code >= 1.0 {
                continue;
            }
            let decoded = apple_log_decode(code);
            worst = worst.max((decoded - linear).abs());
        }
        assert!(
            worst <= 1.0e-5,
            "Apple Log float round trip max abs {worst}"
        );

        // The former quadratic toe maps linear 0 to about 0.150. The
        // exponential inverse maps it back to 0 and uses a different code.
        let zero = apple_log_encode(0.0);
        let quadratic_toe = 47.287_112_36 * (0.0_f64 + 0.056_410_88).powi(2);
        assert!(
            (zero - quadratic_toe).abs() > 1.0e-2,
            "dark code {zero} still matches the unverified camera toe {quadratic_toe}"
        );
        assert!((apple_log_decode(zero) - 0.0).abs() <= 1.0e-5);
        assert!((apple_log_decode(apple_log_encode(0.001)) - 0.001).abs() <= 1.0e-5);
        assert!((apple_log_decode(apple_log_encode(0.009)) - 0.009).abs() <= 1.0e-5);

        // Below linear + B the log is undefined. Clamp the code; do not use the toe.
        assert_eq!(apple_log_encode(-0.05), 0.0);
        assert_eq!(apple_log_encode(-0.1), 0.0);
        let toe_at_dark = 47.287_112_36 * (-0.05_f64 + 0.056_410_88).powi(2);
        assert!(toe_at_dark > 0.0);

        let rgb = [-0.1_f32, 0.0, 0.001, 0.01, 0.18, 1.0, 4.0, 16.0, 100.0];
        let codes = apple_style_linear_thumbnail_codes(&rgb, 1.0).unwrap();
        assert_eq!(codes[0], 0);
        assert_eq!(codes[7], u16::MAX);
        assert_eq!(codes[8], u16::MAX);
        let eight_bit = (apple_log_encode(0.18) * 255.0).round() as u16;
        assert_ne!(
            codes[4],
            eight_bit << 8,
            "16-bit Apple Log code collapsed to an 8-bit high byte"
        );
        // Normalizing by the same exposure used in the metadata must leave
        // the reference unchanged, including HDR channels above 1.0.
        let exposed = rgb.map(|value| value * 4.0);
        assert_eq!(
            apple_style_linear_thumbnail_codes(&exposed, 4.0).unwrap(),
            codes
        );
    }

    #[test]
    fn linear_thumbnail_rejects_invalid_reference_before_quantization() {
        for rgb in [&[][..], &[0.0, 0.0][..]] {
            assert!(matches!(
                apple_style_linear_thumbnail_codes(rgb, 1.0),
                Err(AppleStyleDataError::InvalidRgbRaster { .. })
            ));
        }
        for exposure in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(
                apple_style_linear_thumbnail_codes(&[0.0; 3], exposure),
                Err(AppleStyleDataError::InvalidThumbnailExposure)
            );
        }
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                apple_style_linear_thumbnail_codes(&[0.0, value, 0.0], 1.0),
                Err(AppleStyleDataError::NonFiniteValue { index: 1 })
            );
        }
    }

    #[test]
    fn identity_layout_matches_verified_dimensions_and_values() {
        let data = apple_style_identity_data();
        assert_eq!(data.len(), APPLE_STYLE_BYTE_COUNT);
        let facts = validate_apple_style_data(&data).unwrap();
        assert_eq!(facts.value_count, APPLE_STYLE_BYTE_COUNT / 2);
        assert_eq!(facts.minimum, 0.0);
        assert_eq!(facts.maximum, 1.0);
        assert_eq!(facts.identity_residual_rmse, 0.0);
        assert_eq!(facts.identity_residual_maximum_absolute, 0.0);
        assert!(facts.complete_identity);
        assert_eq!(apple_style_data_sha256(&data), APPLE_STYLE_IDENTITY_SHA256);
        for tile in data.as_chunks::<{ APPLE_STYLE_BLOCK_VALUE_COUNT * 2 }>().0 {
            for index in 0..APPLE_STYLE_BLOCK_VALUE_COUNT {
                let offset = index * 2;
                let value = f16::from_le_bytes([tile[offset], tile[offset + 1]]).to_f32();
                let expected = if IDENTITY_INDICES.contains(&index) {
                    1.0
                } else {
                    0.0
                };
                assert_eq!(value, expected);
            }
        }
    }

    #[test]
    fn identity_global_tone_curve_matches_257_sample_wire_contract() {
        let curve = apple_style_identity_global_tone_curve();
        assert_eq!(curve.len(), 516);
        assert_eq!(&curve[..2], &257_u16.to_le_bytes());
        let samples = curve[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|chunk| u16::from_le_bytes(*chunk))
            .collect::<Vec<_>>();
        assert_eq!(samples.len(), 257);
        assert_eq!(samples.first(), Some(&0));
        assert_eq!(samples.last(), Some(&65_534));
        assert!(samples.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn distribution_matches_swift_percentiles_and_empty_contract() {
        let empty = apple_style_distribution(&[]);
        assert_eq!(empty.high_key, 1.0);
        assert_eq!(empty.black_point, 0.0);
        assert_eq!(empty.p50, 0.0);
        assert_eq!(empty.white_point, 0.0);

        let distribution = apple_style_distribution(&[0.0, 1.0, 2.0, 3.0, f32::NAN]);
        assert_eq!(distribution.black_point, 0.015);
        assert!((distribution.high_key - 2.85).abs() < 1e-12);
        assert_eq!(distribution.p50, 1.5);
        assert!((distribution.white_point - 2.985).abs() < 1e-12);
    }

    #[test]
    fn source_derived_global_tone_curve_follows_channel_codes_without_percentile_scaling() {
        let mut thumbnail = Vec::new();
        let mut primary = Vec::new();
        for index in 0..256 {
            let code = index as f32 / 255.0;
            let target = if code < 0.5 { 0.0 } else { code };
            for _ in 0..3 {
                thumbnail.push(code);
                primary.push(target);
            }
        }
        let curve = apple_style_monotonic_global_tone_curve(&thumbnail, &primary).unwrap();
        assert_eq!(curve.data.len(), 516);
        assert_eq!(curve.linear_samples.len(), 256);
        assert!(curve
            .linear_samples
            .windows(2)
            .all(|pair| pair[0] <= pair[1]));
        assert!(curve.populated_bins > 200);
        assert!(curve.fit_rmse.is_finite());
        let dark = curve.linear_samples[gtc_node_index(0.2)];
        let bright = curve.linear_samples[gtc_node_index(0.8)];
        assert!(
            dark < 0.05,
            "dark code was rescaled or smoothed away: {dark}"
        );
        assert!(
            (bright - 0.8).abs() < 0.05,
            "bright code is not the primary code: {bright}"
        );
        assert_ne!(curve.data, apple_style_identity_global_tone_curve());
        assert!((curve.linear_samples[gtc_node_index(1.0)] - 1.0).abs() < 1e-3);

        let mapped =
            apple_style_apply_global_tone_curve(&[0.2, 0.8, 1.0], &curve.linear_samples).unwrap();
        assert!(mapped[0] < 0.05);
        assert!((mapped[1] - 0.8).abs() < 0.05);
        assert!((mapped[2] - 1.0).abs() < 1e-3);
    }

    #[test]
    fn semantic_tone_at_neutral_ltm_ignores_thumbnail_and_curve() {
        let mut flat = vec![0.25_f32; 256];
        flat[255] = 1.0;
        let neutral = AppleStyleSemanticTone {
            exposure: 1.0,
            tone_mix: 1.0,
            background_ltm: 1.0,
            foreground_ltm: 1.0,
            background_highlight: 0.0,
            foreground_highlight: 0.0,
            person_matte: 0.0,
        };
        let from_thumbnail = apple_style_semantic_tone_channel(0.4, 0.1, &flat, neutral).unwrap();
        let from_other = apple_style_semantic_tone_channel(0.4, 0.9, &flat, neutral).unwrap();
        assert!((from_thumbnail - 0.4).abs() < 1e-12);
        assert!((from_other - from_thumbnail).abs() < 1e-12);

        let engaged = AppleStyleSemanticTone {
            exposure: 1.0,
            tone_mix: 1.0,
            background_ltm: 0.0,
            foreground_ltm: 0.0,
            background_highlight: 0.0,
            foreground_highlight: 0.0,
            person_matte: 0.0,
        };
        let followed = apple_style_semantic_tone_channel(0.5, 0.25, &flat, engaged).unwrap();
        assert!(
            (followed - 0.25).abs() < 1e-6,
            "full tone mix should follow GTC(X), got {followed}"
        );
        flat.fill(0.8);
        let shifted = apple_style_semantic_tone_channel(0.5, 0.25, &flat, engaged).unwrap();
        assert!(
            (shifted - 0.8).abs() < 1e-3,
            "GTC change was ignored: {shifted}"
        );
    }

    #[test]
    fn revert_key1_is_local_and_points_from_styled_primary_to_recovered_scene() {
        assert_eq!(
            style_coefficient_index(1, 0, 0, 0, 0),
            APPLE_STYLE_GRID_HEIGHT * APPLE_STYLE_PLANE_COUNT * APPLE_STYLE_BLOCK_VALUE_COUNT
        );
        let width = 24;
        let height = 18;
        let mut styled = vec![0.0_f32; width * height * 3];
        let mut recovered = vec![0.0_f32; width * height * 3];
        for y in 0..height {
            for x in 0..width {
                let offset = (y * width + x) * 3;
                let level = 0.25 + (x % 4) as f32 * 0.12 + (y % 3) as f32 * 0.08;
                for channel in 0..3 {
                    styled[offset + channel] = level;
                    recovered[offset + channel] = if x < width / 2 { level - 0.05 } else { level };
                }
            }
        }
        let data = apple_style_revert_local_data(&AppleStyleRevertRequest {
            width,
            height,
            styled_rgb: &styled,
            recovered_rgb: &recovered,
        })
        .unwrap();
        let read = |x, y, plane, basis, channel| {
            let index = style_coefficient_index(x, y, plane, basis, channel);
            f16::from_le_bytes([data[index * 2], data[index * 2 + 1]]).to_f32()
        };
        let left = read(0, 4, 3, 0, 0);
        let right = read(11, 4, 3, 0, 0);
        assert!(
            left < -0.01,
            "revert residual should darken the styled left side, got {left}"
        );
        assert!(
            right > left + 0.01,
            "right cell should not copy the left residual: left {left}, right {right}"
        );
        assert_ne!(data, apple_style_identity_data());
    }

    #[test]
    fn linear_metadata_changes_with_photo_distribution_and_gain_range() {
        let base = (0..128)
            .map(|index| 0.05 + index as f32 / 256.0)
            .collect::<Vec<_>>();
        let hdr = base
            .iter()
            .map(|value| (value * 2.0).min(1.0))
            .collect::<Vec<_>>();
        let first = apple_style_linear_metadata(&base, &hdr, 1.0).unwrap();
        let second = apple_style_linear_metadata(&base, &hdr, 4.0).unwrap();
        assert!(first.baseline_exposure.is_finite());
        assert!(first.encoding_gain >= 2.0);
        assert!(second.base_gain > first.base_gain);
        assert!(second.encoding_gain > first.encoding_gain);
        assert_ne!(first, second);
    }

    #[test]
    fn polynomial_basis_matches_swift_contract() {
        assert_eq!(
            apple_style_polynomial_basis(2.0, 3.0, 5.0).unwrap(),
            [1.0, 2.0, 3.0, 5.0, 4.0, 6.0, 10.0, 9.0, 15.0, 25.0]
        );
    }

    #[test]
    fn direct_parameters_change_only_the_six_owned_coefficients() {
        let data =
            apple_style_data_from_parameters(&[0.01, -0.02, 0.03, 0.04, -0.05, 0.06]).unwrap();
        let facts = validate_apple_style_data(&data).unwrap();
        assert!(!facts.complete_identity);
        let first = data
            .as_chunks::<2>()
            .0
            .iter()
            .take(APPLE_STYLE_BLOCK_VALUE_COUNT);
        let values = first
            .map(|bytes| f16::from_le_bytes(*bytes).to_f32())
            .collect::<Vec<_>>();
        assert_eq!(values[0], f16::from_f32(0.01).to_f32());
        assert_eq!(values[1], f16::from_f32(-0.02).to_f32());
        assert_eq!(values[2], f16::from_f32(0.03).to_f32());
        assert_eq!(values[3], f16::from_f32(1.04).to_f32());
        assert_eq!(values[7], f16::from_f32(0.95).to_f32());
        assert_eq!(values[11], f16::from_f32(1.06).to_f32());
        for index in [4, 5, 6, 8, 9, 10] {
            assert_eq!(values[index], 0.0);
        }
    }

    #[test]
    fn coefficient_bounds_and_malformed_payloads_fail_closed() {
        let mut deltas = vec![0.0; APPLE_STYLE_BLOCK_VALUE_COUNT];
        deltas[0] = 0.125 + 1e-6;
        assert!(matches!(
            apple_style_data_from_coefficient_deltas(&deltas),
            Err(AppleStyleDataError::CoefficientOutOfBounds { index: 0, .. })
        ));
        deltas[0] = f64::NAN;
        assert!(matches!(
            apple_style_data_from_coefficient_deltas(&deltas),
            Err(AppleStyleDataError::NonFiniteCoefficient { index: 0 })
        ));
        assert!(matches!(
            validate_apple_style_data(&[0; APPLE_STYLE_BYTE_COUNT - 2]),
            Err(AppleStyleDataError::InvalidLength { .. })
        ));
        assert!(matches!(
            apple_style_data_apply_coefficient_deltas(&apple_style_identity_data(), &[0.0; 11]),
            Err(AppleStyleDataError::InvalidCoefficientCount { .. })
        ));
        assert!(matches!(
            apple_style_fit_global_polynomial(&[], &[]),
            Err(AppleStyleDataError::InvalidRasterPair)
        ));
        assert!(matches!(
            apple_style_fit_global_polynomial(&[0.0, 0.0, 0.0], &[f32::NAN, 0.0, 0.0]),
            Err(AppleStyleDataError::InvalidRasterPair)
        ));
    }

    #[test]
    fn existing_payload_deltas_are_applied_per_repeated_block() {
        let base = apple_style_identity_data();
        let mut deltas = vec![0.0; APPLE_STYLE_BLOCK_VALUE_COUNT];
        deltas[3] = 0.01;
        let output = apple_style_data_apply_coefficient_deltas(&base, &deltas).unwrap();
        let block_size = APPLE_STYLE_BLOCK_VALUE_COUNT * 2;
        let first = &output[..block_size];
        let last = &output[output.len() - block_size..];
        assert_eq!(first, last);
        let value = f16::from_le_bytes([first[6], first[7]]).to_f32();
        assert_eq!(value, f16::from_f32(1.01).to_f32());
    }

    #[test]
    fn global_polynomial_fit_recovers_a_bounded_known_transform() {
        let mut expected = vec![0.0_f64; APPLE_STYLE_BLOCK_VALUE_COUNT];
        for output in 0..APPLE_STYLE_CHANNEL_COUNT {
            expected[output] = [0.010, -0.008, 0.006][output];
            expected[APPLE_STYLE_CHANNEL_COUNT + output] = [0.008, -0.005, 0.004][output];
            expected[2 * APPLE_STYLE_CHANNEL_COUNT + output] = [-0.006, 0.007, -0.003][output];
            expected[3 * APPLE_STYLE_CHANNEL_COUNT + output] = [0.005, 0.004, -0.007][output];
            expected[4 * APPLE_STYLE_CHANNEL_COUNT + output] = [0.002, -0.001, 0.001][output];
            expected[5 * APPLE_STYLE_CHANNEL_COUNT + output] = [-0.001, 0.002, -0.001][output];
            expected[6 * APPLE_STYLE_CHANNEL_COUNT + output] = [0.001, -0.001, 0.002][output];
            expected[7 * APPLE_STYLE_CHANNEL_COUNT + output] = [0.002, 0.001, -0.002][output];
            expected[8 * APPLE_STYLE_CHANNEL_COUNT + output] = [-0.002, 0.001, 0.001][output];
            expected[9 * APPLE_STYLE_CHANNEL_COUNT + output] = [0.001, -0.002, 0.001][output];
        }

        let mut source = Vec::with_capacity(512 * APPLE_STYLE_CHANNEL_COUNT);
        let mut target = Vec::with_capacity(512 * APPLE_STYLE_CHANNEL_COUNT);
        for pixel in 0..512 {
            let source_pixel = [
                16.0 + (pixel * 37 % 220) as f32,
                17.0 + (pixel * 53 % 219) as f32,
                18.0 + (pixel * 71 % 218) as f32,
            ];
            source.extend_from_slice(&source_pixel);
            let basis = apple_style_polynomial_basis(
                source_pixel[0] / 255.0,
                source_pixel[1] / 255.0,
                source_pixel[2] / 255.0,
            )
            .unwrap();
            for output in 0..APPLE_STYLE_CHANNEL_COUNT {
                let delta = basis
                    .iter()
                    .enumerate()
                    .map(|(term, value)| {
                        f64::from(*value) * expected[term * APPLE_STYLE_CHANNEL_COUNT + output]
                    })
                    .sum::<f64>();
                target.push(source_pixel[output] + (delta * 255.0) as f32);
            }
        }

        let fitted = apple_style_fit_global_polynomial(&source, &target).unwrap();
        assert_eq!(fitted.len(), APPLE_STYLE_BLOCK_VALUE_COUNT);
        for (index, value) in fitted.iter().copied().enumerate() {
            assert!(value.is_finite());
            assert!(value.abs() <= coefficient_bound(index) + 1e-9);
            assert!(
                (value - expected[index]).abs() < 2e-3,
                "coefficient {index}"
            );
        }
        assert_eq!(
            fitted,
            apple_style_fit_global_polynomial(&source, &target).unwrap()
        );
    }

    #[test]
    fn sampled_jacobian_recovers_bounded_refinement_update() {
        let current = vec![128.0_f32; 12];
        let mut jacobian = AppleStyleSampledJacobian::new(current.len()).unwrap();
        let expected = (0..APPLE_STYLE_REFINEMENT_PARAMETER_COUNT)
            .map(|index| 0.001 * (index + 1) as f64)
            .collect::<Vec<_>>();

        for parameter in 0..APPLE_STYLE_REFINEMENT_PARAMETER_COUNT {
            let mut rendered = current.clone();
            rendered[parameter] += 1.0;
            let derivative_rms = jacobian
                .populate_parameter(parameter, &rendered, &current, 1.0)
                .unwrap();
            assert!((derivative_rms - (1.0 / 12.0_f64).sqrt()).abs() < 1e-6);
        }

        let target = current
            .iter()
            .enumerate()
            .map(|(index, value)| *value + expected[index] as f32)
            .collect::<Vec<_>>();
        let fitted =
            apple_style_solve_refinement_update(&[128.0; 12], &target, &jacobian, &[]).unwrap();
        assert_eq!(fitted.len(), APPLE_STYLE_REFINEMENT_PARAMETER_COUNT);
        for (actual, expected) in fitted.iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-5);
        }
    }

    #[test]
    fn refinement_scalar_rows_are_typed_and_updates_are_clamped() {
        let current = vec![128.0_f32; 12];
        let mut jacobian = AppleStyleSampledJacobian::new(current.len()).unwrap();
        for parameter in 0..APPLE_STYLE_REFINEMENT_PARAMETER_COUNT {
            let mut rendered = current.clone();
            rendered[parameter] += 1.0;
            jacobian
                .populate_parameter(parameter, &rendered, &current, 1.0)
                .unwrap();
        }
        let mut derivative = [0.0_f64; APPLE_STYLE_REFINEMENT_PARAMETER_COUNT];
        derivative[0] = 1.0;
        let row = AppleStyleScalarRow {
            derivative,
            residual: 1.0,
            weight: 1.0,
        };
        let update =
            apple_style_solve_refinement_update(&current, &current, &jacobian, &[row]).unwrap();
        assert_eq!(update[0], APPLE_STYLE_REFINEMENT_EPSILON);
        assert!(update[1..].iter().all(|value| value.abs() < 1e-6));
    }

    #[test]
    fn refinement_inputs_fail_closed() {
        assert!(matches!(
            AppleStyleSampledJacobian::new(2),
            Err(AppleStyleDataError::InvalidRgbRaster { actual: 2 })
        ));
        let current = vec![128.0_f32; 12];
        let mut jacobian = AppleStyleSampledJacobian::new(current.len()).unwrap();
        assert!(matches!(
            jacobian.populate_parameter(
                APPLE_STYLE_REFINEMENT_PARAMETER_COUNT,
                &current,
                &current,
                1.0,
            ),
            Err(AppleStyleDataError::InvalidRefinementParameter { .. })
        ));
        assert!(matches!(
            jacobian.populate_parameter(0, &current, &current, 0.0),
            Err(AppleStyleDataError::InvalidRefinementStep)
        ));
        let mut derivative = [0.0_f64; APPLE_STYLE_REFINEMENT_PARAMETER_COUNT];
        derivative[0] = f64::NAN;
        let invalid_row = AppleStyleScalarRow {
            derivative,
            residual: 0.0,
            weight: 1.0,
        };
        assert!(matches!(
            apple_style_solve_refinement_update(&current, &current, &jacobian, &[invalid_row]),
            Err(AppleStyleDataError::InvalidScalarRow { index: 0 })
        ));
    }

    #[test]
    fn scene_policy_preserves_priority_and_native_default() {
        let food = resolve_apple_style_scene_type(AppleStyleSceneScores {
            food: 0.08,
            sunset: 1.0,
            indoor: 1.0,
            outdoor: 1.0,
        })
        .unwrap();
        assert_eq!(food.scene_type, 1);
        assert_eq!(food.selected_class, Some(AppleStyleSceneClass::Food));
        assert!(!food.native_default_applied);

        let sunset = resolve_apple_style_scene_type(AppleStyleSceneScores {
            food: 0.0,
            sunset: 0.08,
            indoor: 1.0,
            outdoor: 1.0,
        })
        .unwrap();
        assert_eq!(sunset.scene_type, 3);
        assert_eq!(sunset.selected_class, Some(AppleStyleSceneClass::Sunset));

        let indoor = resolve_apple_style_scene_type(AppleStyleSceneScores {
            food: 0.0,
            sunset: 0.0,
            indoor: 0.15,
            outdoor: 1.0,
        })
        .unwrap();
        assert_eq!(indoor.scene_type, 0);
        assert_eq!(indoor.selected_class, Some(AppleStyleSceneClass::Indoor));
        assert!(!indoor.native_default_applied);

        let outdoor = resolve_apple_style_scene_type(AppleStyleSceneScores {
            food: 0.0,
            sunset: 0.0,
            indoor: 0.0,
            outdoor: 0.15,
        })
        .unwrap();
        assert_eq!(outdoor.scene_type, 2);
        assert_eq!(outdoor.selected_class, Some(AppleStyleSceneClass::Outdoor));

        let default_scene = resolve_apple_style_scene_type(AppleStyleSceneScores {
            food: 0.079,
            sunset: 0.079,
            indoor: 0.149,
            outdoor: 0.149,
        })
        .unwrap();
        assert_eq!(default_scene.scene_type, 0);
        assert_eq!(default_scene.selected_class, None);
        assert!(default_scene.native_default_applied);
        assert!(matches!(
            resolve_apple_style_scene_type(AppleStyleSceneScores {
                food: f64::NAN,
                sunset: 0.0,
                indoor: 0.0,
                outdoor: 0.0,
            }),
            Err(AppleStyleDataError::InvalidSceneScore {
                class: AppleStyleSceneClass::Food,
                ..
            })
        ));
    }

    #[test]
    fn face_exposure_boost_matches_swift_formula_and_neutral_fallback() {
        assert_eq!(
            apple_style_face_exposure_boost(4.0, 1.0, true).unwrap(),
            2.0
        );
        assert_eq!(
            apple_style_face_exposure_boost(4.0, 1.0, false).unwrap(),
            1.0
        );
        assert_eq!(
            apple_style_face_exposure_boost(0.0, 0.0, true).unwrap(),
            1.0
        );
        assert!(matches!(
            apple_style_face_exposure_boost(-1.0, 1.0, true),
            Err(AppleStyleDataError::InvalidLightMapInput)
        ));
    }

    #[test]
    fn light_map_storage_orientation_matches_all_eight_exif_mappings() {
        let side = APPLE_STYLE_LIGHT_MAP_SIDE;
        let luma = (0..side * side)
            .map(|value| value as f32)
            .collect::<Vec<_>>();
        let expected_corners = [
            [0.0, 31.0, 992.0, 1023.0],
            [31.0, 0.0, 1023.0, 992.0],
            [1023.0, 992.0, 31.0, 0.0],
            [992.0, 1023.0, 0.0, 31.0],
            [0.0, 992.0, 31.0, 1023.0],
            [31.0, 1023.0, 0.0, 992.0],
            [1023.0, 31.0, 992.0, 0.0],
            [992.0, 0.0, 1023.0, 31.0],
        ];
        for orientation in 1..=8 {
            let request = AppleStyleLightMapRequest {
                luma: &luma,
                width: side,
                height: side,
                storage_orientation: orientation,
            };
            let data = apple_style_light_map(&request).unwrap();
            assert_eq!(data.len(), APPLE_STYLE_LIGHT_MAP_BYTE_COUNT);
            let corners = [(0, 0), (side - 1, 0), (0, side - 1), (side - 1, side - 1)];
            for (corner_index, (x, y)) in corners.into_iter().enumerate() {
                let offset = (y * side + x) * 2;
                let value = f16::from_le_bytes([data[offset], data[offset + 1]]).to_f32();
                assert_eq!(
                    value,
                    expected_corners[usize::from(orientation - 1)][corner_index]
                );
            }
        }
    }

    #[test]
    fn light_map_rejects_invalid_geometry_orientation_and_samples() {
        let empty = AppleStyleLightMapRequest {
            luma: &[],
            width: 0,
            height: 0,
            storage_orientation: 1,
        };
        assert!(matches!(
            apple_style_light_map(&empty),
            Err(AppleStyleDataError::InvalidLightMapInput)
        ));
        let invalid_orientation = AppleStyleLightMapRequest {
            luma: &[0.0],
            width: 1,
            height: 1,
            storage_orientation: 9,
        };
        assert!(matches!(
            apple_style_light_map(&invalid_orientation),
            Err(AppleStyleDataError::InvalidOrientation(9))
        ));
        let invalid_sample = AppleStyleLightMapRequest {
            luma: &[f32::NAN],
            width: 1,
            height: 1,
            storage_orientation: 1,
        };
        assert!(matches!(
            apple_style_light_map(&invalid_sample),
            Err(AppleStyleDataError::InvalidLightMapSample { index: 0 })
        ));
    }

    #[test]
    fn property_list_serialization_emits_complete_deterministic_contract() {
        let distribution = AppleStyleDistribution {
            black_point: 0.01,
            high_key: 0.99,
            p02: 0.02,
            p10: 0.10,
            p25: 0.25,
            p50: 0.50,
            p75: 0.75,
            p98: 0.98,
            white_point: 1.0,
        };
        let statistics = AppleStyleStatistics {
            linear_gtc_image: distribution,
            linear_image: distribution,
            linear_image_person_segment_based: distribution,
            linear_image_skin_based: distribution,
            tone_mapped_image: distribution,
            tone_mapped_image_blue_channel_skin_based: distribution,
            tone_mapped_image_green_channel_skin_based: distribution,
            tone_mapped_image_person_segment_based: distribution,
            tone_mapped_image_red_channel_skin_based: distribution,
            tone_mapped_image_skin_based: distribution,
        };
        let global_tone_curve = vec![0x34_u8; 516];
        let tone_light_map = vec![0x12_u8; APPLE_STYLE_LIGHT_MAP_BYTE_COUNT];
        let linear_light_map = vec![0x56_u8; APPLE_STYLE_LIGHT_MAP_BYTE_COUNT];
        let request = AppleStylePropertyListRequest {
            style_data: &apple_style_identity_data(),
            global_tone_curve: &global_tone_curve,
            baseline_exposure: 4.0,
            scene_type: 2,
            statistics: &statistics,
            people_ratio: 0.25,
            person_masks_valid_hint: -1.0,
            skin_ratio: 0.10,
            tone_light_map: &tone_light_map,
            linear_light_map: &linear_light_map,
            base_gain: 1.25,
            linear_gain: 5.0,
            original_range_min: 0.0,
            original_range_max: 4.0,
            face_exposure_boost: 1.1,
        };
        let first = apple_style_property_list(&request).unwrap();
        let second = apple_style_property_list(&request).unwrap();
        assert_eq!(first, second);
        assert!(first.starts_with(b"bplist00"));
        assert!(first
            .windows(APPLE_STYLE_BYTE_COUNT)
            .any(|window| { window == request.style_data }));
        for key in [
            b"LinearGTCImage".as_slice(),
            b"LinearImage".as_slice(),
            b"ToneMappedImage".as_slice(),
            b"PeopleRatio".as_slice(),
            b"OriginalRangeMax".as_slice(),
        ] {
            assert!(first.windows(key.len()).any(|window| window == key));
        }
        let trailer = first.len() - 32;
        let object_count = u64::from_be_bytes(first[trailer + 8..trailer + 16].try_into().unwrap());
        let top_object = u64::from_be_bytes(first[trailer + 16..trailer + 24].try_into().unwrap());
        let offset_table =
            u64::from_be_bytes(first[trailer + 24..trailer + 32].try_into().unwrap());
        assert!(object_count > 1);
        assert_eq!(top_object, object_count - 1);
        assert!(offset_table < trailer as u64);
    }

    #[test]
    fn property_list_rejects_incomplete_resources_and_nonfinite_statistics() {
        let distribution = AppleStyleDistribution {
            black_point: 0.0,
            high_key: 1.0,
            p02: 0.0,
            p10: 0.1,
            p25: 0.25,
            p50: 0.5,
            p75: 0.75,
            p98: 0.98,
            white_point: 1.0,
        };
        let statistics = AppleStyleStatistics {
            linear_gtc_image: distribution,
            linear_image: distribution,
            linear_image_person_segment_based: distribution,
            linear_image_skin_based: distribution,
            tone_mapped_image: distribution,
            tone_mapped_image_blue_channel_skin_based: distribution,
            tone_mapped_image_green_channel_skin_based: distribution,
            tone_mapped_image_person_segment_based: distribution,
            tone_mapped_image_red_channel_skin_based: distribution,
            tone_mapped_image_skin_based: distribution,
        };
        let mut invalid_distribution = distribution;
        invalid_distribution.p50 = f64::NAN;
        let invalid_statistics = AppleStyleStatistics {
            linear_gtc_image: invalid_distribution,
            ..statistics
        };
        let style_data = apple_style_identity_data();
        let global_tone_curve = vec![0_u8; 516];
        let light_map = vec![0_u8; APPLE_STYLE_LIGHT_MAP_BYTE_COUNT];
        let valid = AppleStylePropertyListRequest {
            style_data: &style_data,
            global_tone_curve: &global_tone_curve,
            baseline_exposure: 4.0,
            scene_type: 0,
            statistics: &statistics,
            people_ratio: 0.0,
            person_masks_valid_hint: -1.0,
            skin_ratio: 0.0,
            tone_light_map: &light_map,
            linear_light_map: &light_map,
            base_gain: 1.0,
            linear_gain: 4.0,
            original_range_min: 0.0,
            original_range_max: 1.0,
            face_exposure_boost: 1.0,
        };
        let invalid_curve = AppleStylePropertyListRequest {
            global_tone_curve: &global_tone_curve[..515],
            ..valid
        };
        assert!(matches!(
            apple_style_property_list(&invalid_curve),
            Err(AppleStyleDataError::InvalidPropertyListInput)
        ));
        let invalid_stats = AppleStylePropertyListRequest {
            statistics: &invalid_statistics,
            ..valid
        };
        assert!(matches!(
            apple_style_property_list(&invalid_stats),
            Err(AppleStyleDataError::InvalidPropertyListInput)
        ));
    }
}
