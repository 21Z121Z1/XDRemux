use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use xdremux_motion_photo::{Input, MotionPhoto, ParseOptions, TargetProfile, WritePolicy};

fn fixture(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/motion-photo")
        .join(relative)
}

fn run(args: &[OsString]) -> (u8, Value, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = xdremux_cli::run_from(args.to_vec(), &mut stdout, &mut stderr);
    let value = serde_json::from_slice(&stdout).expect("batch JSON receipt");
    (code, value, String::from_utf8(stderr).unwrap())
}

fn args(input: &Path, output: &Path, target: &str, resume: bool) -> Vec<OsString> {
    let mut args = vec![
        "batch".into(),
        "--input".into(),
        input.as_os_str().to_owned(),
        "--output-dir".into(),
        output.as_os_str().to_owned(),
        "--motion-target".into(),
        target.into(),
        "--json".into(),
    ];
    if resume {
        args.push("--resume".into());
    }
    args
}

#[test]
fn google_jpeg_batch_preserves_movie_and_ultra_hdr_and_skips_by_provenance() {
    let root = tempfile::tempdir().unwrap();
    let sources = root.path().join("sources");
    let destination = root.path().join("destination");
    fs::create_dir_all(&sources).unwrap();
    for (name, fixture_name) in [
        ("sdr", "vivo/android-v1-sdr-01.jpg"),
        ("hdr", "xiaomi/android-v1-ultrahdr-01.jpg"),
    ] {
        fs::copy(fixture(fixture_name), sources.join(format!("{name}.jpg"))).unwrap();
    }
    let arguments = vec![
        "batch".into(),
        "--input-dir".into(),
        sources.clone().into_os_string(),
        "--output-dir".into(),
        destination.clone().into_os_string(),
        "--motion-target".into(),
        "google-jpeg".into(),
        "--jobs".into(),
        "2".into(),
        "--resume".into(),
        "--json".into(),
    ];
    let (code, first, stderr) = run(&arguments);
    assert_eq!(code, 0, "{stderr}: {first}");
    assert_eq!(first["succeeded"], 2);
    for (name, fixture_name) in [
        ("sdr", "vivo/android-v1-sdr-01.jpg"),
        ("hdr", "xiaomi/android-v1-ultrahdr-01.jpg"),
    ] {
        let output = destination.join(format!("{name}.xdremuxMP.jpg"));
        assert!(output.is_file());
        assert!(!output.with_extension("mov").exists());
        let source = fs::read(fixture(fixture_name)).unwrap();
        let encoded = fs::read(&output).unwrap();
        let before =
            MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
        let after =
            MotionPhoto::parse(Input::SingleFile(&encoded), ParseOptions::strict()).unwrap();
        after.validate().unwrap();
        assert_eq!(
            before.motion_video_bytes().unwrap(),
            after.motion_video_bytes().unwrap()
        );
        assert_eq!(before.presentation_time(), after.presentation_time());
        if name == "hdr" {
            let source_gain_maps = before
                .resources()
                .filter(|resource| {
                    resource
                        .vendor_role
                        .as_deref()
                        .is_some_and(|role| role.eq_ignore_ascii_case("GainMap"))
                })
                .collect::<Vec<_>>();
            assert!(
                !source_gain_maps.is_empty(),
                "Ultra HDR test must exercise a gain-map resource"
            );
            for resource in source_gain_maps {
                let preserved = after
                    .resources()
                    .find(|candidate| {
                        candidate.role == resource.role
                            && candidate.vendor_role == resource.vendor_role
                            && candidate.mime == resource.mime
                    })
                    .expect("gain-map directory entry must survive");
                let mut original = Vec::new();
                let mut written = Vec::new();
                before.extract(resource, &mut original).unwrap();
                after.extract(preserved, &mut written).unwrap();
                assert_eq!(original, written, "gain-map payload must be byte-identical");
            }
        }
    }
    assert_eq!(first["successes"][0]["kind"], "motion-photo");
    assert_eq!(first["successes"][0]["omitted_resource_count"], 0);

    let checkpoint = destination.join(".xdremux-motion-photo-checkpoint.jsonl");
    let records = fs::read_to_string(&checkpoint).unwrap();
    assert!(records.contains("\"outputTarget\":\"google-jpeg\""));
    assert!(records.contains("\"outputSHA256\""));
    assert!(!records.contains("\"outputVideoPath\""));
    let before = fs::read(destination.join("hdr.xdremuxMP.jpg")).unwrap();
    let (code, second, stderr) = run(&arguments);
    assert_eq!(code, 0, "{stderr}: {second}");
    assert_eq!(second["skipped_existing"], 2);
    assert_eq!(
        second["successes"][0]["omitted_resource_count"],
        Value::Null
    );
    assert_eq!(
        fs::read(destination.join("hdr.xdremuxMP.jpg")).unwrap(),
        before
    );
}

