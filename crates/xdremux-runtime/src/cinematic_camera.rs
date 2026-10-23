//! Process and file boundary for Rust-owned Cinematic camera selection.

use crate::{CinematicFocalLength, Result, RuntimeError};
use std::fs::File;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use xdremux_engine::{
    resolve_recorded_cinematic_focal_length, CinematicCameraEstimate, CinematicCameraSummary,
};

const MAX_JSON: usize = 1_048_576;
const MAX_DIAGNOSTICS: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CinematicFocalLengthSource {
    Explicit,
    RecordedMetadata,
    GeoCalib,
    CameraEstimate,
}

impl CinematicFocalLengthSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::RecordedMetadata => "recorded-metadata",
            Self::GeoCalib => "geocalib",
            Self::CameraEstimate => "camera-estimate",
        }
    }
}

pub(crate) struct ResolvedCamera {
    pub focal: f32,
    pub estimate: Option<CinematicCameraSummary>,
    pub source: CinematicFocalLengthSource,
}

pub(crate) fn resolve(
    adapter: &Path,
    input: &Path,
    staging: &Path,
    focal: &CinematicFocalLength,
    source_hash: &str,
) -> Result<ResolvedCamera> {
    match focal {
        CinematicFocalLength::Assumed(value) => Ok(ResolvedCamera {
            focal: *value,
            estimate: None,
            source: CinematicFocalLengthSource::Explicit,
        }),
        CinematicFocalLength::CameraEstimate(path) => read_estimate(
            path,
            source_hash,
            CinematicFocalLengthSource::CameraEstimate,
        ),
        CinematicFocalLength::Automatic => {
            let metadata = run_tool(
                Command::new(tool_path(adapter, "XDREMUX_EXIFTOOL", "exiftool"))
                    .args([
                        "-j",
                        "-n",
                        "-a",
                        "-G1:3:4",
                        "-HandlerType",
                        "-FocalLengthIn35mmFormat",
                        "--",
                    ])
                    .arg(input),
                "Cinematic focal metadata (ExifTool)",
                Duration::from_secs(30),
            )?;
            if let Some(focal) = recorded_focal(&metadata)? {
                return Ok(ResolvedCamera {
                    focal,
                    estimate: None,
                    source: CinematicFocalLengthSource::RecordedMetadata,
                });
            }
            let estimate = staging.join("camera.json");
            run_tool(
                Command::new(tool_path(adapter, "XDREMUX_GEOCALIB", "xdremux-geocalib"))
                    .arg("--input")
                    .arg(input)
                    .arg("--output")
                    .arg(&estimate),
                "Cinematic GeoCalib (install xdremux-geocalib beside xdremux or set XDREMUX_GEOCALIB)",
                Duration::from_secs(600),
            )?;
            read_estimate(&estimate, source_hash, CinematicFocalLengthSource::GeoCalib)
        }
    }
}

fn tool_path(adapter: &Path, variable: &str, name: &str) -> PathBuf {
    if let Some(path) = std::env::var_os(variable) {
        return path.into();
    }
    let sibling = adapter
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(name);
    if sibling.is_file() {
        sibling
    } else {
        PathBuf::from(name)
    }
}

