use crate::{HdrError, ResolvedScale, Result};

/// Tightly packed scene-linear Display P3 RGB.
///
/// Samples are relative luminance in the base image's linear encoding. This
/// buffer does not carry a nit calibration or a transfer function.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneLinearRgb {
    pub width: usize,
    pub height: usize,
    pub samples: Vec<f32>,
}

/// 8-bit gain-map codes. One channel is monochrome; three channels are RGB.
#[derive(Debug, Clone, Copy)]
pub struct GainMapCodes<'a> {
    pub width: usize,
    pub height: usize,
    pub bytes_per_row: usize,
    pub channels: usize,
    pub data: &'a [u8],
}

/// One ISO 21496-1 / Apple tmap channel after the product has stored the
/// minimum and maximum as log2 boosts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IsoGainChannel {
    pub minimum_log2: f64,
    pub maximum_log2: f64,
    pub gamma: f64,
    pub base_offset: f64,
    pub alternate_offset: f64,
}

/// Gain-map application for a scene-linear alternate image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IsoGainApplication {
    pub channels: [IsoGainChannel; 3],
    /// ISO 21496-1 weight on the log2 gain. The full alternate image uses 1.
    pub weight: f64,
}

/// ISO 21496-1 application weight for a target log2 headroom.
///
/// Headrooms are already log2 values, matching [`ResolvedScale`]. The full
/// alternate image is the target whose headroom equals `alternate_headroom`,
/// which produces weight 1. A target between the base and alternate headrooms
/// scales the log2 gain and does not resample it.
pub fn iso21496_application_weight(
    base_headroom: f64,
    alternate_headroom: f64,
    target_headroom: f64,
) -> Result<f64> {
    if !base_headroom.is_finite() || !alternate_headroom.is_finite() || !target_headroom.is_finite()
    {
        return Err(HdrError::invalid(
            "gain map weight",
            "headroom values must be finite",
        ));
    }
    let span = alternate_headroom - base_headroom;
    if span == 0.0 {
        if target_headroom == alternate_headroom {
            return Ok(1.0);
        }
        return Err(HdrError::invalid(
            "gain map weight",
            "base and alternate headroom are equal, so a different target has no weight",
        ));
    }
    Ok(((target_headroom - base_headroom) / span).clamp(0.0, 1.0))
}

/// Parameters that reconstruct the full alternate image described by `scale`.
pub fn iso_gain_application_for_alternate(scale: &ResolvedScale) -> Result<IsoGainApplication> {
    let weight = iso21496_application_weight(
        scale.base_headroom,
        scale.alternate_headroom,
        scale.alternate_headroom,
    )?;
    let mut channels = [IsoGainChannel {
        minimum_log2: 0.0,
        maximum_log2: 0.0,
        gamma: 1.0,
        base_offset: 0.0,
        alternate_offset: 0.0,
    }; 3];
    for (index, channel) in channels.iter_mut().enumerate() {
        let parameter_index = if scale.channel_count <= 1 { 0 } else { index };
        *channel = IsoGainChannel {
            minimum_log2: channel_value(
                &scale.per_channel_gain_map_min,
                parameter_index,
                scale.gain_map_min,
            ),
            maximum_log2: channel_value(
                &scale.per_channel_gain_map_max,
                parameter_index,
                scale.gain_map_max,
            ),
            gamma: channel_value(&scale.per_channel_gamma, parameter_index, scale.gamma),
            base_offset: channel_value(
                &scale.per_channel_base_offset,
                parameter_index,
                scale.epsilon_sdr,
            ),
            alternate_offset: channel_value(
                &scale.per_channel_alternate_offset,
                parameter_index,
                scale.epsilon_hdr,
            ),
        };
        validate_channel(channel)?;
    }
    Ok(IsoGainApplication { channels, weight })
}

