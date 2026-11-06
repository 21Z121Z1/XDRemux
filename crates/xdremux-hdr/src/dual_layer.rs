//! Relative linear-light reconstruction for the explicit BT.2020 profiles.
use xdremux_metadata::{SdrColorSpace, UltraHdrGainMapMetadata};

use crate::{HdrError, Result, SceneLinearRgb};

/// Tightly packed normalized codes, retaining the decoder's component precision.
#[derive(Debug, Clone, Copy)]
pub struct GainMapSamples<'a> {
    pub width: usize,
    pub height: usize,
    pub channels: usize,
    pub data: &'a [f32],
}

/// Supported D65 linear working spaces. Gain Map compression colour tags do
/// not select this space; it comes from the base/alternate image binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinearRgbPrimaries {
    Bt709,
    DisplayP3,
    Bt2020,
}

impl From<SdrColorSpace> for LinearRgbPrimaries {
    fn from(space: SdrColorSpace) -> Self {
        match space {
            SdrColorSpace::Srgb => Self::Bt709,
            SdrColorSpace::DisplayP3 => Self::DisplayP3,
        }
    }
}

impl LinearRgbPrimaries {
    fn to_bt2020(self, rgb: [f64; 3]) -> [f64; 3] {
        match self {
            Self::Bt709 => sdr_linear_to_bt2020(rgb, SdrColorSpace::Srgb),
            Self::DisplayP3 => sdr_linear_to_bt2020(rgb, SdrColorSpace::DisplayP3),
            Self::Bt2020 => rgb,
        }
    }

    fn convert_from_bt2020(self, rgb: [f64; 3]) -> [f64; 3] {
        let matrix = match self {
            Self::Bt2020 => return rgb,
            Self::Bt709 => [
                [
                    1.6604910021084345,
                    -0.5876411387885495,
                    -0.072_849_863_319_885,
                ],
                [
                    -0.1245504745215907,
                    1.1328998971259598,
                    -0.00834942260436948,
                ],
                [
                    -0.01815076335490524,
                    -0.1005788980080074,
                    1.1187296613629127,
                ],
            ],
            Self::DisplayP3 => [
                [
                    1.3435782525843323,
                    -0.2821796705261357,
                    -0.06139858205819629,
                ],
                [
                    -0.06529745278911953,
                    1.0757879158485744,
                    -0.010490463059454953,
                ],
                [
                    0.0028217872617009505,
                    -0.019598494524494057,
                    1.0167767072627931,
                ],
            ],
        };
        matrix.map(|row| row.into_iter().zip(rgb).map(|(a, b)| a * b).sum())
    }
}

/// D65 matrices derived from the primary chromaticities, without gamut clipping.
pub fn sdr_linear_to_bt2020(rgb: [f64; 3], space: SdrColorSpace) -> [f64; 3] {
    let matrix = match space {
        SdrColorSpace::Srgb => [
            [0.6274038959346991, 0.3292830383778837, 0.04331306568741723],
            [0.06909728935823208, 0.9195403950754587, 0.01136231556630918],
            [0.01639143887515028, 0.08801330787722576, 0.895595253247624],
        ],
        SdrColorSpace::DisplayP3 => [
            [0.7538330343617218, 0.1985973690526163, 0.04756959658566188],
            [0.04574384896535833, 0.9417772198116935, 0.01247893122294812],
            [
                -0.001210340354518324,
                0.01760171730108989,
                0.9836086230534284,
            ],
        ],
    };
    matrix.map(|row| row.into_iter().zip(rgb).map(|(a, b)| a * b).sum())
}

pub fn decode_srgb(value: f64) -> f64 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// Upsample decoded log gains at co-sited coordinates. Gamma inversion occurs
/// at the stored sample sites, before interpolation; gains are exponentiated
/// only after interpolation. The caller validates the raster and metadata.
fn sample_log_gain(
    gain: GainMapSamples<'_>,
    metadata: &UltraHdrGainMapMetadata,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    channel: usize,
) -> f64 {
    let gx = (x as f64 * gain.width as f64 / width as f64).min((gain.width - 1) as f64);
    let gy = (y as f64 * gain.height as f64 / height as f64).min((gain.height - 1) as f64);
    let (x0, y0) = (gx.floor() as usize, gy.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(gain.width - 1), (y0 + 1).min(gain.height - 1));
    let (tx, ty) = (gx - x0 as f64, gy - y0 as f64);
    let m = if metadata.channel_count == 1 {
        0
    } else {
        channel
    };
    let c = if gain.channels == 1 { 0 } else { channel };
    let at = |px: usize, py: usize| {
        let code = f64::from(gain.data[(py * gain.width + px) * gain.channels + c]);
        metadata.gain_map_min[m]
            + (metadata.gain_map_max[m] - metadata.gain_map_min[m])
                * code.powf(1.0 / metadata.gamma[m])
    };
    let top = at(x0, y0) * (1.0 - tx) + at(x1, y0) * tx;
    let bottom = at(x0, y1) * (1.0 - tx) + at(x1, y1) * tx;
    top * (1.0 - ty) + bottom * ty
}

