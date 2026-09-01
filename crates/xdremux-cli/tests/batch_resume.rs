use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use xdremux_motion_photo::{read_apple_content_identifier, read_live_photo_content_identifier};
use xdremux_runtime::DEFAULT_MOTION_PHOTO_CHECKPOINT_NAME;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/motion-photo/samsung/jpeg-ultrahdr-01.jpg")
}

fn unique_dir() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "xdremux-cli-batch-resume-{}-{stamp}",
        std::process::id()
    ))
}

fn path_arg(path: &Path) -> OsString {
    path.as_os_str().to_owned()
}

fn run(output: &Path, resume: bool) -> Value {
    let input = fixture();
    let mut arguments = vec![
        OsString::from("batch"),
        OsString::from("--input"),
        path_arg(&input),
        OsString::from("--output-dir"),
        path_arg(output),
        OsString::from("--json"),
    ];
    if resume {
        arguments.push(OsString::from("--resume"));
    }
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = xdremux_cli::run_from(arguments, &mut stdout, &mut stderr);
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    serde_json::from_slice(&stdout).expect("batch --json must emit one JSON document")
}

#[test]
fn resume_reuses_only_the_checkpoint_proven_live_photo_pair() {
    let root = unique_dir();
    let output = root.join("output");

    let first = run(&output, false);
    assert_eq!(first["processed"], 1);
    assert_eq!(first["succeeded"], 1);
    assert_eq!(first["skipped_existing"], 0);
    assert_eq!(first["failed"], 0);
    assert_eq!(first["successes"][0]["kind"], "live-photo");
    assert_eq!(first["successes"][0]["status"], "converted");

    let image = PathBuf::from(first["successes"][0]["outputs"][0].as_str().unwrap());
    let video = PathBuf::from(first["successes"][0]["outputs"][1].as_str().unwrap());
    let before_image = fs::read(&image).unwrap();
    let before_video = fs::read(&video).unwrap();
    let image_id = read_apple_content_identifier(&before_image)
        .unwrap()
        .expect("converted HEIC must have a ContentIdentifier");
    let video_id = read_live_photo_content_identifier(&before_video)
        .unwrap()
        .expect("converted MOV must have a ContentIdentifier");
    assert_eq!(image_id, video_id);

    let checkpoint = output.join(DEFAULT_MOTION_PHOTO_CHECKPOINT_NAME);
    let checkpoint_text = fs::read_to_string(&checkpoint).unwrap();
    let mut lines = checkpoint_text.lines();
    let header: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
    assert_eq!(header["kind"], "header");
    assert_eq!(header["schemaVersion"], 2);
    let item: Value = serde_json::from_str(lines.last().unwrap()).unwrap();
    assert_eq!(item["status"], "success");
    assert_eq!(item["assetIdentifier"], image_id);
    assert!(item["inputSHA256"].as_str().is_some_and(|value| value.len() == 64));

    let second = run(&output, true);
    assert_eq!(second["processed"], 1);
    assert_eq!(second["succeeded"], 1);
    assert_eq!(second["skipped_existing"], 1);
    assert_eq!(second["failed"], 0);
    assert_eq!(second["successes"][0]["status"], "skipped-existing");
    assert_eq!(fs::read(&image).unwrap(), before_image);
    assert_eq!(fs::read(&video).unwrap(), before_video);

    let checkpoint_text = fs::read_to_string(&checkpoint).unwrap();
    let last: Value = serde_json::from_str(checkpoint_text.lines().last().unwrap()).unwrap();
    assert_eq!(last["status"], "skipped_existing");
    assert_eq!(last["assetIdentifier"], image_id);

    fs::remove_dir_all(root).unwrap();
}