//! Google Motion Photo output is a separate, fail-closed publication route.
//! The Apple pair/PVT route is intentionally unchanged.

use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;

use xdremux_engine::{MotionOutputTarget, MotionVideoCompatibility};
use xdremux_motion_photo::{
    ConversionReport, Disposition, Input, MediaTime, MotionPhoto, ParseOptions,
    PreservationEvidence, ResourceRole, TargetProfile, UnmappedPolicy, WritePolicy,
};
use xdremux_source::{motion_still_encoding, MotionStillEncoding, SourceAsset};

use crate::vendor_motion_metadata::{
    carry_vendor_metadata, VendorMetadataCarry, VendorMetadataReport,
};
use crate::{Result, RuntimeError};

#[derive(Debug, Clone, PartialEq)]
pub struct AndroidMotionPhotoFileReceipt {
    pub output: PathBuf,
    pub conversion_report: ConversionReport,
    /// Complete original resource inventory, linked to every plan disposition.
    pub resources: Vec<AndroidMotionResourceAudit>,
    pub video_compatibility: MotionVideoCompatibility,
    pub video_brand_changed: bool,
    pub vendor_metadata: VendorMetadataReport,
}

/// Caller-owned, explicitly associated Apple Live Photo input files.
/// Association is still checked against both files' embedded identifiers.
#[derive(Debug, Clone, Copy)]
pub struct AndroidMotionPhotoPairSource<'a> {
    pub still: &'a [u8],
    pub movie: &'a [u8],
    pub still_path: &'a Path,
    pub movie_path: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndroidMotionResourceAudit {
    pub id: usize,
    pub source_role: String,
    pub mime: String,
    pub vendor_role: Option<String>,
    pub byte_length: u64,
    pub operations: Vec<AndroidMotionResourceOperation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndroidMotionResourceOperation {
    pub disposition: String,
    pub reason: String,
}

// Apple timed metadata and Android XMP may express the *same exact instant*
// with different rational timescales. Structural MediaTime equality rejects
// that lossless representation change even though the library writer already
// checks the timestamps with MediaTime::equivalent.
fn same_presentation_time(source: Option<MediaTime>, output: Option<MediaTime>) -> bool {
    match (source, output) {
        (Some(a), Some(b)) => a.equivalent(b),
        (None, None) => true,
        _ => false,
    }
}

#[cfg(test)]
mod presentation_time_tests {
    use super::{same_presentation_time, MediaTime};

    #[test]
    fn compares_exact_rational_instants_not_timestamp_encoding() {
        let apple = MediaTime::new(2, 3).unwrap();
        let android = MediaTime::new(200, 300).unwrap();
        assert_ne!(apple, android);
        assert!(same_presentation_time(Some(apple), Some(android)));
        assert!(!same_presentation_time(
            Some(apple),
            Some(MediaTime::new(201, 300).unwrap()),
        ));
        assert!(!same_presentation_time(Some(apple), None));
    }
}

/// Resolve an automatic Android writer using the actual parsed still span.
/// This selects a container; it does not transcode still images or video.
pub fn resolve_motion_output_target(
    target: MotionOutputTarget,
    source: &[u8],
    asset: &SourceAsset,
) -> Result<MotionOutputTarget> {
    resolve_motion_output_target_for(target, source, asset, None)
}

/// Resolve an automatic Android writer for one destination.
///
/// OPPO 25.17.35 `isOlivePhoto` accepts `image/jpeg` only, so a HEIC source is
/// planned as JPEG. That plan does not itself re-encode the still.
pub fn resolve_motion_output_target_for(
    target: MotionOutputTarget,
    source: &[u8],
    asset: &SourceAsset,
    destination: Option<&str>,
) -> Result<MotionOutputTarget> {
    if target != MotionOutputTarget::GoogleAuto {
        return Ok(target);
    }
    if destination == Some("oppo") {
        return match motion_still_encoding(source, asset) {
            Some(MotionStillEncoding::Jpeg | MotionStillEncoding::Heic) => {
                Ok(MotionOutputTarget::GoogleJpeg)
            }
            None => Err(RuntimeError::new(
                "Android Motion Photo target",
                "automatic target requires a JPEG or HEIC primary still; AVIF and other still formats have no compatible writer",
            )),
        };
    }
    match motion_still_encoding(source, asset) {
        Some(MotionStillEncoding::Jpeg) => Ok(MotionOutputTarget::GoogleJpeg),
        Some(MotionStillEncoding::Heic) => Ok(MotionOutputTarget::GoogleHeic),
        None => Err(RuntimeError::new(
            "Android Motion Photo target",
            "automatic target requires a JPEG or HEIC primary still; AVIF and other still formats have no compatible writer",
        )),
    }
}

fn profile(target: MotionOutputTarget) -> Result<(TargetProfile, &'static str, &'static str)> {
    match target {
        MotionOutputTarget::GoogleJpeg => Ok((TargetProfile::AndroidJpeg, "jpg", "image/jpeg")),
        MotionOutputTarget::GoogleHeic => Ok((TargetProfile::AndroidHeif, "heic", "image/heic")),
        _ => Err(RuntimeError::new(
            "Android Motion Photo output",
            "an Android target (google-jpeg or google-heic) is required",
        )),
    }
}

pub(crate) fn validate_android_output_path(
    output: &Path,
    target: MotionOutputTarget,
) -> Result<()> {
    let (_, extension, _) = profile(target)?;
    crate::live_photo_package::validate_output_parent(output)?;
    if output.parent().is_some_and(|parent| {
        parent.ancestors().any(|ancestor| {
            ancestor
                .extension()
                .is_some_and(|part| part.eq_ignore_ascii_case("pvt"))
        })
    }) {
        return Err(RuntimeError::new(
            "Android Motion Photo output",
            "refusing to write inside a PVT package path",
        ));
    }
    let name = output
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            RuntimeError::new("Android Motion Photo output", "invalid output filename")
        })?;
    let stem = output
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| RuntimeError::new("Android Motion Photo output", "invalid output stem"))?;
    // Android Motion Photo 1.0 specifies an MP-ending filename, not merely XMP metadata.
    if stem.is_empty()
        || stem.starts_with(char::is_whitespace)
        || !stem.ends_with("MP")
        || !output.extension().is_some_and(|value| {
            value.eq_ignore_ascii_case(extension)
                || (extension == "jpg" && value.eq_ignore_ascii_case("jpeg"))
        })
        || name.contains(['/', '\\'])
    {
        return Err(RuntimeError::new(
            "Android Motion Photo output",
            format!("Google Motion Photo requires a filename ending in MP.{extension} (no leading whitespace)"),
        ));
    }
    Ok(())
}