/// Apply one gain-map code to one scene-linear SDR sample.
///
/// The code is the stored gain-map sample in \[0, 1\], already resampled.
/// Interpolation belongs in this code domain. Converting to a linear gain
/// before interpolation does not match the ISO / `_flexMap` application.
///
/// ```text
/// shaped = code ^ (1 / gamma)
/// log2_gain = minimum_log2 + shaped * (maximum_log2 - minimum_log2)
/// hdr = exp2(weight * log2_gain) * (sdr + base_offset) - alternate_offset
/// ```
///
/// `1 / gamma` is the ISO 21496-1 and libultrahdr decode exponent and the
/// `invGamma` argument of CoreImage `_flexMap` on macOS 26A434. Encoding a
/// gain map raises the normalized log boost to `gamma`; applying it inverts
/// that exponent.
pub fn apply_iso_gain_sample(
    sdr_linear: f64,
    gain_code: f64,
    channel: &IsoGainChannel,
    weight: f64,
) -> Result<f64> {
    if !sdr_linear.is_finite() || !gain_code.is_finite() {
        return Err(HdrError::invalid(
            "gain map application",
            "SDR sample and gain code must be finite",
        ));
    }
    validate_channel(channel)?;
    if !weight.is_finite() || !(0.0..=1.0).contains(&weight) {
        return Err(HdrError::invalid(
            "gain map application",
            "application weight must be finite and within 0 through 1",
        ));
    }
    let code = gain_code.clamp(0.0, 1.0);
    let shaped = if channel.gamma == 1.0 {
        code
    } else {
        code.powf(1.0 / channel.gamma)
    };
    let log_gain = channel.minimum_log2 + shaped * (channel.maximum_log2 - channel.minimum_log2);
    let hdr =
        (weight * log_gain).exp2() * (sdr_linear + channel.base_offset) - channel.alternate_offset;
    if !hdr.is_finite() {
        return Err(HdrError::invalid(
            "gain map application",
            "reconstructed HDR sample is non-finite",
        ));
    }
    Ok(hdr)
}

/// Reconstruct scene-linear HDR at the SDR image's resolution.
///
/// Each base pixel samples the gain map bilinearly in code space, then
/// [`apply_iso_gain_sample`] runs. The gain map is not resized as a linear
/// gain, and the SDR image is not resized before the gain is applied.
pub fn reconstruct_scene_linear_hdr(
    sdr: &SceneLinearRgb,
    gain_map: GainMapCodes<'_>,
    application: &IsoGainApplication,
) -> Result<SceneLinearRgb> {
    validate_linear(sdr)?;
    validate_gain_map(gain_map)?;
    for channel in &application.channels {
        validate_channel(channel)?;
    }
    if !application.weight.is_finite() || !(0.0..=1.0).contains(&application.weight) {
        return Err(HdrError::invalid(
            "gain map application",
            "application weight must be finite and within 0 through 1",
        ));
    }
    let pixel_count = sdr
        .width
        .checked_mul(sdr.height)
        .ok_or_else(|| HdrError::invalid("scene linear image", "pixel count overflows"))?;
    let mut samples = Vec::with_capacity(pixel_count.saturating_mul(3));
    for y in 0..sdr.height {
        for x in 0..sdr.width {
            let source = (y * sdr.width + x) * 3;
            for (component, sample) in sdr.samples[source..source + 3].iter().enumerate() {
                let code = sample_gain_code(gain_map, x, y, sdr.width, sdr.height, component);
                let hdr = apply_iso_gain_sample(
                    f64::from(*sample),
                    code,
                    &application.channels[component],
                    application.weight,
                )?;
                let stored = hdr as f32;
                if !stored.is_finite() {
                    return Err(HdrError::invalid(
                        "gain map application",
                        "reconstructed HDR sample is outside the f32 range",
                    ));
                }
                samples.push(stored);
            }
        }
    }
    Ok(SceneLinearRgb {
        width: sdr.width,
        height: sdr.height,
        samples,
    })
}

/// Even-rounded fit used by the Styles Linear Thumbnail.
///
/// The longer edge is at most `maximum_edge`, both edges are even, and the
/// aspect ratio follows the source. A source that is already small stays at
/// its even-rounded size instead of being enlarged to the maximum.
pub fn fitted_even_dimensions(
    width: usize,
    height: usize,
    maximum_edge: u32,
) -> Result<(usize, usize)> {
    if width == 0 || height == 0 || maximum_edge < 2 {
        return Err(HdrError::invalid(
            "scene linear resize",
            "dimensions must be positive and the maximum edge must be at least 2",
        ));
    }
    let maximum = f64::from(maximum_edge);
    let scale = (maximum / width as f64)
        .min(maximum / height as f64)
        .min(1.0);
    let target_width = ((width as f64 * scale / 2.0).round() * 2.0).clamp(2.0, maximum) as usize;
    let target_height = ((height as f64 * scale / 2.0).round() * 2.0).clamp(2.0, maximum) as usize;
    Ok((target_width, target_height))
}