/// Reconstruct in the declared application primaries, subtract the alternate
/// offset there, then convert to BT.2020. Inputs and output use relative linear
/// RGB; no display luminance or device tone mapping is inferred.
pub fn reconstruct_bt2020_hdr(
    base: &SceneLinearRgb,
    gain: GainMapSamples<'_>,
    metadata: &UltraHdrGainMapMetadata,
    base_space: SdrColorSpace,
) -> Result<SceneLinearRgb> {
    reconstruct_bt2020_hdr_in_space(
        base,
        gain,
        metadata,
        base_space,
        if metadata.use_base_color_space {
            base_space.into()
        } else {
            LinearRgbPrimaries::Bt2020
        },
    )
}

/// Reconstruct with application primaries already resolved by the container
/// reader, including the JPEG rule that a missing alternate ICC uses the base.
pub fn reconstruct_bt2020_hdr_in_space(
    base: &SceneLinearRgb,
    gain: GainMapSamples<'_>,
    metadata: &UltraHdrGainMapMetadata,
    base_space: SdrColorSpace,
    application_primaries: LinearRgbPrimaries,
) -> Result<SceneLinearRgb> {
    crate::scene_linear::validate_linear(base)?;
    let expected = gain
        .width
        .checked_mul(gain.height)
        .and_then(|n| n.checked_mul(gain.channels));
    if gain.width == 0
        || gain.height == 0
        || !matches!(gain.channels, 1 | 3)
        || expected != Some(gain.data.len())
        || gain
            .data
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
    {
        return Err(HdrError::invalid(
            "HDR reconstruction",
            "invalid normalized Gain Map geometry or samples",
        ));
    }
    metadata
        .validate()
        .map_err(|e| HdrError::invalid("HDR reconstruction", e.to_string()))?;
    if metadata.channel_count == 3 && gain.channels != 3 {
        return Err(HdrError::invalid(
            "HDR reconstruction",
            "RGB metadata requires an RGB Gain Map",
        ));
    }
    let weight = crate::iso21496_application_weight(
        metadata.hdr_capacity_min,
        metadata.hdr_capacity_max,
        metadata.hdr_capacity_max,
    )?;
    let mut samples = Vec::with_capacity(base.samples.len());
    for y in 0..base.height {
        for x in 0..base.width {
            let i = (y * base.width + x) * 3;
            let mut pixel = std::array::from_fn(|c| f64::from(base.samples[i + c]));
            if application_primaries != LinearRgbPrimaries::from(base_space) {
                pixel = application_primaries
                    .convert_from_bt2020(sdr_linear_to_bt2020(pixel, base_space));
            }
            for (c, value) in pixel.iter_mut().enumerate() {
                let m = if metadata.channel_count == 1 { 0 } else { c };
                let log_gain = sample_log_gain(gain, metadata, x, y, base.width, base.height, c);
                *value = (*value + metadata.offset_sdr[m]) * (weight * log_gain).exp2()
                    - metadata.offset_hdr[m];
            }
            pixel = application_primaries.to_bt2020(pixel);
            for value in pixel {
                let value = value as f32;
                if !value.is_finite() {
                    return Err(HdrError::invalid(
                        "HDR reconstruction",
                        "reconstructed pixel exceeds finite RGB range",
                    ));
                }
                samples.push(value);
            }
        }
    }
    Ok(SceneLinearRgb {
        width: base.width,
        height: base.height,
        samples,
    })
}

fn unit(value: f64) -> Result<f64> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(HdrError::invalid(
            "HDR transfer",
            "normalized input must be finite and within 0 through 1",
        ));
    }
    Ok(value)
}

