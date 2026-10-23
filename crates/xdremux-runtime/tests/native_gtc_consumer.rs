#![cfg(target_os = "macos")]

//! Opt-in native verification of the Mac Styles key-3 consumer.
//!
//! Supply `XDREMUX_STYLES_TEST_INPUT` and
//! `XDREMUX_APPLE_ADAPTER_TEST_EXECUTABLE` to run against an original Styles
//! HEIF. `XDREMUX_STYLES_TEST_GENERATED_INPUT` optionally checks a final
//! `--apple-styles` artifact as well. Private media remains outside Git.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use serde_json::json;
use sha2::{Digest, Sha256};
use xdremux_engine::apple_style_monotonic_global_tone_curve;
use xdremux_heif::{extract_style_reconstruction_resources, StyleReconstructionResources};

fn checked(command: &mut Command) -> Output {
    let output = command.output().expect("run native test dependency");
    assert!(
        output.status.success(),
        "{command:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn with_stdin(command: &mut Command, input: &[u8]) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start native test dependency");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input)
        .expect("write native test request");
    let output = child.wait_with_output().expect("wait for native consumer");
    assert!(
        output.status.success(),
        "{command:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn metadata_curve(metadata: &Path) -> Vec<u8> {
    let encoded = checked(Command::new("/usr/bin/plutil").args([
        "-extract",
        "3",
        "raw",
        "-o",
        "-",
        metadata.to_str().unwrap(),
    ]));
    with_stdin(Command::new("/usr/bin/base64").arg("-D"), &encoded.stdout).stdout
}

fn replace_curve(metadata: &Path, curve: &[u8], directory: &Path) {
    let path = directory.join("controlled-curve.bin");
    fs::write(&path, curve).unwrap();
    let encoded = checked(Command::new("/usr/bin/base64").arg("-i").arg(path));
    checked(
        Command::new("/usr/bin/plutil")
            .args(["-replace", "3", "-data"])
            .arg(String::from_utf8(encoded.stdout).unwrap().trim())
            .arg(metadata),
    );
    assert_eq!(metadata_curve(metadata), curve);
}

fn native_curve(
    adapter: &Path,
    source: &Path,
    resources: &StyleReconstructionResources,
    curve: Option<&[u8]>,
    directory: &Path,
) -> (Vec<u8>, Vec<f32>) {
    fs::create_dir(directory).unwrap();
    let metadata = directory.join("styles.plist");
    fs::write(&metadata, &resources.metadata).unwrap();
    fs::write(
        directory.join("linear.heic"),
        &resources.linear_thumbnail_heif,
    )
    .unwrap();
    fs::write(directory.join("delta.heic"), &resources.delta_map_heif).unwrap();
    if let Some(curve) = curve {
        replace_curve(&metadata, curve, directory);
    }
    let wire = metadata_curve(&metadata);
    let response = with_stdin(
        &mut Command::new(adapter),
        &serde_json::to_vec(&json!({
            "schema_version": 2,
            "operation": "styles-decode-reverse",
            "input_path": source,
            "output_path": directory,
            "max_edge": 256,
            "supported_metadata_versions": [15, 16]
        }))
        .unwrap(),
    );
    let facts: serde_json::Value = serde_json::from_slice(&response.stdout).unwrap();
    assert_eq!(facts["schema_version"], 2);
    assert_eq!(facts["globalToneCurveData"], 256 * 4);
    let bytes = fs::read(directory.join("globalToneCurveData.bin")).unwrap();
    assert_eq!(bytes.len(), 256 * 4);
    let values = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|value| f32::from_le_bytes(*value))
        .collect::<Vec<_>>();
    assert!(values.iter().all(|value| value.is_finite()));
    (wire, values)
}

// The reference is the independently measured Mac wire contract. It deliberately
// retains a sub-white endpoint, uses 65535, and performs no half quantization.
fn expected_nodes(wire: &[u8]) -> Vec<f32> {
    assert_eq!(wire.len(), 516);
    assert_eq!(u16::from_le_bytes([wire[0], wire[1]]), 257);
    wire[2..514]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|word| {
            let code = f64::from(u16::from_le_bytes(*word)) / 65_535.0;
            if code <= 0.04045 {
                (code / 12.92) as f32
            } else {
                ((code + 0.055) / 1.055).powf(2.4) as f32
            }
        })
        .collect()
}

