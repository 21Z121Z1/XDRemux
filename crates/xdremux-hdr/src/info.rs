use crate::ResolvedScale;

fn value_or_repeated(values: &[f64], index: usize, fallback: f64) -> f64 {
    if values.is_empty() {
        fallback
    } else if index < values.len() {
        values[index]
    } else {
        values[0]
    }
}

fn positive_value_or_fallback(values: &[f64], index: usize, fallback: f64) -> f64 {
    let value = value_or_repeated(values, index, fallback);
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

fn quantize_f32(value: f64) -> f64 {
    f64::from(value as f32)
}

/// Build the canonical 20-float private Gain Map info representation used by
/// the current product for LHDR sources.
///
/// Values are intentionally quantized through `f32` before being returned as
/// `f64`; the source format stores Float32 and existing Swift behavior depends
/// on that rounding boundary.
pub fn make_private_gain_map_info_floats(scale: &ResolvedScale) -> [f64; 20] {
    let mut output = [0.0_f64; 20];

    for (channel, slot) in output[..3].iter_mut().enumerate() {
        let gain_min =
            value_or_repeated(&scale.per_channel_gain_map_min, channel, scale.gain_map_min);
        *slot = quantize_f32(2.0_f64.powf(gain_min));
    }
    output[3] = quantize_f32(1.0);

    for (channel, slot) in output[4..7].iter_mut().enumerate() {
        let gain_max = positive_value_or_fallback(
            &scale.per_channel_gain_map_max,
            channel,
            scale.gain_map_max,
        );
        *slot = quantize_f32(2.0_f64.powf(gain_max));
    }
    for (channel, slot) in output[7..10].iter_mut().enumerate() {
        *slot = quantize_f32(value_or_repeated(
            &scale.per_channel_gamma,
            channel,
            scale.gamma,
        ));
    }
    for (channel, slot) in output[10..13].iter_mut().enumerate() {
        *slot = quantize_f32(value_or_repeated(
            &scale.per_channel_base_offset,
            channel,
            scale.epsilon_sdr,
        ));
    }
    for (channel, slot) in output[13..16].iter_mut().enumerate() {
        *slot = quantize_f32(value_or_repeated(
            &scale.per_channel_alternate_offset,
            channel,
            scale.epsilon_hdr,
        ));
    }

    output[16] = quantize_f32(scale.display_ratio_sdr);
    output[17] = quantize_f32(scale.display_ratio_hdr);
    output[18] = quantize_f32(scale.scale);
    output[19] = quantize_f32(0.0);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scale() -> ResolvedScale {
        ResolvedScale {
            edr_scale: 4.0,
            ratio_min: 1.0,
            ratio_max: 4.0,
            gamma: 1.25,
            epsilon_sdr: 0.01,
            epsilon_hdr: 0.02,
            display_ratio_sdr: 1.0,
            display_ratio_hdr: 4.0,
            scale: 4.0,
            gain_map_min: 0.0,
            gain_map_max: 2.0,
            base_headroom: 0.0,
            alternate_headroom: 2.0,
            source: "test",
            channel_count: 1,
            per_channel_gain_map_min: vec![0.0],
            per_channel_gain_map_max: vec![2.0],
            per_channel_gamma: vec![1.25],
            per_channel_base_offset: vec![0.01],
            per_channel_alternate_offset: vec![0.02],
        }
    }

    #[test]
    fn repeats_first_channel_and_preserves_float32_boundary() {
        let info = make_private_gain_map_info_floats(&scale());
        assert_eq!(&info[0..3], &[1.0, 1.0, 1.0]);
        assert_eq!(info[3], 1.0);
        assert_eq!(&info[4..7], &[4.0, 4.0, 4.0]);
        assert_eq!(info[7], f64::from(1.25_f32));
        assert_eq!(info[8], info[7]);
        assert_eq!(info[9], info[7]);
        assert_eq!(info[10], f64::from(0.01_f32));
        assert_eq!(info[13], f64::from(0.02_f32));
        assert_eq!(info[16], 1.0);
        assert_eq!(info[17], 4.0);
        assert_eq!(info[18], 4.0);
        assert_eq!(info[19], 0.0);
    }

    #[test]
    fn invalid_positive_per_channel_max_falls_back_to_scalar() {
        let mut value = scale();
        value.per_channel_gain_map_max = vec![f64::NAN, -1.0, 1.0];
        let info = make_private_gain_map_info_floats(&value);
        assert_eq!(info[4], 4.0);
        assert_eq!(info[5], 4.0);
        assert_eq!(info[6], 2.0);
    }
}
