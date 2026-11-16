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

fn run_cli(args: Vec<OsString>) -> (u8, String, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = xdremux_cli::run_from(args, &mut stdout, &mut stderr);
    (
        status,
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    )
}

fn run(mut args: Vec<OsString>) -> (u8, String, String) {
    if args.iter().any(|arg| arg == "--target")
        && !args.iter().any(|arg| arg == "--accept-unverified-target")
    {
        args.push("--accept-unverified-target".into());
    }
    run_cli(args)
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
    let source_photo =
        MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
    let source_video = source_photo.motion_video_bytes().unwrap().to_vec();

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
        assert!(
            !bytes
                .windows(b"OpCamera:".len())
                .any(|window| window == b"OpCamera:"),
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
        if target == "xiaomi" {
            assert!(text.contains("MotionPhotoVersion=\"1\""));
            assert!(bytes.ends_with(source_video.as_slice()));
        }
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
fn oppo_heic_source_plans_jpeg_and_stops_before_still_reencode() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("samsung/heif-ultrahdr-01.heic");
    let output = dir.path().join("oppoMP.jpg");
    let (code, _, stderr) = convert(&input, &output, "oppo");
    assert_eq!(code, 1, "{stderr}");
    assert!(stderr.contains("JPEG Motion Photo"), "{stderr}");
    assert!(stderr.contains("TranscodeRequired"), "{stderr}");
    assert!(!output.exists());
}

#[test]
fn samsung_heic_target_keeps_mpvd_as_the_last_top_level_box() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("samsung/heif-ultrahdr-01.heic");
    let output = dir.path().join("samsungMP.heic");
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
    let bytes = fs::read(&output).unwrap();
    let mut offset = 0usize;
    let mut found = false;
    while offset + 8 <= bytes.len() {
        let size = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        assert!(size >= 8, "special box size {size}");
        let end = offset + size;
        assert!(end <= bytes.len());
        if &bytes[offset + 4..offset + 8] == b"mpvd" {
            assert_eq!(end, bytes.len());
            found = true;
            break;
        }
        offset = end;
    }
    assert!(found);
}

#[test]
fn vivo_target_preserves_a_source_that_already_has_both_uuid_boxes() {
    let dir = tempfile::tempdir().unwrap();
    for (name, meta) in [
        ("vivo/android-v1-sdr-01.jpg", false),
        ("vivo/android-v1-sdr-02.jpg", true),
    ] {
        let input = fixture(name);
        let output = dir.path().join(format!("{meta}MP.jpg"));
        let (code, stdout, stderr) = run(vec![
            "convert".into(),
            "--input".into(),
            input.as_os_str().to_owned(),
            "--output".into(),
            output.as_os_str().to_owned(),
            "--target".into(),
            "vivo".into(),
            "--motion-report-json".into(),
        ]);
        assert_eq!(code, 0, "{name}: {stderr} {stdout}");
        let source = fs::read(&input).unwrap();
        let bytes = fs::read(&output).unwrap();
        assert_eq!(bytes, source, "{name}");
        assert!(bytes
            .windows(b"vivoMediaEStream".len())
            .any(|w| w == b"vivoMediaEStream"));
        assert!(bytes
            .windows(b"vivoMediaExtInfo".len())
            .any(|w| w == b"vivoMediaExtInfo"));
        assert_eq!(
            bytes
                .windows(b"application/livePhoto-meta".len())
                .any(|w| w == b"application/livePhoto-meta"),
            meta,
            "{name}"
        );
        let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(report["container"], "vivo-preserved-source-layout");
        assert_eq!(report["native_vendor_claim"], false);
        assert_eq!(report["motion_video_copies"], 1);
        assert!(report["not_carried"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str().unwrap().contains("not synthesized")));
    }
}