/// Area (box) resample in linear light.
///
/// Each destination pixel is the coverage-weighted mean of the source pixels
/// inside its footprint. This is the resize that follows full-resolution HDR
/// reconstruction. It is not a nearest-neighbor pick and it is not applied to
/// the encoded SDR or the gain-map codes.
pub fn area_resize_linear_rgb(image: &SceneLinearRgb, maximum_edge: u32) -> Result<SceneLinearRgb> {
    validate_linear(image)?;
    let (target_width, target_height) =
        fitted_even_dimensions(image.width, image.height, maximum_edge)?;
    if target_width == image.width && target_height == image.height {
        return Ok(image.clone());
    }
    let mut samples = vec![0.0_f32; target_width * target_height * 3];
    for y in 0..target_height {
        let y0 = y as f64 * image.height as f64 / target_height as f64;
        let y1 = (y + 1) as f64 * image.height as f64 / target_height as f64;
        for x in 0..target_width {
            let x0 = x as f64 * image.width as f64 / target_width as f64;
            let x1 = (x + 1) as f64 * image.width as f64 / target_width as f64;
            let mut accumulator = [0.0_f64; 3];
            let mut coverage = 0.0_f64;
            let row_start = y0.floor() as usize;
            let row_end = (y1.ceil() as usize).min(image.height);
            let column_start = x0.floor() as usize;
            let column_end = (x1.ceil() as usize).min(image.width);
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
                    let offset = (source_y * image.width + source_x) * 3;
                    for (accumulated, sample) in accumulator
                        .iter_mut()
                        .zip(&image.samples[offset..offset + 3])
                    {
                        *accumulated += f64::from(*sample) * weight;
                    }
                    coverage += weight;
                }
            }
            if coverage <= 0.0 {
                return Err(HdrError::invalid(
                    "scene linear resize",
                    "destination pixel has no source coverage",
                ));
            }
            let destination = (y * target_width + x) * 3;
            for (sample, accumulated) in samples[destination..destination + 3]
                .iter_mut()
                .zip(accumulator)
            {
                let value = accumulated / coverage;
                if !value.is_finite() {
                    return Err(HdrError::invalid(
                        "scene linear resize",
                        "resized sample is non-finite",
                    ));
                }
                *sample = value as f32;
            }
        }
    }
    Ok(SceneLinearRgb {
        width: target_width,
        height: target_height,
        samples,
    })
}

fn channel_value(values: &[f64], index: usize, fallback: f64) -> f64 {
    values.get(index).copied().unwrap_or(fallback)
}

fn validate_channel(channel: &IsoGainChannel) -> Result<()> {
    if !channel.minimum_log2.is_finite()
        || !channel.maximum_log2.is_finite()
        || !channel.gamma.is_finite()
        || channel.gamma <= 0.0
        || !channel.base_offset.is_finite()
        || !channel.alternate_offset.is_finite()
    {
        return Err(HdrError::invalid(
            "gain map application",
            "channel minimum, maximum, positive gamma, and offsets must be finite",
        ));
    }
    Ok(())
}

fn validate_linear(image: &SceneLinearRgb) -> Result<()> {
    if image.width == 0 || image.height == 0 {
        return Err(HdrError::invalid(
            "scene linear image",
            "dimensions must be non-zero",
        ));
    }
    let pixel_count = image
        .width
        .checked_mul(image.height)
        .ok_or_else(|| HdrError::invalid("scene linear image", "pixel count overflows"))?;
    let sample_count = pixel_count
        .checked_mul(3)
        .ok_or_else(|| HdrError::invalid("scene linear image", "sample count overflows"))?;
    if image.samples.len() != sample_count {
        return Err(HdrError::invalid(
            "scene linear image",
            format!(
                "expected {sample_count} RGB samples, got {}",
                image.samples.len()
            ),
        ));
    }
    if image.samples.iter().any(|sample| !sample.is_finite()) {
        return Err(HdrError::invalid(
            "scene linear image",
            "samples must be finite",
        ));
    }
    Ok(())
}