pub(crate) fn convert_android_motion_photo_file_with_policy(
    source: &[u8],
    input: &Path,
    output: &Path,
    target: MotionOutputTarget,
    compatibility: MotionVideoCompatibility,
    carry: VendorMetadataCarry,
) -> Result<AndroidMotionPhotoFileReceipt> {
    convert_android_motion_photo(
        Input::SingleFile(source),
        source,
        &[input],
        output,
        target,
        compatibility,
        carry,
    )
}

pub(crate) fn convert_android_motion_photo_pair_file(
    still: &[u8],
    movie: &[u8],
    still_path: &Path,
    movie_path: &Path,
    output: &Path,
    target: MotionOutputTarget,
    compatibility: MotionVideoCompatibility,
    carry: VendorMetadataCarry,
) -> Result<AndroidMotionPhotoFileReceipt> {
    convert_android_motion_photo(
        Input::ApplePair { still, movie },
        still,
        &[still_path, movie_path],
        output,
        target,
        compatibility,
        carry,
    )
}

fn convert_android_motion_photo(
    source: Input<'_>,
    marker_source: &[u8],
    inputs: &[&Path],
    output: &Path,
    target: MotionOutputTarget,
    compatibility: MotionVideoCompatibility,
    carry: VendorMetadataCarry,
) -> Result<AndroidMotionPhotoFileReceipt> {
    validate_android_output_path(output, target)?;
    let (target_profile, _, expected_mime) = profile(target)?;
    if inputs.contains(&output) || fs::symlink_metadata(output).is_ok() {
        return Err(RuntimeError::new(
            "Android Motion Photo output",
            "output already exists; refusing to overwrite the source or unknown provenance",
        ));
    }
    if inputs.len() == 2
        && (inputs[0] == inputs[1]
            || fs::canonicalize(inputs[0]).ok() == fs::canonicalize(inputs[1]).ok())
    {
        return Err(RuntimeError::new(
            "Android Motion Photo input",
            "Apple still and movie inputs must be distinct files",
        ));
    }
    // Paths are checked as well as the captured byte slices. An output symlink
    // must not turn a source file into its own publication target.
    if let Ok(resolved_output) = fs::canonicalize(output) {
        if inputs
            .iter()
            .any(|input| fs::canonicalize(input).is_ok_and(|p| p == resolved_output))
        {
            return Err(RuntimeError::new(
                "Android Motion Photo output",
                "output aliases a source file",
            ));
        }
    }
    let photo = MotionPhoto::parse(source, ParseOptions::compatible())
        .map_err(|error| RuntimeError::external("Android Motion Photo analysis", error))?;
    photo
        .validate()
        .map_err(|error| RuntimeError::external("Android Motion Photo source validation", error))?;
    if carry.destination == "oppo" && photo.still().mime != "image/jpeg" {
        return Err(oppo_jpeg_route_blocked(&photo));
    }
    if carry.destination == "vivo" {
        return publish_preserved_vivo_source(marker_source, &photo, output, compatibility, carry);
    }
    if carry.destination == "huawei" {
        return publish_huawei_live_photo(marker_source, &photo, output, compatibility, carry);
    }
    let (plan, omitted_unmapped) = plan_public_motion(&photo, target_profile, carry)?;
    let omitted_ids = omitted_unmapped
        .iter()
        .map(|(id, _)| *id)
        .collect::<HashSet<_>>();
    let composed = plan
        .execute()
        .map_err(|error| RuntimeError::external("Android Motion Photo execution", error))?;
    if composed.files.len() != 1
        || composed.files[0].role != ResourceRole::PrimaryStill
        || composed.files[0].mime != expected_mime
        || composed.files[0].sidecar_for.is_some()
    {
        return Err(RuntimeError::new(
            "Android Motion Photo output",
            "writer returned an unexpected output graph instead of one self-contained file",
        ));
    }
    let mut bytes = composed.files[0].bytes.clone();
    let checked = MotionPhoto::parse(Input::SingleFile(&bytes), ParseOptions::strict())
        .map_err(|error| RuntimeError::external("Android Motion Photo strict reparse", error))?;
    checked
        .validate()
        .map_err(|error| RuntimeError::external("Android Motion Photo final validation", error))?;
    let original_movie = photo
        .motion_video_bytes()
        .map_err(|error| RuntimeError::external("source video", error))?;
    let written_movie = checked
        .motion_video_bytes()
        .map_err(|error| RuntimeError::external("output video", error))?;
    if written_movie != original_movie
        || !same_presentation_time(photo.presentation_time(), checked.presentation_time())
    {
        return Err(RuntimeError::new(
            "Android Motion Photo verification",
            "video container bytes or exact presentation instant changed",
        ));
    }
    verify_auxiliary_images(&photo, &checked)?;
    let preserved_left_out =
        verify_preserved_resources(&photo, &checked, &composed.report, carry.leave_out_unmapped)?;
    if hdr_gain_map_xmp_signature(&photo)? != hdr_gain_map_xmp_signature(&checked)? {
        return Err(RuntimeError::new(
            "Android HDR metadata verification",
            "HDR gain-map XMP properties changed during Motion Photo assembly",
        ));
    }
    let required_evidence = [
        PreservationEvidence::EncodedStillPayloadsEqual,
        PreservationEvidence::ExactPresentationEqual,
        PreservationEvidence::StillDisplayMetadataEqual,
        PreservationEvidence::StillColorMetadataEqual,
        PreservationEvidence::VideoContainerBytesEqual,
    ];
    for required in required_evidence {
        if !composed.report.evidence.contains(&required) {
            return Err(RuntimeError::new(
                "Android Motion Photo verification",
                format!("the writer did not establish {required:?}"),
            ));
        }
    }

    let resources = photo
        .resources()
        .map(|resource| AndroidMotionResourceAudit {
            id: resource.id.0,
            source_role: format!("{:?}", resource.role),
            mime: resource.mime.clone(),
            vendor_role: resource.vendor_role.clone(),
            byte_length: resource.extents.iter().map(|extent| extent.length()).sum(),
            operations: composed
                .report
                .resources
                .iter()
                .filter(|entry| entry.resource == Some(resource.id))
                .map(|entry| AndroidMotionResourceOperation {
                    disposition: format!("{:?}", entry.disposition),
                    reason: entry.reason.clone(),
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    let blocks_publication = |id: Option<usize>, disposition: &str| {
        if id.is_some_and(|id| omitted_ids.contains(&id))
            && matches!(disposition, "Unmapped" | "Dropped")
        {
            return false;
        }
        matches!(disposition, "Unmapped" | "Dropped" | "TranscodeRequired")
    };
    if resources.iter().any(|item| {
        item.operations.is_empty()
            || item
                .operations
                .iter()
                .any(|op| blocks_publication(Some(item.id), &op.disposition))
    }) || composed.report.resources.iter().any(|entry| {
        blocks_publication(
            entry.resource.map(|id| id.0),
            &format!("{:?}", entry.disposition),
        )
    }) {
        return Err(RuntimeError::new(
            "Android Motion Photo verification",
            "resource audit contains unrepresented or dropped source resources",
        ));
    }

    let source_xmp = photo
        .xmp_bytes()
        .map_err(|error| RuntimeError::external("source Motion Photo XMP", error))?;
    let mut vendor_metadata = carry_vendor_metadata(
        marker_source,
        source_xmp.as_deref(),
        &mut bytes,
        original_movie.len(),
        carry,
    )?;
    vendor_metadata
        .not_carried
        .extend(omitted_unmapped.into_iter().map(|(_, note)| note));
    vendor_metadata.not_carried.extend(preserved_left_out);
    if vendor_metadata.video_byte_length != u64::try_from(original_movie.len()).unwrap_or(u64::MAX)
    {
        return Err(RuntimeError::new(
            "vendor metadata",
            "vendor metadata report does not name the preserved video length",
        ));
    }
    let carried = MotionPhoto::parse(Input::SingleFile(&bytes), ParseOptions::strict())
        .map_err(|error| RuntimeError::external("vendor metadata reparse", error))?;
    carried
        .validate()
        .map_err(|error| RuntimeError::external("vendor metadata validation", error))?;
    let carried_movie = carried
        .motion_video_bytes()
        .map_err(|error| RuntimeError::external("vendor metadata video", error))?;
    if carried_movie != original_movie
        || !same_presentation_time(photo.presentation_time(), carried.presentation_time())
        || hdr_gain_map_xmp_signature(&photo)? != hdr_gain_map_xmp_signature(&carried)?
        || carried
            .resources()
            .filter(|resource| resource.role == ResourceRole::PrimaryMotionVideo)
            .count()
            != 1
    {
        return Err(RuntimeError::new(
            "vendor metadata",
            "copying source vendor attributes changed the video, presentation time, gain map, or video count",
        ));
    }
    if carry.destination == "samsung" && is_heif_file(&bytes) {
        samsung_top_level_mpvd_reaches_eof(&bytes)?;
    }

    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| RuntimeError::external("Android Motion Photo output directory", error))?;
    // NamedTempFile's no-clobber persist atomically fails if another process
    // creates the destination between preflight and publication.
    let mut staging = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| RuntimeError::external("Android Motion Photo staging", error))?;
    staging
        .write_all(&bytes)
        .and_then(|()| staging.as_file().sync_all())
        .map_err(|error| RuntimeError::external("Android Motion Photo durable write", error))?;
    staging.persist_noclobber(output).map_err(|error| {
        RuntimeError::external(
            "Android Motion Photo non-overwriting publication",
            error.error,
        )
    })?;
    Ok(AndroidMotionPhotoFileReceipt {
        output: output.to_path_buf(),
        conversion_report: composed.report,
        resources,
        video_compatibility: compatibility,
        video_brand_changed: false,
        vendor_metadata,
    })
}

fn publish_huawei_live_photo(
    source_bytes: &[u8],
    photo: &MotionPhoto<'_>,
    output: &Path,
    compatibility: MotionVideoCompatibility,
    carry: VendorMetadataCarry,
) -> Result<AndroidMotionPhotoFileReceipt> {
    if photo.asset().sources.len() != 1 {
        return Err(RuntimeError::new(
            "Huawei live photo",
            "the 40-byte tail was checked on one file whose video is a trailing range. A separate still and movie are not concatenated",
        ));
    }
    let video = photo
        .motion_video_bytes()
        .map_err(|error| RuntimeError::external("Huawei live photo video", error))?;
    let mut positions = Vec::new();
    let mut search = 0usize;
    while let Some(offset) = source_bytes[search..]
        .windows(video.len())
        .position(|window| window == video)
    {
        positions.push(search + offset);
        search += offset + 1;
        if positions.len() > 1 {
            break;
        }
    }
    if positions.len() != 1 {
        return Err(RuntimeError::new(
            "Huawei live photo",
            "the preserved video is not a single range in the source file",
        ));
    }
    let at = positions[0];
    if at + video.len() != source_bytes.len() {
        return Err(RuntimeError::new(
            "Huawei live photo",
            "bytes after the motion video are outside the LIVE_ length and are not dropped",
        ));
    }
    let info =
        crate::huawei_live_photo::play_info_for_preserved_video(video, photo.presentation_time())?;
    let bytes = crate::huawei_live_photo::assemble(
        &source_bytes[..at],
        video,
        info.start_ms,
        info.duration_ms,
    )?;
    let plan = photo.plan(
        TargetProfile::PreserveSource,
        xdremux_motion_photo::WritePolicy::default(),
    );
    if !plan.can_execute() {
        return Err(RuntimeError::new(
            "Huawei live photo",
            plan.compatibility().blockers.join("; "),
        ));
    }
    let resources = photo
        .resources()
        .map(|resource| AndroidMotionResourceAudit {
            id: resource.id.0,
            source_role: format!("{:?}", resource.role),
            mime: resource.mime.clone(),
            vendor_role: resource.vendor_role.clone(),
            byte_length: resource.extents.iter().map(|extent| extent.length()).sum(),
            operations: vec![AndroidMotionResourceOperation {
                disposition: "Preserved".to_owned(),
                reason: "prefix and video bytes are unchanged; a 40-byte tail was appended"
                    .to_owned(),
            }],
        })
        .collect();
    let mut not_carried = info.notes;
    not_carried.push(format!(
        "play info is {}:{} ms; DEFAULT_COVER_FRAME is not written",
        info.start_ms, info.duration_ms
    ));
    let vendor_metadata = VendorMetadataReport {
        destination: carry.destination.to_owned(),
        container: carry.container.to_owned(),
        native_vendor_claim: false,
        gallery_verified: false,
        carried_attributes: Vec::new(),
        namespace_absent: true,
        not_carried,
        gaps: carry.gaps.iter().map(|gap| (*gap).to_owned()).collect(),
        exif_identifier_present: crate::vendor_motion_metadata::exif_identifier_present(&bytes),
        video_byte_length: u64::try_from(video.len()).unwrap_or(u64::MAX),
    };
    let parent = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| RuntimeError::external("Android Motion Photo output directory", error))?;
    let mut staging = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| RuntimeError::external("Android Motion Photo staging", error))?;
    staging
        .write_all(&bytes)
        .and_then(|()| staging.as_file().sync_all())
        .map_err(|error| RuntimeError::external("Android Motion Photo durable write", error))?;
    staging.persist_noclobber(output).map_err(|error| {
        RuntimeError::external(
            "Android Motion Photo non-overwriting publication",
            error.error,
        )
    })?;
    Ok(AndroidMotionPhotoFileReceipt {
        output: output.to_path_buf(),
        conversion_report: plan.report().clone(),
        resources,
        video_compatibility: compatibility,
        video_brand_changed: false,
        vendor_metadata,
    })
}

