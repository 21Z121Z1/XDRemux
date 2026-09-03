use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

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
        "xdremux-cli-batch-categorize-{}-{stamp}",
        std::process::id()
    ))
}

fn projected_file(root: &Path, asset_folder: &str, file_name: &str) -> PathBuf {
    let asset_root = root.join(asset_folder);
    let mut matches = Vec::new();
    for entry in fs::read_dir(&asset_root).expect("asset-type directory must exist") {
        let entry = entry.unwrap();
        if !entry.file_type().unwrap().is_dir() {
            continue;
        }
        let candidate = entry.path().join(file_name);
        if candidate.is_file() {
            matches.push(candidate);
        }
    }
    assert_eq!(
        matches.len(),
        1,
        "expected exactly one {file_name} below {}",
        asset_root.display()
    );
    matches.pop().unwrap()
}

#[test]
fn batch_categorize_projects_final_outputs_before_conversion() {
    let root = unique_dir();
    let input = root.join("input");
    let output = root.join("output");
    fs::create_dir_all(&input).unwrap();
    fs::copy(proxdr_fixture(), input.join("static.heic")).unwrap();
    fs::copy(motion_photo_fixture(), input.join("live.jpg")).unwrap();

    let arguments = vec![
        OsString::from("batch"),
        OsString::from("--input-dir"),
        input.as_os_str().to_owned(),
        OsString::from("--output-dir"),
        output.as_os_str().to_owned(),
        OsString::from("--categorize"),
        OsString::from("--jobs"),
        OsString::from("2"),
        OsString::from("--json"),
    ];
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = xdremux_cli::run_from(arguments, &mut stdout, &mut stderr);
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());

    let report: Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(report["processed"], 2);
    assert_eq!(report["succeeded"], 2);
    assert_eq!(report["failed"], 0);

    let static_output = projected_file(&output, "静态照片", "static.xdremux.heic");
    let live_image = projected_file(&output, "实况照片", "live.xdremux.heic");
    let live_video = live_image.with_extension("mov");
    assert!(live_video.is_file(), "{}", live_video.display());
    assert!(!output.join("static.xdremux.heic").exists());
    assert!(!output.join("live.xdremux.heic").exists());

    let successes = report["successes"].as_array().unwrap();
    let output_strings = successes
        .iter()
        .flat_map(|success| success["outputs"].as_array().into_iter().flatten())
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(output_strings
        .iter()
        .any(|path| path == &static_output.to_string_lossy()));
    assert!(output_strings
        .iter()
        .any(|path| path == &live_image.to_string_lossy()));

    fs::remove_dir_all(root).unwrap();
}