#[test]
fn vivo_target_writes_a_combined_jpeg_for_oppo_and_google_sources() {
    let dir = tempfile::tempdir().unwrap();
    for name in [
        "oppo/coloros15-ultrahdr-01.jpg",
        "xiaomi/android-v1-ultrahdr-01.jpg",
    ] {
        let input = fixture(name);
        let output = dir.path().join(format!("{}MP.jpg", name.replace('/', "-")));
        let (code, stdout, stderr) = run(vec![
            "convert".into(),
            "--input".into(),
            input.as_os_str().to_owned(),
            "--output".into(),
            output.as_os_str().to_owned(),
            "--target".into(),
            "vivo".into(),
            "--motion-report-json".into(),
        ]);
        assert_eq!(code, 0, "{name}: {stderr} {stdout}");
        let source = fs::read(&input).unwrap();
        let photo =
            MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
        let video = photo.motion_video_bytes().unwrap();
        let bytes = fs::read(&output).unwrap();
        assert!(bytes.ends_with(video), "{name}");
        assert_eq!(
            bytes
                .windows(video.len())
                .filter(|window| *window == video)
                .count(),
            1,
            "{name}"
        );
        let start = bytes.len() - video.len();
        assert_eq!(&bytes[start + 4..start + 8], b"ftyp", "{name}");
        let prefix = &bytes[..start];
        assert!(prefix
            .windows(b"motionphoto00010000000000000".len())
            .any(|w| w == b"motionphoto00010000000000000"));
        assert!(prefix
            .windows(b"com.android.camera.imageTime".len())
            .any(|w| w == b"com.android.camera.imageTime"));
        assert!(
            prefix
                .windows(b"MotionPhoto=\"1\"".len())
                .any(|w| w == b"MotionPhoto=\"1\"")
                || prefix
                    .windows(b"MotionPhoto='1'".len())
                    .any(|w| w == b"MotionPhoto='1'")
        );
        assert!(prefix
            .windows(b"Padding=\"0\"".len())
            .any(|w| w == b"Padding=\"0\""));
        assert!(prefix
            .windows(b"VCamera:VMotionPhotoVersion=\"1\"".len())
            .any(|w| w == b"VCamera:VMotionPhotoVersion=\"1\""));
        assert!(
            !prefix
                .windows(b"vivoMediaEStream".len())
                .any(|w| w == b"vivoMediaEStream"),
            "{name}"
        );
        assert!(
            !prefix
                .windows(b"firstFrame".len())
                .any(|w| w == b"firstFrame"),
            "{name}"
        );
        let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(report["container"], "vivo-combined-jpeg-mp4");
        assert_eq!(report["native_vendor_claim"], false);
        assert_eq!(report["motion_video_copies"], 1);
        let notes = report["not_carried"].to_string();
        assert!(notes.contains("imageTime"), "{name} {notes}");
        if source
            .windows(b"hdr-gain-map".len())
            .any(|w| w == b"hdr-gain-map")
        {
            assert!(
                bytes
                    .windows(b"hdr-gain-map".len())
                    .any(|w| w == b"hdr-gain-map"),
                "{name}"
            );
        }
    }
}

#[test]
fn vivo_heic_source_stops_before_still_reencode() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("samsung/heif-ultrahdr-01.heic");
    let output = dir.path().join("vivoMP.jpg");
    let (code, _, stderr) = convert(&input, &output, "vivo");
    assert_eq!(code, 1, "{stderr}");
    assert!(stderr.contains("TranscodeRequired"), "{stderr}");
    assert!(!output.exists());
}

#[test]
fn huawei_jpeg_does_not_publish_the_android_gallery_tail() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("vivo/android-v1-sdr-01.jpg");
    let output = dir.path().join("huaweiMP.jpg");
    let (code, _, stderr) = run(vec![
        "convert".into(),
        "--input".into(),
        input.as_os_str().to_owned(),
        "--output".into(),
        output.as_os_str().to_owned(),
        "--target".into(),
        "huawei".into(),
    ]);
    assert_eq!(code, 1, "{stderr}");
    assert!(!output.exists());
    assert!(stderr.contains("11.0.19.478"), "{stderr}");
    assert!(stderr.contains("not invented"), "{stderr}");
    assert!(stderr.contains("does not enable"), "{stderr}");
}

#[test]
fn huawei_keeps_an_existing_openharmony_heic_byte_for_byte() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("mate.heic");
    let source = openharmony_motion_photo();
    fs::write(&input, &source).unwrap();
    let output = dir.path().join("mateMP.heic");
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
    assert_eq!(fs::read(&output).unwrap(), source);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["destination"], "huawei");
    assert_eq!(report["container"], "harmony-openharmony-heic-mp4");
    assert_eq!(report["native_vendor_claim"], false);
    assert_eq!(report["consumer_gallery_verified"], false);
    assert_eq!(report["motion_video_copies"], 1);
    let carried = report["carried_source_attributes"].to_string();
    assert!(
        carried.contains("com.openharmony.covertime=1234.5"),
        "{carried}"
    );
    assert!(
        carried.contains("com.openharmony.videoId=20260907021031595"),
        "{carried}"
    );
    let notes = report["not_carried"].to_string();
    assert!(notes.contains("not generated"), "{notes}");
    assert!(notes.contains("not the Android"), "{notes}");
    assert!(stderr.contains("photoAccessHelper"), "{stderr}");
}