fn publish_preserved_vivo_source(
    source_bytes: &[u8],
    photo: &MotionPhoto<'_>,
    output: &Path,
    compatibility: MotionVideoCompatibility,
    carry: VendorMetadataCarry,
) -> Result<AndroidMotionPhotoFileReceipt> {
    if photo.asset().sources.len() != 1 || photo.still().mime != "image/jpeg" {
        return Err(RuntimeError::new(
            "vivo Motion Photo",
            "public vivo samples are one JPEG file with an appended MP4. A different still or a separate movie is not re-encoded into that layout, because the source has no vivo UUID shell to keep",
        ));
    }
    let video = photo
        .motion_video_bytes()
        .map_err(|error| RuntimeError::external("vivo Motion Photo video", error))?;
    let extensions = photo
        .vivo_media_extensions()
        .map_err(|error| RuntimeError::external("vivo Motion Photo UUID extensions", error))?;
    let Some(extensions) = extensions else {
        return Err(vivo_layout_missing());
    };
    if extensions.stream_box_extent.is_none() || extensions.footer.is_none() {
        return Err(vivo_layout_missing());
    }
    let plan = photo.plan(
        TargetProfile::PreserveSource,
        xdremux_motion_photo::WritePolicy::default(),
    );
    if !plan.can_execute() {
        return Err(RuntimeError::new(
            "vivo Motion Photo",
            plan.compatibility().blockers.join("; "),
        ));
    }
    let composed = plan
        .execute()
        .map_err(|error| RuntimeError::external("vivo Motion Photo preserve", error))?;
    if composed.files.len() != 1 || composed.files[0].bytes.as_slice() != source_bytes {
        return Err(RuntimeError::new(
            "vivo Motion Photo",
            "preserving the source layout did not return the original file bytes",
        ));
    }
    if source_bytes
        .windows(video.len())
        .filter(|window| *window == video)
        .count()
        != 1
    {
        return Err(RuntimeError::new(
            "vivo Motion Photo",
            "the preserved file does not contain exactly one copy of the source video",
        ));
    }
    let resources = photo
        .resources()
        .map(|resource| AndroidMotionResourceAudit {
            id: resource.id.0,
            source_role: format!("{:?}", resource.role),
            mime: resource.mime.clone(),
            vendor_role: resource.vendor_role.clone(),
            byte_length: resource.extents.iter().map(|extent| extent.length()).sum(),
            operations: composed
                .report
                .resources
                .iter()
                .filter(|entry| entry.resource == Some(resource.id))
                .map(|entry| AndroidMotionResourceOperation {
                    disposition: format!("{:?}", entry.disposition),
                    reason: entry.reason.clone(),
                })
                .collect(),
        })
        .collect();
    let namespace = b"http://ns.vivo.com/photos/1.0/camera/";
    let namespace_present = source_bytes
        .windows(namespace.len())
        .any(|window| window == namespace);
    let carried_attributes = [
        "VCamera:VMotionPhotoVersion",
        "VCamera:VMotionPhotoSource",
        "VCamera:VMediaKitVersion",
    ]
    .into_iter()
    .filter(|name| {
        source_bytes
            .windows(name.len())
            .any(|window| window == name.as_bytes())
    })
    .map(str::to_owned)
    .collect::<Vec<_>>();
    let vendor_metadata = VendorMetadataReport {
        destination: carry.destination.to_owned(),
        container: carry.container.to_owned(),
        native_vendor_claim: false,
        gallery_verified: false,
        carried_attributes,
        namespace_absent: !namespace_present,
        not_carried: vec![
            "application/livePhoto-meta is kept only when the source MP4 already contains it; none was added".to_owned(),
            "the repeated sample live-photo token and EStream digest were not synthesized".to_owned(),
        ],
        gaps: carry.gaps.iter().map(|gap| (*gap).to_owned()).collect(),
        exif_identifier_present: crate::vendor_motion_metadata::exif_identifier_present(source_bytes),
        video_byte_length: u64::try_from(video.len()).unwrap_or(u64::MAX),
    };
    let parent = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| RuntimeError::external("Android Motion Photo output directory", error))?;
    let mut staging = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| RuntimeError::external("Android Motion Photo staging", error))?;
    staging
        .write_all(source_bytes)
        .and_then(|()| staging.as_file().sync_all())
        .map_err(|error| RuntimeError::external("Android Motion Photo durable write", error))?;
    staging.persist_noclobber(output).map_err(|error| {
        RuntimeError::external(
            "Android Motion Photo non-overwriting publication",
            error.error,
        )
    })?;
    Ok(AndroidMotionPhotoFileReceipt {
        output: output.to_path_buf(),
        conversion_report: composed.report,
        resources,
        video_compatibility: compatibility,
        video_brand_changed: false,
        vendor_metadata,
    })
}

