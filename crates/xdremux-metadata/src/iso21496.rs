use crate::{MetadataError, Result};

const IMAGEIO_DENOMINATOR: i32 = 1 << 22;
const IMAGEIO_DECIMAL_DENOMINATOR: f64 = 100_000.0;
const NATIVE_RATIONAL_DENOMINATOR: i64 = 100_000;

fn require_info_floats(values: &[f64]) -> Result<()> {
    if values.len() < 18 {
        return Err(MetadataError::invalid(
            "ISO 21496 metadata",
            format!("expected at least 18 info floats, got {}", values.len()),
        ));
    }
    Ok(())
}

fn swift_max(left: f64, right: f64) -> f64 {
    if left < right {
        right
    } else {
        left
    }
}

fn safe_log2(value: f64) -> f64 {
    if value > 0.0 {
        value.log2()
    } else {
        0.0
    }
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

fn append_imageio_rational(value: f64, output: &mut Vec<u8>) {
    if !value.is_finite() {
        push_i32_be(0, output);
        push_i32_be(1, output);
        return;
    }

    let rounded = value.round();
    if (value - rounded).abs() < 1e-12
        && rounded >= f64::from(i32::MIN)
        && rounded <= f64::from(i32::MAX)
    {
        push_i32_be(rounded as i32, output);
        push_i32_be(1, output);
        return;
    }

    let imageio_value = (value * IMAGEIO_DECIMAL_DENOMINATOR).round() / IMAGEIO_DECIMAL_DENOMINATOR;
    let numerator = (imageio_value * f64::from(IMAGEIO_DENOMINATOR)).round();
    let clamped = numerator.clamp(f64::from(i32::MIN), f64::from(i32::MAX));
    push_i32_be(clamped as i32, output);
    push_i32_be(IMAGEIO_DENOMINATOR, output);
}

fn append_native_unsigned_rational(value: f64, output: &mut Vec<u8>) -> Result<()> {
    let rounded = (value * NATIVE_RATIONAL_DENOMINATOR as f64).round();
    let non_negative = swift_max(0.0, rounded);
    if !non_negative.is_finite() || non_negative > f64::from(u32::MAX) {
        return Err(MetadataError::invalid(
            "ImageIO native tmap",
            format!("unsigned rational value {value} is out of range"),
        ));
    }
    push_u32_be(non_negative as u32, output);
    push_u32_be(NATIVE_RATIONAL_DENOMINATOR as u32, output);
    Ok(())
}

fn append_native_signed_rational(value: f64, output: &mut Vec<u8>) -> Result<()> {
    let rounded = (value * NATIVE_RATIONAL_DENOMINATOR as f64).round();
    if !rounded.is_finite() || rounded < f64::from(i32::MIN) || rounded > f64::from(i32::MAX) {
        return Err(MetadataError::invalid(
            "ImageIO native tmap",
            format!("signed rational value {value} is out of range"),
        ));
    }
    push_i32_be(rounded as i32, output);
    push_u32_be(NATIVE_RATIONAL_DENOMINATOR as u32, output);
    Ok(())
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

fn canonical_first_channel_values(info: &[f64]) -> Result<[f64; 7]> {
    let [gain_min, gain_max, gamma, base_offset, alt_offset] = canonical_channel_values(info, 0)?;
    Ok([
        swift_max(swift_max(info[16], 1.0).log2(), 0.0),
        swift_max(info[17], 1.0).log2(),
        gain_min,
        gain_max,
        gamma,
        base_offset,
        alt_offset,
    ])
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
    let [cap_min, cap_max, ..] = canonical_first_channel_values(info)?;
    let channels = [
        canonical_channel_values(info, 0)?,
        canonical_channel_values(info, 1)?,
        canonical_channel_values(info, 2)?,
    ];
    let multichannel = channels[1] != channels[0] || channels[2] != channels[0];
    let flags = if multichannel { 0x80 } else { 0 } | if use_base_colour_space { 0x40 } else { 0 };
    let mut output = vec![0, 0, 0, 0, 0, flags];
    append_imageio_rational(cap_min, &mut output);
    append_imageio_rational(cap_max, &mut output);
    for [gain_min, gain_max, gamma, base_offset, alt_offset] in
        channels.into_iter().take(if multichannel { 3 } else { 1 })
    {
        for value in [gain_min, gain_max, gamma, base_offset, alt_offset] {
            append_imageio_rational(value, &mut output);
        }
    }
    debug_assert_eq!(output.len(), if multichannel { 142 } else { 62 });
    Ok(output)
}

/// Generate a 142-byte three-channel ImageIO-native compatibility payload.
/// Unlike the historical writer, preserve each channel's actual parameters.
pub fn make_imageio_native_tmap_payload(info: &[f64]) -> Result<Vec<u8>> {
    let [cap_min, cap_max, ..] = canonical_first_channel_values(info)?;

    let mut output = Vec::with_capacity(142);
    output.push(0);
    push_u16_be(0, &mut output);
    push_u16_be(0, &mut output);
    output.push(0xC0);
    append_native_unsigned_rational(cap_min, &mut output)?;
    append_native_unsigned_rational(cap_max, &mut output)?;

    for channel in 0..3 {
        let [gain_min, gain_max, gamma, base_offset, alt_offset] =
            canonical_channel_values(info, channel)?;
        append_native_signed_rational(gain_min, &mut output)?;
        append_native_signed_rational(gain_max, &mut output)?;
        append_native_unsigned_rational(gamma, &mut output)?;
        append_native_signed_rational(base_offset, &mut output)?;
        append_native_signed_rational(alt_offset, &mut output)?;
    }
    debug_assert_eq!(output.len(), 142);
    Ok(output)
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

/// Serialize the current Swift `hdrgm` XMP document byte-for-byte.
pub fn make_hdrgm_xmp(info: &[f64]) -> Result<Vec<u8>> {
    require_info_floats(info)?;
    let gain_min = [safe_log2(info[0]), safe_log2(info[1]), safe_log2(info[2])];
    let gain_max = [safe_log2(info[4]), safe_log2(info[5]), safe_log2(info[6])];
    let gamma = [info[7], info[8], info[9]];
    let offset_sdr = [info[10], info[11], info[12]];
    let offset_hdr = [info[13], info[14], info[15]];
    let cap_min = swift_max(safe_log2(info[16]), 0.0);
    let cap_max = safe_log2(info[17]);

    fn format_three(values: [f64; 3]) -> String {
        format!("{:.6} {:.6} {:.6}", values[0], values[1], values[2])
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
            "         <hdrgm:HDRCapacityMin>{:.6}</hdrgm:HDRCapacityMin>\n",
            "         <hdrgm:HDRCapacityMax>{:.6}</hdrgm:HDRCapacityMax>\n",
            "         <hdrgm:BaseRenditionIsHDR>False</hdrgm:BaseRenditionIsHDR>\n",
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
    );
    Ok(xml.into_bytes())
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

    #[test]
    fn apple_tmap_matches_current_swift_golden_bytes() {
        let bytes = make_apple_tmap_payload(&canonical_sample()).unwrap();
        let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(
            hex,
            "000000000040000000000000000100933a9300400000000000000000000100933a9300400000000000010000000100000000000000010000000000000001"
        );
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
        assert!(xmp.contains("0.321928 0.584963 0.807355"));
        assert!(xmp.contains("2.000000 2.321928 2.584963"));
        let multichannel = make_apple_tmap_payload(&info).unwrap();
        assert_eq!(multichannel.len(), 142);
        assert_eq!(multichannel[5], 0xc0);
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
