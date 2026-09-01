use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use uuid::Uuid;
use xdremux_codec::{
    GainMapTileEncodeRequest, JpegRasterDecodeRequest, LibHeifProvider, PrimaryHeifEncodeRequest,
    RasterPixelFormat, ZuneJpegProvider,
};
use xdremux_engine::{
    GainMapChannels, GainMapCodecLayout, GainMapEncodeProfile, GainMapTileEncoder, RasterDecoder,
};
use xdremux_format::{jpeg_icc_profile, jpeg_image_end, probe_jpeg_frame_profile, ChromaSampling};
use xdremux_heif::{
    assemble_iso_gain_map_heif, validate_gain_map_structure, DirectHevcGainMap,
    GainMapChannels as HeifGainMapChannels, GainMapEncodeProfile as HeifGainMapEncodeProfile,
    GainMapTile, IsoGainMapAssembly,
};
use xdremux_metadata::{
    make_apple_tmap_payload, make_hdrgm_xmp, parse_ultrahdr_gain_map_metadata,
    UltraHdrGainMapMetadata,
};
use xdremux_motion_photo::{
    build_live_photo_jpeg_exif, companion_video_path, media_mdat_payloads,
    normalize_embedded_video, parse_oppo_motion_photo, publish_live_photo_pair,
    read_apple_content_identifier, read_live_photo_content_identifier, reconcile_live_photo_pair,
    resolve_live_photo_still_time, validate_live_photo_movie, write_live_photo_heif_still,
    write_live_photo_movie, ByteRange, MotionPhotoAsset, MotionPhotoSourceKind,
};

use crate::{Result, RuntimeError};

#[derive(Debug, Clone, PartialEq)]
pub struct LivePhotoFileReceipt {
    pub image: PathBuf,
    pub video: PathBuf,
    pub content_identifier: String,
    pub still_time_seconds: f64,
    pub source_kind: String,
    pub source_had_gain_map: bool,
    pub removed_vendor_bytes: usize,
}

#[derive(Debug)]
struct PreparedStill {
    bytes: Vec<u8>,
    had_gain_map: bool,
}

#[derive(Debug)]
struct ValidatedGainJpeg<'a> {
    bytes: &'a [u8],
    metadata: UltraHdrGainMapMetadata,
}

fn range_slice<'a>(source: &'a [u8], range: ByteRange, context: &'static str) -> Result<&'a [u8]> {
    let start = usize::try_from(range.lower_bound)
        .map_err(|_| RuntimeError::new(context, "range start exceeds usize"))?;
    let end = usize::try_from(range.upper_bound)
        .map_err(|_| RuntimeError::new(context, "range end exceeds usize"))?;
    source
        .get(start..end)
        .ok_or_else(|| RuntimeError::new(context, "range is outside source bytes"))
}

fn generate_content_identifier() -> String {
    Uuid::new_v4().hyphenated().to_string().to_ascii_uppercase()
}

fn write_synced_new(path: &Path, data: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| RuntimeError::external("Live Photo temporary create", error))?;
    file.write_all(data)
        .map_err(|error| RuntimeError::external("Live Photo temporary write", error))?;
    file.sync_all()
        .map_err(|error| RuntimeError::external("Live Photo temporary sync", error))
}

fn pair_matches(image: &Path, video: &Path) -> bool {
    let Ok(image_bytes) = fs::read(image) else {
        return false;
    };
    let Ok(video_bytes) = fs::read(video) else {
        return false;
    };
    let Ok(Some(image_identifier)) = read_apple_content_identifier(&image_bytes) else {
        return false;
    };
    let Ok(Some(video_identifier)) = read_live_photo_content_identifier(&video_bytes) else {
        return false;
    };
    if image_identifier != video_identifier {
        return false;
    }
    let Ok(Some(still_time)) = xdremux_motion_photo::read_live_photo_still_time(&video_bytes)
    else {
        return false;
    };
    validate_live_photo_movie(&video_bytes, &video_identifier, still_time).is_ok()
}

