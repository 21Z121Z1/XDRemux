use xdremux_format::{HevcDecoderConfigurationProfile, JpegFrameProfile};

use crate::{
    validate_source_profile, GainMapChannels, GainMapCodec, GainMapSourceProfile,
    GainMapStorageProfile, PlannerError, Result,
};

pub fn gain_map_channels_from_count(channel_count: u8) -> Result<GainMapChannels> {
    match channel_count {
        1 => Ok(GainMapChannels::Mono),
        3 => Ok(GainMapChannels::Rgb),
        _ => Err(PlannerError::InvalidGainMapProfile(
            "semantic channel count must be 1 or 3",
        )),
    }
}

/// Converts a probed JPEG frame header into the engine's Gain Map source model.
///
/// Pixel decoding is intentionally not involved: dimensions, precision,
/// component count, and sampling all come from the JPEG SOF profile.
pub fn gain_map_source_profile_from_jpeg(
    frame: &JpegFrameProfile,
) -> Result<GainMapSourceProfile> {
    let source = GainMapSourceProfile {
        width: u32::from(frame.width),
        height: u32::from(frame.height),
        channels: gain_map_channels_from_count(frame.component_count())?,
        storage: GainMapStorageProfile {
            codec: GainMapCodec::Jpeg,
            chroma: frame.chroma_sampling,
            luma_bit_depth: frame.precision,
            chroma_bit_depth: frame.precision,
        },
    };
    validate_source_profile(source)?;
    Ok(source)
}

/// Converts a parsed hvcC storage profile plus HEIF semantic channel evidence
/// into the engine's Gain Map source model.
///
/// `semantic_channel_count` is deliberately separate from hvcC chroma sampling:
/// HEIF `pixi` describes semantic image channels while hvcC describes coded
/// chroma layout. Keeping both inputs prevents the old 3-channel == 4:4:4
/// assumption from reappearing at the analysis boundary.
pub fn gain_map_source_profile_from_hevc(
    width: u32,
    height: u32,
    semantic_channel_count: u8,
    configuration: HevcDecoderConfigurationProfile,
) -> Result<GainMapSourceProfile> {
    let source = GainMapSourceProfile {
        width,
        height,
        channels: gain_map_channels_from_count(semantic_channel_count)?,
        storage: GainMapStorageProfile {
            codec: GainMapCodec::Hevc,
            chroma: Some(configuration.chroma_sampling),
            luma_bit_depth: configuration.luma_bit_depth,
            chroma_bit_depth: configuration.chroma_bit_depth,
        },
    };
    validate_source_profile(source)?;
    Ok(source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use xdremux_format::{parse_hvcc_profile, probe_jpeg_frame_profile, ChromaSampling};

    fn jpeg_with_sof(components: &[(u8, u8, u8, u8)], precision: u8) -> Vec<u8> {
        let mut output = vec![0xff, 0xd8, 0xff, 0xc0];
        let segment_length = 8 + components.len() * 3;
        output.extend_from_slice(&(segment_length as u16).to_be_bytes());
        output.push(precision);
        output.extend_from_slice(&16u16.to_be_bytes());
        output.extend_from_slice(&24u16.to_be_bytes());
        output.push(components.len() as u8);
        for (id, horizontal, vertical, table) in components {
            output.push(*id);
            output.push((horizontal << 4) | vertical);
            output.push(*table);
        }
        output.extend_from_slice(&[0xff, 0xda]);
        output
    }

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
    fn jpeg_probe_flows_into_source_profile_without_decoding() {
        let frame = probe_jpeg_frame_profile(&jpeg_with_sof(
            &[(1, 2, 2, 0), (2, 1, 1, 1), (3, 1, 1, 1)],
            8,
        ))
        .unwrap();
        let source = gain_map_source_profile_from_jpeg(&frame).unwrap();

        assert_eq!(source.width, 24);
        assert_eq!(source.height, 16);
        assert_eq!(source.channels, GainMapChannels::Rgb);
        assert_eq!(source.storage.codec, GainMapCodec::Jpeg);
        assert_eq!(source.storage.chroma, Some(ChromaSampling::Yuv420));
        assert_eq!(source.storage.luma_bit_depth, 8);
        assert_eq!(source.storage.chroma_bit_depth, 8);
    }

    #[test]
    fn hvcc_probe_keeps_semantic_channels_separate_from_storage_sampling() {
        let configuration = parse_hvcc_profile(&hvcc(1, 2, 2)).unwrap();
        let source = gain_map_source_profile_from_hevc(960, 720, 3, configuration).unwrap();

        assert_eq!(source.channels, GainMapChannels::Rgb);
        assert_eq!(source.storage.codec, GainMapCodec::Hevc);
        assert_eq!(source.storage.chroma, Some(ChromaSampling::Yuv420));
        assert_eq!(source.storage.luma_bit_depth, 10);
        assert_eq!(source.storage.chroma_bit_depth, 10);
    }

    #[test]
    fn source_profile_boundary_rejects_semantic_storage_mismatch() {
        let configuration = parse_hvcc_profile(&hvcc(1, 0, 0)).unwrap();
        assert_eq!(
            gain_map_source_profile_from_hevc(960, 720, 1, configuration),
            Err(PlannerError::InvalidGainMapProfile(
                "mono semantics cannot use a color chroma layout"
            ))
        );
    }

    #[test]
    fn source_profile_boundary_rejects_non_gain_map_channel_counts() {
        let frame = probe_jpeg_frame_profile(&jpeg_with_sof(
            &[(1, 1, 1, 0), (2, 1, 1, 0), (3, 1, 1, 0), (4, 1, 1, 0)],
            8,
        ))
        .unwrap();
        assert_eq!(
            gain_map_source_profile_from_jpeg(&frame),
            Err(PlannerError::InvalidGainMapProfile(
                "semantic channel count must be 1 or 3"
            ))
        );
    }
}