fn vivo_layout_missing() -> RuntimeError {
    RuntimeError::new(
        "vivo Motion Photo",
        "vivo gallery 3.2.10.4 does not read GCamera or VCamera XMP. The public JPEG samples keep vivoMediaEStream and vivoMediaExtInfo inside the MP4, and application/livePhoto-meta is optional. This source has no such UUID pair to preserve. The repeated sample token and EStream digest are not written, and a livePhoto-meta DataBlock is not synthesized. This target is not complete for other layouts",
    )
}

fn oppo_jpeg_route_blocked(photo: &MotionPhoto<'_>) -> RuntimeError {
    let plan = photo.plan(
        TargetProfile::AndroidJpeg,
        xdremux_motion_photo::WritePolicy::default(),
    );
    let blocked = plan
        .report()
        .resources
        .iter()
        .filter(|entry| {
            matches!(
                entry.disposition,
                Disposition::TranscodeRequired | Disposition::Unmapped | Disposition::Dropped
            )
        })
        .map(|entry| format!("{:?}: {}", entry.disposition, entry.reason))
        .collect::<Vec<_>>();
    let detail = if blocked.is_empty() {
        plan.compatibility().blockers.join("; ")
    } else {
        blocked.join("; ")
    };
    RuntimeError::new(
        "OPPO Motion Photo",
        format!(
            "25.17.35 isOlivePhoto and 17.10.62 saq.w both require image/jpeg before the JPEG olive reader. Neither package shows a HEIF olive path, so the planned output is a JPEG Motion Photo that keeps the source video bytes. Publication stopped before still re-encoding, which would otherwise drop or rewrite resources: {detail}"
        ),
    )
}

