use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

fn motion_photo_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/motion-photo/samsung/jpeg-ultrahdr-01.jpg")
}

fn unique_dir() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("xdremux-cli-jobs-{}-{stamp}", std::process::id()))
}

fn run_batch(input: &Path, output: &Path) -> (u8, Value, Vec<u8>) {
    let args = vec![
        OsString::from("batch"),
        OsString::from("--input-dir"),
        input.as_os_str().to_owned(),
        OsString::from("--output-dir"),
        output.as_os_str().to_owned(),
        OsString::from("--jobs"),
        OsString::from("2"),
        OsString::from("--resume"),
        OsString::from("--json"),
    ];
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = xdremux_cli::run_from(args, &mut stdout, &mut stderr);
    let value = serde_json::from_slice(&stdout).expect("batch --json must emit JSON");
    (code, value, stderr)
}

#[test]
fn bounded_parallel_batch_keeps_receipts_ordered_and_checkpoint_reusable() {
    let root = unique_dir();
    let input = root.join("input");
    let output = root.join("output");
    fs::create_dir_all(&input).unwrap();
    let fixture = motion_photo_fixture();
    fs::copy(&fixture, input.join("a.jpg")).unwrap();
    fs::copy(&fixture, input.join("b.jpg")).unwrap();

    let (code, first, stderr) = run_batch(&input, &output);
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert_eq!(first["processed"], 2);
    assert_eq!(first["succeeded"], 2);
    assert_eq!(first["skipped_existing"], 0);
    assert_eq!(first["failed"], 0);
    let first_successes = first["successes"].as_array().unwrap();
    assert!(first_successes[0]["input"]
        .as_str()
        .unwrap()
        .ends_with("a.jpg"));
    assert!(first_successes[1]["input"]
        .as_str()
        .unwrap()
        .ends_with("b.jpg"));

    let a_heic = output.join("a.xdremux.heic");
    let a_mov = output.join("a.xdremux.mov");
    let b_heic = output.join("b.xdremux.heic");
    let b_mov = output.join("b.xdremux.mov");
    let before = [
        fs::read(&a_heic).unwrap(),
        fs::read(&a_mov).unwrap(),
        fs::read(&b_heic).unwrap(),
        fs::read(&b_mov).unwrap(),
    ];

    let (code, resumed, stderr) = run_batch(&input, &output);
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    assert_eq!(resumed["processed"], 2);
    assert_eq!(resumed["succeeded"], 2);
    assert_eq!(resumed["skipped_existing"], 2);
    assert_eq!(resumed["failed"], 0);
    let resumed_successes = resumed["successes"].as_array().unwrap();
    assert!(resumed_successes[0]["input"]
        .as_str()
        .unwrap()
        .ends_with("a.jpg"));
    assert!(resumed_successes[1]["input"]
        .as_str()
        .unwrap()
        .ends_with("b.jpg"));

    let after = [
        fs::read(&a_heic).unwrap(),
        fs::read(&a_mov).unwrap(),
        fs::read(&b_heic).unwrap(),
        fs::read(&b_mov).unwrap(),
    ];
    assert_eq!(before, after, "resume must not rewrite proven outputs");

    let checkpoint = output.join(".xdremux-motion-photo-checkpoint.jsonl");
    let state = fs::read_to_string(checkpoint).unwrap();
    assert!(
        state.lines().count() >= 5,
        "header + two success + two skip records expected"
    );

    fs::remove_dir_all(root).unwrap();
}
