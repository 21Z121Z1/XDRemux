use crate::{HdrError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionMode {
    Lhdr,
    Uhdr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedScale {
    pub edr_scale: f64,
    pub ratio_min: f64,
    pub ratio_max: f64,
    pub gamma: f64,
    pub epsilon_sdr: f64,
    pub epsilon_hdr: f64,
    pub display_ratio_sdr: f64,
    pub display_ratio_hdr: f64,
    pub scale: f64,
    pub gain_map_min: f64,
    pub gain_map_max: f64,
    pub base_headroom: f64,
    pub alternate_headroom: f64,
    pub source: &'static str,
    pub channel_count: usize,
    pub per_channel_gain_map_min: Vec<f64>,
    pub per_channel_gain_map_max: Vec<f64>,
    pub per_channel_gamma: Vec<f64>,
    pub per_channel_base_offset: Vec<f64>,
    pub per_channel_alternate_offset: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KneePointResult {
    pub value: f64,
    pub source: &'static str,
}

fn safe_log2(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value.log2()
    } else {
        0.0
    }
}

fn clamp_edr(value: f64) -> f64 {
    value.max(1.0).min(7.9)
}

pub fn resolve(meta_floats: &[f64], mode: ExtractionMode) -> Result<ResolvedScale> {
    if mode == ExtractionMode::Uhdr {
        if meta_floats.len() < 20 {
            return Err(HdrError::invalid(
                "UHDR metadata",
                "local.uhdr.gainmap.info must contain at least 20 float32 values",
            ));
        }

        let ratio_min = meta_floats[0];
        let ratio_max = meta_floats[4];
        let gamma = meta_floats[7];
        let epsilon_sdr = meta_floats[10];
        let epsilon_hdr = meta_floats[13];
        let display_ratio_sdr = meta_floats[16];
        let display_ratio_hdr = meta_floats[17];
        let scale = meta_floats[18];

        return Ok(ResolvedScale {
            edr_scale: scale,
            ratio_min,
            ratio_max,
            gamma,
            epsilon_sdr,
            epsilon_hdr,
            display_ratio_sdr,
            display_ratio_hdr,
            scale,
            gain_map_min: safe_log2(ratio_min),
            gain_map_max: safe_log2(ratio_max),
            base_headroom: safe_log2(display_ratio_sdr),
            alternate_headroom: safe_log2(display_ratio_hdr),
            source: "local.uhdr.gainmap.info",
            channel_count: 3,
            per_channel_gain_map_min: meta_floats[0..3]
                .iter()
                .copied()
                .map(safe_log2)
                .collect(),
            per_channel_gain_map_max: meta_floats[4..7]
                .iter()
                .copied()
                .map(safe_log2)
                .collect(),
            per_channel_gamma: meta_floats[7..10].to_vec(),
            per_channel_base_offset: meta_floats[10..13].to_vec(),
            per_channel_alternate_offset: meta_floats[13..16].to_vec(),
        });
    }

    if meta_floats.len() != 36 {
        return Err(HdrError::invalid(
            "LHDR metadata",
            "local.hdr.meta.data must contain exactly 36 float32 values",
        ));
    }

    let source = if meta_floats[0] < 3.0 {
        "float32_early_lhdr_edr_scale"
    } else {
        "empirical_edrScaleCalculator"
    };
    Ok(resolved_lhdr_scale(edr_scale_calculator(meta_floats), source))
}

fn resolved_lhdr_scale(edr_scale: f64, source: &'static str) -> ResolvedScale {
    let edr_scale = clamp_edr(edr_scale);
    let ratio_min = 1.0;
    let ratio_max = edr_scale;
    let gamma = 1.0;
    let epsilon_sdr = 0.0;
    let epsilon_hdr = 0.0;
    let display_ratio_sdr = 1.0;
    let display_ratio_hdr = ratio_max;

    ResolvedScale {
        edr_scale,
        ratio_min,
        ratio_max,
        gamma,
        epsilon_sdr,
        epsilon_hdr,
        display_ratio_sdr,
        display_ratio_hdr,
        scale: display_ratio_hdr,
        gain_map_min: safe_log2(ratio_min),
        gain_map_max: safe_log2(ratio_max),
        base_headroom: safe_log2(display_ratio_sdr),
        alternate_headroom: safe_log2(display_ratio_hdr),
        source,
        channel_count: 1,
        per_channel_gain_map_min: vec![safe_log2(ratio_min)],
        per_channel_gain_map_max: vec![safe_log2(ratio_max)],
        per_channel_gamma: vec![gamma],
        per_channel_base_offset: vec![epsilon_sdr],
        per_channel_alternate_offset: vec![epsilon_hdr],
    }
}

pub fn get_knee_point(edr: f64) -> f64 {
    get_knee_point_result(edr).value
}

pub fn get_knee_point_result(edr: f64) -> KneePointResult {
    let scale = edr as f32;
    let inv_gamma = 0.454_545_438_289_642_33_f32;
    let t = 1.0_f32 / (scale * 100.0_f32);
    let k = 1.0_f32 - t;
    let p1 = scale.powf(inv_gamma);
    let div1 = 1.0_f32 / p1;
    let x_norm = (0.980_000_019_073_486_3_f32 - t) / k;
    let p2 = x_norm.powf(inv_gamma);
    let y = (p2 * 1.003_937_005_996_704_f32 - div1) / (1.0_f32 - div1);
    KneePointResult {
        value: f64::from(quantized_knee(y, inv_gamma)),
        source: "float32_early_lhdr_knee",
    }
}

fn quantized_knee(base: f32, inv_gamma: f32) -> f32 {
    if !base.is_finite() || base <= 0.0 {
        return f32::NAN;
    }
    let p3 = base.powf(inv_gamma);
    if !p3.is_finite() || p3 == 1.0 {
        return f32::NAN;
    }

    let knee_raw = p3 * 255.0_f32 + -254.0_f32;
    let knee_adj = knee_raw / (p3 - 1.0_f32);
    let mut result = knee_adj.round();
    if result <= 0.0 {
        result = knee_raw;
    }
    result / 255.0_f32
}

/// Current Swift EDR model.
///
/// The early-LHDR branch intentionally performs Float32/FMA arithmetic. The
/// modern branch intentionally performs ordinary Float64 arithmetic; do not
/// replace its multiply-plus-add expressions with fused operations.
pub fn edr_scale_calculator(f: &[f64]) -> f64 {
    debug_assert_eq!(f.len(), 36);

    if f[0] < 3.0 {
        return f64::from(float32_early_lhdr_scale(f));
    }

    if f[0] < 2.0 {
        return 1.0;
    }
    if f[33] >= 1.0 {
        return f[33];
    }
    if f[32] <= 0.0 {
        return 1.0;
    }

    let f23 = f[23];
    let f24 = f[24];
    let f29 = f[29].max(1.0);
    let raw_gain = f[32];
    let cfg = f[34] >= 1.0 && f[34] < 2.0;

    if f23 <= 0.99 || f[0] < 3.0 {
        let exp_arg = raw_gain * -0.1175 + -6.829;
        let mut edr = 780.3 / (2.0_f64.powf(exp_arg) + 1.0) + -772.3;

        if f24 > 0.0 {
            let factor = if f24 < 1.0 { f24 } else { 1.0 / f24 };
            edr = (edr - 1.0) * factor + 1.0;
        }

        if f29 >= 200.0 {
            let s4 = edr.abs().sqrt().abs() - 1.0;
            if f29 >= 320.0 {
                edr = s4 * 1.34 + 1.0;
            } else {
                edr = s4 * (f29 * -0.0205 + 7.9) + 1.0;
            }
        } else {
            let s4 = edr.abs().sqrt().abs() - 1.0;
            edr = s4 * 3.8 + 1.0;
        }

        if cfg {
            edr = (edr.abs().sqrt().abs() - 1.0) * 1.3 + 1.0;
        } else if f24 > 0.0 {
            let adjusted = (edr.abs().sqrt().abs() - 1.0) * 1.85 + 1.0;
            edr = if f29 <= 320.0 {
                adjusted
            } else {
                (adjusted - 1.0) * 0.8 + 1.0
            };
        } else if f29 > 320.0 {
            edr = (edr - 1.0) * 0.8 + 1.0;
        }

        return clamp_edr(edr);
    }

    let norm_gain = (raw_gain * 1023.0) / 65535.0;
    let scaled = (norm_gain * 63.0 + 1.0).log2() / f29 * 100.0;
    let edr = if f29 <= 210.0 {
        scaled * 0.3456 + 1.824
    } else if f29 > 340.0 {
        scaled * 0.1046 + 1.878
    } else {
        scaled * 0.5883 + 1.401
    };
    clamp_edr(edr)
}

fn float32_early_lhdr_scale(f: &[f64]) -> f32 {
    let version = f[0] as f32;
    if version < 2.0_f32 {
        return 1.0;
    }

    let precomputed = f[33] as f32;
    if precomputed >= 1.0_f32 {
        return precomputed;
    }

    let raw_gain = f[32] as f32;
    if raw_gain <= 0.0_f32 {
        return 1.0;
    }

    let face_strength = f[24] as f32;
    let highlight = f[29] as f32;

    let mut edr = raw_gain
        .mul_add(-0.117_499_999_701_976_78_f32, -6.828_999_996_185_303_f32)
        .exp2();
    edr = 780.299_987_792_968_8_f32 / (edr + 1.0_f32) + -772.299_987_792_968_8_f32;

    let mut face_adjusted = edr;
    if face_strength > 0.0_f32 {
        let factor = if face_strength < 1.0_f32 {
            face_strength
        } else {
            1.0_f32 / face_strength
        };
        face_adjusted = (edr - 1.0_f32).mul_add(factor, 1.0_f32);
    }

    let sqrt_term = face_adjusted.sqrt().abs() - 1.0_f32;
    let highlight_adjusted = if highlight >= 200.0_f32 {
        let high_highlight = sqrt_term.mul_add(1.340_000_033_378_601_f32, 1.0_f32);
        let mid_factor = highlight.mul_add(-0.020_500_000_566_244_125_f32, 7.900_000_095_367_432_f32);
        let mid_highlight = sqrt_term.mul_add(mid_factor, 1.0_f32);
        if highlight >= 320.0_f32 {
            high_highlight
        } else {
            mid_highlight
        }
    } else {
        sqrt_term.mul_add(3.799_999_952_316_284_f32, 1.0_f32)
    };

    if (f[34] as f32).to_bits() == 1 {
        let cfg_term = highlight_adjusted.sqrt().abs() - 1.0_f32;
        return cfg_term.mul_add(1.299_999_952_316_284_2_f32, 1.0_f32);
    }

    if face_strength > 0.0_f32 {
        let face_term = highlight_adjusted.sqrt().abs() - 1.0_f32;
        let adjusted = face_term.mul_add(1.850_000_023_841_858_f32, 1.0_f32);
        if highlight <= 320.0_f32 {
            return adjusted;
        }
        return (adjusted - 1.0_f32).mul_add(0.800_000_011_920_929_f32, 1.0_f32);
    }

    if highlight <= 320.0_f32 {
        return highlight_adjusted;
    }
    (highlight_adjusted - 1.0_f32).mul_add(0.800_000_011_920_929_f32, 1.0_f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_lhdr(version: f64) -> [f64; 36] {
        let mut f = [0.0_f64; 36];
        f[0] = version;
        f[2] = 144.0;
        f[5] = -1.0;
        f[18] = 10.0;
        f[19] = 6.0;
        f[23] = 1.0;
        f[29] = 180.0;
        f[32] = 400.0;
        f
    }

    #[test]
    fn rejects_wrong_metadata_lengths() {
        assert!(resolve(&[0.0; 35], ExtractionMode::Lhdr).is_err());
        assert!(resolve(&[0.0; 19], ExtractionMode::Uhdr).is_err());
    }

    #[test]
    fn early_cfg_uses_float32_subnormal_sentinel_not_numeric_one() {
        let mut sentinel = base_lhdr(2.5);
        sentinel[34] = f64::from(f32::from_bits(1));
        let mut numeric_one = sentinel;
        numeric_one[34] = 1.0;
        assert_ne!(
            edr_scale_calculator(&sentinel).to_bits(),
            edr_scale_calculator(&numeric_one).to_bits()
        );
    }

    #[test]
    fn modern_cfg_is_numeric_interval_and_precomputed_remains_f64() {
        let mut modern = base_lhdr(3.5);
        modern[23] = 0.5;
        modern[34] = 1.5;
        let configured = edr_scale_calculator(&modern);
        modern[34] = f64::from(f32::from_bits(1));
        let subnormal = edr_scale_calculator(&modern);
        assert_ne!(configured.to_bits(), subnormal.to_bits());

        modern[33] = 4.123_456_789_012_345;
        let resolved = resolve(&modern, ExtractionMode::Lhdr).unwrap();
        assert_eq!(resolved.edr_scale.to_bits(), modern[33].to_bits());
    }

    #[test]
    fn uhdr_keeps_distinct_channels() {
        let info = [
            1.25, 1.5, 1.75, 1.0, 4.0, 5.0, 6.0, 0.8, 1.1, 1.2, 0.01, 0.02, 0.03,
            0.04, 0.05, 0.06, 1.5, 6.5, 2.0, 0.0,
        ];
        let resolved = resolve(&info, ExtractionMode::Uhdr).unwrap();
        assert_eq!(resolved.channel_count, 3);
        assert_eq!(resolved.per_channel_gamma, vec![0.8, 1.1, 1.2]);
        assert_ne!(resolved.per_channel_gain_map_max[0], resolved.per_channel_gain_map_max[2]);
    }
}