fn temporary_pair_paths(output_image: &Path, identifier: &str) -> Result<(PathBuf, PathBuf)> {
    let parent = match output_image.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let stem = output_image
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| RuntimeError::new("Live Photo output", "output has no UTF-8 stem"))?;
    let token = identifier.replace('-', "").to_ascii_lowercase();
    Ok((
        parent.join(format!(".{stem}.{token}.tmp.heic")),
        parent.join(format!(".{stem}.{token}.tmp.mov")),
    ))
}

fn source_declares_gain_map(asset: &MotionPhotoAsset) -> bool {
    asset
        .items
        .iter()
        .any(|item| item.semantic.eq_ignore_ascii_case("GainMap"))
}

fn declared_gain_map_lengths(asset: &MotionPhotoAsset) -> Vec<u64> {
    let mut lengths = asset
        .items
        .iter()
        .filter(|item| item.semantic.eq_ignore_ascii_case("GainMap") && item.length > 0)
        .map(|item| item.length)
        .collect::<Vec<_>>();
    lengths.sort_unstable();
    lengths.dedup();
    lengths
}

fn next_soi(data: &[u8], start: usize) -> Option<usize> {
    if start >= data.len() {
        return None;
    }
    data[start..]
        .windows(2)
        .position(|window| window == [0xff, 0xd8])
        .and_then(|relative| start.checked_add(relative))
}

/// Find the independently self-describing JPEG/R gain map in the static resource.
///
/// Android vendor padding can contain thumbnails and arbitrary JPEG-looking data.
/// The candidate therefore has to be a complete JPEG *and* carry valid Adobe
/// hdrgm or ISO 21496-1 metadata. A positive Motion Photo directory length is an
/// additional identity check, never a source of truth for locating the bytes.
fn validated_gain_jpeg<'a>(
    static_bytes: &'a [u8],
    primary_end: usize,
    asset: &MotionPhotoAsset,
) -> Result<Option<ValidatedGainJpeg<'a>>> {
    if !source_declares_gain_map(asset) {
        return Ok(None);
    }
    let declared_lengths = declared_gain_map_lengths(asset);
    let mut search = primary_end;
    while let Some(start) = next_soi(static_bytes, search) {
        let end = match jpeg_image_end(static_bytes, start) {
            Ok(end) => end,
            Err(_) => {
                search = start.saturating_add(2);
                continue;
            }
        };
        let candidate = &static_bytes[start..end];
        let metadata = match parse_ultrahdr_gain_map_metadata(candidate) {
            Ok(Some(metadata)) => metadata,
            Ok(None) | Err(_) => {
                search = start.saturating_add(2);
                continue;
            }
        };
        let candidate_length = u64::try_from(candidate.len()).map_err(|_| {
            RuntimeError::new("Ultra HDR JPEG/R", "gain-map JPEG length exceeds u64")
        })?;
        if !declared_lengths.is_empty() && !declared_lengths.contains(&candidate_length) {
            search = start.saturating_add(2);
            continue;
        }
        return Ok(Some(ValidatedGainJpeg {
            bytes: candidate,
            metadata,
        }));
    }

    Err(RuntimeError::new(
        "Ultra HDR JPEG/R",
        if declared_lengths.is_empty() {
            "Motion Photo declares GainMap semantics but its static JPEG/R resource contains no independently validated gain-map JPEG".to_owned()
        } else {
            format!(
                "Motion Photo declares GainMap semantics but no validated gain-map JPEG matches declared lengths {declared_lengths:?}"
            )
        },
    ))
}

fn gain_map_target(jpeg: &[u8]) -> Result<(RasterPixelFormat, GainMapEncodeProfile)> {
    let frame = probe_jpeg_frame_profile(jpeg)
        .map_err(|error| RuntimeError::external("Ultra HDR gain-map JPEG profile", error))?;
    let (format, channels, chroma) = match frame.component_count() {
        1 => (
            RasterPixelFormat::Mono8,
            GainMapChannels::Mono,
            ChromaSampling::Mono400,
        ),
        3 => (
            RasterPixelFormat::Rgb8,
            GainMapChannels::Rgb,
            ChromaSampling::Yuv444,
        ),
        count => {
            return Err(RuntimeError::new(
                "Ultra HDR gain-map JPEG profile",
                format!("unsupported JPEG component count {count}"),
            ));
        }
    };
    Ok((
        format,
        GainMapEncodeProfile {
            width: u32::from(frame.width),
            height: u32::from(frame.height),
            channels,
            layout: GainMapCodecLayout {
                chroma,
                luma_bit_depth: 8,
                chroma_bit_depth: 8,
            },
        },
    ))
}

