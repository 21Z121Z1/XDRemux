#![cfg(target_os = "macos")]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use xdremux_runtime::PortableRuntime;

fn source(root: &Path) -> std::path::PathBuf {
    let path = root.join("source.mov");
    fs::write(&path, b"protected source bytes").unwrap();
    path
}

#[test]
fn cinematic_preflight_preserves_existing_files_and_rejects_invalid_parameters() {
    let temp = tempfile::tempdir().unwrap();
    let input = source(temp.path());
    let output = temp.path().join("existing.mov");
    fs::write(&output, b"existing output").unwrap();
    let runtime = PortableRuntime::new();
    let error = runtime
        .generate_cinematic_envelope_file("/must-not-execute", &input, &output, 24.0, 2.8)
        .unwrap_err();
    assert!(error.to_string().contains("refusing to replace"));
    assert_eq!(fs::read(&output).unwrap(), b"existing output");
    for (focal, aperture) in [
        (0.0, 2.8),
        (f32::NAN, 2.8),
        (24.0, 0.0),
        (24.0, f32::INFINITY),
    ] {
        let target = temp.path().join("invalid.mov");
        assert!(runtime
            .generate_cinematic_envelope_file("/must-not-execute", &input, &target, focal, aperture)
            .is_err());
        assert!(!target.exists());
    }
    assert_eq!(fs::read(input).unwrap(), b"protected source bytes");
}

fn helper(root: &Path, behavior: &str) -> std::path::PathBuf {
    let path = root.join("adapter.py");
    let script = format!(
        r#"#!/usr/bin/python3
import json, pathlib, sys
for line in sys.stdin:
    request = json.loads(line)
    pathlib.Path(request['output_path']).write_bytes(b'candidate')
    if '{behavior}' == 'source-change':
        pathlib.Path(request['input_path']).write_bytes(b'changed source')
    response = {{'schema_version': 2, 'cinematic_envelope': {{
        'processed_frames': 0 if '{behavior}' == 'zero-frames' else 1,
        'focal_length_35mm': 24, 'aperture': 2.8,
        'disparity_track_written': '{behavior}' == 'raster-depth'
    }}}}
    print(json.dumps(response), flush=True)
"#
    );
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}

#[test]
fn cinematic_rejects_incomplete_adapter_output_and_removes_staging() {
    for behavior in ["zero-frames", "raster-depth", "source-change"] {
        let temp = tempfile::tempdir().unwrap();
        let input = source(temp.path());
        let output = temp.path().join("pending.mov");
        let adapter = helper(temp.path(), behavior);
        let error = PortableRuntime::new()
            .generate_cinematic_envelope_file(adapter, &input, &output, 24.0, 2.8)
            .unwrap_err();
        let expected = match behavior {
            "zero-frames" => "without processing a video frame",
            "raster-depth" => "unexpectedly materialized a disparity track",
            _ => "source changed",
        };
        assert!(error.to_string().contains(expected), "{error}");
        assert!(!output.exists());
        assert!(!fs::read_dir(temp.path()).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".xdremux-cinematic-")));
    }
}

#[test]
fn cinematic_rejects_misnamed_output_before_adapter_execution() {
    let temp = tempfile::tempdir().unwrap();
    let input = source(temp.path());
    let output = temp.path().join("image.heic");
    let error = PortableRuntime::new()
        .generate_cinematic_envelope_file("/must-not-execute", &input, &output, 24.0, 2.8)
        .unwrap_err();
    assert!(error.to_string().contains(".mov extension"));
    assert!(!output.exists());
    assert_eq!(fs::read(input).unwrap(), b"protected source bytes");
}

#[test]
fn cinematic_batch_routes_video_intent_and_rejects_incomplete_output() {
    use xdremux_engine::{AppleFeatureRequest, ConversionRequest};
    use xdremux_runtime::{BatchExecutionOptions, BatchItem};
    for jobs in [1, 2] {
        let temp = tempfile::tempdir().unwrap();
        let input = source(temp.path());
        let output = temp.path().join("pending.mov");
        let second_input = temp.path().join("second.mov");
        fs::write(&second_input, b"protected second source").unwrap();
        let second_output = temp.path().join("second-pending.mov");
        let adapter = helper(temp.path(), "zero-frames");
        let request = ConversionRequest {
            apple_features: AppleFeatureRequest {
                cinematic: true,
                ..AppleFeatureRequest::default()
            },
            ..ConversionRequest::default()
        };
        let receipt = PortableRuntime::new().convert_batch_with_options(
            vec![
                BatchItem::new(&input, &output),
                BatchItem::new(&second_input, &second_output),
            ],
            request,
            &BatchExecutionOptions {
                jobs,
                apple_adapter_executable: Some(adapter),
                ..BatchExecutionOptions::default()
            },
        );
        assert_eq!(receipt.failed(), 2);
        assert!(receipt
            .failures
            .iter()
            .all(|failure| failure.error.contains("without processing a video frame")));
        assert!(!output.exists() && !second_output.exists());
        assert_eq!(fs::read(input).unwrap(), b"protected source bytes");
        assert_eq!(fs::read(second_input).unwrap(), b"protected second source");
    }
}
