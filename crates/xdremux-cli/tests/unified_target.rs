use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use xdremux_motion_photo::{
    Input, MotionPhoto, ParseOptions, ResourceRole, TargetProfile, WritePolicy,
};

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
fn vendor_targets_publish_public_motion_photos_without_inventing_fields() {
    let dir = tempfile::tempdir().unwrap();
    let vivo = fixture("vivo/android-v1-sdr-01.jpg");
    let source = fs::read(&vivo).unwrap();
    assert!(source
        .windows(b"VCamera:".len())
        .any(|window| window == b"VCamera:"));
    assert!(!source
        .windows(b"OpCamera:".len())
        .any(|window| window == b"OpCamera:"));

    for target in ["xiaomi", "samsung", "standard", "android-standard"] {
        let output = dir.path().join(format!("{target}MP.jpg"));
        let (code, stdout, stderr) = run(vec![
            "convert".into(),
            "--input".into(),
            vivo.as_os_str().to_owned(),
            "--output".into(),
            output.as_os_str().to_owned(),
            "--target".into(),
            target.into(),
            "--motion-report-json".into(),
        ]);
        assert_eq!(code, 0, "{target}: {stderr} {stdout}");
        assert_preserved_motion(&vivo, &output);
        let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(report["container"], "public-motion-photo-1.0");
        assert_eq!(report["native_vendor_claim"], false);
        assert_eq!(report["consumer_gallery_verified"], false);
        assert_eq!(report["motion_video_copies"], 1);
        assert_eq!(report["exif_identifier_present"], true, "{target}");
        let bytes = fs::read(&output).unwrap();
        assert_eq!(
            bytes
                .windows(b"OpCamera:".len())
                .any(|window| window == b"OpCamera:"),
            false,
            "{target} invented OpCamera metadata"
        );
        let carried = report["carried_source_attributes"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert!(
            carried.is_empty(),
            "{target} copied vendor attributes it does not own: {carried:?}"
        );
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("MotionPhoto=\"1\""), "{target}");
        assert!(text.contains("Item:Padding=\"0\""), "{target}");
        assert!(!text.contains("MicroVideoVersion"), "{target}");
    }
}

#[test]
fn oppo_target_carries_source_oplus_fields_and_one_video() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("oppo/coloros15-ultrahdr-01.jpg");
    let output = dir.path().join("oppoMP.jpg");
    let (code, stdout, stderr) = run(vec![
        "convert".into(),
        "--input".into(),
        input.as_os_str().to_owned(),
        "--output".into(),
        output.as_os_str().to_owned(),
        "--target".into(),
        "oppo".into(),
        "--motion-report-json".into(),
    ]);
    assert_eq!(code, 0, "{stderr} {stdout}");
    assert_preserved_motion(&input, &output);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let carried = report["carried_source_attributes"].as_array().unwrap();
    assert!(
        carried.iter().any(|item| item == "OpCamera:VideoLength"),
        "{report}"
    );
    assert!(carried
        .iter()
        .any(|item| item == "OpCamera:MotionPhotoOwner"));
    let bytes = fs::read(&output).unwrap();
    let photo = MotionPhoto::parse(Input::SingleFile(&bytes), ParseOptions::strict()).unwrap();
    let video = photo.motion_video_bytes().unwrap().to_vec();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains(&format!("OpCamera:VideoLength=\"{}\"", video.len())));
    assert!(
        text.contains("Camera:MotionPhoto=\"1\"") || text.contains("GCamera:MotionPhoto=\"1\"")
    );
    assert_eq!(
        photo
            .resources()
            .filter(|resource| resource.role == ResourceRole::PrimaryMotionVideo)
            .count(),
        1
    );
    assert_eq!(
        bytes
            .windows(video.len())
            .filter(|window| *window == video.as_slice())
            .count(),
        1
    );
    assert!(report["not_carried"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item.as_str().unwrap_or("").contains("left out")));
}

#[test]
fn samsung_target_publishes_primary_video_and_names_the_secondary_stream() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("samsung/jpeg-ultrahdr-01.jpg");
    let output = dir.path().join("samsungMP.jpg");
    let (code, stdout, stderr) = run(vec![
        "convert".into(),
        "--input".into(),
        input.as_os_str().to_owned(),
        "--output".into(),
        output.as_os_str().to_owned(),
        "--target".into(),
        "samsung".into(),
        "--motion-report-json".into(),
    ]);
    assert_eq!(code, 0, "{stderr} {stdout}");
    assert_preserved_motion(&input, &output);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["native_vendor_claim"], false);
    let left_out = report["not_carried"].as_array().unwrap();
    assert!(
        left_out
            .iter()
            .any(|item| item.as_str().unwrap_or("").contains("MotionPhoto_AutoPlay")),
        "{left_out:?}"
    );
}

#[test]
fn vivo_target_refuses_to_publish_a_google_container() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("vivo/android-v1-sdr-01.jpg");
    let output = dir.path().join("vivoMP.jpg");
    let (code, _, stderr) = convert(&input, &output, "vivo");
    assert_eq!(code, 1, "{stderr}");
    assert!(stderr.contains("livePhoto-meta"), "{stderr}");
    assert!(!output.exists());
}

#[test]
fn huawei_target_appends_one_preserved_video_and_a_40_byte_tail() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("vivo/android-v1-sdr-01.jpg");
    let output = dir.path().join("huaweiMP.jpg");
    let (code, stdout, stderr) = run(vec![
        "convert".into(),
        "--input".into(),
        input.as_os_str().to_owned(),
        "--output".into(),
        output.as_os_str().to_owned(),
        "--target".into(),
        "huawei".into(),
        "--motion-report-json".into(),
    ]);
    assert_eq!(code, 0, "{stderr} {stdout}");
    let source = fs::read(&input).unwrap();
    let before =
        MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
    let video = before.motion_video_bytes().unwrap().to_vec();
    let bytes = fs::read(&output).unwrap();
    assert_eq!(
        &bytes[bytes.len() - video.len() - 40..bytes.len() - 40],
        video.as_slice()
    );
    let tag = std::str::from_utf8(&bytes[bytes.len() - 20..])
        .unwrap()
        .trim();
    assert_eq!(tag, format!("LIVE_{}", video.len()));
    assert!(bytes[bytes.len() - 40..bytes.len() - 20]
        .iter()
        .all(|byte| *byte == b' '));
    assert_eq!(
        bytes
            .windows(video.len())
            .filter(|window| *window == video.as_slice())
            .count(),
        1
    );
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["container"], "huawei-live-tail-40");
    assert_eq!(report["native_vendor_claim"], false);
    assert_eq!(report["consumer_gallery_verified"], false);
    assert_eq!(report["motion_video_copies"], 1);
}

#[test]
fn conflicting_legacy_options_fail_before_publication() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("vivo/android-v1-sdr-01.jpg");
    let output = dir.path().join("not-writtenMP.jpg");
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
