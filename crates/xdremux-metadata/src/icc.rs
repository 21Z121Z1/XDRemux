//! Minimum admission checks for an ISO JPEG's alternate RGB encoding.
//! This does not evaluate ICC transforms or certify complete ICC conformance.
use std::collections::BTreeSet;

use crate::{MetadataError, Result};

const CONTEXT: &str = "ISO alternate ICC";

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

/// Validate the bounded profile structure and the RGB/CICP binding before
/// copying the original bytes to tmap.prof. Missing ICC is handled by the caller;
/// a present invalid profile must never be treated as absent.
pub fn validate_iso_alternate_icc(profile: &[u8], alternate_is_hdr: bool) -> Result<()> {
    if profile.len() < 132 || u32_at(profile, 0)? != profile.len() {
        return Err(invalid("profile length does not match its header"));
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
    let mut cicp = None;
    for entry in profile[132..table_end].as_chunks::<12>().0 {
        if !signatures.insert(&entry[..4]) {
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
        if &entry[..4] == b"cicp" {
            cicp = Some(&profile[offset..end]);
        }
    }
    let Some(cicp) = cicp else {
        return if alternate_is_hdr {
            Err(invalid("HDR alternate profile requires a CICP tag"))
        } else {
            Ok(())
        };
    };
    // ICC.1:2022 9.2.17 and 10.3. A valid range flag can be zero or one;
    // gain compression's separate BT.601 matrix is not this RGB ICC matrix.
    if !matches!(&profile[12..16], b"scnr" | b"mntr") {
        return Err(invalid(
            "CICP is permitted only in input or display profiles",
        ));
    }
    if cicp.len() != 12 || &cicp[..4] != b"cicp" || cicp[4..8] != [0; 4] {
        return Err(invalid("invalid CICP tag structure"));
    }
    // H.273 (07/2024), Tables 2 and 3. Unspecified primaries/transfer (2)
    // retain the ICC's encoding; do not substitute a guessed PQ/BT.2020 profile.
    if !matches!(cicp[8], 1 | 2 | 4..=12 | 22)
        || !matches!(cicp[9], 1 | 2 | 4..=18)
        || cicp[10] != 0
        || cicp[11] > 1
    {
        return Err(invalid("invalid or incompatible RGB CICP code points"));
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

    #[test]
    fn checks_rgb_cicp_without_imposing_a_guessed_hdr_gamut() {
        for codes in [[9, 16, 0, 1], [12, 18, 0, 0], [2, 8, 0, 1], [1, 13, 0, 1]] {
            validate_iso_alternate_icc(&profile(Some(codes)), true).unwrap();
        }
        let srgb = include_bytes!("../../xdremux-codec/src/portable/srgb.icc");
        validate_iso_alternate_icc(srgb, false).unwrap();
        assert!(validate_iso_alternate_icc(srgb, true).is_err());
        validate_iso_alternate_icc(&profile(None), false).unwrap();
        assert!(validate_iso_alternate_icc(&profile(None), true).is_err());
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
