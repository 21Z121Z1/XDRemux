use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use uuid::Uuid;
use xdremux_codec::{
    GainMapTileEncodeRequest, JpegRasterDecodeRequest, LibHeifProvider, PrimaryHeifEncodeRequest,
    RasterPixelFormat, ZuneJpegProvider,
};
use xdremux_engine::{
    GainMapChannels, GainMapCodecLayout, GainMapEncodeProfile, GainMapTileEncoder, LivePhotoOutput,
    RasterDecoder,
};
use xdremux_format::{jpeg_icc_profile, jpeg_image_end, probe_jpeg_frame_profile, ChromaSampling};
use xdremux_heif::{
    assemble_iso_gain_map_heif, validate_gain_map_structure, DirectHevcGainMap,
    GainMapChannels as HeifGainMapChannels, GainMapEncodeProfile as HeifGainMapEncodeProfile,
    GainMapTile, IsoGainMapAssembly,
};
use xdremux_metadata::{
    parse_ultrahdr_gain_map_metadata, validate_iso_alternate_icc, UltraHdrGainMapMetadata,
};
use xdremux_motion_photo::{
    apple, build_live_photo_jpeg_exif, companion_video_path, parse_first_lpex_object,
    presentation_geometry, product_source_kind, resolve_live_photo_still_time,
    write_live_photo_heif_still, ConversionReport, Disposition, Input, MediaTime, MotionAsset,
    MotionPhoto, PairingMetadata, ParseOptions, PreservationEvidence, RelationshipKind,
    ResourceConversion, ResourceRole, TargetProfile, VendorDialect,
};

use crate::{Result, RuntimeError};

mod pair_publication;

#[derive(Debug, Clone, PartialEq)]
pub struct LivePhotoFileReceipt {
    pub image: PathBuf,
    pub video: PathBuf,
    pub package: Option<PathBuf>,
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
    pub fn output_paths(&self) -> Vec<PathBuf> {
        match &self.package {
            Some(package) => vec![package.clone()],
            None => vec![self.image.clone(), self.video.clone()],
        }
    }

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

pub(crate) fn write_synced_new(path: &Path, data: &[u8]) -> Result<()> {
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
    let declared = source_declares_gain_map(asset);
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
        let candidate_length = u64::try_from(candidate.len()).map_err(|_| {
            RuntimeError::new("Ultra HDR JPEG/R", "gain-map JPEG length exceeds u64")
        })?;
        if !declared_lengths.is_empty() && !declared_lengths.contains(&candidate_length) {
            search = start.saturating_add(2);
            continue;
        }
        let Some(metadata) = parse_ultrahdr_gain_map_metadata(candidate)
            .map_err(|error| RuntimeError::external("Ultra HDR gain-map metadata", error))?
        else {
            search = start.saturating_add(2);
            continue;
        };
        if !declared {
            return Err(RuntimeError::new(
                "Ultra HDR JPEG/R",
                "static JPEG contains validated HDR data without a mapped GainMap resource; refusing SDR conversion",
            ));
        }
        return Ok(Some(ValidatedGainJpeg {
            bytes: candidate,
            metadata,
        }));
    }

