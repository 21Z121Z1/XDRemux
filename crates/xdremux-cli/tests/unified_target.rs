use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use xdremux_motion_photo::{Input, MotionPhoto, ParseOptions, TargetProfile, WritePolicy};

fn fixture(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/motion-photo")
        .join(relative)
}

fn run(args: Vec<OsString>) -> (u8, String, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = xdremux_cli::run_from(args, &mut stdout, &mut stderr);
    (
        status,
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    )
}

fn convert(input: &Path, output: &Path, target: &str) -> (u8, String, String) {
    run(vec![
        "convert".into(),
        "--input".into(),
        input.as_os_str().to_owned(),
        "--output".into(),
        output.as_os_str().to_owned(),
        "--target".into(),
        target.into(),
    ])
}

fn clean_samsung_heic(output: &Path) {
    let original = fs::read(fixture("samsung/heif-ultrahdr-01.heic")).unwrap();
    let photo =
        MotionPhoto::parse(Input::SingleFile(&original), ParseOptions::compatible()).unwrap();
    let clean = MotionPhoto::parse(
        Input::Parts {
            still: photo.still_bytes().unwrap(),
            movie: photo.motion_video_bytes().unwrap(),
            presentation: photo.presentation_time(),
        },
        ParseOptions::compatible(),
    )
    .unwrap();
    let composed = clean
        .plan(TargetProfile::AndroidHeif, WritePolicy::default())
        .execute()
        .unwrap();
    fs::write(output, &composed.files[0].bytes).unwrap();
}

fn assert_preserved_motion(source: &Path, output: &Path) {
    let source_bytes = fs::read(source).unwrap();
    let output_bytes = fs::read(output).unwrap();
    let before =
        MotionPhoto::parse(Input::SingleFile(&source_bytes), ParseOptions::compatible()).unwrap();
    let after =
        MotionPhoto::parse(Input::SingleFile(&output_bytes), ParseOptions::strict()).unwrap();
    after.validate().unwrap();
    assert_eq!(
        before.motion_video_bytes().unwrap(),
        after.motion_video_bytes().unwrap()
    );
    assert_eq!(before.presentation_time(), after.presentation_time());
}

#[test]
fn google_target_chooses_original_still_container_and_preserves_video() {
    let dir = tempfile::tempdir().unwrap();
    let jpeg = fixture("vivo/android-v1-sdr-01.jpg");
    let output_jpeg = dir.path().join("vivoMP.jpg");
    let (status, _, stderr) = convert(&jpeg, &output_jpeg, "google");
    assert_eq!(status, 0, "{stderr}");
    assert_preserved_motion(&jpeg, &output_jpeg);

    let heic = dir.path().join("samsung.heic");
    clean_samsung_heic(&heic);
    let output_heic = dir.path().join("samsungMP.heic");
    let (status, _, stderr) = convert(&heic, &output_heic, "google");
    assert_eq!(status, 0, "{stderr}");
    assert_preserved_motion(&heic, &output_heic);
}

#[test]
fn apple_and_oppo_targets_use_existing_safe_writers() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("vivo/android-v1-sdr-01.jpg");
    let apple = dir.path().join("live.heic");
    let (status, _, stderr) = convert(&input, &apple, "apple");
    assert_eq!(status, 0, "{stderr}");
    assert!(apple.is_file() && apple.with_extension("mov").is_file());

    let oppo = dir.path().join("oppoMP.jpg");
    let (status, _, stderr) = convert(&input, &oppo, "oppo");
    assert_eq!(status, 0, "{stderr}");
    assert_preserved_motion(&input, &oppo);
}

#[test]
fn google_batch_routes_mixed_still_formats_and_resumes_each_concrete_writer() {
    let dir = tempfile::tempdir().unwrap();
    let jpeg = dir.path().join("jpeg.jpg");
    fs::copy(fixture("vivo/android-v1-sdr-01.jpg"), &jpeg).unwrap();
    let heic = dir.path().join("heic.heic");
    clean_samsung_heic(&heic);
    let out = dir.path().join("out");
    let args = vec![
        "batch".into(),
        "--input".into(),
        jpeg.as_os_str().to_owned(),
        "--input".into(),
        heic.as_os_str().to_owned(),
        "--output-dir".into(),
        out.as_os_str().to_owned(),
        "--target".into(),
        "google".into(),
        "--resume".into(),
        "--json".into(),
    ];
    let (code, first, stderr) = run(args.clone());
    assert_eq!(code, 0, "{stderr}: {first}");
    assert_preserved_motion(&jpeg, &out.join("jpeg.xdremuxMP.jpg"));
    assert_preserved_motion(&heic, &out.join("heic.xdremuxMP.heic"));

    let (code, second, stderr) = run(args);
    assert_eq!(code, 0, "{stderr}: {second}");
    let receipt: serde_json::Value = serde_json::from_str(&second).unwrap();
    assert_eq!(receipt["skipped_existing"], 2);
}

#[test]
fn placeholders_and_conflicting_legacy_options_fail_before_publication() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("vivo/android-v1-sdr-01.jpg");
    let output = dir.path().join("not-writtenMP.jpg");
    for target in ["vivo", "xiaomi", "huawei"] {
        let (code, _, stderr) = convert(&input, &output, target);
        assert_eq!(code, 1, "{target}: {stderr}");
        assert!(stderr.contains("reserved"), "{stderr}");
        assert!(!output.exists());
    }
    let (code, _, stderr) = run(vec![
        "convert".into(),
        "--input".into(),
        input.as_os_str().to_owned(),
        "--target".into(),
        "google".into(),
        "--motion-target".into(),
        "google-jpeg".into(),
    ]);
    assert_eq!(code, 2, "{stderr}");
    assert!(!output.exists());
}
