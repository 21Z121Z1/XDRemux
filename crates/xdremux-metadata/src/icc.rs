//! Admission of ISO alternate ICC profiles without rewriting their color data.
//! A bounded CMM probe detects missing transforms and CICP contradictions; it
//! does not certify every rendering intent or every native consumer.
use std::collections::BTreeSet;

use crate::{MetadataError, Result};
use moxcms::{
    curve_from_gamma, CicpColorPrimaries, CicpProfile, ColorProfile, Layout, MatrixCoefficients,
    ParsingOptions, RenderingIntent, TransferCharacteristics, TransformOptions,
};

const CONTEXT: &str = "ISO alternate ICC";
const MAX_PROFILE_BYTES: usize = 16 * 1024 * 1024;

fn invalid(message: &str) -> MetadataError {
    MetadataError::invalid(CONTEXT, message)
}
fn u32_at(bytes: &[u8], offset: usize) -> Result<usize> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| invalid("truncated profile field"))?;
    usize::try_from(u32::from_be_bytes(
        value.try_into().expect("four-byte field"),
    ))
    .map_err(|_| invalid("profile field exceeds addressable size"))
}

/// Check structure, required transform data, executable forward/reverse CMM
/// transforms and agreement with specified CICP fields. Missing ICC is a
/// separate caller case. An unsupported or invalid present ICC fails closed.
pub fn validate_iso_alternate_icc(profile: &[u8], alternate_is_hdr: bool) -> Result<()> {
    if profile.len() < 132
        || profile.len() >= MAX_PROFILE_BYTES
        || u32_at(profile, 0)? != profile.len()
    {
        return Err(invalid(
            "profile length does not match its header or exceeds the size limit",
        ));
    }
    if &profile[36..40] != b"acsp" {
        return Err(invalid("invalid profile signature"));
    }
    if !matches!(profile[8], 2 | 4) {
        return Err(invalid("unsupported profile version"));
    }
    if &profile[16..20] != b"RGB " || !matches!(&profile[20..24], b"XYZ " | b"Lab ") {
        return Err(invalid(
            "alternate encoding must be RGB with XYZ or Lab PCS",
        ));
    }
    if !matches!(&profile[12..16], b"scnr" | b"mntr" | b"prtr" | b"spac") {
        return Err(invalid(
            "profile class cannot describe an embedded RGB image",
        ));
    }
    let count = u32_at(profile, 128)?;
    let table_end = count
        .checked_mul(12)
        .and_then(|n| n.checked_add(132))
        .filter(|&end| end <= profile.len())
        .ok_or_else(|| invalid("tag table exceeds profile length"))?;
    let mut signatures = BTreeSet::new();
    let mut ranges = Vec::new();
    let mut cicp = None;
    for entry in profile[132..table_end].as_chunks::<12>().0 {
        if !signatures.insert(<[u8; 4]>::try_from(&entry[..4]).expect("tag signature")) {
            return Err(invalid("duplicate tag signature"));
        }
        let offset = u32_at(entry, 4)?;
        let size = u32_at(entry, 8)?;
        let end = offset
            .checked_add(size)
            .filter(|&end| end <= profile.len())
            .ok_or_else(|| invalid("tag data exceeds profile length"))?;
        if offset < table_end || offset % 4 != 0 || size < 8 {
            return Err(invalid("invalid tag data offset or size"));
        }
        if profile[offset + 4..offset + 8] != [0; 4] {
            return Err(invalid("nonzero reserved tag bytes"));
        }
        ranges.push((offset, end));
        if &entry[..4] == b"cicp" {
            cicp = Some(&profile[offset..end]);
        }
    }
    ranges.sort_unstable();
    for pair in ranges.windows(2) {
        if pair[1].0 < pair[0].1 && pair[0] != pair[1] {
            return Err(invalid("partially overlapping tag data"));
        }
    }
    // ICC.1 permits exact tag-data sharing. It does not permit a CICP-only
    // header to replace the required descriptive, white-point and transform tags.
    for tag in [*b"desc", *b"cprt", *b"wtpt"] {
        if !signatures.contains(&tag) {
            return Err(invalid("missing required profile tag"));
        }
    }
    let matrix = [*b"rXYZ", *b"gXYZ", *b"bXYZ", *b"rTRC", *b"gTRC", *b"bTRC"]
        .iter()
        .all(|t| signatures.contains(t))
        && &profile[20..24] == b"XYZ ";
    let lut = [*b"A2B0", *b"A2B1", *b"A2B2", *b"B2A0", *b"B2A1", *b"B2A2"]
        .iter()
        .any(|t| signatures.contains(t));
    if !matrix && !signatures.contains(b"A2B0") {
        return Err(invalid("missing RGB matrix/TRC or device-to-PCS transform"));
    }
    let codes = if let Some(cicp) = cicp {
        if !matches!(&profile[12..16], b"scnr" | b"mntr") {
            return Err(invalid(
                "CICP is permitted only in input or display profiles",
            ));
        }
        if cicp.len() != 12 || &cicp[..4] != b"cicp" || cicp[4..8] != [0; 4] {
            return Err(invalid("invalid CICP tag structure"));
        }
        if !matches!(cicp[8], 1 | 2 | 4..=12 | 22)
            || !matches!(cicp[9], 1 | 2 | 4..=18)
            || cicp[10] != 0
            || cicp[11] > 1
        {
            return Err(invalid("invalid or incompatible RGB CICP code points"));
        }
        Some([cicp[8], cicp[9], cicp[10], cicp[11]])
    } else {
        if alternate_is_hdr {
            return Err(invalid("HDR alternate profile requires a CICP tag"));
        }
        None
    };
    let mut parsed = ColorProfile::new_from_slice_with_options(
        profile,
        ParsingOptions {
            max_profile_size: MAX_PROFILE_BYTES,
            max_allowed_clut_size: MAX_PROFILE_BYTES,
            max_allowed_trc_size: 65_536,
        },
    )
    .map_err(|e| invalid(&format!("invalid or unsupported ICC transform data: {e}")))?;
    if parsed.description.is_none()
        || parsed.copyright.is_none()
        || !parsed
            .media_white_point
            .is_some_and(|w| [w.x, w.y, w.z].iter().all(|v| v.is_finite() && *v > 0.0))
    {
        return Err(invalid(
            "invalid required profile description, copyright or media white point",
        ));
    }
    // Never let CICP override the profile data whose consistency is being tested.
    parsed.cicp = None;
    check_transforms(&parsed, codes, lut)
}

