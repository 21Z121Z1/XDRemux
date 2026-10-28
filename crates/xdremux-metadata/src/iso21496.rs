use crate::{MetadataError, Result};

fn require_info_floats(values: &[f64]) -> Result<()> {
    if values.len() < 18 {
        return Err(MetadataError::invalid(
            "ISO 21496 metadata",
            format!("expected at least 18 info floats, got {}", values.len()),
        ));
    }
    Ok(())
}

fn push_i32_be(value: i32, output: &mut Vec<u8>) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn push_u16_be(value: u16, output: &mut Vec<u8>) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn push_u32_be(value: u32, output: &mut Vec<u8>) {
    output.extend_from_slice(&value.to_be_bytes());
}

// Bounded continued fractions. Source ISO never goes through this projection.
// Generated/XMP values must meet both wire bounds and the documented error
// budget; a value that cannot be represented is an error, never a clamp.
fn rational(value: f64, signed: bool) -> Result<(i64, u32)> {
    let limit = if signed {
        if value.is_sign_negative() {
            1u64 << 31
        } else {
            i32::MAX as u64
        }
    } else {
        u32::MAX as u64
    };
    if !value.is_finite() || (!signed && value < 0.0) || value.abs() > limit as f64 {
        return Err(MetadataError::invalid(
            "ISO 21496 tmap",
            format!("rational value {value} is out of range"),
        ));
    }
    let x = value.abs();
    let (mut p0, mut q0, mut p1, mut q1) = (0u64, 1u64, 1u64, 0u64);
    let mut remainder = x;
    loop {
        let a = remainder.floor();
        let k = (limit - p0).checked_div(p1).unwrap_or(u64::MAX).min(
            (u64::from(u32::MAX) - q0)
                .checked_div(q1)
                .unwrap_or(u64::MAX),
        );
        if a > k as f64 {
            let candidate = (p0 + k * p1, q0 + k * q1);
            if candidate.1 != 0
                && (q1 == 0
                    || (candidate.0 as f64 / candidate.1 as f64 - x).abs()
                        < (p1 as f64 / q1 as f64 - x).abs())
            {
                (p1, q1) = candidate;
            }
            break;
        }
        let a = a as u64;
        let (p, q) = (p0 + a * p1, q0 + a * q1);
        (p0, q0, p1, q1) = (p1, q1, p, q);
        let fraction = remainder - a as f64;
        if fraction == 0.0 || (p as f64 / q as f64 - x).abs() <= x * f64::EPSILON {
            break;
        }
        remainder = 1.0 / fraction;
    }
    if q1 == 0
        || (p1 as f64 / q1 as f64 - x).abs() > 1e-12_f64.max(x * 1e-9)
        || (x > 0.0 && p1 == 0)
    {
        return Err(MetadataError::invalid(
            "ISO 21496 tmap",
            format!("rational value {value} cannot meet the serialization error budget"),
        ));
    }
    Ok((
        if value.is_sign_negative() {
            -(p1 as i64)
        } else {
            p1 as i64
        },
        q1 as u32,
    ))
}

fn append_rational(value: f64, signed: bool, output: &mut Vec<u8>) -> Result<()> {
    let (numerator, denominator) = rational(value, signed)?;
    if signed {
        push_i32_be(numerator as i32, output);
    } else {
        push_u32_be(numerator as u32, output);
    }
    push_u32_be(denominator, output);
    Ok(())
}

/// Serialize log-domain fields directly, without an exp2/log2 round trip.
pub(crate) fn make_tmap_from_log_metadata(
    metadata: &crate::UltraHdrGainMapMetadata,
    use_base_colour_space: bool,
) -> Result<Vec<u8>> {
    make_log_tmap(
        [metadata.hdr_capacity_min, metadata.hdr_capacity_max],
        std::array::from_fn(|c| {
            [
                metadata.gain_map_min[c],
                metadata.gain_map_max[c],
                metadata.gamma[c],
                metadata.offset_sdr[c],
                metadata.offset_hdr[c],
            ]
        }),
        use_base_colour_space,
        metadata.channel_count,
    )
}

