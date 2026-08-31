use crate::codec::ChromaSampling;
use crate::error::{FormatError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HevcDecoderConfigurationProfile {
    pub general_profile_idc: u8,
    pub chroma_sampling: ChromaSampling,
    pub luma_bit_depth: u8,
    pub chroma_bit_depth: u8,
}

fn invalid(message: impl Into<String>) -> FormatError {
    FormatError::invalid("hvcC", message)
}

/// Parses the codec-layout fields of an HEVCDecoderConfigurationRecord payload.
///
/// The input is the hvcC payload without the outer ISO-BMFF box header. This
/// parser deliberately reports storage properties only; semantic image channels
/// are a higher-level Gain Map concern.
pub fn parse_hvcc_profile(payload: &[u8]) -> Result<HevcDecoderConfigurationProfile> {
    if payload.len() <= 18 {
        return Err(FormatError::UnexpectedEof {
            context: "hvcC",
            offset: payload.len(),
            needed: 19usize.saturating_sub(payload.len()),
            end: payload.len(),
        });
    }
    if payload[0] != 1 {
        return Err(invalid(format!(
            "configurationVersion {} is unsupported",
            payload[0]
        )));
    }

    let general_profile_idc = payload[1] & 0x1f;
    let chroma_format_idc = payload[16] & 0x03;
    let chroma_sampling = ChromaSampling::from_hevc_chroma_format_idc(chroma_format_idc)?;
    let luma_bit_depth = (payload[17] & 0x07) + 8;
    let chroma_bit_depth = (payload[18] & 0x07) + 8;

    Ok(HevcDecoderConfigurationProfile {
        general_profile_idc,
        chroma_sampling,
        luma_bit_depth,
        chroma_bit_depth,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hvcc(chroma: u8, luma_minus_8: u8, chroma_minus_8: u8) -> Vec<u8> {
        let mut payload = vec![0u8; 19];
        payload[0] = 1;
        payload[1] = 4;
        payload[16] = 0xfc | chroma;
        payload[17] = 0xf8 | luma_minus_8;
        payload[18] = 0xf8 | chroma_minus_8;
        payload
    }

    #[test]
    fn parses_all_hevc_chroma_classes() {
        let cases = [
            (0, ChromaSampling::Mono400),
            (1, ChromaSampling::Yuv420),
            (2, ChromaSampling::Yuv422),
            (3, ChromaSampling::Yuv444),
        ];
        for (chroma, expected) in cases {
            let profile = parse_hvcc_profile(&hvcc(chroma, 0, 0)).unwrap();
            assert_eq!(profile.general_profile_idc, 4);
            assert_eq!(profile.chroma_sampling, expected);
            assert_eq!(profile.luma_bit_depth, 8);
            assert_eq!(profile.chroma_bit_depth, 8);
        }
    }

    #[test]
    fn preserves_declared_bit_depth() {
        let profile = parse_hvcc_profile(&hvcc(3, 2, 2)).unwrap();
        assert_eq!(profile.chroma_sampling, ChromaSampling::Yuv444);
        assert_eq!(profile.luma_bit_depth, 10);
        assert_eq!(profile.chroma_bit_depth, 10);
    }

    #[test]
    fn rejects_truncated_or_wrong_version_records() {
        assert!(parse_hvcc_profile(&[1; 18]).is_err());
        let mut payload = hvcc(1, 0, 0);
        payload[0] = 2;
        assert!(parse_hvcc_profile(&payload).is_err());
    }
}