fn validate_gain_map(gain_map: GainMapCodes<'_>) -> Result<()> {
    if gain_map.width == 0 || gain_map.height == 0 {
        return Err(HdrError::invalid(
            "gain map codes",
            "dimensions must be non-zero",
        ));
    }
    if gain_map.channels != 1 && gain_map.channels != 3 {
        return Err(HdrError::invalid(
            "gain map codes",
            "gain map must have 1 or 3 channels",
        ));
    }
    let row_bytes = gain_map
        .width
        .checked_mul(gain_map.channels)
        .ok_or_else(|| HdrError::invalid("gain map codes", "row size overflows"))?;
    if gain_map.bytes_per_row < row_bytes {
        return Err(HdrError::invalid(
            "gain map codes",
            "bytes_per_row is smaller than width times channels",
        ));
    }
    let required = gain_map
        .bytes_per_row
        .checked_mul(gain_map.height)
        .ok_or_else(|| HdrError::invalid("gain map codes", "raster size overflows"))?;
    if gain_map.data.len() < required {
        return Err(HdrError::invalid(
            "gain map codes",
            format!(
                "data is too short: need at least {required} bytes, got {}",
                gain_map.data.len()
            ),
        ));
    }
    Ok(())
}

fn sample_gain_code(
    gain_map: GainMapCodes<'_>,
    target_x: usize,
    target_y: usize,
    target_width: usize,
    target_height: usize,
    component: usize,
) -> f64 {
    let x = (((target_x as f64 + 0.5) * gain_map.width as f64 / target_width as f64) - 0.5)
        .clamp(0.0, gain_map.width.saturating_sub(1) as f64);
    let y = (((target_y as f64 + 0.5) * gain_map.height as f64 / target_height as f64) - 0.5)
        .clamp(0.0, gain_map.height.saturating_sub(1) as f64);
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(gain_map.width - 1);
    let y1 = (y0 + 1).min(gain_map.height - 1);
    let x_fraction = x - x0 as f64;
    let y_fraction = y - y0 as f64;
    let channel = if gain_map.channels == 1 {
        0
    } else {
        component.min(2)
    };
    let at = |sample_x: usize, sample_y: usize| {
        let offset = sample_y * gain_map.bytes_per_row + sample_x * gain_map.channels + channel;
        f64::from(gain_map.data[offset]) / 255.0
    };
    let top = at(x0, y0) * (1.0 - x_fraction) + at(x1, y0) * x_fraction;
    let bottom = at(x0, y1) * (1.0 - x_fraction) + at(x1, y1) * x_fraction;
    top * (1.0 - y_fraction) + bottom * y_fraction
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel(gamma: f64, base_offset: f64, alternate_offset: f64) -> IsoGainChannel {
        IsoGainChannel {
            minimum_log2: 0.0,
            maximum_log2: 1.0,
            gamma,
            base_offset,
            alternate_offset,
        }
    }

    fn application(channel: IsoGainChannel, weight: f64) -> IsoGainApplication {
        IsoGainApplication {
            channels: [channel; 3],
            weight,
        }
    }

    #[test]
    fn gain_application_matches_known_hdr_for_gamma_offsets_and_weight() {
        let identity = channel(1.0, 0.0, 0.0);
        let hdr = apply_iso_gain_sample(0.25, 0.5, &identity, 1.0).unwrap();
        assert!((hdr - 0.25 * 2.0_f64.powf(0.5)).abs() < 1e-12);

        // gamma 2 decodes with exponent 1/2. code 0.25 becomes 0.5, not 0.0625.
        let curved = channel(2.0, 0.0, 0.0);
        let curved_hdr = apply_iso_gain_sample(0.25, 0.25, &curved, 1.0).unwrap();
        assert!((curved_hdr - 0.25 * 2.0_f64.sqrt()).abs() < 1e-12);
        let encoded_exponent = apply_iso_gain_sample(0.25, 0.25_f64.powf(2.0), &curved, 1.0);
        assert!((encoded_exponent.unwrap() - curved_hdr).abs() > 1e-6);

        let offsets = channel(1.0, 1.0 / 64.0, 1.0 / 64.0);
        let black = apply_iso_gain_sample(0.0, 0.0, &offsets, 1.0).unwrap();
        assert!(black.abs() < 1e-15);
        let lifted = apply_iso_gain_sample(0.0, 1.0, &offsets, 1.0).unwrap();
        assert!((lifted - (2.0 / 64.0 - 1.0 / 64.0)).abs() < 1e-12);

        let weighted = apply_iso_gain_sample(0.25, 1.0, &identity, 0.5).unwrap();
        assert!((weighted - 0.25 * 2.0_f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn headroom_weight_selects_the_alternate_image_or_a_fraction() {
        assert_eq!(iso21496_application_weight(0.0, 2.0, 2.0).unwrap(), 1.0);
        assert_eq!(iso21496_application_weight(0.0, 2.0, 1.0).unwrap(), 0.5);
        assert_eq!(iso21496_application_weight(0.0, 2.0, 0.0).unwrap(), 0.0);
        assert_eq!(iso21496_application_weight(0.0, 2.0, 4.0).unwrap(), 1.0);
        assert_eq!(iso21496_application_weight(0.0, 0.0, 0.0).unwrap(), 1.0);
        assert!(iso21496_application_weight(0.0, 0.0, 1.0).is_err());
        assert!(iso21496_application_weight(f64::NAN, 1.0, 1.0).is_err());
    }

    #[test]
    fn alternate_application_uses_resolved_channel_metadata_and_full_weight() {
        let mut info = vec![0.0; 20];
        info[0..3].copy_from_slice(&[1.0, 1.0, 1.0]);
        info[4..7].copy_from_slice(&[2.0, 4.0, 8.0]);
        info[7..10].copy_from_slice(&[2.0, 1.0, 0.5]);
        info[10..13].copy_from_slice(&[0.01, 0.02, 0.03]);
        info[13..16].copy_from_slice(&[0.04, 0.05, 0.06]);
        info[16] = 1.0;
        info[17] = 8.0;
        info[18] = 8.0;
        let scale = crate::resolve(&info, crate::ExtractionMode::Uhdr).unwrap();
        let application = iso_gain_application_for_alternate(&scale).unwrap();
        assert_eq!(application.weight, 1.0);
        assert!((application.channels[0].maximum_log2 - 2.0_f64.log2()).abs() < 1e-12);
        assert!((application.channels[1].maximum_log2 - 4.0_f64.log2()).abs() < 1e-12);
        assert!((application.channels[2].maximum_log2 - 8.0_f64.log2()).abs() < 1e-12);
        assert_eq!(application.channels[0].gamma, 2.0);
        assert_eq!(application.channels[2].gamma, 0.5);
        assert_eq!(application.channels[1].base_offset, 0.02);
        assert_eq!(application.channels[2].alternate_offset, 0.06);
    }

    #[test]
    fn reconstruction_upsamples_gain_codes_before_application() {
        let sdr = SceneLinearRgb {
            width: 4,
            height: 1,
            samples: vec![0.2, 0.0, 0.0, 0.2, 0.0, 0.0, 0.2, 0.0, 0.0, 0.8, 0.0, 0.0],
        };
        let gain = [0_u8, 255];
        let hdr = reconstruct_scene_linear_hdr(
            &sdr,
            GainMapCodes {
                width: 2,
                height: 1,
                bytes_per_row: 2,
                channels: 1,
                data: &gain,
            },
            &application(channel(1.0, 0.0, 0.0), 1.0),
        )
        .unwrap();
        let red = |pixel: usize| f64::from(hdr.samples[pixel * 3]);
        assert!((red(0) - 0.2).abs() < 1e-6);
        assert!((red(1) - 0.2 * 2.0_f64.powf(0.25)).abs() < 1e-6);
        assert!((red(2) - 0.2 * 2.0_f64.powf(0.75)).abs() < 1e-6);
        assert!((red(3) - 1.6).abs() < 1e-6);

        // The same codes converted to linear gain and then interpolated do not
        // match the code-space upsample at the interior pixels.
        let linear_gain_at = |fraction: f64| (1.0 - fraction) * 1.0 + fraction * 2.0;
        assert!((red(1) - 0.2 * linear_gain_at(0.25)).abs() > 1e-3);
    }

    #[test]
    fn edge_and_near_black_samples_stay_on_the_gain_formula() {
        let sdr = SceneLinearRgb {
            width: 2,
            height: 2,
            samples: vec![0.0; 12],
        };
        let gain = [0_u8, 255, 255, 0];
        let offsets = channel(1.0, 1.0 / 64.0, 1.0 / 64.0);
        let hdr = reconstruct_scene_linear_hdr(
            &sdr,
            GainMapCodes {
                width: 2,
                height: 2,
                bytes_per_row: 2,
                channels: 1,
                data: &gain,
            },
            &application(offsets, 1.0),
        )
        .unwrap();
        // Equal offsets cancel only where the gain is 1. Code 0 stays at black;
        // code 1 lifts the 1/64 offset by one stop and then subtracts it again.
        let near_black = |x: usize, y: usize| f64::from(hdr.samples[(y * 2 + x) * 3]);
        assert!(near_black(0, 0).abs() < 1e-6);
        assert!(near_black(1, 1).abs() < 1e-6);
        assert!((near_black(1, 0) - (2.0 / 64.0 - 1.0 / 64.0)).abs() < 1e-6);
        assert!((near_black(0, 1) - (2.0 / 64.0 - 1.0 / 64.0)).abs() < 1e-6);

        let negative = channel(1.0, -0.01, 0.0);
        let below = apply_iso_gain_sample(0.0, 0.0, &negative, 1.0).unwrap();
        assert!((below + 0.01).abs() < 1e-15);

        let corner = SceneLinearRgb {
            width: 3,
            height: 1,
            samples: vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        };
        let single = [128_u8];
        let reconstructed = reconstruct_scene_linear_hdr(
            &corner,
            GainMapCodes {
                width: 1,
                height: 1,
                bytes_per_row: 1,
                channels: 1,
                data: &single,
            },
            &application(channel(1.0, 0.0, 0.0), 1.0),
        )
        .unwrap();
        let expected = 2.0_f64.powf(128.0 / 255.0);
        assert!(reconstructed
            .samples
            .iter()
            .step_by(3)
            .all(|sample| (f64::from(*sample) - expected).abs() < 1e-6));
    }

    #[test]
    fn area_resize_follows_full_resolution_reconstruction() {
        let mut samples = vec![2.0_f32; 4 * 2 * 3];
        samples[3] = 0.0;
        samples[4] = 0.0;
        samples[5] = 0.0;
        let image = SceneLinearRgb {
            width: 4,
            height: 2,
            samples,
        };
        let resized = area_resize_linear_rgb(&image, 2).unwrap();
        assert_eq!((resized.width, resized.height), (2, 2));
        assert_eq!(resized.samples[0], 1.0_f32);
        assert_eq!(resized.samples[3], 2.0_f32);

        let sdr = SceneLinearRgb {
            width: 4,
            height: 2,
            samples: {
                let mut rgb = vec![1.0_f32; 24];
                rgb[3] = 0.0;
                rgb[4] = 0.0;
                rgb[5] = 0.0;
                rgb
            },
        };
        let gain = [255_u8; 4];
        let full = reconstruct_scene_linear_hdr(
            &sdr,
            GainMapCodes {
                width: 2,
                height: 2,
                bytes_per_row: 2,
                channels: 1,
                data: &gain,
            },
            &application(channel(1.0, 0.0, 0.0), 1.0),
        )
        .unwrap();
        let thumbnail = area_resize_linear_rgb(&full, 2).unwrap();
        assert_eq!(thumbnail.samples[0], 1.0_f32);
        // Nearest 2x2 from the SDR would have kept only the white samples and
        // produced 2 at this pixel.
        assert_ne!(thumbnail.samples[0], 2.0_f32);
    }

    #[test]
    fn invalid_gain_metadata_and_rasters_fail_closed() {
        assert!(apply_iso_gain_sample(0.0, 0.0, &channel(0.0, 0.0, 0.0), 1.0).is_err());
        assert!(apply_iso_gain_sample(0.0, 0.0, &channel(-1.0, 0.0, 0.0), 1.0).is_err());
        assert!(apply_iso_gain_sample(f64::NAN, 0.0, &channel(1.0, 0.0, 0.0), 1.0).is_err());
        assert!(apply_iso_gain_sample(0.0, 0.0, &channel(1.0, 0.0, 0.0), 1.5).is_err());
        let broken = SceneLinearRgb {
            width: 1,
            height: 1,
            samples: vec![f32::NAN, 0.0, 0.0],
        };
        assert!(reconstruct_scene_linear_hdr(
            &broken,
            GainMapCodes {
                width: 1,
                height: 1,
                bytes_per_row: 1,
                channels: 1,
                data: &[0],
            },
            &application(channel(1.0, 0.0, 0.0), 1.0),
        )
        .is_err());
        assert!(area_resize_linear_rgb(
            &SceneLinearRgb {
                width: 1,
                height: 1,
                samples: vec![0.0, 0.0],
            },
            2,
        )
        .is_err());
    }
}