pub(crate) fn make_log_tmap(
    headrooms: [f64; 2],
    channels: [[f64; 5]; 3],
    use_base_colour_space: bool,
    channel_count: u8,
) -> Result<Vec<u8>> {
    if !matches!(channel_count, 1 | 3)
        || (channel_count == 1 && (channels[1] != channels[0] || channels[2] != channels[0]))
    {
        return Err(MetadataError::invalid(
            "ISO 21496 tmap",
            "metadata channel count does not match its parameters",
        ));
    }
    let flags =
        if channel_count == 3 { 0x80 } else { 0 } | if use_base_colour_space { 0x40 } else { 0 };
    let mut output = vec![0];
    push_u16_be(0, &mut output);
    push_u16_be(0, &mut output);
    output.push(flags);
    for headroom in headrooms {
        append_rational(headroom, false, &mut output)?;
    }
    for channel in channels.into_iter().take(usize::from(channel_count)) {
        if channel[0] > channel[1] || channel[2] <= 0.0 {
            return Err(MetadataError::invalid(
                "ISO 21496 tmap",
                "invalid gain interval or gamma",
            ));
        }
        for (i, value) in channel.into_iter().enumerate() {
            append_rational(value, i != 2, &mut output)?;
        }
    }
    crate::ultrahdr_jpeg::validate_tmap_payload(&output)?;
    Ok(output)
}

fn canonical_channel_values(info: &[f64], channel: usize) -> Result<[f64; 5]> {
    require_info_floats(info)?;
    let gain_min_ratio = info[channel];
    let gain_max_ratio = info[4 + channel];
    let gamma = info[7 + channel];
    let base_offset = info[10 + channel];
    let alternate_offset = info[13 + channel];
    if !gain_min_ratio.is_finite()
        || gain_min_ratio <= 0.0
        || !gain_max_ratio.is_finite()
        || gain_max_ratio <= 0.0
        || gain_max_ratio < gain_min_ratio
        || !gamma.is_finite()
        || gamma <= 0.0
        || !base_offset.is_finite()
        || !alternate_offset.is_finite()
    {
        return Err(MetadataError::invalid(
            "ISO 21496 tmap",
            format!("invalid gain-map parameters for channel {channel}"),
        ));
    }
    Ok([
        gain_min_ratio.log2(),
        gain_max_ratio.log2(),
        gamma,
        base_offset,
        alternate_offset,
    ])
}

fn canonical_headrooms(info: &[f64]) -> Result<[f64; 2]> {
    require_info_floats(info)?;
    if [info[16], info[17]]
        .iter()
        .any(|v| !v.is_finite() || *v < 1.0)
    {
        return Err(MetadataError::invalid(
            "ISO 21496 tmap",
            "headroom ratios must be finite and at least one",
        ));
    }
    if info[16] == info[17] {
        return Err(MetadataError::invalid(
            "ISO 21496 tmap",
            "baseline and alternate HDR headrooms must differ",
        ));
    }
    Ok([info[16].log2(), info[17].log2()])
}

/// Generate an ISO 21496-1:2025 GainMapMetadata payload inside HEIF tmap.
///
/// A single set of metadata applies to RGB gain-map pixels when the three
/// parameter sets agree. Distinct RGB parameters require the multichannel flag.
pub fn make_apple_tmap_payload(info: &[f64]) -> Result<Vec<u8>> {
    make_apple_tmap_payload_with_colour_space(info, true)
}

/// As above, using the source's declared gain-map application colour space.
pub fn make_apple_tmap_payload_with_colour_space(
    info: &[f64],
    use_base_colour_space: bool,
) -> Result<Vec<u8>> {
    let channels = [
        canonical_channel_values(info, 0)?,
        canonical_channel_values(info, 1)?,
        canonical_channel_values(info, 2)?,
    ];
    let multichannel = channels[1] != channels[0] || channels[2] != channels[0];
    make_apple_tmap_payload_with_channel_count(
        info,
        use_base_colour_space,
        if multichannel { 3 } else { 1 },
    )
}