#[test]
fn google_batch_refuses_tampered_output_and_recovers_missing_output() {
    let root = tempfile::tempdir().unwrap();
    let input = fixture("vivo/android-v1-sdr-01.jpg");
    let destination = root.path().join("out");
    let arguments = args(&input, &destination, "google-jpeg", true);
    let (code, _, stderr) = run(&arguments);
    assert_eq!(code, 0, "{stderr}");
    let output = destination.join("android-v1-sdr-01.xdremuxMP.jpg");
    let original = fs::read(&output).unwrap();
    fs::write(&output, b"different image").unwrap();
    let (code, receipt, _) = run(&arguments);
    assert_eq!(code, 1);
    assert_eq!(receipt["failed"], 1);
    assert!(receipt["failures"][0]["error"]
        .as_str()
        .unwrap()
        .contains("provenance"));
    assert_eq!(fs::read(&output).unwrap(), b"different image");

    fs::remove_file(&output).unwrap();
    let (code, receipt, stderr) = run(&arguments);
    assert_eq!(code, 0, "{stderr}: {receipt}");
    assert_eq!(receipt["successes"][0]["status"], "converted");
    assert_eq!(fs::read(&output).unwrap(), original);
}

#[test]
fn google_batch_reserves_same_stem_outputs_and_does_not_rescan_them() {
    let root = tempfile::tempdir().unwrap();
    let input_dir = root.path().join("input");
    fs::create_dir_all(&input_dir).unwrap();
    let source = fixture("vivo/android-v1-sdr-01.jpg");
    fs::copy(&source, input_dir.join("capture.jpg")).unwrap();
    fs::copy(&source, input_dir.join("capture.jpeg")).unwrap();
    let arguments = vec![
        "batch".into(),
        "--input-dir".into(),
        input_dir.clone().into_os_string(),
        "--motion-target".into(),
        "google-jpeg".into(),
        "--jobs".into(),
        "2".into(),
        "--json".into(),
    ];
    let (code, first, stderr) = run(&arguments);
    assert_eq!(code, 0, "{stderr}: {first}");
    assert_eq!(first["processed"], 2);
    assert!(input_dir.join("capture.xdremuxMP.jpg").exists());
    assert!(input_dir.join("capture.xdremux (2)MP.jpg").exists());
    let (code, second, stderr) = run(&arguments);
    assert_eq!(code, 0, "{stderr}: {second}");
    assert_eq!(
        second["processed"], 2,
        "generated MP files must not become inputs"
    );
}

#[test]
fn google_heic_batch_strict_readback_and_route_specific_resume() {
    let root = tempfile::tempdir().unwrap();
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
    let composed = clean
        .plan(TargetProfile::AndroidHeif, WritePolicy::default())
        .execute()
        .unwrap();
    let input = root.path().join("sourceMP.heic");
    fs::write(&input, &composed.files[0].bytes).unwrap();
    let destination = root.path().join("out");
    let arguments = args(&input, &destination, "google-heic", true);
    let (code, receipt, stderr) = run(&arguments);
    assert_eq!(code, 0, "{stderr}: {receipt}");
    let output = destination.join("sourceMP.xdremuxMP.heic");
    let bytes = fs::read(&output).unwrap();
    let parsed = MotionPhoto::parse(Input::SingleFile(&bytes), ParseOptions::strict()).unwrap();
    parsed.validate().unwrap();
    let (code, receipt, stderr) = run(&arguments);
    assert_eq!(code, 0, "{stderr}: {receipt}");
    assert_eq!(receipt["skipped_existing"], 1);

    let mut changed_policy = arguments.clone();
    changed_policy.extend(["--motion-video-compat".into(), "relabel-mp42".into()]);
    let (code, receipt, stderr) = run(&changed_policy);
    assert_eq!(code, 0, "{stderr}: {receipt}");
    assert_eq!(
        receipt["skipped_existing"], 0,
        "video policy is part of checkpoint identity"
    );
    assert!(destination.join("sourceMP.xdremux (2)MP.heic").is_file());
}

