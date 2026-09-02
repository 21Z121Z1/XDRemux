use xdremux_container::{OppoPortraitConfig, OppoPortraitDepth};
use xdremux_engine::AppleGainMapFacts;

#[cfg(any(target_os = "macos", test))]
use std::io::Read;
#[cfg(any(target_os = "macos", test))]
use ruzstd::decoding::StreamingDecoder;
#[cfg(any(target_os = "macos", test))]
use xdremux_container::{extract_oppo_portrait_source, parse_oppo_portrait_depth};
#[cfg(any(target_os = "macos", test))]
use xdremux_format::{jpeg_image_end, probe_jpeg_frame_profile};
#[cfg(any(target_os = "macos", test))]
use crate::{Result, RuntimeError};

#[cfg(target_os = "macos")]
use std::io::Write;
#[cfg(target_os = "macos")]
use std::path::Path;

#[cfg(target_os = "macos")]
use crate::apple_adapter::AppleAdapterClient;

#[cfg(any(target_os = "macos", test))]
const MAX_DECODED_PORTRAIT_DEPTH_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct ApplePortraitSourcePreflight {
    pub base_jpeg: Vec<u8>,
    pub gain_map_jpeg: Vec<u8>,
    pub depth: OppoPortraitDepth,
    pub config: OppoPortraitConfig,
    pub private_gain_map_info: Option<Vec<u8>>,
    pub base_width: u32,
    pub base_height: u32,
    pub gain_map: AppleGainMapFacts,
}

#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct SplitPortraitSourceImage {
    base_jpeg: Vec<u8>,
    gain_map_jpeg: Vec<u8>,
    base_width: u32,
    base_height: u32,
    gain_map_width: u32,
    gain_map_height: u32,
}

#[cfg(any(target_os = "macos", test))]
fn split_portrait_source_image(source_image: &[u8]) -> Result<SplitPortraitSourceImage> {
    let base_end = jpeg_image_end(source_image, 0)
        .map_err(|error| RuntimeError::external("Portrait src.image base JPEG", error))?;
    let second_marker_end = base_end.checked_add(3).ok_or_else(|| {
        RuntimeError::new("Portrait src.image", "second JPEG marker offset overflows")
    })?;
    if source_image.get(base_end..second_marker_end) != Some(&[0xff, 0xd8, 0xff]) {
        return Err(RuntimeError::new(
            "Portrait src.image",
            "does not contain adjacent base and Gain Map JPEGs",
        ));
    }

    jpeg_image_end(source_image, base_end)
        .map_err(|error| RuntimeError::external("Portrait src.image Gain Map JPEG", error))?;

    let base_jpeg = source_image
        .get(..base_end)
        .ok_or_else(|| RuntimeError::new("Portrait src.image", "base JPEG is out of bounds"))?
        .to_vec();
    let gain_map_jpeg = source_image
        .get(base_end..)
        .ok_or_else(|| RuntimeError::new("Portrait src.image", "Gain Map JPEG is out of bounds"))?
        .to_vec();
    let base_profile = probe_jpeg_frame_profile(&base_jpeg)
        .map_err(|error| RuntimeError::external("Portrait src.image base JPEG profile", error))?;
    let gain_profile = probe_jpeg_frame_profile(&gain_map_jpeg).map_err(|error| {
        RuntimeError::external("Portrait src.image Gain Map JPEG profile", error)
    })?;

    Ok(SplitPortraitSourceImage {
        base_jpeg,
        gain_map_jpeg,
        base_width: u32::from(base_profile.width),
        base_height: u32::from(base_profile.height),
        gain_map_width: u32::from(gain_profile.width),
        gain_map_height: u32::from(gain_profile.height),
    })
}

#[cfg(any(target_os = "macos", test))]
fn decode_oppo_portrait_depth(compressed: &[u8]) -> Result<OppoPortraitDepth> {
    if compressed.is_empty() {
        return Err(RuntimeError::new(
            "OPPO Portrait depth",
            "compressed rear.depth is empty",
        ));
    }

    let decoder = StreamingDecoder::new(compressed)
        .map_err(|error| RuntimeError::external("OPPO Portrait zstd decoder", error))?;
    let mut bounded = decoder.take(MAX_DECODED_PORTRAIT_DEPTH_BYTES + 1);
    let mut decoded = Vec::new();
    bounded
        .read_to_end(&mut decoded)
        .map_err(|error| RuntimeError::external("OPPO Portrait zstd decode", error))?;
    if u64::try_from(decoded.len()).unwrap_or(u64::MAX) > MAX_DECODED_PORTRAIT_DEPTH_BYTES {
        return Err(RuntimeError::new(
            "OPPO Portrait depth",
            format!(
                "decoded rear.depth exceeds {} MiB safety limit",
                MAX_DECODED_PORTRAIT_DEPTH_BYTES / (1024 * 1024)
            ),
        ));
    }

    parse_oppo_portrait_depth(&decoded)
        .map_err(|error| RuntimeError::external("OPPO Portrait depth parse", error))
}

