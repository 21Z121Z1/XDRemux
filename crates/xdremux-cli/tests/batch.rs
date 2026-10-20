use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use xdremux_heif::validate_gain_map_structure;

fn public_proxdr_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/proxdr/oppo/find-x6-pro/lhdr-v1-01.heic")
}

fn unique_dir() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("xdremux-cli-batch-{}-{stamp}", std::process::id()))
}

fn path_arg(path: &Path) -> OsString {
    path.as_os_str().to_owned()
}

#[test]
fn batch_command_is_deterministic_machine_readable_and_failure_isolating() {
    let work = unique_dir();
    let inputs = work.join("inputs");
    let outputs = work.join("outputs");
    fs::create_dir_all(&inputs).expect("input directory must be created");

    let good = inputs.join("good.heic");
    let bad = inputs.join("bad.heic");
    fs::copy(public_proxdr_fixture(), &good).expect("fixture must be copied");
    fs::write(&bad, [0_u8; 32]).expect("invalid fixture must be written");

    let args = vec![
        OsString::from("batch"),
        OsString::from("--input-dir"),
        path_arg(&inputs),
        OsString::from("--output-dir"),
        path_arg(&outputs),
        OsString::from("--json"),
    ];
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = xdremux_cli::run_from(args, &mut stdout, &mut stderr);

    assert_eq!(
        code, 1,
        "one bad asset should make the batch partially fail without aborting the good conversion"
    );
    assert!(
        stderr.is_empty(),
        "machine-readable runtime failures belong in the JSON receipt: {}",
        String::from_utf8_lossy(&stderr)
    );

    let receipt: Value =
        serde_json::from_slice(&stdout).expect("batch --json must emit one valid JSON document");
    assert_eq!(receipt["schema_version"], 1);
    assert_eq!(receipt["command"], "batch");
    assert_eq!(receipt["processed"], 2);
    assert_eq!(receipt["succeeded"], 1);
    assert_eq!(receipt["failed"], 1);
    assert!(receipt["successes"][0]["omitted_resource_count"].is_null());

    let good_output = outputs.join("good.xdremux.heic");
    let bad_output = outputs.join("bad.xdremux.heic");
    assert!(good_output.is_file());
    assert!(!bad_output.exists());

    let converted = fs::read(&good_output).expect("successful batch output must exist");
    validate_gain_map_structure(&converted)
        .expect("successful batch output must be a valid ISO Gain Map HEIF");

    fs::remove_dir_all(work).expect("temporary test directory must be removed");
}

#[test]
fn live_photo_batch_reports_direct_conversion_omissions_and_preserves_source_media() {
    use xdremux_motion_photo::{apple, Input, MotionPhoto, ParseOptions};
    use xdremux_runtime::PortableRuntime;

    let work = unique_dir();
    fs::create_dir_all(&work).unwrap();
    for (name, relative) in [
        ("coloros16", "oppo/coloros16-dualstream-ultrahdr-01.jpg"),
        ("samsung", "samsung/jpeg-ultrahdr-01.jpg"),
    ] {
        let input = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/motion-photo")
            .join(relative);
        let before = fs::read(&input).unwrap();
        let parsed =
            MotionPhoto::parse(Input::SingleFile(&before), ParseOptions::compatible()).unwrap();
        let mut source_movie = Vec::new();
        parsed
            .extract(parsed.motion_video(), &mut source_movie)
            .unwrap();
        let source_media = apple::media_payloads(&source_movie).unwrap();
        let direct = PortableRuntime::new()
            .convert_motion_photo_file(&before, &input, work.join(format!("{name}-direct.heic")))
            .unwrap();
        let omitted = direct.omitted_resource_count();
        if name == "coloros16" {
            assert!(
                omitted > 0,
                "dual-stream source must report omitted resources"
            );
        }

        let output = work.join(format!("{name}-batch"));
        let arguments = vec![
            OsString::from("batch"),
            OsString::from("--input"),
            path_arg(&input),
            OsString::from("--output-dir"),
            path_arg(&output),
        ];
        let mut json_arguments = arguments.clone();
        json_arguments.push(OsString::from("--json"));
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(
            xdremux_cli::run_from(json_arguments, &mut stdout, &mut stderr),
            0,
            "{}",
            String::from_utf8_lossy(&stderr)
        );
        let receipt: Value = serde_json::from_slice(&stdout).unwrap();
        assert_eq!(receipt["successes"][0]["omitted_resource_count"], omitted);
        let movie = PathBuf::from(receipt["successes"][0]["outputs"][1].as_str().unwrap());
        let movie_bytes = fs::read(movie).unwrap();
        assert_eq!(
            apple::media_payloads(&movie_bytes).unwrap(),
            source_media,
            "batch must retain the source's compressed video/audio payloads"
        );

        let mut resumed = arguments;
        resumed.extend([OsString::from("--resume"), OsString::from("--json")]);
        stdout.clear();
        assert_eq!(
            xdremux_cli::run_from(resumed.clone(), &mut stdout, &mut stderr),
            0
        );
        let receipt: Value = serde_json::from_slice(&stdout).unwrap();
        assert_eq!(receipt["skipped_existing"], 1);
        assert!(
            receipt["successes"][0]["omitted_resource_count"].is_null(),
            "reuse has no conversion report and must not claim zero omissions"
        );
        resumed.pop();
        stdout.clear();
        assert_eq!(xdremux_cli::run_from(resumed, &mut stdout, &mut stderr), 0);
        assert!(!String::from_utf8(stdout)
            .unwrap()
            .contains("source resources remain"));

        let human_arguments = vec![
            OsString::from("batch"),
            OsString::from("--input"),
            path_arg(&input),
            OsString::from("--output-dir"),
            path_arg(&work.join(format!("{name}-human"))),
        ];
        let mut stdout = Vec::new();
        assert_eq!(
            xdremux_cli::run_from(human_arguments, &mut stdout, &mut stderr),
            0
        );
        let text = String::from_utf8(stdout).unwrap();
        if omitted > 0 {
            assert!(text.contains(&format!(
                "{omitted} source resources remain only in the original input"
            )));
        } else {
            assert!(!text.contains("source resources remain"));
        }
        assert_eq!(
            fs::read(&input).unwrap(),
            before,
            "direct, batch, and reuse must preserve the original input exactly"
        );
    }
    fs::remove_dir_all(work).unwrap();
}