fn recorded_focal(bytes: &[u8]) -> Result<Option<f32>> {
    let facts: Vec<serde_json::Map<String, serde_json::Value>> = serde_json::from_slice(bytes)
        .map_err(|error| RuntimeError::external("Cinematic focal metadata JSON", error))?;
    if facts.len() != 1 {
        return Err(RuntimeError::new(
            "Cinematic focal metadata",
            "expected one source record",
        ));
    }
    let mut equivalents = Vec::new();
    for (key, value) in &facts[0] {
        let groups = key.split(':').collect::<Vec<_>>();
        // Group family 3 distinguishes the main asset from embedded thumbnails.
        // Composite values and physical FocalLength are not recorded equivalents.
        if groups.last() != Some(&"FocalLengthIn35mmFormat")
            || !groups.contains(&"Main")
            || !matches!(
                groups[0],
                "ExifIFD" | "XMP-exif" | "VideoKeys" | "Keys" | "QuickTime"
            )
        {
            continue;
        }
        let value = value
            .as_f64()
            .or_else(|| {
                let text = value.as_str()?.trim();
                text.strip_suffix("mm")
                    .unwrap_or(text)
                    .trim()
                    .parse::<f64>()
                    .ok()
            })
            .ok_or_else(|| {
                RuntimeError::new(
                    "Cinematic focal metadata",
                    format!("invalid numeric equivalent in {key}"),
                )
            })?;
        equivalents.push(value);
    }
    let focal = resolve_recorded_cinematic_focal_length(&equivalents)
        .map_err(|error| RuntimeError::external("Cinematic focal metadata", error))?;
    if focal.is_some() {
        // VideoKeys identifies a metadata namespace, not its owning track. The
        // native adapter processes the first video track, so an unbound recorded
        // equivalent is safe only when the main asset has exactly one video track.
        let mut video_tracks = std::collections::BTreeSet::new();
        for (key, value) in &facts[0] {
            let groups = key.split(':').collect::<Vec<_>>();
            if groups.last() != Some(&"HandlerType")
                || !groups.contains(&"Main")
                || !groups[0].strip_prefix("Track").is_some_and(|number| {
                    !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
                })
            {
                continue;
            }
            let handler = value.as_str().ok_or_else(|| {
                RuntimeError::new(
                    "Cinematic focal metadata",
                    format!("invalid track handler in {key}"),
                )
            })?;
            if handler == "vide" {
                video_tracks.insert(groups[0]);
            }
        }
        if video_tracks.len() != 1 {
            return Err(RuntimeError::new(
                "Cinematic focal metadata",
                "recorded focal length requires exactly one verified video track; specify a fixed focal length explicitly",
            ));
        }
    }
    Ok(focal)
}

fn read_estimate(
    path: &Path,
    source_hash: &str,
    source: CinematicFocalLengthSource,
) -> Result<ResolvedCamera> {
    if !path.is_file() {
        return Err(RuntimeError::new(
            "Cinematic camera estimate",
            "estimate is missing or is not a regular file",
        ));
    }
    let file = File::open(path)
        .map_err(|error| RuntimeError::external("Cinematic camera estimate", error))?;
    let mut bytes = Vec::new();
    file.take(MAX_JSON as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| RuntimeError::external("Cinematic camera estimate read", error))?;
    if bytes.len() > MAX_JSON {
        return Err(RuntimeError::new(
            "Cinematic camera estimate",
            "estimate exceeds 1 MiB",
        ));
    }
    let estimate: CinematicCameraEstimate = serde_json::from_slice(&bytes)
        .map_err(|error| RuntimeError::external("Cinematic camera estimate JSON", error))?;
    if estimate.source_sha256 != source_hash {
        return Err(RuntimeError::new(
            "Cinematic camera estimate",
            "source SHA-256 does not match the input video",
        ));
    }
    let summary = estimate
        .resolve()
        .map_err(|error| RuntimeError::external("Cinematic camera estimate", error))?;
    Ok(ResolvedCamera {
        focal: summary.focal_length_35mm,
        estimate: Some(summary),
        source,
    })
}

struct ToolProcess(Child);

impl Drop for ToolProcess {
    fn drop(&mut self) {
        // Also stop FFmpeg or any other descendants holding the pipes open.
        if let Some(pid) = rustix::process::Pid::from_raw(self.0.id() as i32) {
            let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn read_pipe(
    pipe: impl Read + Send + 'static,
    limit: usize,
) -> mpsc::Receiver<std::io::Result<Vec<u8>>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = pipe
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = sender.send(result);
    });
    receiver
}