fn assert_matches(label: &str, actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), 256);
    assert_eq!(actual.len(), expected.len());
    let max_error = actual
        .iter()
        .zip(expected)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f32, f32::max);
    assert!(
        max_error <= 3e-7,
        "{label}: native GTC nodes differ by {max_error}; endpoint={}",
        actual[255]
    );
    println!(
        "{label}: 256 finite native nodes; max absolute error {max_error:.9}; endpoint {:.9}",
        actual[255]
    );
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn native_mac_gtc_matches_emitted_nodes_without_endpoint_normalization() {
    let Some(source) = std::env::var_os("XDREMUX_STYLES_TEST_INPUT") else {
        return;
    };
    let source = fs::canonicalize(source).unwrap();
    let adapter = fs::canonicalize(
        std::env::var_os("XDREMUX_APPLE_ADAPTER_TEST_EXECUTABLE")
            .expect("native GTC check requires the Apple adapter"),
    )
    .unwrap();
    let original = fs::read(&source).unwrap();
    let resources = extract_style_reconstruction_resources(&original).unwrap();
    let temporary = tempfile::tempdir().unwrap();
    println!(
        "Native GTC source SHA256={}; adapter SHA256={}",
        sha256(&original),
        sha256(&fs::read(&adapter).unwrap())
    );

    let (wire, native) = native_curve(
        &adapter,
        &source,
        &resources,
        None,
        &temporary.path().join("original"),
    );
    assert_matches("original", &native, &expected_nodes(&wire));

    let mut constant = 257_u16.to_le_bytes().to_vec();
    for _ in 0..257 {
        constant.extend_from_slice(&32_768_u16.to_le_bytes());
    }
    let (wire, native) = native_curve(
        &adapter,
        &source,
        &resources,
        Some(&constant),
        &temporary.path().join("constant-half-code"),
    );
    assert_matches("constant-half-code", &native, &expected_nodes(&wire));
    assert!(native.iter().all(|value| (value - 0.2140482).abs() < 3e-7));

    let thumbnail = (0..256)
        .flat_map(|index| [index as f32 / 255.0; 3])
        .collect::<Vec<_>>();
    let primary = thumbnail
        .iter()
        .map(|code| 0.02 + 0.6 * code.powf(1.8))
        .collect::<Vec<_>>();
    let generated = apple_style_monotonic_global_tone_curve(&thumbnail, &primary).unwrap();
    let (wire, native) = native_curve(
        &adapter,
        &source,
        &resources,
        Some(&generated.data),
        &temporary.path().join("generated-sub-white-ramp"),
    );
    assert_matches("generated wire", &native, &expected_nodes(&wire));
    assert_matches("generator samples", &native, &generated.linear_samples);
    assert!(
        native[255] < 0.65,
        "the Mac consumer must retain the endpoint"
    );

    let mut changed_tail = generated.data.clone();
    changed_tail[514..516].copy_from_slice(&0_u16.to_le_bytes());
    assert_ne!(&changed_tail[514..516], &generated.data[514..516]);
    let (_, native_changed_tail) = native_curve(
        &adapter,
        &source,
        &resources,
        Some(&changed_tail),
        &temporary.path().join("ignored-tail"),
    );
    assert_eq!(native_changed_tail, native, "word 257 is not a Mac node");
    println!("word 257 change: native nodes remain bit-identical");

    if let Some(path) = std::env::var_os("XDREMUX_STYLES_TEST_GENERATED_INPUT") {
        let path = fs::canonicalize(path).unwrap();
        let before = fs::read(&path).unwrap();
        let resources = extract_style_reconstruction_resources(&before).unwrap();
        let (wire, native) = native_curve(
            &adapter,
            &path,
            &resources,
            None,
            &temporary.path().join("published-artifact"),
        );
        assert_matches("published artifact", &native, &expected_nodes(&wire));
        assert_eq!(fs::read(path).unwrap(), before, "preserve generated input");
    }
    assert_eq!(
        fs::read(source).unwrap(),
        original,
        "preserve original input"
    );
}