#[cfg(target_os = "macos")]
pub(crate) fn prepare_apple_portrait_source(
    adapter_executable: &Path,
    source: &[u8],
) -> Result<ApplePortraitSourcePreflight> {
    let source = extract_oppo_portrait_source(source)
        .map_err(|error| RuntimeError::external("OPPO Portrait source extraction", error))?;
    let split = split_portrait_source_image(&source.source_image)?;
    let depth = decode_oppo_portrait_depth(&source.compressed_depth)?;

    let mut image_file = tempfile::Builder::new()
        .prefix("xdremux-portrait-src-")
        .suffix(".jpg")
        .tempfile()
        .map_err(|error| RuntimeError::external("Portrait src.image temporary file", error))?;
    image_file
        .write_all(&source.source_image)
        .map_err(|error| RuntimeError::external("Portrait src.image temporary write", error))?;
    image_file
        .flush()
        .map_err(|error| RuntimeError::external("Portrait src.image temporary flush", error))?;

    let gain_map = AppleAdapterClient::new(adapter_executable.to_path_buf())
        .imageio_gain_map_facts(image_file.path())?;
    if !gain_map.supports_portrait_source() {
        return Err(RuntimeError::new(
            "Apple Portrait source",
            format!(
                "unsupported ImageIO Gain Map pixel format {}",
                gain_map.pixel_format
            ),
        ));
    }
    if !gain_map.has_geometry(split.gain_map_width, split.gain_map_height) {
        return Err(RuntimeError::new(
            "Apple Portrait source",
            format!(
                "ImageIO Gain Map geometry {}x{} does not match JPEG {}x{}",
                gain_map.width, gain_map.height, split.gain_map_width, split.gain_map_height
            ),
        ));
    }

    Ok(ApplePortraitSourcePreflight {
        base_jpeg: split.base_jpeg,
        gain_map_jpeg: split.gain_map_jpeg,
        depth,
        config: source.config,
        private_gain_map_info: source.private_gain_map_info,
        base_width: split.base_width,
        base_height: split.base_height,
        gain_map,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn portrait_fixture() -> Vec<u8> {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/proxdr/oppo/find-x9-ultra/uhdr-portrait-01.heic");
        fs::read(fixture).expect("read committed portrait fixture")
    }

    #[test]
    fn splits_committed_portrait_source_with_hardened_jpeg_boundaries() {
        let source = portrait_fixture();
        let source = extract_oppo_portrait_source(&source).expect("extract OPPO portrait source");
        let split = split_portrait_source_image(&source.source_image)
            .expect("split adjacent base/Gain Map JPEGs");

        assert!(split.base_width > 0);
        assert!(split.base_height > 0);
        assert!(split.gain_map_width > 0);
        assert!(split.gain_map_height > 0);
        assert_eq!(split.base_jpeg.get(..2), Some(&[0xff, 0xd8][..]));
        assert_eq!(split.gain_map_jpeg.get(..2), Some(&[0xff, 0xd8][..]));
    }

    #[test]
    fn decodes_committed_portrait_depth_without_external_zstd() {
        let source = portrait_fixture();
        let source = extract_oppo_portrait_source(&source).expect("extract OPPO portrait source");
        let depth = decode_oppo_portrait_depth(&source.compressed_depth)
            .expect("decode and parse OPPO Portrait depth");

        assert!(depth.header.width > 0);
        assert!(depth.header.height > 0);
        assert!(!depth.ranks.is_empty());
        assert_eq!(
            depth.ranks.len(),
            usize::try_from(depth.header.width).unwrap()
                * usize::try_from(depth.header.height).unwrap()
        );
    }

    #[test]
    fn rejects_one_jpeg_from_the_portrait_pair_as_a_complete_source_image() {
        let source = portrait_fixture();
        let source = extract_oppo_portrait_source(&source).expect("extract OPPO portrait source");
        let split = split_portrait_source_image(&source.source_image)
            .expect("split adjacent base/Gain Map JPEGs");

        assert!(split_portrait_source_image(&split.base_jpeg).is_err());
    }

    #[test]
    fn rejects_empty_compressed_depth() {
        assert!(decode_oppo_portrait_depth(&[]).is_err());
    }
}