/// Preserve an input gain map's metadata component count, even if its RGB
/// parameter sets happen to agree. This count is independent of pixel channels.
pub fn make_apple_tmap_payload_with_channel_count(
    info: &[f64],
    use_base_colour_space: bool,
    channel_count: u8,
) -> Result<Vec<u8>> {
    let headrooms = canonical_headrooms(info)?;
    let channels = [
        canonical_channel_values(info, 0)?,
        canonical_channel_values(info, 1)?,
        canonical_channel_values(info, 2)?,
    ];
    make_log_tmap(headrooms, channels, use_base_colour_space, channel_count)
}

/// Generate three metadata component records for ImageIO consumers.
/// Uses the same checked signed/unsigned rational writer as the standard path.
pub fn make_imageio_native_tmap_payload(info: &[f64]) -> Result<Vec<u8>> {
    make_apple_tmap_payload_with_channel_count(info, true, 3)
}

/// Legacy padded metadata helper retained for compatibility with older diagnostics.
/// Its three inserted bytes are NOT part of ISO 21496-1:2025 Annex C.2.2.
pub fn make_strict_tmap_payload(payload: &[u8]) -> Result<Vec<u8>> {
    if payload.len() != 62 && payload.len() != 142 {
        return Err(MetadataError::invalid(
            "strict ISO 21496 tmap",
            format!(
                "expected a 62- or 142-byte ImageIO payload, got {}",
                payload.len()
            ),
        ));
    }
    let mut output = Vec::with_capacity(payload.len() + 3);
    output.extend_from_slice(&payload[..6]);
    output.extend_from_slice(&[0, 0, 0]);
    output.extend_from_slice(&payload[6..]);
    Ok(output)
}

/// Serialize `hdrgm` XMP with the source's base/alternate direction and offsets.
pub fn make_hdrgm_xmp(info: &[f64]) -> Result<Vec<u8>> {
    let headrooms = canonical_headrooms(info)?;
    let c = [
        canonical_channel_values(info, 0)?,
        canonical_channel_values(info, 1)?,
        canonical_channel_values(info, 2)?,
    ];
    Ok(write_hdrgm_xmp(
        std::array::from_fn(|i| c[i][0]),
        std::array::from_fn(|i| c[i][1]),
        std::array::from_fn(|i| c[i][2]),
        [
            std::array::from_fn(|i| c[i][3]),
            std::array::from_fn(|i| c[i][4]),
        ],
        headrooms,
    ))
}

pub(crate) fn write_hdrgm_xmp(
    gain_min: [f64; 3],
    gain_max: [f64; 3],
    gamma: [f64; 3],
    offsets: [[f64; 3]; 2],
    headrooms: [f64; 2],
) -> Vec<u8> {
    let base_is_hdr = headrooms[0] > headrooms[1];
    write_hdrgm_xmp_with_headrooms(
        gain_min,
        gain_max,
        gamma,
        offsets,
        headrooms.map(|v| v.to_string()),
        base_is_hdr,
    )
}