#[test]
fn batch_harmony_uses_the_same_preserve_path_as_convert() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("mate.heic");
    let source = openharmony_motion_photo();
    fs::write(&input, &source).unwrap();
    let out = dir.path().join("out");
    let (code, stdout, stderr) = run(vec![
        "batch".into(),
        "--input".into(),
        input.as_os_str().to_owned(),
        "--output-dir".into(),
        out.as_os_str().to_owned(),
        "--target".into(),
        "huawei".into(),
        "--json".into(),
    ]);
    assert_eq!(code, 0, "{stderr} {stdout}");
    let published = out.join("mate.xdremuxMP.heic");
    assert_eq!(fs::read(&published).unwrap(), source);
    let receipt: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(receipt["succeeded"], 1);
    assert_eq!(receipt["failed"], 0);
    assert_eq!(receipt["successes"][0]["omitted_resource_count"], 0);
}

#[test]
fn batch_harmony_writes_nothing_before_risk_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("mate.heic");
    fs::write(&input, openharmony_motion_photo()).unwrap();
    let out = dir.path().join("out");
    let (code, _, stderr) = run_cli(vec![
        "batch".into(),
        "--input".into(),
        input.as_os_str().to_owned(),
        "--output-dir".into(),
        out.as_os_str().to_owned(),
        "--target".into(),
        "huawei".into(),
    ]);
    assert_eq!(code, 1, "{stderr}");
    assert!(!out.join("mate.xdremuxMP.heic").exists());
    assert!(stderr.contains("--accept-unverified-target"), "{stderr}");
}

#[test]
fn batch_harmony_keeps_a_good_file_and_rejects_jpeg_and_a_bad_container() {
    let dir = tempfile::tempdir().unwrap();
    let good = dir.path().join("good.heic");
    let bad = dir.path().join("bad.heic");
    let jpeg = dir.path().join("still.jpg");
    let source = openharmony_motion_photo();
    fs::write(&good, &source).unwrap();
    let mut broken = source.clone();
    let marker = b"com.openharmony.videoId";
    let at = broken
        .windows(marker.len())
        .position(|w| w == marker)
        .unwrap();
    broken[at..at + marker.len()].fill(b'x');
    fs::write(&bad, broken).unwrap();
    fs::copy(fixture("vivo/android-v1-sdr-01.jpg"), &jpeg).unwrap();
    let out = dir.path().join("out");
    let (code, stdout, stderr) = run(vec![
        "batch".into(),
        "--input".into(),
        good.as_os_str().to_owned(),
        "--input".into(),
        bad.as_os_str().to_owned(),
        "--input".into(),
        jpeg.as_os_str().to_owned(),
        "--output-dir".into(),
        out.as_os_str().to_owned(),
        "--target".into(),
        "huawei".into(),
        "--json".into(),
    ]);
    assert_ne!(code, 0, "{stdout}");
    assert_eq!(fs::read(out.join("good.xdremuxMP.heic")).unwrap(), source);
    assert!(!out.join("bad.xdremuxMP.heic").exists());
    assert!(!out.join("still.xdremuxMP.jpg").exists());
    let receipt: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(receipt["succeeded"], 1);
    assert_eq!(receipt["failed"], 2);
    let errors = receipt["failures"].to_string();
    assert!(
        errors.contains("not invented") || errors.contains("not a HEIC"),
        "{stderr} {errors}"
    );
}

#[test]
fn batch_harmony_resume_does_not_skip_a_changed_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("mate.heic");
    let source = openharmony_motion_photo();
    fs::write(&input, &source).unwrap();
    let out = dir.path().join("out");
    let args = vec![
        "batch".into(),
        "--input".into(),
        input.as_os_str().to_owned(),
        "--output-dir".into(),
        out.as_os_str().to_owned(),
        "--target".into(),
        "huawei".into(),
        "--resume".into(),
        "--json".into(),
    ];
    let (code, first, stderr) = run(args.clone());
    assert_eq!(code, 0, "{stderr} {first}");
    let published = out.join("mate.xdremuxMP.heic");
    assert_eq!(fs::read(&published).unwrap(), source);
    let (code, second, stderr) = run(args.clone());
    assert_eq!(code, 0, "{stderr} {second}");
    let skipped: serde_json::Value = serde_json::from_str(&second).unwrap();
    assert_eq!(skipped["skipped_existing"], 1);
    let mut changed = fs::read(&published).unwrap();
    changed[0] ^= 0xff;
    fs::write(&published, &changed).unwrap();
    let (code, third, _stderr) = run(args);
    assert_ne!(code, 0, "{third}");
    let receipt: serde_json::Value = serde_json::from_str(&third).unwrap();
    assert_eq!(receipt["skipped_existing"], 0);
    assert_ne!(fs::read(&published).unwrap(), source);
}