fn plan_public_motion<'a>(
    photo: &'a MotionPhoto<'_>,
    target_profile: TargetProfile,
    carry: VendorMetadataCarry,
) -> Result<(
    xdremux_motion_photo::ConversionPlan<'a, 'a>,
    Vec<(usize, String)>,
)> {
    let strict = photo.plan(target_profile, WritePolicy::default());
    if strict.can_execute() || !carry.leave_out_unmapped {
        if !strict.can_execute() {
            return Err(RuntimeError::new(
                "Android Motion Photo plan",
                strict.compatibility().blockers.join("; "),
            ));
        }
        return Ok((strict, Vec::new()));
    }
    let blockers = strict.compatibility().blockers.clone();
    let only_unmapped = !blockers.is_empty()
        && blockers
            .iter()
            .all(|blocker| blocker.starts_with("target cannot express resource "));
    let unmapped = photo
        .resources()
        .filter(|resource| {
            strict.report().resources.iter().any(|entry| {
                entry.resource == Some(resource.id) && entry.disposition == Disposition::Unmapped
            })
        })
        .collect::<Vec<_>>();
    let safe_to_leave_out = only_unmapped
        && !unmapped.is_empty()
        && unmapped.iter().all(|resource| {
            !matches!(
                resource.role,
                ResourceRole::PrimaryStill
                    | ResourceRole::PrimaryMotionVideo
                    | ResourceRole::AuxiliaryImage
            )
        });
    if !safe_to_leave_out {
        return Err(RuntimeError::new(
            "Android Motion Photo plan",
            blockers.join("; "),
        ));
    }
    let notes = unmapped
        .iter()
        .map(|resource| {
            let bytes = resource.extents.iter().map(|extent| extent.length()).sum::<u64>();
            (
                resource.id.0,
                format!(
                    "resource {} ({:?}, {}, {} bytes, {}) has no public Motion Photo mapping and was left out",
                    resource.id.0,
                    resource.vendor_role,
                    resource.mime,
                    bytes,
                    resource.provenance.evidence
                ),
            )
        })
        .collect::<Vec<_>>();
    let plan = photo.plan(
        target_profile,
        WritePolicy {
            unmapped: UnmappedPolicy::DropExplicitly,
            ..WritePolicy::default()
        },
    );
    if !plan.can_execute() {
        return Err(RuntimeError::new(
            "Android Motion Photo plan",
            plan.compatibility().blockers.join("; "),
        ));
    }
    Ok((plan, notes))
}