pub(crate) fn write_hdrgm_xmp_with_headrooms(
    gain_min: [f64; 3],
    gain_max: [f64; 3],
    gamma: [f64; 3],
    offsets: [[f64; 3]; 2],
    headrooms: [String; 2],
    base_is_hdr: bool,
) -> Vec<u8> {
    let (offset_sdr, offset_hdr) = if base_is_hdr {
        (offsets[1], offsets[0])
    } else {
        (offsets[0], offsets[1])
    };
    let (cap_min, cap_max) = if base_is_hdr {
        (&headrooms[1], &headrooms[0])
    } else {
        (&headrooms[0], &headrooms[1])
    };

    fn format_three(values: [f64; 3]) -> String {
        format!("{} {} {}", values[0], values[1], values[2])
    }

    let xml = format!(
        concat!(
            "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n",
            "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"XMP Core 6.0.0\">\n",
            "   <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n",
            "      <rdf:Description rdf:about=\"\"\n",
            "            xmlns:hdrgm=\"http://ns.adobe.com/hdr-gain-map/1.0/\"\n",
            "            xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"\n",
            "            xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\">\n",
            "         <hdrgm:Version>1.0</hdrgm:Version>\n",
            "         <hdrgm:GainMapMin>{}</hdrgm:GainMapMin>\n",
            "         <hdrgm:GainMapMax>{}</hdrgm:GainMapMax>\n",
            "         <hdrgm:Gamma>{}</hdrgm:Gamma>\n",
            "         <hdrgm:OffsetSDR>{}</hdrgm:OffsetSDR>\n",
            "         <hdrgm:OffsetHDR>{}</hdrgm:OffsetHDR>\n",
            "         <hdrgm:HDRCapacityMin>{}</hdrgm:HDRCapacityMin>\n",
            "         <hdrgm:HDRCapacityMax>{}</hdrgm:HDRCapacityMax>\n",
            "         <hdrgm:BaseRenditionIsHDR>{}</hdrgm:BaseRenditionIsHDR>\n",
            "      </rdf:Description>\n",
            "   </rdf:RDF>\n",
            "</x:xmpmeta>\n",
            "<?xpacket end=\"w\"?>"
        ),
        format_three(gain_min),
        format_three(gain_max),
        format_three(gamma),
        format_three(offset_sdr),
        format_three(offset_hdr),
        cap_min,
        cap_max,
        if base_is_hdr { "True" } else { "False" },
    );
    xml.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_sample() -> [f64; 20] {
        let ratio = 4.926108360290527;
        [
            1.0, 1.0, 1.0, 1.0, ratio, ratio, ratio, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            1.0, ratio, ratio, 0.0,
        ]
    }

    fn read_rational(bytes: &[u8], offset: usize, signed: bool) -> f64 {
        let raw: [u8; 4] = bytes[offset..offset + 4].try_into().unwrap();
        let n = if signed {
            f64::from(i32::from_be_bytes(raw))
        } else {
            f64::from(u32::from_be_bytes(raw))
        };
        n / f64::from(u32::from_be_bytes(
            bytes[offset + 4..offset + 8].try_into().unwrap(),
        ))
    }

    #[test]
    fn generated_tmap_preserves_values_with_a_bounded_rational_error() {
        let mut info = canonical_sample();
        info[17] = 2_f64.powf(1e-6);
        info[7..10].fill(600.1);
        info[10..13].fill(1. / 64.);
        let bytes = make_apple_tmap_payload(&info).unwrap();
        assert!((read_rational(&bytes, 14, false) - 1e-6).abs() < 1e-12);
        assert_eq!(read_rational(&bytes, 38, false), 600.1);
        assert_eq!(read_rational(&bytes, 46, true), 1. / 64.);
        let sample = make_apple_tmap_payload(&canonical_sample()).unwrap();
        assert!((read_rational(&sample, 30, true) - canonical_sample()[4].log2()).abs() < 1e-9);
        for value in [
            f64::NAN,
            f64::INFINITY,
            -1.,
            1e-20,
            f64::from(u32::MAX) + 1.,
        ] {
            info[7..10].fill(value);
            assert!(make_apple_tmap_payload(&info).is_err(), "gamma {value}");
            assert!(
                make_imageio_native_tmap_payload(&info).is_err(),
                "gamma {value}"
            );
        }
        info[7..10].fill(3_000_000_000.);
        assert_eq!(
            read_rational(&make_apple_tmap_payload(&info).unwrap(), 38, false),
            3_000_000_000.
        );
        info[17] = 1.;
        assert!(
            make_apple_tmap_payload(&info).is_err(),
            "equal emitted headrooms must fail"
        );
        info[17] = f64::NAN;
        assert!(make_apple_tmap_payload(&info).is_err());
    }

    #[test]
    fn native_and_strict_lengths_match_swift_contract() {
        let native = make_imageio_native_tmap_payload(&canonical_sample()).unwrap();
        assert_eq!(native.len(), 142);
        assert_eq!(make_strict_tmap_payload(&native).unwrap().len(), 145);
        let apple = make_apple_tmap_payload(&canonical_sample()).unwrap();
        assert_eq!(make_strict_tmap_payload(&apple).unwrap().len(), 65);
    }

    #[test]
    fn distinct_channels_are_preserved_in_xmp_and_tmap() {
        let info = [
            1.25, 1.5, 1.75, 1.0, 4.0, 5.0, 6.0, 0.8, 1.1, 1.2, 0.01, 0.02, 0.03, 0.04, 0.05, 0.06,
            1.5, 6.5, 2.0, 0.0,
        ];
        let xmp = String::from_utf8(make_hdrgm_xmp(&info).unwrap()).unwrap();
        assert!(xmp.contains("0.32192809488736235 0.5849625007211562 0.8073549220576041"));
        assert!(xmp.contains("2 2.321928094887362 2.584962500721156"));
        let multichannel = make_apple_tmap_payload(&info).unwrap();
        assert_eq!(multichannel.len(), 142);
        assert_eq!(multichannel[5], 0xc0);
    }

    #[test]
    fn hdr_baseline_xmp_preserves_direction_capacity_and_offsets() {
        let mut info = canonical_sample();
        info[16] = 4.0;
        info[17] = 1.0;
        info[10..13].fill(0.02);
        info[13..16].fill(0.01);
        let xmp = String::from_utf8(make_hdrgm_xmp(&info).unwrap()).unwrap();
        assert!(xmp.contains("<hdrgm:BaseRenditionIsHDR>True</hdrgm:BaseRenditionIsHDR>"));
        assert!(xmp.contains("<hdrgm:HDRCapacityMin>0</hdrgm:HDRCapacityMin>"));
        assert!(xmp.contains("<hdrgm:HDRCapacityMax>2</hdrgm:HDRCapacityMax>"));
        assert!(xmp.contains("<hdrgm:OffsetSDR>0.01 0.01 0.01</hdrgm:OffsetSDR>"));
        assert!(xmp.contains("<hdrgm:OffsetHDR>0.02 0.02 0.02</hdrgm:OffsetHDR>"));
        let iso = make_apple_tmap_payload(&info).unwrap();
        assert_eq!(iso[5], 0x40);
        assert_eq!(i32::from_be_bytes(iso[6..10].try_into().unwrap()), 2);
        assert_eq!(i32::from_be_bytes(iso[14..18].try_into().unwrap()), 0);
    }

    #[test]
    fn reversed_per_channel_gain_ranges_are_rejected_by_both_tmap_writers() {
        // Ratios below unity and equal bounds are valid; only reversed ranges fail.
        for channel in 0..3 {
            let mut info = canonical_sample();
            info[channel] = 4.0;
            info[4 + channel] = 2.0;
            assert!(make_apple_tmap_payload(&info).is_err(), "channel {channel}");
            assert!(
                make_imageio_native_tmap_payload(&info).is_err(),
                "channel {channel}"
            );

            info[4 + channel] = info[channel];
            assert!(make_apple_tmap_payload(&info).is_ok(), "channel {channel}");
            assert!(
                make_imageio_native_tmap_payload(&info).is_ok(),
                "channel {channel}"
            );

            info[channel] = 0.5;
            info[4 + channel] = 1.0;
            assert!(make_apple_tmap_payload(&info).is_ok(), "channel {channel}");
        }
    }

    #[test]
    fn hdrgm_xmp_preserves_current_swift_indentation_contract() {
        let xmp = String::from_utf8(make_hdrgm_xmp(&canonical_sample()).unwrap()).unwrap();
        assert!(xmp.contains(
            "\n   <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n"
        ));
        assert!(xmp.contains(
            "\n      <rdf:Description rdf:about=\"\"\n            xmlns:hdrgm=\"http://ns.adobe.com/hdr-gain-map/1.0/\"\n"
        ));
        assert!(xmp.contains("\n         <hdrgm:Version>1.0</hdrgm:Version>\n"));
        assert!(xmp.contains("\n      </rdf:Description>\n   </rdf:RDF>\n"));
        assert!(!xmp.ends_with('\n'));
    }

    #[test]
    fn malformed_lengths_and_strict_payloads_fail_closed() {
        assert!(make_apple_tmap_payload(&[1.0; 17]).is_err());
        assert!(make_hdrgm_xmp(&[1.0; 17]).is_err());
        assert!(make_strict_tmap_payload(&[0; 63]).is_err());
    }
}