#[test]
fn batch_omitted_count_matches_left_out_resources_and_stays_zero_when_none() {
    let dir = tempfile::tempdir().unwrap();
    let oppo = fixture("oppo/coloros15-ultrahdr-01.jpg");
    let oppo_out = dir.path().join("oppoMP.jpg");
    let (code, convert_stdout, stderr) = run(vec![
        "convert".into(),
        "--input".into(),
        oppo.as_os_str().to_owned(),
        "--output".into(),
        oppo_out.as_os_str().to_owned(),
        "--target".into(),
        "oppo".into(),
        "--motion-report-json".into(),
    ]);
    assert_eq!(code, 0, "{stderr}");
    let convert_report: serde_json::Value = serde_json::from_str(&convert_stdout).unwrap();
    let left_out = convert_report["not_carried"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item.as_str().unwrap_or("").contains("left out"))
        .count();
    assert!(left_out > 0, "{convert_report}");

    let out = dir.path().join("batch");
    let (code, stdout, stderr) = run(vec![
        "batch".into(),
        "--input".into(),
        oppo.as_os_str().to_owned(),
        "--output-dir".into(),
        out.as_os_str().to_owned(),
        "--target".into(),
        "oppo".into(),
        "--json".into(),
    ]);
    assert_eq!(code, 0, "{stderr} {stdout}");
    let receipt: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(receipt["successes"][0]["omitted_resource_count"], left_out);

    let (code, human, stderr) = run(vec![
        "batch".into(),
        "--input".into(),
        oppo.as_os_str().to_owned(),
        "--output-dir".into(),
        dir.path().join("human").as_os_str().to_owned(),
        "--target".into(),
        "oppo".into(),
    ]);
    assert_eq!(code, 0, "{stderr} {human}");
    assert!(
        human.contains(&format!("{left_out} source resources")),
        "{human}"
    );

    let clean = fixture("vivo/android-v1-sdr-01.jpg");
    let (code, stdout, stderr) = run(vec![
        "batch".into(),
        "--input".into(),
        clean.as_os_str().to_owned(),
        "--output-dir".into(),
        dir.path().join("clean").as_os_str().to_owned(),
        "--target".into(),
        "google".into(),
        "--json".into(),
    ]);
    assert_eq!(code, 0, "{stderr} {stdout}");
    let clean_receipt: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(clean_receipt["successes"][0]["omitted_resource_count"], 0);
    let (code, human, stderr) = run(vec![
        "batch".into(),
        "--input".into(),
        clean.as_os_str().to_owned(),
        "--output-dir".into(),
        dir.path().join("clean-human").as_os_str().to_owned(),
        "--target".into(),
        "google".into(),
    ]);
    assert_eq!(code, 0, "{stderr}");
    assert!(!human.contains("remain only"), "{human}");
}

#[test]
fn other_heic_motion_is_not_rewritten_into_harmony() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("samsung/heif-ultrahdr-01.heic");
    let output = dir.path().join("heldMP.heic");
    let (code, _, stderr) = run(vec![
        "convert".into(),
        "--input".into(),
        input.as_os_str().to_owned(),
        "--output".into(),
        output.as_os_str().to_owned(),
        "--target".into(),
        "huawei".into(),
    ]);
    assert_eq!(code, 1, "{stderr}");
    assert!(!output.exists());
    assert!(stderr.contains("not invented"), "{stderr}");
}

fn box_bytes(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&u32::try_from(payload.len() + 8).unwrap().to_be_bytes());
    bytes.extend_from_slice(kind);
    bytes.extend_from_slice(payload);
    bytes
}