fn verify_auxiliary_images(source: &MotionPhoto<'_>, target: &MotionPhoto<'_>) -> Result<()> {
    // For HEIF, item identifiers are stable; for JPEG, the XMP directory
    // semantic and item order identify the gain-map byte region.
    for item in source.resources().filter(|r| {
        r.role == ResourceRole::AuxiliaryImage
            || r.vendor_role
                .as_deref()
                .is_some_and(|v| v.eq_ignore_ascii_case("GainMap"))
    }) {
        let matching = target.resources().find(|r| {
            if let Some(id) = item.container_item_id {
                return r.container_item_id == Some(id) && r.role == item.role;
            }
            r.role == item.role
                && r.vendor_role == item.vendor_role
                && r.mime == item.mime
                && r.provenance.evidence == item.provenance.evidence
        });
        let Some(matching) = matching else {
            return Err(RuntimeError::new(
                "Android HDR resource verification",
                format!("auxiliary image {:?} was omitted", item.vendor_role),
            ));
        };
        let mut before = Vec::new();
        let mut after = Vec::new();
        source
            .extract(item, &mut before)
            .map_err(|error| RuntimeError::external("source auxiliary extraction", error))?;
        target
            .extract(matching, &mut after)
            .map_err(|error| RuntimeError::external("output auxiliary extraction", error))?;
        if before != after {
            return Err(RuntimeError::new(
                "Android HDR resource verification",
                format!("auxiliary image {:?} changed", item.vendor_role),
            ));
        }
    }
    Ok(())
}

