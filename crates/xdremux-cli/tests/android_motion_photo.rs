use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use xdremux_motion_photo::{Input, MotionPhoto, ParseOptions, TargetProfile, WritePolicy};

fn run_with_args(args: Vec<OsString>) -> (u8, String, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = xdremux_cli::run_from(args, &mut stdout, &mut stderr);
    (
        status,
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    )
}

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
    if cfg!(windows) {
        // Exclusive PVT directory publication is intentionally unavailable on
        // Windows. Verify the existing fail-closed product contract instead
        // of claiming a successful package publication on that platform.
        assert_eq!(status, 1, "{error}");
        assert!(
            error.contains("atomic exclusive PVT directory publication is unavailable"),
            "{error}"
        );
        assert!(!package.exists());
    } else {
        assert_eq!(status, 0, "{error}");
        assert!(package.join("packaged.heic").is_file());
        assert!(package.join("packaged.mov").is_file());
    }
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
fn google_output_never_creates_a_pvt_package_ancestor() {
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("not-created.pvt");
    let output = package.join("captureMP.jpg");
    let (status, _, error) = run(
        &fixture("vivo/android-v1-sdr-01.jpg"),
        &output,
        "google-jpeg",
    );
    assert_eq!(status, 1);
    assert!(error.contains("PVT"), "{error}");
    assert!(!package.exists());
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

#[test]
fn apple_heic_mov_pair_converts_to_android_heic_without_transcoding() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("samsung/heif-ultrahdr-01.heic");
    let pair = dir.path().join("apple.heic");
    let (status, _, error) = run(&input, &pair, "apple-pair");
    assert_eq!(status, 0, "{error}");
    let still = fs::read(&pair).unwrap();
    let movie = fs::read(pair.with_extension("mov")).unwrap();
    let apple = MotionPhoto::parse(
        Input::ApplePair {
            still: &still,
            movie: &movie,
        },
        ParseOptions::strict(),
    )
    .unwrap();

    let output = dir.path().join("fromAppleMP.heic");
    let (status, stdout, error) = run_with_args(vec![
        "convert".into(),
        "--input".into(),
        pair.as_os_str().into(),
        "--motion-video".into(),
        pair.with_extension("mov").as_os_str().into(),
        "--output".into(),
        output.as_os_str().into(),
        "--motion-target".into(),
        "google-heic".into(),
        "--motion-report-json".into(),
    ]);
    assert_eq!(status, 0, "{error}");
    assert!(stdout.contains("\"resources\""), "{stdout}");
    assert!(stdout.contains("\"source_role\""), "{stdout}");
    let encoded = fs::read(output).unwrap();
    let android = MotionPhoto::parse(Input::SingleFile(&encoded), ParseOptions::strict()).unwrap();
    android.validate().unwrap();
    assert_eq!(apple.presentation_time(), android.presentation_time());
    assert_eq!(
        apple.motion_video_bytes().unwrap(),
        android.motion_video_bytes().unwrap()
    );
    assert_eq!(
        xdremux_motion_photo::apple::media_payloads(&movie).unwrap(),
        xdremux_motion_photo::apple::media_payloads(android.motion_video_bytes().unwrap()).unwrap()
    );
}

#[test]
fn ultra_hdr_gain_map_resource_is_byte_identical_after_google_jpeg_conversion() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("xiaomi/android-v1-ultrahdr-01.jpg");
    let output = dir.path().join("retainedMP.jpg");
    let (status, stdout, error) = run_with_args(vec![
        "convert".into(),
        "--input".into(),
        input.as_os_str().into(),
        "--output".into(),
        output.as_os_str().into(),
        "--motion-target".into(),
        "google-jpeg".into(),
        "--motion-report-json".into(),
    ]);
    assert_eq!(status, 0, "{error}");
    assert!(stdout.contains("\"resources\""), "{stdout}");
    let source = fs::read(input).unwrap();
    let encoded = fs::read(output).unwrap();
    let before =
        MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
    let after = MotionPhoto::parse(Input::SingleFile(&encoded), ParseOptions::strict()).unwrap();
    let gain_maps = |asset: &MotionPhoto<'_>| {
        asset
            .resources()
            .filter(|r| {
                r.vendor_role
                    .as_deref()
                    .is_some_and(|s| s.eq_ignore_ascii_case("GainMap"))
            })
            .map(|r| {
                let mut bytes = Vec::new();
                asset.extract(r, &mut bytes).unwrap();
                bytes
            })
            .collect::<Vec<_>>()
    };
    assert!(
        !gain_maps(&before).is_empty(),
        "fixture must contain an XMP GainMap entry"
    );
    assert_eq!(gain_maps(&before), gain_maps(&after));
}

