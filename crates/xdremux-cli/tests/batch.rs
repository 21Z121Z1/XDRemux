use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use xdremux_heif::validate_gain_map_structure;

fn public_proxdr_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/proxdr/oppo/find-x6-pro/lhdr-v1-01.heic")
}

fn unique_dir() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("xdremux-cli-batch-{}-{stamp}", std::process::id()))
}

fn path_arg(path: &Path) -> OsString {
    path.as_os_str().to_owned()
}

#[test]
fn batch_command_is_deterministic_machine_readable_and_failure_isolating() {
    let work = unique_dir();
    let inputs = work.join("inputs");
    let outputs = work.join("outputs");
    fs::create_dir_all(&inputs).expect("input directory must be created");

    let good = inputs.join("good.heic");
    let bad = inputs.join("bad.heic");
    fs::copy(public_proxdr_fixture(), &good).expect("fixture must be copied");
    fs::write(&bad, [0_u8; 32]).expect("invalid fixture must be written");

    let args = vec![
        OsString::from("batch"),
        OsString::from("--input-dir"),
        path_arg(&inputs),
        OsString::from("--output-dir"),
        path_arg(&outputs),
        OsString::from("--json"),
    ];
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = xdremux_cli::run_from(args, &mut stdout, &mut stderr);

    assert_eq!(
        code, 1,
        "one bad asset should make the batch partially fail without aborting the good conversion"
    );
    assert!(
        stderr.is_empty(),
        "machine-readable runtime failures belong in the JSON receipt: {}",
        String::from_utf8_lossy(&stderr)
    );

    let receipt: Value =
        serde_json::from_slice(&stdout).expect("batch --json must emit one valid JSON document");
    assert_eq!(receipt["schema_version"], 1);
    assert_eq!(receipt["command"], "batch");
    assert_eq!(receipt["processed"], 2);
    assert_eq!(receipt["succeeded"], 1);
    assert_eq!(receipt["failed"], 1);

    let good_output = outputs.join("good.xdremux.heic");
    let bad_output = outputs.join("bad.xdremux.heic");
    assert!(good_output.is_file());
    assert!(!bad_output.exists());

    let converted = fs::read(&good_output).expect("successful batch output must exist");
    validate_gain_map_structure(&converted)
        .expect("successful batch output must be a valid ISO Gain Map HEIF");

    fs::remove_dir_all(work).expect("temporary test directory must be removed");
}
