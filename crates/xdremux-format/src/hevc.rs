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

/// Colour description from the first SPS `vui_parameters`, when the bitstream
/// carries one. `Ok(None)` means the VUI has no colour description. A present
/// SPS that cannot be walked is an error, not an implied SDR transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HevcVuiColor {
    pub full_range: bool,
    pub primaries: u8,
    pub transfer: u8,
    pub matrix: u8,
}

pub fn hvcc_vui_color(payload: &[u8]) -> Result<Option<HevcVuiColor>> {
    let sps = first_sps_nal(payload)?;
    vui_color_from_sps_nal(&sps)
}

fn first_sps_nal(payload: &[u8]) -> Result<Vec<u8>> {
    if payload.len() < 23 || payload[0] != 1 {
        return Err(invalid(
            "HEVCDecoderConfigurationRecord is truncated or not version 1",
        ));
    }
    let arrays = usize::from(payload[22]);
    let mut offset = 23usize;
    for _ in 0..arrays {
        let header = *payload
            .get(offset)
            .ok_or_else(|| invalid("hvcC array header is truncated"))?;
        let nal_type = header & 0x3f;
        let count = u16::from_be_bytes(
            payload
                .get(offset + 1..offset + 3)
                .ok_or_else(|| invalid("hvcC NAL count is truncated"))?
                .try_into()
                .expect("two bytes"),
        );
        offset += 3;
        for _ in 0..count {
            let length = u16::from_be_bytes(
                payload
                    .get(offset..offset + 2)
                    .ok_or_else(|| invalid("hvcC NAL length is truncated"))?
                    .try_into()
                    .expect("two bytes"),
            ) as usize;
            offset += 2;
            let nal = payload
                .get(offset..offset + length)
                .ok_or_else(|| invalid("hvcC NAL payload is truncated"))?;
            offset += length;
            if nal_type == 33 {
                return Ok(nal.to_vec());
            }
        }
    }
    Err(invalid("hvcC does not contain an SPS"))
}

fn vui_color_from_sps_nal(nal: &[u8]) -> Result<Option<HevcVuiColor>> {
    if nal.len() < 2 || (nal[0] >> 1) & 0x3f != 33 {
        return Err(invalid("SPS NAL header is missing"));
    }
    let rbsp = nal_rbsp(&nal[2..]);
    let mut bits = BitReader::new(&rbsp);
    // Production HEVC streams and ExoPlayer store this 0..15 id in four bits.
    // A ue(v) read of those same bytes does not land on the profile that follows.
    let _vps_id = bits.bits(4)?;
    let max_sub_layers = bits.bits(3)?;
    let _nesting = bits.bit()?;
    // profile_space (2), tier (1), profile_idc (5), most significant bit first.
    let profile_idc = bits.bits(8)? & 0x1f;
    let compatibility = bits.bits(32)?;
    bits.bits(4)?;
    let special = (4..=11).contains(&(profile_idc as u8))
        || (4..=11).any(|flag| (compatibility >> flag) & 1 == 1);
    if special {
        return Err(invalid(
            "SPS profile constraint flags are not the Main or Main Still path",
        ));
    }
    bits.bits(44)?;
    let _level = bits.bits(8)?;
    for _ in 0..max_sub_layers {
        let _profile = bits.bit()?;
        let _level_present = bits.bit()?;
    }
    if max_sub_layers > 0 {
        for _ in max_sub_layers..8 {
            bits.bits(2)?;
        }
    }
    let _sps_id = bits.ue()?;
    let chroma = bits.ue()?;
    if chroma == 3 {
        let _separate = bits.bit()?;
    }
    let _width = bits.ue()?;
    let _height = bits.ue()?;
    if bits.bit()? == 1 {
        for _ in 0..4 {
            let _window = bits.ue()?;
        }
    }
    let _luma = bits.ue()?;
    let _chroma_depth = bits.ue()?;
    let _poc = bits.ue()?;
    let ordering = bits.bit()?;
    let start = if ordering == 1 { 0 } else { max_sub_layers };
    for _ in start..=max_sub_layers {
        let _buffer = bits.ue()?;
        let _reorder = bits.ue()?;
        let _latency = bits.ue()?;
    }
    for _ in 0..6 {
        let _coding = bits.ue()?;
    }
    if bits.bit()? == 1 {
        return Err(invalid("SPS scaling list is not read for the VUI"));
    }
    let _amp = bits.bit()?;
    let _sao = bits.bit()?;
    if bits.bit()? == 1 {
        return Err(invalid("SPS PCM block is not read for the VUI"));
    }
    let sets = bits.ue()?;
    if sets > 64 {
        return Err(invalid(
            "SPS short-term reference picture set count is too large",
        ));
    }
    for index in 0..sets {
        if index != 0 && bits.bit()? == 1 {
            return Err(invalid(
                "inter-predicted short-term reference picture set is not read",
            ));
        }
        let negative = bits.ue()?;
        let positive = bits.ue()?;
        if negative.saturating_add(positive) > 64 {
            return Err(invalid("short-term reference picture set is too large"));
        }
        for _ in 0..negative.saturating_add(positive) {
            let _delta = bits.ue()?;
            let _used = bits.bit()?;
        }
    }
    let _long_term = bits.bit()?;
    let _mvp = bits.bit()?;
    let _strong = bits.bit()?;
    if bits.bit()? == 0 {
        return Ok(None);
    }
    if bits.bit()? == 1 {
        let aspect = bits.bits(8)?;
        if aspect == 255 {
            bits.bits(32)?;
        }
    }
    if bits.bit()? == 1 {
        let _overscan = bits.ue()?;
    }
    if bits.bit()? == 0 {
        return Ok(None);
    }
    let _format = bits.bits(3)?;
    let full_range = bits.bit()? == 1;
    if bits.bit()? == 0 {
        return Ok(None);
    }
    Ok(Some(HevcVuiColor {
        full_range,
        primaries: bits.bits(8)? as u8,
        transfer: bits.bits(8)? as u8,
        matrix: bits.bits(8)? as u8,
    }))
}