#[test]
fn explicit_mp42_relabel_changes_only_ftyp_brand_and_keeps_encoded_video() {
    let dir = tempfile::tempdir().unwrap();
    // Force a non-mp42 major brand while preserving the actual sample tracks.
    // The unmodified Vivo fixture already uses mp42, so it cannot exercise
    // the opt-in rewrite branch by itself.
    let mut source = fs::read(fixture("vivo/android-v1-sdr-01.jpg")).unwrap();
    let ftyp = source
        .windows(4)
        .rposition(|bytes| bytes == b"ftyp")
        .unwrap();
    assert_eq!(&source[ftyp + 4..ftyp + 8], b"mp42");
    source[ftyp + 4..ftyp + 8].copy_from_slice(b"isom");
    let input = dir.path().join("sourceMP.jpg");
    fs::write(&input, &source).unwrap();
    let output = dir.path().join("mp42MP.jpg");
    let (status, stdout, error) = run_with_args(vec![
        "convert".into(),
        "--input".into(),
        input.as_os_str().into(),
        "--output".into(),
        output.as_os_str().into(),
        "--motion-target".into(),
        "google-jpeg".into(),
        "--motion-video-compat".into(),
        "relabel-mp42".into(),
        "--motion-report-json".into(),
    ]);
    assert_eq!(status, 0, "{error}");
    let encoded = fs::read(output).unwrap();
    let before =
        MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
    let after = MotionPhoto::parse(Input::SingleFile(&encoded), ParseOptions::strict()).unwrap();
    let video_before = before.motion_video_bytes().unwrap();
    let video_after = after.motion_video_bytes().unwrap();
    let mut expected = video_before.to_vec();
    assert_eq!(&expected[8..12], b"isom");
    expected[8..12].copy_from_slice(b"mp42");
    assert_eq!(expected, video_after);
    assert_eq!(
        xdremux_motion_photo::apple::media_payloads(before.motion_video_bytes().unwrap()).unwrap(),
        xdremux_motion_photo::apple::media_payloads(after.motion_video_bytes().unwrap()).unwrap()
    );
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["video_container_policy"], "relabel-mp42");
    assert_eq!(report["video_brand_changed"], true);
    assert!(report["resources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|resource| {
            resource["operations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|operation| {
                    operation["disposition"] == "Rewritten"
                        && operation["reason"]
                            .as_str()
                            .is_some_and(|r| r.contains("mp42"))
                })
        }));
}

#[test]
fn apple_pair_video_cannot_be_confused_with_still_or_source_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = fixture("samsung/heif-ultrahdr-01.heic");
    let output = dir.path().join("unsafeMP.heic");
    let (status, _, error) = run_with_args(vec![
        "convert".into(),
        "--input".into(),
        input.as_os_str().into(),
        "--motion-video".into(),
        input.as_os_str().into(),
        "--output".into(),
        output.as_os_str().into(),
        "--motion-target".into(),
        "google-heic".into(),
    ]);
    assert_eq!(status, 1);
    assert!(error.contains("distinct"), "{error}");
    assert!(!output.exists());
}