/// PQ OETF, normalized to 10000 cd/m2. This function does not choose SDR white.
pub fn encode_pq(normalized_luminance: f64) -> Result<f64> {
    let p = unit(normalized_luminance)?.powf(2610.0 / 16384.0);
    Ok(
        ((3424.0 / 4096.0 + (2413.0 / 128.0) * p) / (1.0 + (2392.0 / 128.0) * p))
            .powf(2523.0 / 32.0),
    )
}

pub fn decode_pq(code: f64) -> Result<f64> {
    let p = unit(code)?.powf(32.0 / 2523.0);
    Ok(
        ((p - 3424.0 / 4096.0).max(0.0) / (2413.0 / 128.0 - (2392.0 / 128.0) * p))
            .powf(16384.0 / 2610.0),
    )
}

/// HLG scene OETF. Display OOTF and viewing conditions are separate operations.
pub fn encode_hlg(linear: f64) -> Result<f64> {
    let linear = unit(linear)?;
    let a = 0.17883277;
    let b = 1.0 - 4.0 * a;
    let c = 0.5 - a * (4.0_f64 * a).ln();
    Ok(if linear <= 1.0 / 12.0 {
        (3.0 * linear).sqrt()
    } else {
        a * (12.0 * linear - b).ln() + c
    })
}