fn nal_rbsp(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len());
    let mut zeros = 0u8;
    for byte in payload {
        if zeros == 2 && *byte == 3 {
            zeros = 0;
            continue;
        }
        out.push(*byte);
        zeros = if *byte == 0 { zeros + 1 } else { 0 };
    }
    out
}

struct BitReader<'a> {
    data: &'a [u8],
    index: usize,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, index: 0 }
    }

    fn bit(&mut self) -> Result<u32> {
        let byte = self
            .data
            .get(self.index / 8)
            .copied()
            .ok_or_else(|| invalid("SPS ended before the VUI"))?;
        let value = u32::from((byte >> (7 - (self.index % 8))) & 1);
        self.index += 1;
        Ok(value)
    }

    fn bits(&mut self, count: u32) -> Result<u32> {
        let mut value = 0u32;
        for _ in 0..count {
            value = (value << 1) | self.bit()?;
        }
        Ok(value)
    }

    fn ue(&mut self) -> Result<u32> {
        let mut zeros = 0u32;
        while self.bit()? == 0 {
            zeros += 1;
            if zeros > 31 {
                return Err(invalid("SPS exp-golomb value is too large"));
            }
        }
        Ok(((1u32 << zeros) - 1).saturating_add(if zeros == 0 { 0 } else { self.bits(zeros)? }))
    }
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
    fn reads_pq_vui_from_an_x265_sps() {
        // Annex-B SPS from an 8-bit x265 frame tagged BT.2020 / SMPTE ST 2084 / full range.
        let nal = [
            0x42, 0x01, 0x01, 0x01, 0x60, 0x00, 0x00, 0x03, 0x00, 0x90, 0x00, 0x00, 0x03, 0x00,
            0x00, 0x03, 0x00, 0x1e, 0xa0, 0x20, 0x81, 0x05, 0x96, 0x56, 0x69, 0x24, 0xca, 0xf0,
            0x16, 0xe1, 0x22, 0x01, 0x20, 0x80, 0x00, 0x00, 0x03, 0x00, 0x80, 0x00, 0x00, 0x03,
            0x00, 0x84,
        ];
        let mut hvcc = vec![0u8; 23];
        hvcc[0] = 1;
        hvcc[22] = 1;
        hvcc.push(0x20 | 33);
        hvcc.extend((1u16).to_be_bytes());
        hvcc.extend((nal.len() as u16).to_be_bytes());
        hvcc.extend(nal);
        let color = hvcc_vui_color(&hvcc).unwrap().unwrap();
        assert!(color.full_range);
        assert_eq!(color.primaries, 9);
        assert_eq!(color.transfer, 16);
        assert_eq!(color.matrix, 9);
    }

    #[test]
    fn rejects_truncated_or_wrong_version_records() {
        assert!(parse_hvcc_profile(&[1; 18]).is_err());
        let mut payload = hvcc(1, 0, 0);
        payload[0] = 2;
        assert!(parse_hvcc_profile(&payload).is_err());
    }
}
