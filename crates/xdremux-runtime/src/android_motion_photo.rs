//! Google Motion Photo output is a separate, fail-closed publication route.
//! The Apple pair/PVT route is intentionally unchanged.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use xdremux_engine::MotionOutputTarget;
use xdremux_motion_photo::{
    ConversionReport, Input, MotionPhoto, ParseOptions, PreservationEvidence, ResourceRole,
    TargetProfile, WritePolicy,
};

use crate::{Result, RuntimeError};

#[derive(Debug, Clone, PartialEq)]
pub struct AndroidMotionPhotoFileReceipt {
    pub output: PathBuf,
    pub conversion_report: ConversionReport,
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

pub(crate) fn convert_android_motion_photo_file(
    source: &[u8],
    input: &Path,
    output: &Path,
    target: MotionOutputTarget,
) -> Result<AndroidMotionPhotoFileReceipt> {
    validate_android_output_path(output, target)?;
    let (target_profile, _, expected_mime) = profile(target)?;
    if input == output || fs::symlink_metadata(output).is_ok() {
        return Err(RuntimeError::new(
            "Android Motion Photo output",
            "output already exists; refusing to overwrite the source or unknown provenance",
        ));
    }
    let photo = MotionPhoto::parse(Input::SingleFile(source), ParseOptions::compatible())
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
    if photo
        .motion_video_bytes()
        .map_err(|error| RuntimeError::external("source video", error))?
        != checked
            .motion_video_bytes()
            .map_err(|error| RuntimeError::external("output video", error))?
        || photo.presentation_time() != checked.presentation_time()
    {
        return Err(RuntimeError::new(
            "Android Motion Photo verification",
            "video container bytes or exact presentation timestamp changed",
        ));
    }
    for required in [
        PreservationEvidence::EncodedStillPayloadsEqual,
        PreservationEvidence::VideoContainerBytesEqual,
        PreservationEvidence::ExactPresentationEqual,
    ] {
        if !composed.report.evidence.contains(&required) {
            return Err(RuntimeError::new(
                "Android Motion Photo verification",
                format!("the writer did not establish {required:?}"),
            ));
        }
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
    })
}