fn run_tool(command: &mut Command, context: &'static str, timeout: Duration) -> Result<Vec<u8>> {
    let mut child = ToolProcess(
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()
            .map_err(|error| RuntimeError::external(context, error))?,
    );
    let stdout = read_pipe(child.0.stdout.take().expect("piped stdout"), MAX_JSON);
    let stderr = read_pipe(
        child.0.stderr.take().expect("piped stderr"),
        MAX_DIAGNOSTICS,
    );
    let started = Instant::now();
    let mut output = None;
    let mut diagnostics = None;
    let mut status = None;
    loop {
        for (receiver, destination, limit) in [
            (&stdout, &mut output, MAX_JSON),
            (&stderr, &mut diagnostics, MAX_DIAGNOSTICS),
        ] {
            if destination.is_none() {
                if let Ok(result) = receiver.try_recv() {
                    let bytes = result.map_err(|error| RuntimeError::external(context, error))?;
                    if bytes.len() > limit {
                        return Err(RuntimeError::new(
                            context,
                            "helper output exceeded its byte limit",
                        ));
                    }
                    *destination = Some(bytes);
                }
            }
        }
        if status.is_none() {
            status = child
                .0
                .try_wait()
                .map_err(|error| RuntimeError::external(context, error))?;
        }
        if let (Some(status), Some(output), Some(diagnostics)) = (&status, &output, &diagnostics) {
            if !status.success() {
                return Err(RuntimeError::new(
                    context,
                    format!("helper {status}: {}", String::from_utf8_lossy(diagnostics)),
                ));
            }
            return Ok(output.clone());
        }
        if started.elapsed() >= timeout {
            return Err(RuntimeError::new(context, "helper timed out"));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorded_equivalent_requires_one_verified_video_track() {
        for metadata in [
            // Only the second video track has the tag. ExifTool's VideoKeys
            // group does not retain that ownership, so it must not be selected.
            r#"[{"Track1:Main:HandlerType":"vide","Track2:Main:HandlerType":"vide","VideoKeys:Main:FocalLengthIn35mmFormat":"70.00mm"}]"#,
            r#"[{"VideoKeys:Main:FocalLengthIn35mmFormat":70}]"#,
            r#"[{"Track1:Doc1:HandlerType":"vide","VideoKeys:Main:FocalLengthIn35mmFormat":70}]"#,
            r#"[{"Track1:Main:HandlerType":"soun","ExifIFD:Main:FocalLengthIn35mmFormat":70}]"#,
        ] {
            let error = recorded_focal(metadata.as_bytes()).unwrap_err();
            assert!(
                error.to_string().contains("exactly one verified video track"),
                "{error}"
            );
        }
    }

    #[test]
    fn recorded_equivalent_accepts_one_video_with_other_tracks_and_duplicate_observations() {
        let metadata = br#"[{
            "Track1:Main:HandlerType":"vide",
            "Track1:Main:Copy1:HandlerType":"vide",
            "Track2:Main:HandlerType":"soun",
            "Track3:Main:HandlerType":"meta",
            "Track4:Doc1:HandlerType":"vide",
            "VideoKeys:Main:FocalLengthIn35mmFormat":"70.00mm",
            "ExifIFD:Main:FocalLengthIn35mmFormat":70
        }]"#;
        assert_eq!(recorded_focal(metadata).unwrap(), Some(70.0));
        let malformed =
            br#"[{"Track1:Main:HandlerType":7,"VideoKeys:Main:FocalLengthIn35mmFormat":70}]"#;
        assert!(recorded_focal(malformed)
            .unwrap_err()
            .to_string()
            .contains("invalid track handler"));
    }

    #[test]
    fn missing_equivalent_keeps_model_fallback_available_without_quicktime_track_facts() {
        for metadata in [
            "[{}]",
            r#"[{"ExifIFD:Main:FocalLengthIn35mmFormat":0}]"#,
            r#"[{"ExifIFD:Doc1:FocalLengthIn35mmFormat":70}]"#,
        ] {
            assert_eq!(recorded_focal(metadata.as_bytes()).unwrap(), None);
        }
    }

    #[test]
    fn camera_tools_bound_lifetime_and_both_pipes() {
        for (script, expected) in [
            ("sleep 60 & wait", "timed out"),
            ("head -c 1048577 /dev/zero; sleep 60", "byte limit"),
            ("head -c 65537 /dev/zero >&2; sleep 60", "byte limit"),
            ("echo model-failure >&2; exit 2", "model-failure"),
        ] {
            let start = Instant::now();
            let error = run_tool(
                Command::new("/bin/sh").args(["-c", script]),
                "test",
                Duration::from_secs(1),
            )
            .unwrap_err();
            assert!(error.to_string().contains(expected), "{error}");
            assert!(start.elapsed() < Duration::from_secs(5));
        }
    }
}
