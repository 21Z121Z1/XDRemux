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
    apple, build_live_photo_jpeg_exif, companion_video_path, parse_first_lpex_object,
    presentation_geometry, product_source_kind, publish_live_photo_pair, reconcile_live_photo_pair,
    resolve_live_photo_still_time, write_live_photo_heif_still, ConversionReport, Disposition,
    Input, MediaTime, MotionAsset, MotionPhoto, PairingMetadata, ParseOptions,
    PreservationEvidence, RelationshipKind, ResourceConversion, ResourceRole, TargetProfile,
    VendorDialect,
};

use crate::{Result, RuntimeError};

#[derive(Debug, Clone, PartialEq)]
pub struct LivePhotoFileReceipt {
    pub image: PathBuf,
    pub video: PathBuf,
    pub content_identifier: String,
    pub still_time_seconds: f64,
    pub presentation: MediaTime,
    pub presentation_inferred: bool,
    pub conversion_report: ConversionReport,
    pub source_kind: String,
    pub source_had_gain_map: bool,
    pub removed_vendor_bytes: usize,
}

impl LivePhotoFileReceipt {
    pub fn omitted_resource_count(&self) -> usize {
        self.conversion_report
            .resources
            .iter()
            .filter(|r| r.disposition == Disposition::Dropped)
            .count()
    }
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
    MotionPhoto::parse(
        Input::ApplePair {
            still: &image_bytes,
            movie: &video_bytes,
        },
        ParseOptions::strict(),
    )
    .is_ok()
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

fn source_declares_gain_map(asset: &MotionAsset) -> bool {
    asset.auxiliary.iter().any(|r| {
        r.vendor_role
            .as_deref()
            .is_some_and(|n| n.eq_ignore_ascii_case("GainMap"))
    })
}

fn declared_gain_map_lengths(asset: &MotionAsset) -> Vec<u64> {
    let mut lengths = asset
        .auxiliary
        .iter()
        .filter(|r| {
            r.vendor_role
                .as_deref()
                .is_some_and(|n| n.eq_ignore_ascii_case("GainMap"))
        })
        .filter(|r| {
            !r.relationship
                .as_ref()
                .is_some_and(|r| r.kind == RelationshipKind::SharedData)
        })
        .map(|r| r.extents.iter().map(|e| e.length()).sum::<u64>())
        .filter(|n| *n > 0)
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
    asset: &MotionAsset,
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
    asset: &MotionAsset,
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
    asset: &MotionAsset,
    content_identifier: &str,
) -> Result<PreparedStill> {
    if asset.still.mime == "image/heic" {
        Ok(PreparedStill {
            bytes: write_live_photo_heif_still(static_bytes, content_identifier)
                .map_err(|error| RuntimeError::external("Live Photo HEIF still", error))?,
            had_gain_map: false,
        })
    } else if asset.still.mime == "image/jpeg" {
        prepare_jpeg_still(jpeg, heif, static_bytes, asset, content_identifier)
    } else {
        Err(RuntimeError::new(
            "Live Photo still",
            "unsupported still container",
        ))
    }
}

// This report describes the product's HEIC/MOV output. JPEG/HDR encoding is
// an explicit XDRemux policy and is not a LibLivePhoto lossless conversion.
fn product_conversion_report(
    photo: &MotionPhoto<'_>,
    identifier: &str,
    had_gain_map: bool,
) -> Result<ConversionReport> {
    let jpeg = photo.still().mime == "image/jpeg";
    let mut report = ConversionReport {
        target: TargetProfile::ApplePair,
        pairing: PairingMetadata {
            identifier: Some(identifier.to_owned()),
        },
        resources: Vec::new(),
        evidence: vec![
            PreservationEvidence::EncodedMoviePayloadsEqual,
            PreservationEvidence::ExactPresentationEqual,
        ],
    };
    for resource in photo.resources() {
        let (disposition, reason) = if resource.id == photo.still().id && jpeg {
            (
                Disposition::TranscodeRequired,
                "XDRemux decodes JPEG and encodes HEIC under its product policy",
            )
        } else if resource.id == photo.still().id || resource.id == photo.motion_video().id {
            (
                Disposition::Rewritten,
                "pairing and presentation container metadata are rewritten",
            )
        } else if jpeg && resource.vendor_role.as_deref() == Some("GainMap") && had_gain_map {
            (
                Disposition::TranscodeRequired,
                "XDRemux decodes and encodes the declared gain map",
            )
        } else if jpeg && resource.vendor_role.as_deref() == Some("JPEG EXIF") {
            (
                Disposition::Rewritten,
                "raw TIFF is transferred; active MakerNote pairing is replaced",
            )
        } else if !jpeg && resource.parent == Some(photo.still().id) {
            if resource.mime == "application/x-exif"
                || resource.provenance.evidence == "HEIF primary XMP item"
            {
                (
                    Disposition::Rewritten,
                    "active HEIF pairing or recognition metadata is rewritten",
                )
            } else {
                (
                    Disposition::Preserved,
                    "HEIF item bytes remain in the output still",
                )
            }
        } else {
            let mut bytes = Vec::new();
            if jpeg && resource.vendor_role.as_deref() == Some("JPEG APP2") {
                photo
                    .extract(resource, &mut bytes)
                    .map_err(|e| RuntimeError::external("source metadata report", e))?;
            }
            if bytes.starts_with(b"ICC_PROFILE\0") {
                (
                    Disposition::Rewritten,
                    "ICC content is transferred into the HEIF color profile",
                )
            } else {
                (
                    Disposition::Dropped,
                    "resource is not represented in the output pair; original input is retained",
                )
            }
        };
        report.resources.push(ResourceConversion {
            resource: Some(resource.id),
            disposition,
            reason: reason.into(),
        });
    }
    report.resources.push(ResourceConversion {
        resource: None,
        disposition: Disposition::Generated,
        reason: "new pairing identity and still-time metadata are generated".into(),
    });
    Ok(report)
}

pub(crate) fn convert_motion_photo_file(
    jpeg: &ZuneJpegProvider,
    heif: &LibHeifProvider,
    source: &[u8],
    input: &Path,
    output_image: &Path,
) -> Result<LivePhotoFileReceipt> {
    let photo = MotionPhoto::parse(Input::SingleFile(source), ParseOptions::compatible())
        .map_err(|error| RuntimeError::external("Motion Photo analysis", error))?;
    photo
        .validate()
        .map_err(|error| RuntimeError::external("Motion Photo validation", error))?;
    let asset = photo.asset();

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

    let static_bytes = photo
        .still_bytes()
        .map_err(|error| RuntimeError::external("Motion Photo still resource", error))?;
    let primary_video = photo
        .motion_video_bytes()
        .map_err(|error| RuntimeError::external("Motion Photo primary motion resource", error))?;
    let presentation = match photo.presentation_time() {
        Some(time) => time,
        None => {
            // The existing product fallback chooses a presentation position.
            // Its inferred time is not a source-format fact.
            let seconds = resolve_live_photo_still_time(primary_video, None)
                .map_err(|error| RuntimeError::external("Live Photo still-time fallback", error))?;
            let timescale = asset
                .containers
                .iter()
                .find(|c| c.resource == photo.motion_video().id)
                .and_then(|c| c.movie_timescale)
                .ok_or_else(|| {
                    RuntimeError::new("Live Photo still-time fallback", "movie timescale missing")
                })?;
            MediaTime::new((seconds * f64::from(timescale)).round() as i64, timescale).ok_or_else(
                || RuntimeError::new("Live Photo still-time fallback", "invalid movie timescale"),
            )?
        }
    };
    let still_time_seconds = presentation.seconds();
    let metadata = if asset.dialect == VendorDialect::Oplus {
        parse_first_lpex_object(source)
    } else {
        None
    };
    let content_identifier = generate_content_identifier();
    let still = prepare_still(jpeg, heif, static_bytes, asset, &content_identifier)?;
    let movie = apple::remux_movie_with_geometry(
        primary_video,
        &content_identifier,
        presentation,
        presentation_geometry(metadata.as_ref()),
    )
    .map_err(|error| RuntimeError::external("Live Photo MOV", error))?;
    let pair = MotionPhoto::parse(
        Input::ApplePair {
            still: &still.bytes,
            movie: &movie,
        },
        ParseOptions::strict(),
    )
    .map_err(|error| RuntimeError::external("Live Photo pair validation", error))?;
    if !pair
        .presentation_time()
        .is_some_and(|time| time.equivalent(presentation))
    {
        return Err(RuntimeError::new(
            "Live Photo pair validation",
            "exact presentation time changed",
        ));
    }

    if pair.asset().pairing.identifier.as_deref() != Some(content_identifier.as_str()) {
        return Err(RuntimeError::new(
            "Live Photo pair validation",
            "pairing identifier changed",
        ));
    }

    let source_media = apple::media_payloads(primary_video)
        .map_err(|error| RuntimeError::external("source Motion Photo media validation", error))?;
    let output_media = apple::media_payloads(&movie)
        .map_err(|error| RuntimeError::external("Live Photo media validation", error))?;
    if source_media != output_media {
        return Err(RuntimeError::new(
            "Live Photo media validation",
            "compressed video/audio mdat payload changed during MOV remux",
        ));
    }

    let conversion_report =
        product_conversion_report(&photo, &content_identifier, still.had_gain_map)?;
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
        presentation,
        presentation_inferred: photo.presentation_time().is_none(),
        conversion_report,
        source_kind: product_source_kind(asset).to_owned(),
        source_had_gain_map: still.had_gain_map,
        removed_vendor_bytes: photo
            .resources()
            .filter(|r| {
                r.parent.is_none()
                    && r.id != photo.still().id
                    && r.id != photo.motion_video().id
                    && r.role != ResourceRole::ContainerMetadata
            })
            .map(|r| r.extents.iter().map(|e| e.length() as usize).sum::<usize>())
            .sum(),
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
        use xdremux_motion_photo::{
            AssetFormat, ByteRange, PairingMetadata, PhysicalLayout, Provenance, Resource,
            ResourceId,
        };
        let resource = |id, role, range, name| Resource {
            id: ResourceId(id),
            role,
            mime: "image/jpeg".into(),
            source_index: 0,
            extents: vec![range],
            parent: None,
            container_item_id: None,
            relationship: None,
            vendor_role: name,
            provenance: Provenance::declared("test vector"),
        };
        let asset = MotionAsset {
            layout: PhysicalLayout::AppendedResources,
            format: AssetFormat::AndroidMotionPhoto,
            dialect: VendorDialect::AndroidStandard,
            provenance: Provenance::declared("test vector"),
            sources: vec![],
            still: resource(
                0,
                ResourceRole::PrimaryStill,
                ByteRange::new(0, 8).unwrap(),
                None,
            ),
            motion: resource(
                1,
                ResourceRole::PrimaryMotionVideo,
                ByteRange::new(8, 108).unwrap(),
                None,
            ),
            presentation: None,
            pairing: PairingMetadata::default(),
            containers: vec![],
            auxiliary: vec![resource(
                2,
                ResourceRole::AuxiliaryImage,
                ByteRange::new(4, 8).unwrap(),
                Some("GainMap".into()),
            )],
        };
        let data = [0xff, 0xd8, 0xff, 0xd9, 0xff, 0xd8, 0xff, 0xd9];
        assert!(validated_gain_jpeg(&data, 4, &asset).is_err());
    }
}
