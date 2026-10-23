#![cfg(target_os = "macos")]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use xdremux_runtime::{CinematicEnvelopeOptions, CinematicFocalLength, PortableRuntime};

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
        .generate_cinematic_envelope_file(
            "/must-not-execute",
            &input,
            &output,
            &CinematicEnvelopeOptions::default(),
        )
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
            .generate_cinematic_envelope_file(
                "/must-not-execute",
                &input,
                &target,
                &CinematicEnvelopeOptions {
                    focal_length: CinematicFocalLength::Assumed(focal),
                    aperture
                }
            )
            .is_err());
        assert!(!target.exists());
    }
    assert_eq!(fs::read(input).unwrap(), b"protected source bytes");
}

fn helper(root: &Path, behavior: &str) -> std::path::PathBuf {
    if !root.join("exiftool").exists() {
        let metadata = root.join("exiftool");
        fs::write(
            &metadata,
            "#!/bin/sh\nprintf '%s\\n' '[{\"ExifIFD:Main:FocalLengthIn35mmFormat\":24}]'\n",
        )
        .unwrap();
        fs::set_permissions(metadata, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let path = root.join("adapter.py");
    let script = format!(
        r#"#!/usr/bin/python3
import json, pathlib, sys
for line in sys.stdin:
    request = json.loads(line)
    pathlib.Path(request['output_path']).write_text(str(request['cinematic_envelope']['focal_length_35mm']))
    if '{behavior}' == 'source-change':
        pathlib.Path(request['input_path']).write_bytes(b'changed source')
    response = {{'schema_version': 2, 'cinematic_envelope': {{
        'processed_frames': 0 if '{behavior}' == 'zero-frames' else 1,
        'focal_length_35mm': 24 if '{behavior}' == 'wrong-camera' else request['cinematic_envelope']['focal_length_35mm'], 'aperture': request['cinematic_envelope']['aperture'],
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
            .generate_cinematic_envelope_file(
                adapter,
                &input,
                &output,
                &CinematicEnvelopeOptions::default(),
            )
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
        .generate_cinematic_envelope_file(
            "/must-not-execute",
            &input,
            &output,
            &CinematicEnvelopeOptions::default(),
        )
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

fn camera_estimate(root: &Path, input: &Path) -> std::path::PathBuf {
    use sha2::{Digest, Sha256};
    let hash: String = Sha256::digest(fs::read(input).unwrap())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let estimate = serde_json::json!({
        "schema_version": 1,
        "source_sha256": hash,
        "estimator": {"name": "test-camera", "revision": "b".repeat(40), "weights_sha256": "c".repeat(64)},
        "samples": (0..3).map(|i| serde_json::json!({
            "time_seconds": i,
            "intrinsics": {
                "image_size": [3600, 2400],
                "focal_length_pixels": [3500, 3500],
                "principal_point_pixels": [1800, 1200],
                "focal_uncertainty_pixels": 35
            }
        })).collect::<Vec<_>>()
    });
    let path = root.join("camera.json");
    fs::write(&path, serde_json::to_vec(&estimate).unwrap()).unwrap();
    path
}

fn camera_tools(root: &Path, input: &Path, metadata: &str) {
    let metadata_script = format!("#!/usr/bin/python3\nprint({metadata:?})\n");
    let path = root.join("exiftool");
    fs::write(&path, metadata_script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    let estimate = camera_estimate(root, input);
    let path = root.join("xdremux-geocalib");
    fs::write(&path, format!(
        "#!/usr/bin/python3\nimport pathlib,sys\npathlib.Path({:?}).write_text('called')\npathlib.Path(sys.argv[sys.argv.index('--output')+1]).write_bytes(pathlib.Path({:?}).read_bytes())\n",
        root.join("model-called"), estimate
    )).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
}

#[test]
fn automatic_cinematic_prefers_recorded_equivalent_and_only_estimates_when_missing() {
    for (metadata, expected, estimates) in [
        (
            r#"[{"ExifIFD:Main:FocalLengthIn35mmFormat":70}]"#,
            70.0,
            false,
        ),
        (
            r#"[{"VideoKeys:Main:FocalLengthIn35mmFormat":"70.00mm"}]"#,
            70.0,
            false,
        ),
        (r#"[{}]"#, 35.0, true),
        (r#"[{"ExifIFD:Main:FocalLength":8.5}]"#, 35.0, true),
        (
            r#"[{"ExifIFD:Doc1:FocalLengthIn35mmFormat":24}]"#,
            35.0,
            true,
        ),
        (
            r#"[{"ExifIFD:Main:FocalLengthIn35mmFormat":0}]"#,
            35.0,
            true,
        ),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let input = source(temp.path());
        camera_tools(temp.path(), &input, metadata);
        let output = temp.path().join("pending.mov");
        let receipt = PortableRuntime::new()
            .generate_cinematic_envelope_file(
                helper(temp.path(), "valid"),
                &input,
                &output,
                &CinematicEnvelopeOptions::default(),
            )
            .unwrap();
        assert_eq!(receipt.focal_length_35mm, expected, "{metadata}");
        assert_eq!(
            temp.path().join("model-called").exists(),
            estimates,
            "{metadata}"
        );
        assert_eq!(receipt.camera_estimate.is_some(), estimates);
        assert_eq!(
            receipt.focal_length_source.as_str(),
            if estimates {
                "geocalib"
            } else {
                "recorded-metadata"
            }
        );
        assert_eq!(fs::read(input).unwrap(), b"protected source bytes");
    }
}

#[test]
fn automatic_cinematic_fails_without_guessing_and_cleans_model_staging() {
    for scenario in [
        "conflict",
        "malformed-tag",
        "metadata-failure",
        "model-failure",
        "missing-model",
        "wrong-source",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let input = source(temp.path());
        let metadata = match scenario {
            "conflict" => {
                r#"[{"ExifIFD:Main:FocalLengthIn35mmFormat":70,"VideoKeys:Main:FocalLengthIn35mmFormat":35}]"#
            }
            "malformed-tag" => r#"[{"ExifIFD:Main:FocalLengthIn35mmFormat":"bad"}]"#,
            _ => "[{}]",
        };
        camera_tools(temp.path(), &input, metadata);
        let expected = match scenario {
            "conflict" => "disagree",
            "malformed-tag" => "invalid numeric",
            "metadata-failure" => {
                fs::write(temp.path().join("exiftool"), "#!/bin/sh\nexit 2\n").unwrap();
                "helper exit status"
            }
            "model-failure" => {
                fs::write(temp.path().join("xdremux-geocalib"), "#!/bin/sh\nexit 3\n").unwrap();
                "helper exit status"
            }
            "missing-model" => {
                // An existing but nonexecutable sibling prevents PATH fallback.
                fs::set_permissions(
                    temp.path().join("xdremux-geocalib"),
                    fs::Permissions::from_mode(0o600),
                )
                .unwrap();
                "Cinematic GeoCalib"
            }
            _ => {
                fs::write(&input, b"different source").unwrap();
                "SHA-256 does not match"
            }
        };
        let output = temp.path().join("pending.mov");
        let error = PortableRuntime::new()
            .generate_cinematic_envelope_file(
                helper(temp.path(), "valid"),
                &input,
                &output,
                &CinematicEnvelopeOptions::default(),
            )
            .unwrap_err();
        assert!(error.to_string().contains(expected), "{scenario}: {error}");
        assert!(!output.exists());
        assert_eq!(
            fs::read(&input).unwrap(),
            if scenario == "wrong-source" {
                b"different source".as_slice()
            } else {
                b"protected source bytes".as_slice()
            }
        );
        assert!(!fs::read_dir(temp.path()).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".xdremux-cinematic-")));
        if matches!(scenario, "conflict" | "malformed-tag" | "metadata-failure") {
            assert!(!temp.path().join("model-called").exists());
        }
    }
}

#[test]
fn explicit_cinematic_focal_bypasses_both_automatic_tools() {
    let temp = tempfile::tempdir().unwrap();
    let input = source(temp.path());
    let adapter = helper(temp.path(), "valid");
    fs::write(temp.path().join("exiftool"), "#!/bin/sh\nexit 2\n").unwrap();
    let receipt = PortableRuntime::new()
        .generate_cinematic_envelope_file(
            adapter,
            &input,
            temp.path().join("pending.mov"),
            &CinematicEnvelopeOptions {
                focal_length: CinematicFocalLength::Assumed(70.0),
                aperture: 2.8,
            },
        )
        .unwrap();
    assert_eq!(receipt.focal_length_35mm, 70.0);
    assert_eq!(receipt.focal_length_source.as_str(), "explicit");
}

#[test]
fn cinematic_batch_resolves_each_camera_independently_in_serial_and_parallel() {
    use xdremux_engine::{AppleFeatureRequest, ConversionRequest};
    use xdremux_runtime::{BatchExecutionOptions, BatchItem};
    for jobs in [1, 2] {
        let temp = tempfile::tempdir().unwrap();
        let estimated = source(temp.path());
        let recorded = temp.path().join("recorded.mov");
        fs::write(&recorded, b"a distinct camera source").unwrap();
        camera_tools(temp.path(), &estimated, "[{}]");
        fs::write(temp.path().join("exiftool"), r#"#!/usr/bin/python3
import json,sys
print(json.dumps([{'VideoKeys:Main:FocalLengthIn35mmFormat':70}] if sys.argv[-1].endswith('/recorded.mov') else [{}]))
"#).unwrap();
        let estimated_output = temp.path().join("estimated-pending.mov");
        let recorded_output = temp.path().join("recorded-pending.mov");
        let receipt = PortableRuntime::new().convert_batch_with_options(
            vec![
                BatchItem::new(&estimated, &estimated_output),
                BatchItem::new(&recorded, &recorded_output),
            ],
            ConversionRequest {
                apple_features: AppleFeatureRequest {
                    cinematic: true,
                    ..AppleFeatureRequest::default()
                },
                ..ConversionRequest::default()
            },
            &BatchExecutionOptions {
                jobs,
                apple_adapter_executable: Some(helper(temp.path(), "valid")),
                ..BatchExecutionOptions::default()
            },
        );
        assert!(receipt.failures.is_empty(), "{:?}", receipt.failures);
        assert_eq!(receipt.successes.len(), 2);
        assert_eq!(fs::read_to_string(estimated_output).unwrap(), "35.0");
        assert_eq!(fs::read_to_string(recorded_output).unwrap(), "70.0");
        assert!(temp.path().join("model-called").exists());
        assert_eq!(fs::read(estimated).unwrap(), b"protected source bytes");
        assert_eq!(fs::read(recorded).unwrap(), b"a distinct camera source");
    }
}

#[test]
fn cinematic_estimate_reaches_adapter_and_keeps_source_bound() {
    let temp = tempfile::tempdir().unwrap();
    let input = source(temp.path());
    let estimate = camera_estimate(temp.path(), &input);
    let options = CinematicEnvelopeOptions {
        focal_length: CinematicFocalLength::CameraEstimate(estimate),
        aperture: 4.0,
    };
    let output = temp.path().join("pending.mov");
    let adapter = helper(temp.path(), "valid");
    let receipt = PortableRuntime::new()
        .generate_cinematic_envelope_file(&adapter, &input, &output, &options)
        .unwrap();
    assert_eq!(receipt.focal_length_35mm, 35.0);
    assert_eq!(receipt.aperture, 4.0);
    assert_eq!(receipt.camera_estimate.unwrap().sample_count, 3);
    assert_eq!(fs::read(&input).unwrap(), b"protected source bytes");

    fs::write(&input, b"different video").unwrap();
    let wrong = temp.path().join("wrong-source.mov");
    let error = PortableRuntime::new()
        .generate_cinematic_envelope_file("/must-not-execute", &input, &wrong, &options)
        .unwrap_err();
    assert!(error.to_string().contains("SHA-256 does not match"));
    assert!(!wrong.exists());
}

#[test]
fn cinematic_rejects_unsafe_estimates_before_native_effects() {
    for scenario in [
        "uncertain",
        "changing",
        "malformed",
        "oversized",
        "wrong-camera",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let input = source(temp.path());
        let estimate = camera_estimate(temp.path(), &input);
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&estimate).unwrap()).unwrap();
        let expected = match scenario {
            "uncertain" => {
                value["samples"][0]["intrinsics"]["focal_uncertainty_pixels"] =
                    serde_json::json!(701);
                "uncertainty"
            }
            "changing" => {
                value["samples"][2]["intrinsics"]["focal_length_pixels"] =
                    serde_json::json!([7000, 7000]);
                "disagree"
            }
            "wrong-camera" => "do not match",
            _ => "camera estimate",
        };
        fs::write(
            &estimate,
            match scenario {
                "malformed" => b"invalid JSON".to_vec(),
                "oversized" => vec![b' '; 1_048_577],
                _ => serde_json::to_vec(&value).unwrap(),
            },
        )
        .unwrap();
        let output = temp.path().join("pending.mov");
        let adapter = if scenario == "wrong-camera" {
            helper(temp.path(), scenario)
        } else {
            temp.path().join("must-not-execute")
        };
        let error = PortableRuntime::new()
            .generate_cinematic_envelope_file(
                &adapter,
                &input,
                &output,
                &CinematicEnvelopeOptions {
                    focal_length: CinematicFocalLength::CameraEstimate(estimate),
                    aperture: 2.8,
                },
            )
            .unwrap_err();
        assert!(error.to_string().contains(expected), "{scenario}: {error}");
        assert!(!output.exists());
        assert_eq!(fs::read(&input).unwrap(), b"protected source bytes");
        assert!(!fs::read_dir(temp.path()).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".xdremux-cinematic-")));
    }
}