/// A `Preserved` disposition is a byte-level claim. Verify it for every
/// resource that the format plan did not also mark `Rewritten`.
fn verify_preserved_resources(
    source: &MotionPhoto<'_>,
    target: &MotionPhoto<'_>,
    report: &ConversionReport,
    leave_out_unmapped: bool,
) -> Result<Vec<String>> {
    let mut left_out = Vec::new();
    for resource in source.resources() {
        let operations = report
            .resources
            .iter()
            .filter(|entry| entry.resource == Some(resource.id));
        if !operations
            .clone()
            .any(|entry| entry.disposition == Disposition::Preserved)
            || operations
                .clone()
                .any(|entry| entry.disposition == Disposition::Rewritten)
        {
            continue;
        }
        let same_kind = |candidate: &xdremux_motion_photo::Resource| {
            candidate.role == resource.role
                && candidate.mime == resource.mime
                && candidate.vendor_role == resource.vendor_role
                && candidate.container_item_id == resource.container_item_id
        };
        let ordinal = source
            .resources()
            .filter(|item| same_kind(item))
            .position(|item| item.id == resource.id)
            .ok_or_else(|| {
                RuntimeError::new(
                    "Android Motion Photo resource verification",
                    "source resource inventory changed",
                )
            })?;
        let matching = target
            .resources()
            .filter(|item| same_kind(item))
            .nth(ordinal);
        let Some(matching) = matching else {
            if leave_out_vendor_record(leave_out_unmapped, resource) {
                left_out.push(format!(
                    "resource {} ({:?}, {}, {}) was not kept in the public Motion Photo container",
                    resource.id.0,
                    resource.vendor_role,
                    resource.mime,
                    resource.provenance.evidence
                ));
                continue;
            }
            return Err(RuntimeError::new(
                "Android Motion Photo resource verification",
                format!(
                    "preserved resource {} is missing from the output",
                    resource.id.0
                ),
            ));
        };
        let mut before = Vec::new();
        let mut after = Vec::new();
        source
            .extract(resource, &mut before)
            .map_err(|error| RuntimeError::external("source preserved resource", error))?;
        target
            .extract(matching, &mut after)
            .map_err(|error| RuntimeError::external("output preserved resource", error))?;
        if before != after {
            if leave_out_vendor_record(leave_out_unmapped, resource) {
                left_out.push(format!(
                    "resource {} ({:?}, {}, {}) changed and was not kept as a vendor-private record",
                    resource.id.0,
                    resource.vendor_role,
                    resource.mime,
                    resource.provenance.evidence
                ));
                continue;
            }
            return Err(RuntimeError::new(
                "Android Motion Photo resource verification",
                format!(
                    "resource {} was reported preserved but its bytes changed",
                    resource.id.0
                ),
            ));
        }
    }
    Ok(left_out)
}