fn assemble_jpeg_gain_map(
    jpeg: &ZuneJpegProvider,
    heif: &LibHeifProvider,
    base_heif: &[u8],
    gain: &ValidatedGainJpeg<'_>,
) -> Result<Vec<u8>> {
    let (format, target) = gain_map_target(gain.bytes)?;
    let decoded = jpeg
        .decode_raster(&JpegRasterDecodeRequest {
            data: gain.bytes.to_vec(),
            format,
        })
        .map_err(|error| RuntimeError::external("Ultra HDR gain-map JPEG decode", error))?;
    if decoded.width != target.width || decoded.height != target.height {
        return Err(RuntimeError::new(
            "Ultra HDR gain-map JPEG decode",
            "decoded gain-map dimensions differ from JPEG SOF",
        ));
    }
    let encoded = heif
        .encode_gain_map_tiles(&GainMapTileEncodeRequest::reference_compatible(
            decoded, target,
        ))
        .map_err(|error| RuntimeError::external("Ultra HDR HEVC Gain Map encode", error))?;

    let info = gain
        .metadata
        .to_info_floats()
        .map_err(|error| RuntimeError::external("Ultra HDR metadata normalization", error))?;
    let tmap_payload = make_apple_tmap_payload(&info)
        .map_err(|error| RuntimeError::external("Ultra HDR ISO tmap", error))?;
    let xmp_payload = make_hdrgm_xmp(&info)
        .map_err(|error| RuntimeError::external("Ultra HDR hdrgm XMP", error))?;
    let tiles = encoded
        .tiles
        .iter()
        .map(|tile| GainMapTile {
            payload: &tile.payload,
            width: tile.width,
            height: tile.height,
        })
        .collect::<Vec<_>>();
    let direct = DirectHevcGainMap {
        gain_map_width: encoded.gain_map_width,
        gain_map_height: encoded.gain_map_height,
        tile_width: encoded.tile_width,
        tile_height: encoded.tile_height,
        tiles: &tiles,
        hvcc: &encoded.hvcc,
        profile: HeifGainMapEncodeProfile {
            channels: match encoded.profile.channels {
                GainMapChannels::Mono => HeifGainMapChannels::Mono,
                GainMapChannels::Rgb => HeifGainMapChannels::Rgb,
            },
            chroma: encoded.profile.layout.chroma,
            luma_bit_depth: encoded.profile.layout.luma_bit_depth,
            chroma_bit_depth: encoded.profile.layout.chroma_bit_depth,
        },
    };
    let output = assemble_iso_gain_map_heif(
        base_heif,
        &IsoGainMapAssembly {
            gain_map: direct,
            tmap_payload: &tmap_payload,
            xmp_payload: &xmp_payload,
        },
    )
    .map_err(|error| RuntimeError::external("Ultra HDR native HEIF assembly", error))?;
    validate_gain_map_structure(&output)
        .map_err(|error| RuntimeError::external("Ultra HDR HEIF validation", error))?;
    Ok(output)
}

fn prepare_jpeg_still(
    jpeg: &ZuneJpegProvider,
    heif: &LibHeifProvider,
    static_bytes: &[u8],
    asset: &MotionPhotoAsset,
    content_identifier: &str,
) -> Result<PreparedStill> {
    let primary_end = jpeg_image_end(static_bytes, 0)
        .map_err(|error| RuntimeError::external("Motion Photo primary JPEG boundary", error))?;
    let primary = static_bytes.get(..primary_end).ok_or_else(|| {
        RuntimeError::new(
            "Motion Photo primary JPEG",
            "primary range is outside static resource",
        )
    })?;
    let raster = jpeg
        .decode_raster(&JpegRasterDecodeRequest {
            data: primary.to_vec(),
            format: RasterPixelFormat::Rgb8,
        })
        .map_err(|error| RuntimeError::external("Motion Photo primary JPEG decode", error))?;
    let icc_profile = jpeg_icc_profile(primary)
        .map_err(|error| RuntimeError::external("Motion Photo primary JPEG ICC", error))?;
    let exif_tiff = build_live_photo_jpeg_exif(primary, content_identifier)
        .map_err(|error| RuntimeError::external("Live Photo JPEG EXIF transfer", error))?;
    let encoded_base = heif
        .encode_primary_heif(
            &PrimaryHeifEncodeRequest::live_photo(raster, icc_profile).with_exif_tiff(exif_tiff),
        )
        .map_err(|error| RuntimeError::external("Motion Photo primary HEIC encode", error))?;

    let gain = validated_gain_jpeg(static_bytes, primary_end, asset)?;
    match gain {
        Some(gain) => Ok(PreparedStill {
            bytes: assemble_jpeg_gain_map(jpeg, heif, &encoded_base, &gain)?,
            had_gain_map: true,
        }),
        None => Ok(PreparedStill {
            bytes: encoded_base,
            had_gain_map: false,
        }),
    }
}