fn check_transforms(profile: &ColorProfile, codes: Option<[u8; 4]>, has_lut: bool) -> Result<()> {
    let mut working = ColorProfile::new_srgb();
    working.red_trc = Some(curve_from_gamma(1.0));
    working.green_trc = working.red_trc.clone();
    working.blue_trc = working.red_trc.clone();
    working.cicp = None;
    let options = TransformOptions {
        rendering_intent: RenderingIntent::RelativeColorimetric,
        allow_use_cicp_transfer: false,
        prefer_fixed_point: false,
        ..Default::default()
    };
    // A cube exercises cross-channel LUTs; dense neutral and per-channel ramps
    // cover transfer curves. All observations are bounded, deterministic f64.
    let mut samples = Vec::new();
    for r in 0..=8 {
        for g in 0..=8 {
            for b in 0..=8 {
                samples.extend([f64::from(r) / 8., f64::from(g) / 8., f64::from(b) / 8.]);
            }
        }
    }
    for i in 0..=256 {
        let v = f64::from(i) / 256.;
        samples.extend([v, v, v, v, 0., 0., 0., v, 0., 0., 0., v]);
    }
    // Limited RGB code ranges depend on component bit depth. JPEG alternate
    // ICC does not supply that fact, so do not guess an 8/10/12/16-bit mapping.
    if codes.is_some_and(|c| c[3] == 0) {
        return Err(invalid("limited-range alternate ICC consistency requires a known component bit depth; unsupported"));
    }
    let input = &samples;
    let forward = profile
        .create_transform_f64(Layout::Rgb, &working, Layout::Rgb, options)
        .map_err(|e| invalid(&format!("cannot build ICC to linear RGB transform: {e}")))?;
    let reverse = working
        .create_transform_f64(Layout::Rgb, profile, Layout::Rgb, options)
        .map_err(|e| invalid(&format!("cannot build linear RGB to ICC transform: {e}")))?;
    let mut actual = vec![0.; input.len()];
    forward
        .transform(input, &mut actual)
        .map_err(|e| invalid(&format!("ICC transform failed: {e}")))?;
    let mut readback = vec![0.; input.len()];
    reverse
        .transform(&actual, &mut readback)
        .map_err(|e| invalid(&format!("ICC reverse transform failed: {e}")))?;
    if actual.iter().chain(&readback).any(|v| !v.is_finite())
        || actual[..3]
            .iter()
            .zip(&actual[728 * 3..729 * 3])
            .all(|(a, b)| (a - b).abs() < 1e-6)
        || readback[..3]
            .iter()
            .zip(&readback[728 * 3..729 * 3])
            .all(|(a, b)| (a - b).abs() < 1e-6)
    {
        return Err(invalid("ICC transform is non-finite or degenerate"));
    }
    let Some(c) = codes else {
        return Ok(());
    };
    if c[0] == 2 && c[1] == 2 && c[3] == 1 {
        return Ok(());
    }
    if (c[0] == 2 || c[1] == 2) && has_lut {
        return Err(invalid(
            "cannot verify partially unspecified CICP with LUT encoding",
        ));
    }
    let mut reference = ColorProfile::new_from_cicp(CicpProfile {
        color_primaries: CicpColorPrimaries::try_from(if c[0] == 2 { 1 } else { c[0] })
            .map_err(|_| invalid("unsupported CICP primaries"))?,
        transfer_characteristics: TransferCharacteristics::try_from(if c[1] == 2 {
            8
        } else {
            c[1]
        })
        .map_err(|_| invalid("unsupported CICP transfer"))?,
        matrix_coefficients: MatrixCoefficients::Identity,
        full_range: true,
    });
    if c[0] == 2 {
        reference.red_colorant = profile.red_colorant;
        reference.green_colorant = profile.green_colorant;
        reference.blue_colorant = profile.blue_colorant;
    }
    if c[1] == 2 {
        reference.red_trc = profile.red_trc.clone();
        reference.green_trc = profile.green_trc.clone();
        reference.blue_trc = profile.blue_trc.clone();
    }
    reference.cicp = None;
    let reference_transform = reference
        .create_transform_f64(Layout::Rgb, &working, Layout::Rgb, options)
        .map_err(|e| invalid(&format!("cannot verify CICP encoding: {e}")))?;
    let mut expected = vec![0.; samples.len()];
    reference_transform
        .transform(&samples, &mut expected)
        .map_err(|e| invalid(&format!("CICP reference transform failed: {e}")))?;
    // Product admission tolerance in normalized linear RGB, not an ICC/ISO
    // conformance threshold or a final image Delta E / compression error budget.
    if let Some((index, (a, b))) = actual
        .iter()
        .zip(&expected)
        .enumerate()
        .find(|(_, (a, b))| !b.is_finite() || (*a - *b).abs() > 0.002 + 0.005 * b.abs())
    {
        return Err(invalid(&format!("ICC transform contradicts its CICP encoding or exceeds the supported consistency tolerance at component {index}: {a} versus {b}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Deliberately synthetic: these are admission-test headers, not a color
    // transform or native-decoder acceptance fixture.
    fn profile(cicp: Option<[u8; 4]>) -> Vec<u8> {
        let mut p = vec![0; if cicp.is_some() { 156 } else { 132 }];
        let size = u32::try_from(p.len()).unwrap();
        p[..4].copy_from_slice(&size.to_be_bytes());
        p[8] = 4;
        p[9] = 0x40;
        p[12..16].copy_from_slice(b"mntr");
        p[16..20].copy_from_slice(b"RGB ");
        p[20..24].copy_from_slice(b"XYZ ");
        p[36..40].copy_from_slice(b"acsp");
        if let Some(codes) = cicp {
            p[128..132].copy_from_slice(&1_u32.to_be_bytes());
            p[132..136].copy_from_slice(b"cicp");
            p[136..140].copy_from_slice(&144_u32.to_be_bytes());
            p[140..144].copy_from_slice(&12_u32.to_be_bytes());
            p[144..148].copy_from_slice(b"cicp");
            p[152..156].copy_from_slice(&codes);
        }
        p
    }

    // Assemble fresh synthetic profiles with 4-byte tag alignment. The CMM's
    // encoder does not pad every variable-length text tag; no fixture is edited.
    fn encode_aligned(profile: &ColorProfile) -> Vec<u8> {
        let raw = profile.encode().unwrap();
        let count = u32_at(&raw, 128).unwrap();
        let table_end = 132 + 12 * count;
        let mut bytes = raw[..table_end].to_vec();
        for i in 0..count {
            let entry = 132 + 12 * i;
            let offset = u32_at(&raw, entry + 4).unwrap();
            let size = u32_at(&raw, entry + 8).unwrap();
            while !bytes.len().is_multiple_of(4) {
                bytes.push(0);
            }
            let new_offset = u32::try_from(bytes.len()).unwrap();
            bytes[entry + 4..entry + 8].copy_from_slice(&new_offset.to_be_bytes());
            bytes.extend_from_slice(&raw[offset..offset + size]);
        }
        let size = u32::try_from(bytes.len()).unwrap();
        bytes[..4].copy_from_slice(&size.to_be_bytes());
        bytes
    }

    #[test]
    fn accepts_complete_sdr_and_consistent_hdr_profiles() {
        let srgb = include_bytes!("../../xdremux-codec/src/portable/srgb.icc");
        validate_iso_alternate_icc(srgb, false).unwrap();
        assert!(validate_iso_alternate_icc(srgb, true).is_err());
        let mut p3 = ColorProfile::new_display_p3_pq();
        // D65 Display P3 is code 12, distinct from DCI P3's code 11.
        p3.cicp.as_mut().unwrap().color_primaries = CicpColorPrimaries::Smpte432;
        let mut hlg = ColorProfile::new_bt2020_hlg();
        // Use the declared reference HLG EOTF (nominal system gamma 1.2),
        // rather than the generator's different black-level/OOTF convention.
        hlg.red_trc = Some(TransferCharacteristics::Hlg.try_into().unwrap());
        hlg.green_trc = hlg.red_trc.clone();
        hlg.blue_trc = hlg.red_trc.clone();
        for mut p in [
            ColorProfile::new_bt2020_pq(),
            p3,
            hlg,
            ColorProfile::new_srgb(),
        ] {
            p.cicp.as_mut().unwrap().matrix_coefficients = MatrixCoefficients::Identity;
            p.cicp.as_mut().unwrap().full_range = true;
            let bytes = encode_aligned(&p);
            validate_iso_alternate_icc(&bytes, true)
                .unwrap_or_else(|e| panic!("{:?}: {e}", p.cicp));
            // Correct curves with conflicting primaries and correct primaries
            // with conflicting transfer must each fail, independently.
            let mut conflicting = p.clone();
            let codes = conflicting.cicp.as_mut().unwrap();
            codes.color_primaries = if codes.color_primaries == CicpColorPrimaries::Bt709 {
                CicpColorPrimaries::Bt2020
            } else {
                CicpColorPrimaries::Bt709
            };
            assert!(
                validate_iso_alternate_icc(&encode_aligned(&conflicting), true)
                    .unwrap_err()
                    .to_string()
                    .contains("contradicts")
            );
            let codes = p.cicp.as_mut().unwrap();
            codes.transfer_characteristics =
                if codes.transfer_characteristics == TransferCharacteristics::Srgb {
                    TransferCharacteristics::Smpte2084
                } else {
                    TransferCharacteristics::Srgb
                };
            assert!(validate_iso_alternate_icc(&encode_aligned(&p), true)
                .unwrap_err()
                .to_string()
                .contains("contradicts"));
        }
    }

    #[test]
    fn unspecified_cicp_retains_profile_data_and_limited_range_is_explicitly_unsupported() {
        let mut p = ColorProfile::new_srgb();
        for (primaries, transfer) in [(1, 2), (2, 13), (2, 2)] {
            p.cicp = Some(CicpProfile {
                color_primaries: CicpColorPrimaries::try_from(primaries).unwrap(),
                transfer_characteristics: TransferCharacteristics::try_from(transfer).unwrap(),
                matrix_coefficients: MatrixCoefficients::Identity,
                full_range: true,
            });
            validate_iso_alternate_icc(&encode_aligned(&p), true).unwrap();
        }
        p.cicp.as_mut().unwrap().full_range = false;
        assert!(validate_iso_alternate_icc(&encode_aligned(&p), true)
            .unwrap_err()
            .to_string()
            .contains("bit depth; unsupported"));
    }

    #[test]
    fn rejects_profiles_without_transforms_and_conflicting_cicp() {
        assert!(validate_iso_alternate_icc(&profile(Some([9, 16, 0, 1])), true).is_err());
        let srgb = include_bytes!("../../xdremux-codec/src/portable/srgb.icc");
        let count = u32::from_be_bytes(srgb[128..132].try_into().unwrap()) as usize;
        let table_end = 132 + 12 * count;
        let mut bad = srgb[..table_end].to_vec();
        bad.extend_from_slice(b"cicp");
        bad.extend_from_slice(&u32::try_from(srgb.len() + 12).unwrap().to_be_bytes());
        bad.extend_from_slice(&12_u32.to_be_bytes());
        bad.extend_from_slice(&srgb[table_end..]);
        for i in 0..count {
            let at = 136 + i * 12;
            let offset = u32::from_be_bytes(bad[at..at + 4].try_into().unwrap());
            bad[at..at + 4].copy_from_slice(&(offset + 12).to_be_bytes());
        }
        bad.extend_from_slice(b"cicp\0\0\0\0\x09\x10\0\x01");
        let size = u32::try_from(bad.len()).unwrap();
        bad[..4].copy_from_slice(&size.to_be_bytes());
        bad[128..132].copy_from_slice(&u32::try_from(count + 1).unwrap().to_be_bytes());
        assert!(validate_iso_alternate_icc(&bad, true).is_err());
    }

    #[test]
    fn rejects_damaged_profile_headers_tables_and_cicp() {
        let good = profile(Some([9, 16, 0, 1]));
        for length in [0, 127, 131, 143, 155] {
            assert!(validate_iso_alternate_icc(&good[..length], true).is_err());
        }
        for (offset, bytes) in [
            (0, 200_u32.to_be_bytes()),
            (36, *b"bad!"),
            (16, *b"GRAY"),
            (20, *b"CMYK"),
            (12, *b"link"),
            (128, u32::MAX.to_be_bytes()),
            (136, u32::MAX.to_be_bytes()),
            (136, 140_u32.to_be_bytes()),
            (140, u32::MAX.to_be_bytes()),
            (144, *b"bad!"),
            (148, [1, 0, 0, 0]),
            (152, [0, 16, 0, 1]),
            (152, [9, 3, 0, 1]),
            (152, [9, 16, 6, 1]),
            (152, [9, 16, 0, 2]),
        ] {
            let mut bad = good.clone();
            bad[offset..offset + 4].copy_from_slice(&bytes);
            assert!(
                validate_iso_alternate_icc(&bad, true).is_err(),
                "offset {offset}"
            );
        }
    }
}