fn is_heif_file(bytes: &[u8]) -> bool {
    bytes.len() > 12 && &bytes[4..8] == b"ftyp"
}

/// Samsung 15.1.03.4 `ex.a.d` returns the `mpvd` payload start and the caller
/// then reads through EOF. A normal 32-bit size must make that payload the
/// rest of the file, so `mpvd` is the last top-level box.
fn samsung_top_level_mpvd_reaches_eof(data: &[u8]) -> Result<()> {
    let mut offset = 0usize;
    while offset + 8 <= data.len() {
        let size = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        if size < 8 {
            return Err(RuntimeError::new(
                "Samsung HEIF Motion Photo",
                "a top-level box uses size 0 or 1; Samsung 15.1.03.4 does not handle those forms",
            ));
        }
        let end = offset.checked_add(size).ok_or_else(|| {
            RuntimeError::new("Samsung HEIF Motion Photo", "top-level box size overflows")
        })?;
        if end > data.len() {
            return Err(RuntimeError::new(
                "Samsung HEIF Motion Photo",
                "a top-level box extends past the file",
            ));
        }
        if &data[offset + 4..offset + 8] == b"mpvd" {
            if end != data.len() {
                return Err(RuntimeError::new(
                    "Samsung HEIF Motion Photo",
                    "mpvd is not the last top-level box, so its payload would not run to EOF",
                ));
            }
            return Ok(());
        }
        offset = end;
    }
    Err(RuntimeError::new(
        "Samsung HEIF Motion Photo",
        "no top-level mpvd box was found",
    ))
}

fn leave_out_vendor_record(
    leave_out_unmapped: bool,
    resource: &xdremux_motion_photo::Resource,
) -> bool {
    leave_out_unmapped
        && (resource.role == ResourceRole::Unknown
            || resource.role == ResourceRole::UnknownVideo
            || resource.provenance.evidence.starts_with("Samsung SEF"))
}

/// Compare gain-map XMP properties by namespace and local name, not by the
/// entire XMP packet: Google Motion Photo camera/directory fields must change.
/// Keep repeated `rdf:li` text in order (important for channel arrays).
fn hdr_gain_map_xmp_signature(photo: &MotionPhoto<'_>) -> Result<Vec<(String, String)>> {
    const HDRGM: &str = "http://ns.adobe.com/hdr-gain-map/1.0/";
    let packet = photo
        .xmp_bytes()
        .map_err(|error| RuntimeError::external("HDR gain-map XMP extraction", error))?;
    let Some(packet) = packet else {
        return Ok(Vec::new());
    };
    let mut reader = NsReader::from_reader(packet.as_ref());
    let mut depth = 0usize;
    let mut active: Vec<(usize, String)> = Vec::new();
    let mut properties = Vec::new();
    loop {
        let event = reader
            .read_event()
            .map_err(|error| RuntimeError::external("HDR gain-map XMP parsing", error))?;
        match event {
            Event::Start(ref node) | Event::Empty(ref node) => {
                let is_start = matches!(event, Event::Start(_));
                let (ns, local) = reader.resolver().resolve_element(node.name());
                if matches!(ns, ResolveResult::Bound(n) if n.as_ref() == HDRGM) {
                    let name = local.as_ref().to_string();
                    properties.push((format!("element:{name}"), String::new()));
                    if is_start {
                        active.push((depth + 1, name));
                    }
                }
                let mut attributes = Vec::new();
                for attribute in node.attributes() {
                    let attribute = attribute.map_err(|error| {
                        RuntimeError::external("HDR gain-map XMP attribute", error)
                    })?;
                    let (ns, local) = reader.resolver().resolve_attribute(attribute.key);
                    if matches!(ns, ResolveResult::Bound(n) if n.as_ref() == HDRGM) {
                        let key = local.as_ref().to_string();
                        let value = attribute
                            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                            .map_err(|error| {
                                RuntimeError::external("HDR gain-map XMP value", error)
                            })?;
                        attributes.push((format!("attribute:{key}"), value.into_owned()));
                    }
                }
                attributes.sort();
                properties.extend(attributes);
                if is_start {
                    depth += 1;
                }
            }
            Event::Text(ref content) if !active.is_empty() => {
                let value = content.xml10_content();
                if !value.trim().is_empty() {
                    let key = active.last().unwrap().1.clone();
                    properties.push((format!("value:{key}"), value.trim().to_string()));
                }
            }
            Event::End(_) => {
                if active.last().is_some_and(|(start, _)| *start == depth) {
                    active.pop();
                }
                depth = depth.saturating_sub(1);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(properties)
}