#[test]
fn apple_pair_and_google_routes_keep_independent_resume_records() {
    let root = tempfile::tempdir().unwrap();
    let input = fixture("vivo/android-v1-sdr-01.jpg");
    let destination = root.path().join("output");
    let apple = args(&input, &destination, "apple-pair", true);
    let google = args(&input, &destination, "google-jpeg", true);
    let (code, first, stderr) = run(&apple);
    assert_eq!(code, 0, "{stderr}: {first}");
    assert_eq!(first["successes"][0]["kind"], "live-photo");
    let (code, second, stderr) = run(&google);
    assert_eq!(code, 0, "{stderr}: {second}");
    assert_eq!(second["successes"][0]["kind"], "motion-photo");
    let (code, third, stderr) = run(&apple);
    assert_eq!(code, 0, "{stderr}: {third}");
    assert_eq!(
        third["skipped_existing"], 1,
        "Google checkpoint must not replace Apple identity"
    );
    let (code, fourth, stderr) = run(&google);
    assert_eq!(code, 0, "{stderr}: {fourth}");
    assert_eq!(
        fourth["skipped_existing"], 1,
        "Apple checkpoint must not replace Google identity"
    );
}

#[test]
fn google_batch_relabel_only_changes_requested_video_brand() {
    let root = tempfile::tempdir().unwrap();
    let mut source = fs::read(fixture("vivo/android-v1-sdr-01.jpg")).unwrap();
    let ftyp = source.windows(4).rposition(|part| part == b"ftyp").unwrap();
    assert_eq!(&source[ftyp + 4..ftyp + 8], b"mp42");
    source[ftyp + 4..ftyp + 8].copy_from_slice(b"isom");
    let input = root.path().join("sourceMP.jpg");
    let destination = root.path().join("out");
    fs::write(&input, &source).unwrap();
    let mut arguments = args(&input, &destination, "google-jpeg", false);
    arguments.extend(["--motion-video-compat".into(), "relabel-mp42".into()]);
    let (code, receipt, stderr) = run(&arguments);
    assert_eq!(code, 0, "{stderr}: {receipt}");
    let encoded = fs::read(destination.join("sourceMP.xdremuxMP.jpg")).unwrap();
    let before =
        MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
    let after = MotionPhoto::parse(Input::SingleFile(&encoded), ParseOptions::strict()).unwrap();
    let mut expected_movie = before.motion_video_bytes().unwrap().to_vec();
    assert_eq!(&expected_movie[8..12], b"isom");
    expected_movie[8..12].copy_from_slice(b"mp42");
    assert_eq!(after.motion_video_bytes().unwrap(), expected_movie);
    assert_eq!(before.presentation_time(), after.presentation_time());
}

#[test]
fn google_batch_fails_closed_for_unsupported_vendor_and_avif_targets() {
    let root = tempfile::tempdir().unwrap();
    let original = fixture("samsung/heif-ultrahdr-01.heic");
    let output = root.path().join("out");
    let (code, receipt, _) = run(&args(&original, &output, "google-heic", false));
    assert_eq!(code, 1);
    assert_eq!(receipt["failures"].as_array().unwrap().len(), 1);
    assert!(!output.join("heif-ultrahdr-01.xdremuxMP.heic").exists());

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = xdremux_cli::run_from(
        [
            "batch",
            "--input",
            original.to_str().unwrap(),
            "--motion-target",
            "google-avif",
            "--json",
        ],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(
        code, 2,
        "AVIF is a matrix gap, not an available product writer"
    );
    assert!(String::from_utf8(stderr).unwrap().contains("google-avif"));
}

#[test]
fn categorized_google_batch_keeps_success_when_another_item_is_incompatible() {
    let root = tempfile::tempdir().unwrap();
    let sources = root.path().join("sources");
    let destination = root.path().join("destination");
    fs::create_dir_all(&sources).unwrap();
    fs::copy(
        fixture("vivo/android-v1-sdr-01.jpg"),
        sources.join("good.jpg"),
    )
    .unwrap();
    fs::copy(
        fixture("samsung/heif-ultrahdr-01.heic"),
        sources.join("bad.heic"),
    )
    .unwrap();
    let arguments = vec![
        "batch".into(),
        "--input-dir".into(),
        sources.into_os_string(),
        "--output-dir".into(),
        destination.clone().into_os_string(),
        "--motion-target".into(),
        "google-jpeg".into(),
        "--categorize".into(),
        "--jobs".into(),
        "2".into(),
        "--json".into(),
    ];
    let (code, receipt, stderr) = run(&arguments);
    assert_eq!(code, 1, "{stderr}: {receipt}");
    assert_eq!(receipt["processed"], 2);
    assert_eq!(receipt["succeeded"], 1);
    assert_eq!(receipt["failed"], 1);
    let output = PathBuf::from(receipt["successes"][0]["outputs"][0].as_str().unwrap());
    assert!(output.starts_with(&destination));
    assert!(output
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .ends_with("MP.jpg"));
    assert!(output.is_file());
    assert!(!output.with_extension("mov").exists());
    assert!(!PathBuf::from(receipt["failures"][0]["output"].as_str().unwrap()).exists());
}
