use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static VALIDATE_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

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
    let seq = VALIDATE_DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "xdremux-cli-validate-{}-{stamp}-{seq}",
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
fn explicit_hdr_profile_runs_through_convert_and_final_file_validation() {
    let root = unique_dir();
    fs::create_dir_all(&root).unwrap();
    let input = proxdr_fixture();
    let original = fs::read(&input).unwrap();
    let output = root.join("hlg.heic");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = xdremux_cli::run_from(
        vec![
            OsString::from("convert"),
            "--input".into(),
            input.as_os_str().to_owned(),
            "--output".into(),
            output.as_os_str().to_owned(),
            "--hdr-profile".into(),
            "bt2020-hlg".into(),
        ],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    let bytes = fs::read(&output).unwrap();
    stdout.clear();
    stderr.clear();
    let code = xdremux_cli::run_from(
        vec![
            OsString::from("validate"),
            output.as_os_str().to_owned(),
            "--hdr-profile".into(),
            "bt2020-hlg".into(),
            "--decode-hdr".into(),
            "--json".into(),
        ],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    let report: Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(report["valid"], true);
    let semantics = &report["details"]["value"]["semantics"];
    assert_eq!(semantics["hdr_profile"], "bt2020-hlg");
    assert_eq!(semantics["output_nclx"], serde_json::json!([9, 18, 9]));
    assert_eq!(
        semantics["static_metadata"]["dynamic_metadata"],
        "not-present"
    );
    assert_eq!(semantics["decoded_hdr"]["color_space"], "linear-bt2020");
    stdout.clear();
    stderr.clear();
    let code = xdremux_cli::run_from(
        vec![
            OsString::from("validate"),
            output.as_os_str().to_owned(),
            "--hdr-profile".into(),
            "bt2020-pq".into(),
            "--json".into(),
        ],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(code, 1);
    let failure: Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(failure["valid"], false);
    assert_eq!(fs::read(&output).unwrap(), bytes);
    assert_eq!(fs::read(&input).unwrap(), original);
    if let Some(evidence) = std::env::var_os("XDREMUX_HDR_PROFILE_OUTPUT_DIR") {
        let evidence = PathBuf::from(evidence);
        fs::create_dir_all(&evidence).unwrap();
        fs::write(evidence.join("cli-hlg.heic"), &bytes).unwrap();
        let geometry = xdremux_heif::primary_coded_geometry(&bytes).unwrap();
        let gain = xdremux_heif::validate_gain_map_structure(&bytes).unwrap();
        let (dw, dh) = if geometry.quarter_turns_ccw % 2 == 1 {
            (geometry.height, geometry.width)
        } else {
            (geometry.width, geometry.height)
        };
        let (gw, gh) = if geometry.quarter_turns_ccw % 2 == 1 {
            (gain.height, gain.width)
        } else {
            (gain.width, gain.height)
        };
        fs::write(evidence.join("cli-hlg-geometry.json"),serde_json::to_vec_pretty(&serde_json::json!({"primary_coded_width":geometry.width,"primary_coded_height":geometry.height,"primary_display_width":dw,"primary_display_height":dh,"quarter_turns_ccw":geometry.quarter_turns_ccw,"gain_coded_width":gain.width,"gain_coded_height":gain.height,"gain_display_width":gw,"gain_display_height":gh})).unwrap()).unwrap();
    }
    fs::remove_dir_all(root).unwrap();
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
