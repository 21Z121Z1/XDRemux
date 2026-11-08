//! Google Motion Photo output is a separate, fail-closed publication route.
//! The Apple pair/PVT route is intentionally unchanged.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;

use xdremux_engine::{MotionOutputTarget, MotionVideoCompatibility};
use xdremux_source::{motion_still_encoding, MotionStillEncoding, SourceAsset};
use xdremux_motion_photo::{
    ConversionReport, Disposition, Input, MediaTime, MotionPhoto, ParseOptions, PreservationEvidence,
    ResourceRole, TargetProfile, WritePolicy,
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
    if target != MotionOutputTarget::GoogleAuto {
        return Ok(target);
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
) -> Result<AndroidMotionPhotoFileReceipt> {
    convert_android_motion_photo(
        Input::SingleFile(source),
        &[input],
        output,
        target,
        compatibility,
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
) -> Result<AndroidMotionPhotoFileReceipt> {
    convert_android_motion_photo(
        Input::ApplePair { still, movie },
        &[still_path, movie_path],
        output,
        target,
        compatibility,
    )
}

fn convert_android_motion_photo(
    source: Input<'_>,
    inputs: &[&Path],
    output: &Path,
    target: MotionOutputTarget,
    compatibility: MotionVideoCompatibility,
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
    let plan = photo.plan(target_profile, WritePolicy::default());
    if !plan.can_execute() {
        return Err(RuntimeError::new(
            "Android Motion Photo plan",
            plan.compatibility().blockers.join("; "),
        ));
    }
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
    let bytes = &composed.files[0].bytes;
    let checked = MotionPhoto::parse(Input::SingleFile(bytes), ParseOptions::strict())
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
    verify_preserved_resources(&photo, &checked, &composed.report)?;
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
    if resources.iter().any(|item| {
        item.operations.is_empty()
            || item.operations.iter().any(|op| {
                matches!(
                    op.disposition.as_str(),
                    "Unmapped" | "Dropped" | "TranscodeRequired"
                )
            })
    }) || composed.report.resources.iter().any(|entry| {
        matches!(
            entry.disposition,
            Disposition::Unmapped | Disposition::Dropped | Disposition::TranscodeRequired
        )
    }) {
        return Err(RuntimeError::new(
            "Android Motion Photo verification",
            "resource audit contains unrepresented or dropped source resources",
        ));
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
        .write_all(bytes)
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
    })
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
) -> Result<()> {
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
            .nth(ordinal)
            .ok_or_else(|| {
                RuntimeError::new(
                    "Android Motion Photo resource verification",
                    format!(
                        "preserved resource {} is missing from the output",
                        resource.id.0
                    ),
                )
            })?;
        let mut before = Vec::new();
        let mut after = Vec::new();
        source
            .extract(resource, &mut before)
            .map_err(|error| RuntimeError::external("source preserved resource", error))?;
        target
            .extract(matching, &mut after)
            .map_err(|error| RuntimeError::external("output preserved resource", error))?;
        if before != after {
            return Err(RuntimeError::new(
                "Android Motion Photo resource verification",
                format!(
                    "resource {} was reported preserved but its bytes changed",
                    resource.id.0
                ),
            ));
        }
    }
    Ok(())
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