#[test]
fn mismatched_apple_pair_identifiers_fail_before_publication() {
    let dir = tempfile::tempdir().unwrap();
    let source = fixture("samsung/heif-ultrahdr-01.heic");
    let first = dir.path().join("first.heic");
    let second = dir.path().join("second.heic");
    assert_eq!(run(&source, &first, "apple-pair").0, 0);
    assert_eq!(run(&source, &second, "apple-pair").0, 0);
    let output = dir.path().join("unsafeMP.heic");
    let (status, _, error) = run_with_args(vec![
        "convert".into(),
        "--input".into(),
        first.as_os_str().into(),
        "--motion-video".into(),
        second.with_extension("mov").as_os_str().into(),
        "--motion-target".into(),
        "google-heic".into(),
        "--output".into(),
        output.as_os_str().into(),
    ]);
    assert_eq!(
        status, 1,
        "unexpected success for mismatched Apple identity"
    );
    assert!(
        error.contains("identifier") || error.contains("Identifier"),
        "{error}"
    );
    assert!(!output.exists());
}

#[test]
fn apple_android_heic_roundtrip_preserves_gain_map_and_movie_samples() {
    let dir = tempfile::tempdir().unwrap();
    let apple_still = dir.path().join("start.heic");
    let source = fixture("samsung/heif-ultrahdr-01.heic");
    let (status, _, error) = run(&source, &apple_still, "apple-pair");
    assert_eq!(status, 0, "{error}");

    let google = dir.path().join("firstMP.heic");
    let (status, stdout, error) = run_with_args(vec![
        "convert".into(),
        "--input".into(),
        apple_still.as_os_str().into(),
        "--motion-video".into(),
        apple_still.with_extension("mov").as_os_str().into(),
        "--motion-target".into(),
        "google-heic".into(),
        "--output".into(),
        google.as_os_str().into(),
        "--motion-report-json".into(),
    ]);
    assert_eq!(status, 0, "{error}");
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let before_still = fs::read(&apple_still).unwrap();
    let before_movie = fs::read(apple_still.with_extension("mov")).unwrap();
    let apple = MotionPhoto::parse(
        Input::ApplePair {
            still: &before_still,
            movie: &before_movie,
        },
        ParseOptions::strict(),
    )
    .unwrap();
    assert_eq!(
        report["resources"].as_array().unwrap().len(),
        apple.resources().count()
    );
    assert!(report["resources"].as_array().unwrap().iter().all(|r| {
        r["operations"].as_array().is_some_and(|ops| {
            !ops.is_empty()
                && ops.iter().all(|op| {
                    !matches!(
                        op["disposition"].as_str(),
                        Some("Dropped" | "Unmapped" | "TranscodeRequired")
                    )
                })
        })
    }));
    assert!(!report["generated"].as_array().unwrap().is_empty());
    let first = fs::read(&google).unwrap();
    let first_parsed =
        MotionPhoto::parse(Input::SingleFile(&first), ParseOptions::strict()).unwrap();

    let round_trip_still = dir.path().join("roundtrip.heic");
    let (status, _, error) = run(&google, &round_trip_still, "apple-pair");
    assert_eq!(status, 0, "{error}");
    let round_trip_movie = round_trip_still.with_extension("mov");
    let second_google = dir.path().join("secondMP.heic");
    let (status, _, error) = run_with_args(vec![
        "convert".into(),
        "--input".into(),
        round_trip_still.as_os_str().into(),
        "--motion-video".into(),
        round_trip_movie.as_os_str().into(),
        "--motion-target".into(),
        "google-heic".into(),
        "--output".into(),
        second_google.as_os_str().into(),
    ]);
    assert_eq!(status, 0, "{error}");
    let second = fs::read(second_google).unwrap();
    let second_parsed =
        MotionPhoto::parse(Input::SingleFile(&second), ParseOptions::strict()).unwrap();

    assert_eq!(
        first_parsed.presentation_time(),
        second_parsed.presentation_time()
    );
    assert_eq!(
        xdremux_motion_photo::apple::media_payloads(first_parsed.motion_video_bytes().unwrap())
            .unwrap(),
        xdremux_motion_photo::apple::media_payloads(second_parsed.motion_video_bytes().unwrap())
            .unwrap()
    );
    let aux = |photo: &MotionPhoto<'_>| {
        photo
            .resources()
            .filter(|r| r.role == xdremux_motion_photo::ResourceRole::AuxiliaryImage)
            .map(|r| {
                let mut payload = Vec::new();
                photo.extract(r, &mut payload).unwrap();
                (r.container_item_id, payload)
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(aux(&first_parsed), aux(&second_parsed));
}
