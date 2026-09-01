use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
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

    let static_output = output
        .join("静态照片")
        .join("未分类")
        .join("static.xdremux.heic");
    let live_image = output
        .join("实况照片")
        .join("未分类")
        .join("live.xdremux.heic");
    let live_video = live_image.with_extension("mov");
    assert!(static_output.is_file(), "{}", static_output.display());
    assert!(live_image.is_file(), "{}", live_image.display());
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
        .any(|path| path.ends_with("静态照片/未分类/static.xdremux.heic") || path.ends_with("静态照片\\未分类\\static.xdremux.heic")));
    assert!(output_strings
        .iter()
        .any(|path| path.ends_with("实况照片/未分类/live.xdremux.heic") || path.ends_with("实况照片\\未分类\\live.xdremux.heic")));

    fs::remove_dir_all(root).unwrap();
}