pub fn decode_hlg(code: f64) -> Result<f64> {
    let code = unit(code)?;
    let a = 0.17883277;
    let b = 1.0 - 4.0 * a;
    let c = 0.5 - a * (4.0_f64 * a).ln();
    Ok(if code <= 0.5 {
        code * code / 3.0
    } else {
        (((code - c) / a).exp() + b) / 12.0
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata() -> UltraHdrGainMapMetadata {
        let mut bytes = vec![0, 0, 0, 0, 0, 0x40];
        for (n, d) in [
            (0_i32, 1_u32),
            (3, 1),
            (-1, 1),
            (3, 1),
            (2, 1),
            (1, 64),
            (1, 64),
        ] {
            bytes.extend_from_slice(&n.to_be_bytes());
            bytes.extend_from_slice(&d.to_be_bytes());
        }
        xdremux_metadata::parse_heif_tmap_metadata(&bytes).unwrap()
    }

    #[test]
    fn wolfram_gain_gamma_offset_references() {
        let channel = crate::IsoGainChannel {
            minimum_log2: -1.0,
            maximum_log2: 3.0,
            gamma: 2.0,
            base_offset: 1.0 / 64.0,
            alternate_offset: 1.0 / 64.0,
        };
        for (code, expected) in [
            (0.0, 0.2421875),
            (0.25, 1.015625),
            (0.5, 1.8156154604955352),
            (1.0, 4.109375),
        ] {
            let actual = crate::apply_iso_gain_sample(0.5, code, &channel, 1.0).unwrap();
            assert!((actual - expected).abs() < 2e-14);
        }
    }

    #[test]
    fn co_sited_sampling_inverts_gamma_before_interpolating() {
        let base = SceneLinearRgb {
            width: 4,
            height: 1,
            samples: vec![0.5; 12],
        };
        let codes = [0.0, 1.0];
        let gain = GainMapSamples {
            width: 2,
            height: 1,
            channels: 1,
            data: &codes,
        };
        let output =
            reconstruct_bt2020_hdr(&base, gain, &metadata(), SdrColorSpace::DisplayP3).unwrap();
        for (i, expected) in [0.2421875, 1.015625, 4.109375, 4.109375]
            .into_iter()
            .enumerate()
        {
            assert!((f64::from(output.samples[i * 3]) - expected).abs() < 1e-6);
        }
        let mut metadata = metadata();
        // The exact rational metadata is immutable; build independent values.
        let mut payload = metadata.to_tmap_payload(true).unwrap();
        // 16/255 and 255/255, with gamma 2, distinguish decoding-before-resize
        // from resizing the normalized codes before gamma inversion.
        payload[22..26].copy_from_slice(&(-1_i32).to_be_bytes());
        metadata = xdremux_metadata::parse_heif_tmap_metadata(&payload).unwrap();
        let codes = [16.0 / 255.0, 1.0];
        let output = reconstruct_bt2020_hdr(
            &base,
            GainMapSamples {
                data: &codes,
                ..gain
            },
            &metadata,
            SdrColorSpace::DisplayP3,
        )
        .unwrap();
        let middle_log = (-1.0 + 4.0 * (16.0_f64 / 255.0).sqrt() + 3.0) / 2.0;
        assert!(
            (f64::from(output.samples[3]) - ((0.5 + 1.0 / 64.0) * middle_log.exp2() - 1.0 / 64.0))
                .abs()
                < 1e-6
        );
    }

    #[test]
    fn application_primaries_precede_offsets_and_gamut_conversion() {
        let base = SceneLinearRgb {
            width: 1,
            height: 1,
            samples: vec![0.5, 0.25, 0.125],
        };
        let gain = GainMapSamples {
            width: 1,
            height: 1,
            channels: 3,
            data: &[0.0, 128.0 / 255.0, 1.0],
        };
        let mono = metadata().to_tmap_payload(true).unwrap();
        let mut bytes = mono[..22].to_vec();
        bytes[5] |= 0x80;
        for _ in 0..3 {
            bytes.extend_from_slice(&mono[22..]);
        }
        let meta = xdremux_metadata::parse_heif_tmap_metadata(&bytes).unwrap();
        let a = reconstruct_bt2020_hdr(&base, gain, &meta, SdrColorSpace::DisplayP3).unwrap();
        let meta =
            xdremux_metadata::parse_heif_tmap_metadata(&meta.to_tmap_payload(false).unwrap())
                .unwrap();
        let b = reconstruct_bt2020_hdr(&base, gain, &meta, SdrColorSpace::DisplayP3).unwrap();
        assert!((a.samples[0] - b.samples[0]).abs() > 0.01);
        let white = sdr_linear_to_bt2020([1.0; 3], SdrColorSpace::DisplayP3);
        assert!(white.iter().all(|v| (v - 1.0).abs() < 1e-14));
        assert!(sdr_linear_to_bt2020([1.0, 0.0, 0.0], SdrColorSpace::DisplayP3)[2] < 0.0);
    }

    #[test]
    fn pq_hlg_match_independent_values_and_round_trip() {
        for (nits, code) in [
            (100.0, 0.5080784215173949),
            (203.0, 0.5806888810416078),
            (1000.0, 0.7518270962470418),
            (10000.0, 1.0),
        ] {
            assert!((encode_pq(nits / 10000.0).unwrap() - code).abs() < 2e-13);
            assert!((decode_pq(code).unwrap() * 10000.0 - nits).abs() < 1e-7);
        }
        assert!((encode_hlg(0.25).unwrap() - 0.7385492675953893).abs() < 1e-14);
        for x in [0.0, 1.0 / 12.0, 0.25, 1.0] {
            assert!((decode_hlg(encode_hlg(x).unwrap()).unwrap() - x).abs() < 1e-14);
        }
        for x in [-1.0, 1.01, f64::NAN, f64::INFINITY] {
            assert!(encode_pq(x).is_err());
            assert!(decode_pq(x).is_err());
            assert!(encode_hlg(x).is_err());
            assert!(decode_hlg(x).is_err());
        }
    }

    #[test]
    fn application_gamut_conversion_preserves_extended_components() {
        for space in [
            LinearRgbPrimaries::Bt709,
            LinearRgbPrimaries::DisplayP3,
            LinearRgbPrimaries::Bt2020,
        ] {
            for rgb in [[1.0; 3], [-0.25, 2.0, 0.4], [0.0, 0.0, 0.0]] {
                let actual = space.convert_from_bt2020(space.to_bt2020(rgb));
                assert!(actual
                    .into_iter()
                    .zip(rgb)
                    .all(|(a, b)| (a - b).abs() < 3e-14));
            }
        }
        let base = SceneLinearRgb {
            width: 1,
            height: 1,
            samples: vec![0.5, 0.25, 0.125],
        };
        let gain = GainMapSamples {
            width: 1,
            height: 1,
            channels: 3,
            data: &[0.0, 0.25, 1.0],
        };
        let actual = reconstruct_bt2020_hdr_in_space(
            &base,
            gain,
            &metadata(),
            SdrColorSpace::Srgb,
            LinearRgbPrimaries::DisplayP3,
        )
        .unwrap();
        // Independent NumPy chromaticity-matrix calculation: change sRGB to
        // P3, apply gains [0.5, 2, 8] and offsets there, then change to BT.2020.
        let expected = [0.330191872416722, 0.5266848543593263, 1.2219111855804998];
        assert!(actual
            .samples
            .iter()
            .zip(expected)
            .all(|(a, b)| (f64::from(*a) - b).abs() < 1e-7));
    }
}
