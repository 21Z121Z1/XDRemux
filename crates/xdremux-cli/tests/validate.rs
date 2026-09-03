use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use xdremux_engine::ConversionRequest;
use xdremux_runtime::PortableRuntime;

fn proxdr_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/proxdr/oppo/find-x6-pro/lhdr-v1-01.heic")
}

fn motion_photo_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/motion-photo/samsung/jpeg-ultrahdr-01.jpg")
}

fn unique_dir() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "xdremux-cli-validate-{}-{stamp}",
        std::process::id()
    ))
}

fn validate_json(input: &Path) -> (u8, Value, Vec<u8>) {
    let arguments = vec![
        OsString::from("validate"),
        input.as_os_str().to_owned(),
        OsString::from("--json"),
    ];
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = xdremux_cli::run_from(arguments, &mut stdout, &mut stderr);
    let value =
        serde_json::from_slice(&stdout).expect("validate --json must emit one JSON document");
    (code, value, stderr)
}

#[test]
fn validate_reports_iso_hdr_live_photo_and_failures_as_stable_json() {
    let root = unique_dir();
    fs::create_dir_all(&root).unwrap();
    let runtime = PortableRuntime::new();

    let source = fs::read(proxdr_fixture()).unwrap();
    let iso_output = root.join("iso.heic");
    runtime
        .convert_proxdr_file(&source, &iso_output, ConversionRequest::default(), |_| {})
        .unwrap();
    let (code, report, stderr) = validate_json(&iso_output);
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["command"], "validate");
    assert_eq!(report["valid"], true);
    assert_eq!(report["kind"], "iso-hdr-heif");
    assert!(report["details"]["value"]["tile_item_ids"]
        .as_array()
        .is_some_and(|tiles| !tiles.is_empty()));

    let motion_source = fs::read(motion_photo_fixture()).unwrap();
    let live_still = root.join("live.heic");
    let live = runtime
        .convert_motion_photo_file(&motion_source, motion_photo_fixture(), &live_still)
        .unwrap();
    let (code, report, stderr) = validate_json(&live.image);
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert_eq!(report["valid"], true);
    assert_eq!(report["kind"], "live-photo");
    assert_eq!(
        report["details"]["value"]["content_identifier"],
        live.content_identifier
    );

    let (code, movie_report, stderr) = validate_json(&live.video);
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert_eq!(movie_report["kind"], "live-photo");
    assert_eq!(
        movie_report["details"]["value"]["content_identifier"],
        live.content_identifier
    );

    let invalid = root.join("invalid.heic");
    fs::write(&invalid, [0_u8; 32]).unwrap();
    let (code, report, stderr) = validate_json(&invalid);
    assert_eq!(code, 1);
    assert!(stderr.is_empty());
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["command"], "validate");
    assert_eq!(report["valid"], false);
    assert!(report["error"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));

    fs::remove_dir_all(root).unwrap();
}
