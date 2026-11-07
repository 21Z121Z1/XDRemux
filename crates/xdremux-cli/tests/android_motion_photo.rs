use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use xdremux_motion_photo::{Input, MotionPhoto, ParseOptions, TargetProfile, WritePolicy};

fn fixture(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/motion-photo")
        .join(relative)
}

fn run(input: &Path, output: &Path, target: &str) -> (u8, String, String) {
    let args = vec![
        OsString::from("convert"),
        OsString::from("--input"),
        input.as_os_str().to_os_string(),
        OsString::from("--output"),
        output.as_os_str().to_os_string(),
        OsString::from("--motion-target"),
        OsString::from(target),
    ];
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = xdremux_cli::run_from(args, &mut stdout, &mut stderr);
    (
        status,
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    )
}

fn assert_conversion(relative: &str, target: &str, extension: &str) {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture(relative);
    let output = dir.path().join(format!("captureMP.{extension}"));
    let (status, stdout, stderr) = run(&input, &output, target);
    assert_eq!(status, 0, "{relative}: {stderr}");
    assert!(stderr.is_empty());
    assert!(stdout.contains(&output.display().to_string()));
    assert!(output.is_file());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);

    let source = fs::read(input).unwrap();
    let encoded = fs::read(output).unwrap();
    let before =
        MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
    let after = MotionPhoto::parse(Input::SingleFile(&encoded), ParseOptions::strict()).unwrap();
    after.validate().unwrap();
    assert_eq!(
        before.motion_video_bytes().unwrap(),
        after.motion_video_bytes().unwrap()
    );
    assert_eq!(before.presentation_time(), after.presentation_time());
    let declares_gain_map = |photo: &MotionPhoto<'_>| {
        photo.asset().auxiliary.iter().any(|resource| {
            resource
                .vendor_role
                .as_deref()
                .is_some_and(|name| name.eq_ignore_ascii_case("GainMap"))
        })
    };
    if declares_gain_map(&before) {
        assert!(
            declares_gain_map(&after),
            "{relative}: Gain Map directory entry was lost"
        );
    }
}

#[test]
fn google_jpeg_output_is_one_strict_motion_photo_with_preserved_video() {
    assert_conversion("vivo/android-v1-sdr-01.jpg", "google-jpeg", "jpg");
}

#[test]
fn google_jpeg_ultra_hdr_input_preserves_motion_video() {
    assert_conversion("xiaomi/android-v1-ultrahdr-01.jpg", "google-jpeg", "jpg");
}

#[test]
fn google_heic_output_is_one_strict_motion_photo_with_preserved_video() {
    // Samsung's original HEIC has additional SEF resources which cannot be
    // silently discarded. Build a clean HEIC Motion Photo from its real still
    // and video using the same library's documented Parts composition API.
    let dir = tempfile::tempdir().unwrap();
    let original = fs::read(fixture("samsung/heif-ultrahdr-01.heic")).unwrap();
    let original =
        MotionPhoto::parse(Input::SingleFile(&original), ParseOptions::compatible()).unwrap();
    let clean = MotionPhoto::parse(
        Input::Parts {
            still: original.still_bytes().unwrap(),
            movie: original.motion_video_bytes().unwrap(),
            presentation: original.presentation_time(),
        },
        ParseOptions::compatible(),
    )
    .unwrap();
    let files = clean
        .plan(TargetProfile::AndroidHeif, WritePolicy::default())
        .execute()
        .unwrap()
        .files;
    let input = dir.path().join("sourceMP.heic");
    fs::write(&input, &files[0].bytes).unwrap();
    let output = dir.path().join("captureMP.heic");
    let (status, _, stderr) = run(&input, &output, "google-heic");
    assert_eq!(status, 0, "{stderr}");
    let source = fs::read(input).unwrap();
    let encoded = fs::read(output).unwrap();
    let before = MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::strict()).unwrap();
    let after = MotionPhoto::parse(Input::SingleFile(&encoded), ParseOptions::strict()).unwrap();
    assert_eq!(
        before.motion_video_bytes().unwrap(),
        after.motion_video_bytes().unwrap()
    );
    assert_eq!(before.presentation_time(), after.presentation_time());
    after.validate().unwrap();
}

