use crate::{get_knee_point_result, HdrError, ResolvedScale, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Auto,
    X6,
    X7,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GainMapRaster {
    pub width: usize,
    pub height: usize,
    pub bytes_per_row: usize,
    pub channel_count: usize,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GainMapParams {
    pub family: Family,
    pub knee: f64,
    pub knee_range: f64,
    pub headroom_scale: f64,
    pub max_boost: f64,
    pub log2_scale: f64,
    pub knee_source: &'static str,
}

pub fn gain_map_parameters(
    family: Family,
    scale: &ResolvedScale,
    meta_floats: &[f64],
) -> Result<GainMapParams> {
    if !scale.edr_scale.is_finite() || scale.edr_scale <= 0.0 {
        return Err(HdrError::invalid(
            "gain map params",
            format!("invalid EDR scale {}", scale.edr_scale),
        ));
    }

    let gamma_factor = (1.0 / scale.edr_scale).powf(1.0 / 2.2);
    let headroom_scale = (1.0 - gamma_factor) / gamma_factor;
    let max_boost = if scale.edr_scale > 1.0 {
        scale.edr_scale
    } else {
        2.0
    };
    let log2_scale = if scale.edr_scale > 1.0 {
        255.0 / scale.edr_scale.log2()
    } else {
        0.0
    };

    let edr_version = meta_floats.first().copied().unwrap_or(3.0);
    if !edr_version.is_finite() {
        return Err(HdrError::invalid(
            "gain map params",
            "EDR version must be finite",
        ));
    }

    let (knee, knee_source) = if edr_version >= 3.0 {
        (0.0, "edr_ge3_log2_path")
    } else {
        let result = get_knee_point_result(scale.edr_scale);
        (result.value, result.source)
    };
    let knee_range = 1.0 - knee;

    if !knee.is_finite() || !knee_range.is_finite() || knee_range <= 0.0 {
        return Err(HdrError::invalid(
            "gain map params",
            format!("non-finite gain map params: knee={knee}, kneeRange={knee_range}"),
        ));
    }
    if !headroom_scale.is_finite() || !max_boost.is_finite() || !log2_scale.is_finite() {
        return Err(HdrError::invalid(
            "gain map params",
            "derived gain map scale values must be finite",
        ));
    }

    Ok(GainMapParams {
        family,
        knee,
        knee_range,
        headroom_scale,
        max_boost,
        log2_scale,
        knee_source,
    })
}

pub fn reconstruct_gain_map(
    mask: &GainMapRaster,
    family: Family,
    scale: &ResolvedScale,
    meta_floats: &[f64],
) -> Result<(GainMapRaster, GainMapParams)> {
    validate_mask(mask)?;
    let params = gain_map_parameters(family, scale, meta_floats)?;

    let lut0 = make_lut(1001, |x| x.powf(0.625));
    let lut1 = make_lut(1001, |x| x.powf(2.2));
    let lut2 = make_lut(1001, |x| {
        (x * params.headroom_scale + 1.0).powf(2.2)
    });
    let lut3 = make_lut(8001, |x| {
        if x == 0.0 {
            0.0
        } else {
            let clamped = x.max(1.0).min(params.max_boost);
            params.log2_scale * clamped.log2()
        }
    });

    let output_bytes_per_row = checked_align_up(mask.width, 256)?;
    let output_len = output_bytes_per_row
        .checked_mul(mask.height)
        .ok_or_else(|| HdrError::invalid("gain map raster", "output raster size overflow"))?;
    let mut output = vec![0_u8; output_len];

    for y in 0..mask.height {
        let input_row = y
            .checked_mul(mask.bytes_per_row)
            .ok_or_else(|| HdrError::invalid("gain map raster", "input row offset overflow"))?;
        let output_row = y
            .checked_mul(output_bytes_per_row)
            .ok_or_else(|| HdrError::invalid("gain map raster", "output row offset overflow"))?;

        for x in 0..mask.width {
            let mask_value = f64::from(mask.data[input_row + x]) / 255.0;
            let idx0 = clamp_index((mask_value * 1000.0) as usize, 1000);
            let lin_gray = lut0[idx0];

            let boosted = if lin_gray < params.knee {
                1.0
            } else {
                let t = (lin_gray - params.knee) / params.knee_range;
                let idx1 = clamp_index((t * 1000.0) as usize, 1000);
                let linear = lut1[idx1];
                let idx2 = clamp_index((linear * 1000.0) as usize, 1000);
                lut2[idx2]
            };

            let idx3 = if boosted < 1.0 {
                1000
            } else {
                clamp_index((boosted.min(8.0) * 1000.0) as usize, 8000)
            };

            // Swift's Int(Double) truncates toward zero. Do not replace this
            // with round-to-nearest: that produces stable 1-LSB differences.
            let log_gain = (lut3[idx3] as i32).clamp(0, 255);
            output[output_row + x] = log_gain as u8;
        }
    }

    Ok((
        GainMapRaster {
            width: mask.width,
            height: mask.height,
            bytes_per_row: output_bytes_per_row,
            channel_count: 1,
            data: output,
        },
        params,
    ))
}

fn validate_mask(mask: &GainMapRaster) -> Result<()> {
    if mask.width == 0 || mask.height == 0 {
        return Err(HdrError::invalid(
            "gain map raster",
            "mask dimensions must be non-zero",
        ));
    }
    if mask.channel_count != 1 {
        return Err(HdrError::invalid(
            "gain map raster",
            "LHDR reconstruction requires a single-channel mask",
        ));
    }
    if mask.bytes_per_row < mask.width {
        return Err(HdrError::invalid(
            "gain map raster",
            "mask bytes_per_row is smaller than width",
        ));
    }
    let required = mask
        .bytes_per_row
        .checked_mul(mask.height)
        .ok_or_else(|| HdrError::invalid("gain map raster", "input raster size overflow"))?;
    if mask.data.len() < required {
        return Err(HdrError::invalid(
            "gain map raster",
            format!(
                "mask data is too short: need at least {required} bytes, got {}",
                mask.data.len()
            ),
        ));
    }
    Ok(())
}

fn checked_align_up(value: usize, multiple: usize) -> Result<usize> {
    debug_assert!(multiple > 0);
    let remainder = value % multiple;
    if remainder == 0 {
        Ok(value)
    } else {
        value
            .checked_add(multiple - remainder)
            .ok_or_else(|| HdrError::invalid("gain map raster", "row alignment overflow"))
    }
}

fn clamp_index(value: usize, maximum: usize) -> usize {
    value.min(maximum)
}

fn make_lut(count: usize, generator: impl Fn(f64) -> f64) -> Vec<f64> {
    (0..count)
        .map(|index| generator(index as f64 / 1000.0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{resolve, ExtractionMode};

    fn modern_scale() -> (ResolvedScale, [f64; 36]) {
        let mut meta = [0.0_f64; 36];
        meta[0] = 3.5;
        meta[2] = 144.0;
        meta[5] = -1.0;
        meta[23] = 1.2;
        meta[29] = 180.0;
        meta[32] = 400.0;
        meta[33] = 4.5;
        let scale = resolve(&meta, ExtractionMode::Lhdr).unwrap();
        (scale, meta)
    }

    #[test]
    fn rejects_malformed_masks_fail_closed() {
        let (scale, meta) = modern_scale();
        let short = GainMapRaster {
            width: 257,
            height: 2,
            bytes_per_row: 300,
            channel_count: 1,
            data: vec![0; 599],
        };
        assert!(reconstruct_gain_map(&short, Family::X7, &scale, &meta).is_err());

        let rgb = GainMapRaster {
            width: 1,
            height: 1,
            bytes_per_row: 4,
            channel_count: 3,
            data: vec![0; 4],
        };
        assert!(reconstruct_gain_map(&rgb, Family::X7, &scale, &meta).is_err());
    }

    #[test]
    fn output_stride_is_256_aligned_and_padding_stays_zero() {
        let (scale, meta) = modern_scale();
        let mask = GainMapRaster {
            width: 257,
            height: 2,
            bytes_per_row: 300,
            channel_count: 1,
            data: vec![128; 600],
        };
        let (output, _) = reconstruct_gain_map(&mask, Family::X7, &scale, &meta).unwrap();
        assert_eq!(output.bytes_per_row, 512);
        assert_eq!(output.data.len(), 1024);
        assert!(output.data[257..512].iter().all(|byte| *byte == 0));
        assert!(output.data[(512 + 257)..1024].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn quantization_truncates_instead_of_rounding() {
        let (scale, meta) = modern_scale();
        let mut data = vec![0_u8; 256];
        for (index, byte) in data.iter_mut().enumerate() {
            *byte = index as u8;
        }
        let mask = GainMapRaster {
            width: 256,
            height: 1,
            bytes_per_row: 256,
            channel_count: 1,
            data,
        };
        let (output, _) = reconstruct_gain_map(&mask, Family::X7, &scale, &meta).unwrap();
        assert_eq!(output.data[6], 1);
    }
}
