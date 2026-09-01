use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

fn unique_dir() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "xdremux-cli-categorize-it-{}-{stamp}",
        std::process::id()
    ))
}

fn path_arg(path: &Path) -> OsString {
    path.as_os_str().to_owned()
}

#[test]
fn categorize_command_copies_then_reports_duplicate_on_rerun() {
    let root = unique_dir();
    let input_dir = root.join("input");
    let output_dir = root.join("output");
    fs::create_dir_all(&input_dir).unwrap();
    let input = input_dir.join("portrait.heic");
    fs::write(&input, b"synthetic metadata Oplus_16 payload").unwrap();

    let arguments = vec![
        OsString::from("categorize"),
        OsString::from("--input"),
        path_arg(&input_dir),
        OsString::from("--output-dir"),
        path_arg(&output_dir),
        OsString::from("--json"),
    ];
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = xdremux_cli::run_from(arguments.clone(), &mut stdout, &mut stderr);
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let value: Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["command"], "categorize");
    assert_eq!(value["copied"], 1);
    assert_eq!(value["duplicates"], 0);
    assert_eq!(
        value["items"][0]["classification"]["primary_capture_mode"],
        "portrait"
    );
    let destination = output_dir.join("静态照片").join("人像").join("portrait.heic");
    assert_eq!(fs::read(&destination).unwrap(), fs::read(&input).unwrap());

    stdout.clear();
    stderr.clear();
    let code = xdremux_cli::run_from(arguments, &mut stdout, &mut stderr);
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    let rerun: Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(rerun["copied"], 0);
    assert_eq!(rerun["duplicates"], 1);
    assert_eq!(fs::read(&destination).unwrap(), fs::read(&input).unwrap());

    fs::remove_dir_all(root).unwrap();
}