#[test]
fn samsung_heic_vendor_resources_are_rejected_without_loss() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("captureMP.heic");
    let (status, _, stderr) = run(
        &fixture("samsung/heif-ultrahdr-01.heic"),
        &output,
        "google-heic",
    );
    assert_eq!(status, 1);
    assert!(
        stderr.contains("target cannot express resource"),
        "{stderr}"
    );
    assert!(!output.exists());
}

#[test]
fn explicit_apple_pair_and_pvt_targets_reuse_existing_conversion() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("samsung/heif-ultrahdr-01.heic");
    let pair = dir.path().join("paired.heic");
    let (status, _, error) = run(&input, &pair, "apple-pair");
    assert_eq!(status, 0, "{error}");
    assert!(pair.is_file());
    assert!(pair.with_extension("mov").is_file());

    let package = dir.path().join("packaged.pvt");
    let (status, _, error) = run(&input, &package, "apple-pvt");
    assert_eq!(status, 0, "{error}");
    assert!(package.join("packaged.heic").is_file());
    assert!(package.join("packaged.mov").is_file());
}

#[test]
fn google_output_does_not_replace_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("captureMP.jpg");
    fs::write(&output, b"existing user content").unwrap();
    let (status, _, error) = run(
        &fixture("vivo/android-v1-sdr-01.jpg"),
        &output,
        "google-jpeg",
    );
    assert_eq!(status, 1);
    assert!(error.contains("already exists"), "{error}");
    assert_eq!(fs::read(&output).unwrap(), b"existing user content");
}

#[test]
fn google_default_name_is_recognizable_and_skips_collisions() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("capture.jpg");
    fs::copy(fixture("vivo/android-v1-sdr-01.jpg"), &input).unwrap();
    let input_before = fs::read(&input).unwrap();
    let previous = dir.path().join("captureMP.jpg");
    fs::write(&previous, b"previous output").unwrap();
    let args = [
        OsString::from("convert"),
        OsString::from("--input"),
        input.as_os_str().to_os_string(),
        OsString::from("--motion-target"),
        OsString::from("google-jpeg"),
    ];
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    assert_eq!(
        xdremux_cli::run_from(args, &mut stdout, &mut stderr),
        0,
        "{}",
        String::from_utf8_lossy(&stderr)
    );
    assert_eq!(fs::read(&previous).unwrap(), b"previous output");
    assert_eq!(fs::read(&input).unwrap(), input_before);
    let output = dir.path().join("capture (2)MP.jpg");
    assert!(output.is_file());
    assert!(String::from_utf8(stdout)
        .unwrap()
        .contains(&output.display().to_string()));
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 3);
}

#[test]
fn google_output_rejects_transcoding_and_nonconforming_name() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("vivo/android-v1-sdr-01.jpg");
    let output = dir.path().join("captureMP.heic");
    let (status, _, error) = run(&input, &output, "google-heic");
    assert_eq!(status, 1);
    assert!(error.contains("transcoding"), "{error}");
    assert!(!output.exists());

    let bad_name = dir.path().join("capture.jpg");
    let (status, _, error) = run(&input, &bad_name, "google-jpeg");
    assert_eq!(status, 1);
    assert!(error.contains("MP.jpg"), "{error}");
    assert!(!bad_name.exists());
}

#[test]
fn motion_target_is_not_silently_applied_to_other_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("vivo/android-v1-sdr-01.jpg");
    let output = dir.path().join("captureMP.jpg");
    let args = [
        OsString::from("convert"),
        OsString::from("--input"),
        input.as_os_str().to_os_string(),
        OsString::from("--motion-target"),
        OsString::from("google-jpeg"),
        OsString::from("--live-photo-output"),
        OsString::from("pvt"),
        OsString::from("--output"),
        output.as_os_str().to_os_string(),
    ];
    let mut out = Vec::new();
    let mut err = Vec::new();
    let status = xdremux_cli::run_from(args, &mut out, &mut err);
    assert_ne!(status, 0);
    assert!(!output.exists());
}