    if !declared {
        return Ok(None);
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
    let alternate_icc = if gain.metadata.uses_iso_colorimetry {
        let profile = jpeg_icc_profile(gain.bytes)
            .map_err(|error| RuntimeError::external("Ultra HDR alternate ICC", error))?;
        if let Some(profile) = &profile {
            validate_iso_alternate_icc(profile, gain.metadata.hdr_capacity_max > 0.0)
                .map_err(|error| RuntimeError::external("Ultra HDR alternate ICC", error))?;
        }
        profile
    } else {
        None
    };
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

    // ISO 21496-1 C.4.7: without an alternate ICC the application primaries
    // are the base primaries, including when the source flag is zero. The
    // HEIF writer may choose a new alternate encoding, but must not move the
    // per-channel gain operation into that new space.
    let use_base_colour_space = gain.metadata.use_base_color_space || alternate_icc.is_none();
    let tmap_payload = gain
        .metadata
        .to_tmap_payload(use_base_colour_space)
        .map_err(|error| RuntimeError::external("Ultra HDR ISO tmap", error))?;
    let xmp_payload = gain
        .metadata
        .to_hdrgm_xmp()
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
            alternate_icc: alternate_icc.as_deref(),
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

fn primary_jpeg_icc(primary: &[u8], iso_colorimetry: bool) -> Result<Option<Vec<u8>>> {
    // ISO 21496-1 C.4.4 gives Exif ColorSpace=1 precedence over an ICC.
    // None selects the encoder's standard sRGB ICC without changing RGB codes.
    if iso_colorimetry
        && xdremux_format::jpeg::exif_color_space(primary)
            .map_err(|error| RuntimeError::external("ISO JPEG base colorimetry", error))?
            == Some(1)
    {
        return Ok(None);
    }
    jpeg_icc_profile(primary)
        .map_err(|error| RuntimeError::external("Motion Photo primary JPEG ICC", error))
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
    let gain = validated_gain_jpeg(static_bytes, primary_end, asset)?;
    let raster = jpeg
        .decode_raster(&JpegRasterDecodeRequest {
            data: primary.to_vec(),
            format: RasterPixelFormat::Rgb8,
        })
        .map_err(|error| RuntimeError::external("Motion Photo primary JPEG decode", error))?;
    let icc_profile = primary_jpeg_icc(
        primary,
        gain.as_ref()
            .is_some_and(|g| g.metadata.uses_iso_colorimetry),
    )?;
    let exif_tiff = build_live_photo_jpeg_exif(primary, content_identifier)
        .map_err(|error| RuntimeError::external("Live Photo JPEG EXIF transfer", error))?;
    let encoded_base = heif
        .encode_primary_heif(
            &PrimaryHeifEncodeRequest::live_photo(raster, icc_profile).with_exif_tiff(exif_tiff),
        )
        .map_err(|error| RuntimeError::external("Motion Photo primary HEIC encode", error))?;

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
    format: LivePhotoOutput,
) -> Result<LivePhotoFileReceipt> {
    let photo = MotionPhoto::parse(Input::SingleFile(source), ParseOptions::compatible())
        .map_err(|error| RuntimeError::external("Motion Photo analysis", error))?;
    photo
        .validate()
        .map_err(|error| RuntimeError::external("Motion Photo validation", error))?;
    let asset = photo.asset();

    crate::live_photo_package::validate_output_parent(output_image)?;
    let extension = output_image
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if match format {
        LivePhotoOutput::Pair => {
            !extension.eq_ignore_ascii_case("heic") && !extension.eq_ignore_ascii_case("heif")
        }
        LivePhotoOutput::Pvt => !extension.eq_ignore_ascii_case("pvt"),
    } {
        return Err(RuntimeError::new(
            "Live Photo output",
            "pair output must use .heic/.heif; PVT output must use .pvt",
        ));
    }
    if output_image == input {
        return Err(RuntimeError::new(
            "Live Photo output",
            "Motion Photo conversion never overwrites the source",
        ));
    }
    let output_video = companion_video_path(output_image);
    if fs::symlink_metadata(output_image).is_ok()
        || (format == LivePhotoOutput::Pair && fs::symlink_metadata(&output_video).is_ok())
    {
        return Err(RuntimeError::new(
            "Live Photo output",
            "output already exists; refusing to overwrite unknown provenance",
        ));
    }

    let parent = output_image
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| RuntimeError::external("Live Photo output directory", error))?;

    let static_bytes = photo
        .still_bytes()
        .map_err(|error| RuntimeError::external("Motion Photo still resource", error))?;
    let primary_video = photo
        .motion_video_bytes()
        .map_err(|error| RuntimeError::external("Motion Photo primary motion resource", error))?;
    let fallback_seconds = resolve_live_photo_still_time(primary_video, None)
        .map_err(|error| RuntimeError::external("Live Photo video sample validation", error))?;
    let presentation = match photo.presentation_time() {
        Some(time) => time,
        None => {
            // The existing product fallback chooses a presentation position.
            // Its inferred time is not a source-format fact.
            let timescale = asset
                .containers
                .iter()
                .find(|c| c.resource == photo.motion_video().id)
                .and_then(|c| c.movie_timescale)
                .ok_or_else(|| {
                    RuntimeError::new("Live Photo still-time fallback", "movie timescale missing")
                })?;
            MediaTime::new(
                (fallback_seconds * f64::from(timescale)).round() as i64,
                timescale,
            )
            .ok_or_else(|| {
                RuntimeError::new("Live Photo still-time fallback", "invalid movie timescale")
            })?
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
    let had_gain_map = still.had_gain_map;
    let (image_path, video_path, package, conversion_report) = if format == LivePhotoOutput::Pvt {
        let pair = pair_publication::check_pair(
            &still.bytes,
            &movie,
            primary_video,
            &content_identifier,
            presentation,
        )?;
        let conversion_report =
            product_conversion_report(&photo, &content_identifier, had_gain_map)?;
        let members = crate::live_photo_package::publish_pair(&pair, output_image)?;
        (
            members.image,
            members.video,
            Some(output_image.to_path_buf()),
            conversion_report,
        )
    } else {
        let pair = pair_publication::CheckedPair::check(
            still.bytes,
            movie,
            primary_video,
            &content_identifier,
            presentation,
        )?;
        let conversion_report =
            product_conversion_report(&photo, &content_identifier, had_gain_map)?;
        let members = pair_publication::publish(pair, output_image)
            .map_err(pair_publication::PublicationFailure::into_runtime_error)?;
        (members.image, members.video, None, conversion_report)
    };

    Ok(LivePhotoFileReceipt {
        image: image_path,
        video: video_path,
        package,
        content_identifier,
        still_time_seconds,
        presentation,
        presentation_inferred: photo.presentation_time().is_none(),
        conversion_report,
        source_kind: product_source_kind(asset).to_owned(),
        source_had_gain_map: had_gain_map,
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
    fn iso_jpeg_exif_srgb_takes_precedence_over_conflicting_icc() {
        let mut jpeg = vec![0xff, 0xd8];
        // IFD0 points to an ExifIFD with the SHORT ColorSpace tag set to 1.
        let tiff = b"MM\0*\0\0\0\x08\0\x01\x87\x69\0\x04\0\0\0\x01\0\0\0\x1a\0\0\0\0\0\x01\xa0\x01\0\x03\0\0\0\x01\0\x01\0\0\0\0\0\0";
        let mut exif = b"Exif\0\0".to_vec();
        exif.extend_from_slice(tiff);
        let icc = b"source profile";
        let mut app2 = b"ICC_PROFILE\0\x01\x01".to_vec();
        app2.extend_from_slice(icc);
        for (marker, payload) in [(0xe1, exif.as_slice()), (0xe2, app2.as_slice())] {
            jpeg.extend([0xff, marker]);
            jpeg.extend(u16::try_from(payload.len() + 2).unwrap().to_be_bytes());
            jpeg.extend(payload);
        }
        jpeg.extend([0xff, 0xd9]);
        // None selects the primary encoder's explicit standard sRGB profile.
        assert_eq!(primary_jpeg_icc(&jpeg, true).unwrap(), None);
        assert_eq!(
            primary_jpeg_icc(&jpeg, false).unwrap().as_deref(),
            Some(icc.as_slice())
        );
        // Exif uncalibrated leaves the source ICC in charge for ISO as well.
        let color_value = 12 + 36;
        jpeg[color_value..color_value + 2].copy_from_slice(&[0xff, 0xff]);
        assert_eq!(
            primary_jpeg_icc(&jpeg, true).unwrap().as_deref(),
            Some(icc.as_slice())
        );
    }

    #[test]
    fn iso_gain_without_alternate_icc_keeps_the_base_application_primaries() {
        use xdremux_format::isobmff::{parse_meta_box, scan_top_level_boxes, META};
        let raster =
            xdremux_codec::Raster8::new(16, 16, 48, RasterPixelFormat::Rgb8, vec![128; 16 * 48])
                .unwrap();
        let base = LibHeifProvider::new()
            .encode_primary_heif(&PrimaryHeifEncodeRequest::live_photo(raster, None))
            .unwrap();
        let mut jpeg = Vec::new();
        jpeg_encoder::Encoder::new(&mut jpeg, 100)
            .encode(&[128; 16 * 16], 16, 16, jpeg_encoder::ColorType::Luma)
            .unwrap();
        let fixture = fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/motion-photo/samsung/jpeg-ultrahdr-01.jpg"),
        )
        .unwrap();
        let profile = jpeg_icc_profile(&fixture).unwrap().unwrap();
        for (is_iso, alternate_icc) in [
            (true, None),
            (true, Some(profile.clone())),
            (false, None),
            (false, Some(profile)),
        ] {
            let mut gain_jpeg = jpeg[..2].to_vec();
            if let Some(icc) = &alternate_icc {
                let mut app2 = b"ICC_PROFILE\0\x01\x01".to_vec();
                app2.extend_from_slice(icc);
                gain_jpeg.extend_from_slice(&[0xff, 0xe2]);
                gain_jpeg.extend_from_slice(&u16::try_from(app2.len() + 2).unwrap().to_be_bytes());
                gain_jpeg.extend_from_slice(&app2);
            }
            let mut info = [0.; 20];
            info[..3].fill(1.);
            info[4..7].fill(4.);
            info[7..10].fill(1.);
            // A real SDR alternate exercises the decreasing-headroom case.
            info[16] = if is_iso && alternate_icc.is_some() {
                4.
            } else {
                1.
            };
            info[17] = if is_iso && alternate_icc.is_some() {
                1.
            } else {
                4.
            };
            let (marker, mut app2) = if is_iso {
                let tmap =
                    xdremux_metadata::make_apple_tmap_payload_with_colour_space(&info, false)
                        .unwrap();
                (
                    0xe2,
                    [b"urn:iso:std:iso:ts:21496:-1\0".as_slice(), &tmap[1..]].concat(),
                )
            } else {
                (
                    0xe1,
                    [
                        b"http://ns.adobe.com/xap/1.0/\0".as_slice(),
                        &xdremux_metadata::make_hdrgm_xmp(&info).unwrap(),
                    ]
                    .concat(),
                )
            };
            gain_jpeg.extend_from_slice(&[0xff, marker]);
            gain_jpeg.extend_from_slice(&u16::try_from(app2.len() + 2).unwrap().to_be_bytes());
            gain_jpeg.append(&mut app2);
            gain_jpeg.extend_from_slice(&jpeg[2..]);
            let metadata = parse_ultrahdr_gain_map_metadata(&gain_jpeg)
                .unwrap()
                .unwrap();
            let gain = ValidatedGainJpeg {
                bytes: &gain_jpeg,
                metadata,
            };
            let output = assemble_jpeg_gain_map(
                &ZuneJpegProvider::new(),
                &LibHeifProvider::new(),
                &base,
                &gain,
            )
            .unwrap();
            let structure = validate_gain_map_structure(&output).unwrap();
            let top = scan_top_level_boxes(&output).unwrap();
            let meta = parse_meta_box(&output, top.boxes.iter().find(|b| b.kind == META).unwrap())
                .unwrap();
            let location = meta
                .iloc
                .entries
                .iter()
                .find(|e| e.item_id == structure.tmap_item_id)
                .unwrap();
            let payload =
                xdremux_format::exif::read_item_payload(&output, location, meta.idat.as_ref())
                    .unwrap();
            // ISO 21496-1 C.4.7: without an alternate ICC, both application
            // spaces have the base primaries. A new BT.2020 output must not move
            // the gain operation into BT.2020.
            assert_eq!(payload[5] & 0x40 != 0, !is_iso || alternate_icc.is_none());
            let entry = meta
                .ipma
                .entries
                .iter()
                .find(|e| e.item_id == structure.tmap_item_id)
                .unwrap();
            let colr = entry
                .associations
                .iter()
                .find_map(|a| {
                    meta.properties.iter().find(|p| {
                        p.index == u32::from(a.property_index)
                            && p.kind == xdremux_format::FourCC::new(*b"colr")
                    })
                })
                .unwrap();
            let color = &output[colr.box_range.start + 8..colr.box_range.end];
            if let Some(icc) = alternate_icc.filter(|_| is_iso) {
                assert_eq!(&color[..4], b"prof");
                assert_eq!(&color[4..], icc);
            } else {
                assert_eq!(color, b"nclx\x00\x09\x00\x10\x00\x09\x80");
            }
        }
    }

    #[test]
    fn generated_pair_identifier_is_uuid_v4() {
        let value = generate_content_identifier();
        let parsed = Uuid::parse_str(&value).expect("generated identifier must parse as UUID");
        assert_eq!(parsed.get_version_num(), 4);
        assert_eq!(value, value.to_ascii_uppercase());
    }

    #[test]
    fn sef_hdr_without_a_mapped_gain_map_cannot_be_classified_as_sdr() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/motion-photo/samsung/jpeg-ultrahdr-01.jpg");
        let mut source = fs::read(path).unwrap();
        let flag = b":MotionPhoto=\"1\"";
        let start = source.windows(flag.len()).position(|w| w == flag).unwrap();
        source[start + flag.len() - 2] = b'0';
        let photo =
            MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
        assert_eq!(photo.asset().dialect, VendorDialect::SamsungSef);
        assert!(!source_declares_gain_map(photo.asset()));
        let still = photo.still_bytes().unwrap();
        let primary_end = jpeg_image_end(still, 0).unwrap();
        assert!(validated_gain_jpeg(still, primary_end, photo.asset()).is_err());
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
