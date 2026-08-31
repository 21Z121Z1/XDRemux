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
    if left < right { right } else { left }
}

fn safe_log2(value: f64) -> f64 {
    if value > 0.0 { value.log2() } else { 0.0 }
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
    if (value - rounded).abs() < 1e-12 && rounded >= f64::from(i32::MIN) && rounded <= f64::from(i32::MAX) {
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

fn canonical_first_channel_values(info: &[f64]) -> Result<[f64; 7]> {
    require_info_floats(info)?;
    Ok([
        swift_max(swift_max(info[16], 1.0).log2(), 0.0),
        swift_max(info[17], 1.0).log2(),
        swift_max(swift_max(info[0], 1.0).log2(), 0.0),
        swift_max(info[4], 1.0).log2(),
        info[7],
        info[10],
        info[13],
    ])
}

/// Generate the 62-byte Apple/ImageIO tmap payload used by the current Swift core.
pub fn make_apple_tmap_payload(info: &[f64]) -> Result<Vec<u8>> {
    let values = canonical_first_channel_values(info)?;
    let mut output = vec![0, 0, 0, 0, 0, 0x40];
    for value in values {
        append_imageio_rational(value, &mut output);
    }
    debug_assert_eq!(output.len(), 62);
    Ok(output)
}

/// Generate the 142-byte multichannel ImageIO-native tmap compatibility payload.
///
/// The current Swift implementation intentionally repeats the proven first-channel
/// values across all three channels rather than consuming per-channel variants.
pub fn make_imageio_native_tmap_payload(info: &[f64]) -> Result<Vec<u8>> {
    let [cap_min, cap_max, gain_min, gain_max, gamma, base_offset, alt_offset] =
        canonical_first_channel_values(info)?;

    let mut output = Vec::with_capacity(142);
    output.push(0);
    push_u16_be(0, &mut output);
    push_u16_be(0, &mut output);
    output.push(0xC0);
    append_native_unsigned_rational(cap_min, &mut output)?;
    append_native_unsigned_rational(cap_max, &mut output)?;

    for _ in 0..3 {
        append_native_signed_rational(gain_min, &mut output)?;
        append_native_signed_rational(gain_max, &mut output)?;
        append_native_unsigned_rational(gamma, &mut output)?;
        append_native_signed_rational(base_offset, &mut output)?;
        append_native_signed_rational(alt_offset, &mut output)?;
    }
    debug_assert_eq!(output.len(), 142);
    Ok(output)
}

/// Restore the three reserved ISO GainMapMetadata bytes omitted by ImageIO.
pub fn make_strict_tmap_payload(payload: &[u8]) -> Result<Vec<u8>> {
    if payload.len() != 62 && payload.len() != 142 {
        return Err(MetadataError::invalid(
            "strict ISO 21496 tmap",
            format!("expected a 62- or 142-byte ImageIO payload, got {}", payload.len()),
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
        "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"XMP Core 6.0.0\">\n\
   <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
      <rdf:Description rdf:about=\"\"\n\
            xmlns:hdrgm=\"http://ns.adobe.com/hdr-gain-map/1.0/\"\n\
            xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"\n\
            xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\">\n\
         <hdrgm:Version>1.0</hdrgm:Version>\n\
         <hdrgm:GainMapMin>{}</hdrgm:GainMapMin>\n\
         <hdrgm:GainMapMax>{}</hdrgm:GainMapMax>\n\
         <hdrgm:Gamma>{}</hdrgm:Gamma>\n\
         <hdrgm:OffsetSDR>{}</hdrgm:OffsetSDR>\n\
         <hdrgm:OffsetHDR>{}</hdrgm:OffsetHDR>\n\
         <hdrgm:HDRCapacityMin>{:.6}</hdrgm:HDRCapacityMin>\n\
         <hdrgm:HDRCapacityMax>{:.6}</hdrgm:HDRCapacityMax>\n\
         <hdrgm:BaseRenditionIsHDR>False</hdrgm:BaseRenditionIsHDR>\n\
      </rdf:Description>\n\
   </rdf:RDF>\n\
</x:xmpmeta>\n\
<?xpacket end=\"w\"?>",
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
            1.0, 1.0, 1.0, 1.0, ratio, ratio, ratio, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 1.0, ratio, ratio, 0.0,
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
    fn distinct_channels_are_preserved_in_xmp_but_not_first_channel_tmap() {
        let info = [
            1.25, 1.5, 1.75, 1.0, 4.0, 5.0, 6.0, 0.8, 1.1, 1.2, 0.01, 0.02, 0.03,
            0.04, 0.05, 0.06, 1.5, 6.5, 2.0, 0.0,
        ];
        let xmp = String::from_utf8(make_hdrgm_xmp(&info).unwrap()).unwrap();
        assert!(xmp.contains("0.321928 0.584963 0.807355"));
        assert!(xmp.contains("2.000000 2.321928 2.584963"));
        let first_channel_only = make_apple_tmap_payload(&info).unwrap();
        assert_eq!(first_channel_only.len(), 62);
    }

    #[test]
    fn malformed_lengths_and_strict_payloads_fail_closed() {
        assert!(make_apple_tmap_payload(&[1.0; 17]).is_err());
        assert!(make_hdrgm_xmp(&[1.0; 17]).is_err());
        assert!(make_strict_tmap_payload(&[0; 63]).is_err());
    }
}