fn openharmony_motion_photo() -> Vec<u8> {
    let names = [
        b"com.openharmony.covertime".as_slice(),
        b"com.openharmony.deferredVideoEnhanceFlag".as_slice(),
        b"com.openharmony.starttime".as_slice(),
        b"com.openharmony.videoId".as_slice(),
    ];
    let values: [&[u8]; 4] = [
        &1234.5f32.to_be_bytes(),
        b"0",
        b"1788718230582",
        b"20260907021031595",
    ];
    let mut keys_payload = vec![0, 0, 0, 0];
    keys_payload.extend_from_slice(&u32::try_from(names.len()).unwrap().to_be_bytes());
    for name in names {
        keys_payload.extend_from_slice(&u32::try_from(name.len() + 8).unwrap().to_be_bytes());
        keys_payload.extend_from_slice(b"mdta");
        keys_payload.extend_from_slice(name);
    }
    let mut ilst_payload = Vec::new();
    for (index, value) in values.into_iter().enumerate() {
        let mut data_payload = vec![0; 8];
        data_payload.extend_from_slice(value);
        let data = box_bytes(b"data", &data_payload);
        let item_id = u32::try_from(index + 1).unwrap().to_be_bytes();
        ilst_payload.extend_from_slice(&box_bytes(&item_id, &data));
    }
    let mut meta_payload = vec![0, 0, 0, 0];
    meta_payload.extend_from_slice(&box_bytes(b"hdlr", &[0; 8]));
    meta_payload.extend_from_slice(&box_bytes(b"keys", &keys_payload));
    meta_payload.extend_from_slice(&box_bytes(b"ilst", &ilst_payload));
    let mut bytes = box_bytes(b"ftyp", b"heic\0\0\0\0heicmif1");
    bytes.extend_from_slice(&box_bytes(b"mdat", b"heif-still-hdr"));
    bytes.extend_from_slice(&box_bytes(b"ftyp", b"mp42\0\0\0\0isommp42"));
    bytes.extend_from_slice(&box_bytes(b"mdat", b"video-samples"));
    bytes.extend_from_slice(&box_bytes(b"moov", &box_bytes(b"meta", &meta_payload)));
    bytes.extend_from_slice(b"v6_f27LIVE_");
    bytes.extend(std::iter::repeat_n(b' ', 49));
    bytes
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

#[test]
fn standard_aliases_share_plan_and_output_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("vivo/android-v1-sdr-01.jpg");
    let mut bytes = Vec::new();
    for name in ["android-standard", "standard", "google"] {
        let output = dir.path().join(format!("{name}MP.jpg"));
        let (status, stdout, stderr) = run(vec![
            "convert".into(),
            "--input".into(),
            input.as_os_str().to_owned(),
            "--output".into(),
            output.as_os_str().to_owned(),
            "--target".into(),
            name.into(),
            "--motion-report-json".into(),
        ]);
        assert_eq!(status, 0, "{stderr}");
        let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(report["destination"], "android-standard");
        assert!(stderr.contains("not a player"), "{stderr}");
        bytes.push(fs::read(output).unwrap());
    }
    assert_eq!(bytes[0], bytes[1]);
    assert_eq!(bytes[0], bytes[2]);
}

#[test]
fn unverified_target_writes_nothing_until_the_risk_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("vivo/android-v1-sdr-01.jpg");
    let output = dir.path().join("heldMP.jpg");
    let (status, _, stderr) = run_cli(vec![
        "convert".into(),
        "--input".into(),
        input.as_os_str().to_owned(),
        "--output".into(),
        output.as_os_str().to_owned(),
        "--target".into(),
        "google".into(),
    ]);
    assert_eq!(status, 1, "{stderr}");
    assert!(!output.exists());
    assert!(stderr.contains("android-standard"), "{stderr}");
    assert!(stderr.contains("not a player"), "{stderr}");
    assert!(stderr.contains("--accept-unverified-target"), "{stderr}");

    let (status, _, stderr) = convert(&input, &output, "standard");
    assert_eq!(status, 0, "{stderr}");
    assert!(output.is_file());
}

#[test]
fn blocked_heic_stays_blocked_after_the_warning_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("samsung/heif-ultrahdr-01.heic");
    let output = dir.path().join("blockedMP.jpg");
    let (status, _, stderr) = run(vec![
        "convert".into(),
        "--input".into(),
        input.as_os_str().to_owned(),
        "--output".into(),
        output.as_os_str().to_owned(),
        "--target".into(),
        "oppo".into(),
        "--accept-unverified-target".into(),
    ]);
    assert_eq!(status, 1, "{stderr}");
    assert!(!output.exists());
    assert!(stderr.contains("TranscodeRequired"), "{stderr}");
    assert!(stderr.contains("does not enable"), "{stderr}");
}

#[test]
fn batch_target_writes_nothing_without_acceptance() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("vivo/android-v1-sdr-01.jpg");
    let out = dir.path().join("out");
    let (status, _, stderr) = run_cli(vec![
        "batch".into(),
        "--input".into(),
        input.as_os_str().to_owned(),
        "--output-dir".into(),
        out.as_os_str().to_owned(),
        "--target".into(),
        "android-standard".into(),
    ]);
    assert_eq!(status, 1, "{stderr}");
    assert!(!out.join("android-v1-sdr-01.xdremuxMP.jpg").exists());
    assert!(stderr.contains("android-standard"), "{stderr}");
}