fn prepare_still(
    jpeg: &ZuneJpegProvider,
    heif: &LibHeifProvider,
    static_bytes: &[u8],
    asset: &MotionPhotoAsset,
    content_identifier: &str,
) -> Result<PreparedStill> {
    match asset.source_kind {
        MotionPhotoSourceKind::AndroidHeifMotionPhotoV1 => Ok(PreparedStill {
            bytes: write_live_photo_heif_still(static_bytes, content_identifier)
                .map_err(|error| RuntimeError::external("Live Photo HEIF still", error))?,
            // Existing HEIF Motion Photo conversion preserves the complete static
            // HEIF graph. Whether that graph carries a gain map is not inferred
            // from the Android directory alone, so report only the JPEG/R case
            // here until the HEIF validator exposes that property directly.
            had_gain_map: false,
        }),
        MotionPhotoSourceKind::AndroidMotionPhotoV1
        | MotionPhotoSourceKind::LegacyMicroVideoV1b
        | MotionPhotoSourceKind::OppoLivePhoto => {
            prepare_jpeg_still(jpeg, heif, static_bytes, asset, content_identifier)
        }
    }
}

pub(crate) fn convert_motion_photo_file(
    jpeg: &ZuneJpegProvider,
    heif: &LibHeifProvider,
    source: &[u8],
    input: &Path,
    output_image: &Path,
) -> Result<LivePhotoFileReceipt> {
    let asset = parse_oppo_motion_photo(source)
        .map_err(|error| RuntimeError::external("Motion Photo analysis", error))?
        .ok_or_else(|| RuntimeError::new("Motion Photo analysis", "input is not a Motion Photo"))?;

    let extension = output_image
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !extension.eq_ignore_ascii_case("heic") && !extension.eq_ignore_ascii_case("heif") {
        return Err(RuntimeError::new(
            "Live Photo output",
            "still output must use .heic or .heif",
        ));
    }
    if output_image == input {
        return Err(RuntimeError::new(
            "Live Photo output",
            "Motion Photo conversion never overwrites the source",
        ));
    }
    let output_video = companion_video_path(output_image);
    if output_image.exists() || output_video.exists() {
        return Err(RuntimeError::new(
            "Live Photo output",
            "output HEIC/MOV pair already exists; refusing to overwrite unknown provenance",
        ));
    }

    reconcile_live_photo_pair(output_image, &output_video, pair_matches)
        .map_err(|error| RuntimeError::external("Live Photo pair reconciliation", error))?;

    let static_bytes = range_slice(
        source,
        asset.still_resource_range,
        "Motion Photo still range",
    )?;
    let embedded_video = range_slice(
        source,
        asset.video_resource_range,
        "Motion Photo video range",
    )?;
    let normalized_video = normalize_embedded_video(embedded_video)
        .map_err(|error| RuntimeError::external("Motion Photo video normalization", error))?;
    let still_time_seconds =
        resolve_live_photo_still_time(normalized_video.data, asset.presentation_timestamp_us)
            .map_err(|error| RuntimeError::external("Live Photo still-time resolution", error))?;

    let content_identifier = generate_content_identifier();
    let still = prepare_still(jpeg, heif, static_bytes, &asset, &content_identifier)?;
    let movie = write_live_photo_movie(
        normalized_video.data,
        &content_identifier,
        still_time_seconds,
        asset.vendor_metadata.as_ref(),
    )
    .map_err(|error| RuntimeError::external("Live Photo MOV", error))?;

    let still_identifier = read_apple_content_identifier(&still.bytes)
        .map_err(|error| RuntimeError::external("Live Photo still validation", error))?;
    if still_identifier.as_deref() != Some(content_identifier.as_str()) {
        return Err(RuntimeError::new(
            "Live Photo pair validation",
            "HEIF Apple MakerNote ContentIdentifier mismatch",
        ));
    }
    if read_live_photo_content_identifier(&movie)
        .map_err(|error| RuntimeError::external("Live Photo movie validation", error))?
        .as_deref()
        != Some(content_identifier.as_str())
    {
        return Err(RuntimeError::new(
            "Live Photo pair validation",
            "MOV QuickTime ContentIdentifier mismatch",
        ));
    }
    validate_live_photo_movie(&movie, &content_identifier, still_time_seconds)
        .map_err(|error| RuntimeError::external("Live Photo pair validation", error))?;

    let source_media = media_mdat_payloads(normalized_video.data)
        .map_err(|error| RuntimeError::external("source Motion Photo media validation", error))?;
    let output_media = media_mdat_payloads(&movie)
        .map_err(|error| RuntimeError::external("Live Photo media validation", error))?;
    if source_media != output_media {
        return Err(RuntimeError::new(
            "Live Photo media validation",
            "compressed video/audio mdat payload changed during MOV remux",
        ));
    }

    let (temporary_image, temporary_video) =
        temporary_pair_paths(output_image, &content_identifier)?;
    let publish_result: Result<()> = (|| {
        write_synced_new(&temporary_image, &still.bytes)?;
        write_synced_new(&temporary_video, &movie)?;
        publish_live_photo_pair(
            &temporary_image,
            &temporary_video,
            output_image,
            &output_video,
        )
        .map_err(|error| RuntimeError::external("Live Photo pair publication", error))?;
        Ok(())
    })();
    if publish_result.is_err() {
        let _ = fs::remove_file(&temporary_image);
        let _ = fs::remove_file(&temporary_video);
    }
    publish_result?;

    Ok(LivePhotoFileReceipt {
        image: output_image.to_path_buf(),
        video: output_video,
        content_identifier,
        still_time_seconds,
        source_kind: asset.source_kind.as_str().to_owned(),
        source_had_gain_map: still.had_gain_map,
        removed_vendor_bytes: normalized_video.removed_vendor_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_pair_identifier_is_uuid_v4() {
        let value = generate_content_identifier();
        let parsed = Uuid::parse_str(&value).expect("generated identifier must parse as UUID");
        assert_eq!(parsed.get_version_num(), 4);
        assert_eq!(value, value.to_ascii_uppercase());
    }

    #[test]
    fn candidate_scanner_ignores_unvalidated_jpeg_blobs() {
        let asset = MotionPhotoAsset {
            source_kind: MotionPhotoSourceKind::AndroidMotionPhotoV1,
            items: vec![
                xdremux_motion_photo::MotionPhotoItem {
                    mime: "image/jpeg".to_owned(),
                    semantic: "Primary".to_owned(),
                    length: 0,
                    padding: 0,
                },
                xdremux_motion_photo::MotionPhotoItem {
                    mime: "image/jpeg".to_owned(),
                    semantic: "GainMap".to_owned(),
                    length: 4,
                    padding: 0,
                },
                xdremux_motion_photo::MotionPhotoItem {
                    mime: "video/mp4".to_owned(),
                    semantic: "MotionPhoto".to_owned(),
                    length: 100,
                    padding: 0,
                },
            ],
            still_resource_range: ByteRange::new(0, 8).unwrap(),
            video_resource_range: ByteRange::new(8, 108).unwrap(),
            presentation_timestamp_us: None,
            presentation_source: None,
            vendor_metadata: None,
        };
        let data = [0xff, 0xd8, 0xff, 0xd9, 0xff, 0xd8, 0xff, 0xd9];
        assert!(validated_gain_jpeg(&data, 4, &asset).is_err());
    }
}
