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
    assert_eq!(
        code,
        0,
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&stdout),
        String::from_utf8_lossy(&stderr)
    );
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
    assert!(first["successes"][0]["omitted_resource_count"].is_u64());

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

    let checkpoint = output.join(format!("{DEFAULT_MOTION_PHOTO_CHECKPOINT_NAME}.v4"));
    let checkpoint_text = fs::read_to_string(&checkpoint).unwrap();
    let mut lines = checkpoint_text.lines();
    let header: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
    assert_eq!(header["kind"], "header");
    assert_eq!(header["schema_version"], 4);
    let item: Value = serde_json::from_str(lines.last().unwrap()).unwrap();
    assert_eq!(item["publication"]["committed"], true);
    assert_eq!(item["publication"]["layout"], "pair");
    assert_eq!(item["publication"]["sha256"].as_array().unwrap().len(), 2);
    assert!(item["key"]["source_sha256"]
        .as_str()
        .is_some_and(|value| value.len() == 64));

    let second = run(&output, true);
    assert_eq!(second["processed"], 1);
    assert_eq!(second["succeeded"], 1);
    assert_eq!(second["skipped_existing"], 1);
    assert_eq!(second["failed"], 0);
    assert_eq!(second["successes"][0]["status"], "skipped-existing");
    assert!(second["successes"][0]["omitted_resource_count"].is_null());
    assert_eq!(fs::read(&image).unwrap(), before_image);
    assert_eq!(fs::read(&video).unwrap(), before_video);

    let checkpoint_text = fs::read_to_string(&checkpoint).unwrap();
    let records = checkpoint_text.lines().collect::<Vec<_>>();
    assert_eq!(
        records.len(),
        3,
        "a successful reuse does not append another commit"
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn checkpoint_cannot_append_to_a_source_hard_link() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source.heic");
    let original = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/motion-photo/samsung/heif-ultrahdr-01.heic"),
    )
    .unwrap();
    fs::write(&source, &original).unwrap();
    let checkpoint = root.path().join("journal");
    let actual = root.path().join("journal.motion-photo.v4");
    fs::hard_link(&source, actual).unwrap();
    let output = root.path().join("output");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = xdremux_cli::run_from(
        [
            OsString::from("batch"),
            OsString::from("--input"),
            path_arg(&source),
            OsString::from("--output-dir"),
            path_arg(&output),
            OsString::from("--checkpoint"),
            path_arg(&checkpoint),
            OsString::from("--json"),
        ],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(fs::metadata(&source).unwrap().len(), original.len() as u64);
    assert!(
        fs::read(&source).unwrap() == original,
        "source bytes changed"
    );
    assert_ne!(code, 0);
    assert!(
        !output.exists(),
        "invalid checkpoint must fail before publication"
    );
}

#[test]
fn resume_reconciles_published_pair_when_commit_and_tail_are_interrupted() {
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("out");
    let first = run(&output, false);
    let outputs = first["successes"][0]["outputs"].as_array().unwrap();
    let image = PathBuf::from(outputs[0].as_str().unwrap());
    let video = PathBuf::from(outputs[1].as_str().unwrap());
    let original_image = fs::read(&image).unwrap();
    let original_video = fs::read(&video).unwrap();
    let checkpoint = output.join(format!("{DEFAULT_MOTION_PHOTO_CHECKPOINT_NAME}.v4"));
    let journal = fs::read_to_string(&checkpoint).unwrap();
    let mut lines = journal.lines();
    let header = lines.next().unwrap();
    let prepared = lines.next().unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(prepared).unwrap()["publication"]["committed"],
        false
    );
    let torn = format!("{header}\n{prepared}\n{{\"kind\":\"publication\"");
    fs::write(&checkpoint, torn).unwrap();

    let result = run(&output, true);
    let resumed_outputs = result["successes"][0]["outputs"].as_array().unwrap();
    assert_eq!(resumed_outputs.len(), 2);
    for (path, expected) in resumed_outputs.iter().zip([&image, &video]) {
        assert_eq!(
            fs::canonicalize(path.as_str().unwrap()).unwrap(),
            fs::canonicalize(expected).unwrap()
        );
    }
    assert_eq!(result["skipped_existing"], 1);
    assert!(fs::read(image).unwrap() == original_image);
    assert!(fs::read(video).unwrap() == original_video);
    let repaired = fs::read_to_string(checkpoint).unwrap();
    let records = repaired
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 3);
    assert_eq!(records[2]["publication"]["committed"], true);
    assert_eq!(
        fs::read_dir(output).unwrap().count(),
        3,
        "only one pair and its journal"
    );
}
