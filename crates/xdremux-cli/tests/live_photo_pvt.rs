use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/motion-photo/samsung/heif-ultrahdr-01.heic")
}

fn run(arguments: &[&std::ffi::OsStr]) -> (u8, String, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = xdremux_cli::run_from(
        arguments.iter().map(|value| OsString::from(*value)),
        &mut stdout,
        &mut stderr,
    );
    (
        code,
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    )
}

fn convert(source: &Path, output: &Path, format: &str) -> (u8, String, String) {
    run(&[
        "convert".as_ref(),
        "--input".as_ref(),
        source.as_os_str(),
        "--output".as_ref(),
        output.as_os_str(),
        "--live-photo-output".as_ref(),
        format.as_ref(),
    ])
}

#[test]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn pvt_conversion_and_validation_publish_a_real_directory_and_preserve_source() {
    let root = tempfile::tempdir().unwrap();
    let source = fixture();
    let original = fs::read(&source).unwrap();
    let output = root.path().join("capture.pvt");
    let (code, stdout, stderr) = convert(&source, &output, "pvt");
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("capture.pvt"), "{stdout}");
    assert!(output.is_dir());
    assert!(!root.path().join("capture.heic").exists());
    assert!(!root.path().join("capture.mov").exists());
    let mut members = fs::read_dir(&output)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    members.sort();
    assert_eq!(members, ["capture.heic", "capture.mov", "metadata.plist"]);
    let (code, stdout, stderr) = run(&["validate".as_ref(), output.as_os_str(), "--json".as_ref()]);
    assert_eq!(code, 0, "{stderr}");
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["kind"], "live-photo");
    assert_eq!(report["valid"], true);
    assert!(
        fs::read(&source).unwrap() == original,
        "source bytes changed"
    );
}

#[test]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn pvt_batch_resumes_only_its_recorded_container_and_keeps_pair_outputs() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source.heic");
    let original = fs::read(fixture()).unwrap();
    fs::write(&source, &original).unwrap();
    let actual = root.path().join("actual-output-parent");
    fs::create_dir(&actual).unwrap();
    let alias = root.path().join("output-parent-alias");
    std::os::unix::fs::symlink(&actual, &alias).unwrap();
    let output = alias.join("out");
    let batch = |format: &str, resume: bool| {
        let mut args = vec![
            "batch".as_ref(),
            "--input".as_ref(),
            source.as_os_str(),
            "--output-dir".as_ref(),
            output.as_os_str(),
            "--live-photo-output".as_ref(),
            format.as_ref(),
            "--json".as_ref(),
        ];
        if resume {
            args.push("--resume".as_ref());
        }
        let (code, stdout, stderr) = run(&args);
        assert_eq!(code, 0, "{stderr}");
        serde_json::from_str::<serde_json::Value>(&stdout).unwrap()
    };
    let pair = batch("pair", false);
    let pair_paths = pair["successes"][0]["outputs"].as_array().unwrap();
    assert_eq!(pair_paths.len(), 2);
    let pair_bytes = pair_paths
        .iter()
        .map(|path| fs::read(path.as_str().unwrap()).unwrap())
        .collect::<Vec<_>>();
    let pvt = batch("pvt", true);
    assert_eq!(pvt["successes"][0]["status"], "converted");
    let pvt_paths = pvt["successes"][0]["outputs"].as_array().unwrap();
    assert_eq!(pvt_paths.len(), 1);
    let package = PathBuf::from(pvt_paths[0].as_str().unwrap());
    assert_eq!(package.extension().unwrap(), "pvt");
    assert!(package.is_dir());
    let package_bytes = || {
        fs::read_dir(&package)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (entry.file_name(), fs::read(entry.path()).unwrap())
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let before_resume = package_bytes();
    let reused = batch("pvt", true);
    assert_eq!(reused["successes"][0]["status"], "skipped-existing");
    let reused_paths = reused["successes"][0]["outputs"].as_array().unwrap();
    assert_eq!(reused_paths.len(), 1);
    assert_eq!(
        fs::canonicalize(reused_paths[0].as_str().unwrap()).unwrap(),
        fs::canonicalize(&package).unwrap()
    );
    assert!(
        package_bytes() == before_resume,
        "resume changed package bytes"
    );
    let metadata = package.join("metadata.plist");
    let tampered = b"<plist version=\"1.0\"><dict><key>PFVideoComplementMetadataVersionKey</key><integer>1</integer></dict></plist>";
    fs::write(&metadata, tampered).unwrap();
    let packaged_movie = fs::read(package.join(format!(
        "{}.mov",
        package.file_stem().unwrap().to_str().unwrap()
    )))
    .unwrap();
    let (code, stdout, stderr) = run(&[
        "batch".as_ref(),
        "--input".as_ref(),
        source.as_os_str(),
        "--output-dir".as_ref(),
        output.as_os_str(),
        "--live-photo-output".as_ref(),
        "pvt".as_ref(),
        "--resume".as_ref(),
        "--json".as_ref(),
    ]);
    assert_eq!(code, 1, "{stderr}: {stdout}");
    let failed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(failed["failed"], 1);
    assert_eq!(failed["skipped_existing"], 0);
    assert_eq!(fs::read(&metadata).unwrap(), tampered);
    assert!(
        fs::read(package.join(format!(
            "{}.mov",
            package.file_stem().unwrap().to_str().unwrap()
        )))
        .unwrap()
            == packaged_movie,
        "invalid recorded package movie changed"
    );
    for (path, bytes) in pair_paths.iter().zip(pair_bytes) {
        assert!(
            fs::read(path.as_str().unwrap()).unwrap() == bytes,
            "published pair bytes changed"
        );
    }
    assert!(
        fs::read(source).unwrap() == original,
        "source bytes changed"
    );
}

#[test]
fn a_pvt_destination_cannot_disguise_pair_output_or_replace_unknown_entries() {
    let root = tempfile::tempdir().unwrap();
    let source = fixture();
    let unknown = root.path().join("capture.pvt");
    fs::create_dir(&unknown).unwrap();
    let member = unknown.join("keep-original");
    fs::write(&member, b"user bytes").unwrap();
    for format in ["pair", "pvt"] {
        let (code, _, _) = convert(&source, &unknown, format);
        assert_ne!(code, 0);
        assert_eq!(fs::read(&member).unwrap(), b"user bytes");
        assert_eq!(fs::read_dir(&unknown).unwrap().count(), 1);
    }
}

#[test]
fn categorization_keeps_packages_opaque_and_cannot_publish_inside_one() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source.heic");
    let bytes = fs::read(fixture()).unwrap();
    fs::write(&source, &bytes).unwrap();
    let package = root.path().join("nested/unknown.pvt");
    fs::create_dir_all(&package).unwrap();
    let member = package.join("member.heic");
    fs::write(&member, &bytes).unwrap();
    let output = root.path().join("categories");
    let (code, stdout, stderr) = run(&[
        "categorize".as_ref(),
        "--input".as_ref(),
        root.path().as_os_str(),
        "--output-dir".as_ref(),
        output.as_os_str(),
        "--json".as_ref(),
    ]);
    assert_eq!(code, 0, "{stderr}: {stdout}");
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["copied"], 1);
    for input in [&package, &member] {
        let (code, _, _) = run(&[
            "categorize".as_ref(),
            "--input".as_ref(),
            input.as_os_str(),
            "--output-dir".as_ref(),
            output.as_os_str(),
        ]);
        assert_ne!(code, 0);
    }
    let (code, _, stderr) = run(&[
        "categorize".as_ref(),
        "--input".as_ref(),
        source.as_os_str(),
        "--output-dir".as_ref(),
        package.as_os_str(),
    ]);
    assert_ne!(code, 0, "{stderr}");
    assert_eq!(fs::read_dir(&package).unwrap().count(), 1);
    assert!(
        fs::read(member).unwrap() == bytes,
        "categorization changed package member bytes"
    );
}

#[test]
fn relative_output_from_a_package_working_directory_cannot_mutate_it() {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("unknown.pvt");
    fs::create_dir(&package).unwrap();
    fs::write(package.join("keep"), b"original").unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_xdremux"))
        .current_dir(&package)
        .args(["convert", "--input"])
        .arg(fixture())
        .args(["--output", "capture.heic"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("inside an existing PVT"));
    assert_eq!(fs::read_dir(&package).unwrap().count(), 1);
    assert_eq!(fs::read(package.join("keep")).unwrap(), b"original");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn mixed_batch_pvt_keeps_ordinary_hdr_semantics_and_publishes_motion_as_a_package() {
    let root = tempfile::tempdir().unwrap();
    let ordinary = root.path().join("ordinary.heic");
    let motion = root.path().join("motion.heic");
    let ordinary_source = fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/proxdr/oppo/find-x6-pro/lhdr-v1-01.heic"),
    )
    .unwrap();
    let motion_source = fs::read(fixture()).unwrap();
    fs::write(&ordinary, &ordinary_source).unwrap();
    fs::write(&motion, &motion_source).unwrap();
    let output = root.path().join("out");
    let (code, stdout, stderr) = run(&[
        "batch".as_ref(),
        "--input".as_ref(),
        ordinary.as_os_str(),
        "--input".as_ref(),
        motion.as_os_str(),
        "--output-dir".as_ref(),
        output.as_os_str(),
        "--live-photo-output".as_ref(),
        "pvt".as_ref(),
        "--json".as_ref(),
    ]);
    assert_eq!(code, 0, "{stderr}: {stdout}");
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["succeeded"], 2);
    assert!(output.join("motion.xdremux.pvt").is_dir());
    let still = fs::read(output.join("ordinary.xdremux.heic")).unwrap();
    xdremux_heif::validate_gain_map_structure(&still).unwrap();
    assert!(!output.join("ordinary.xdremux.pvt").exists());
    assert!(
        fs::read(&ordinary).unwrap() == ordinary_source,
        "ordinary source changed"
    );
    assert!(
        fs::read(&motion).unwrap() == motion_source,
        "motion source changed"
    );
    let unknown = root.path().join("unknown.pvt");
    fs::create_dir(&unknown).unwrap();
    fs::write(unknown.join("keep"), b"unknown bytes").unwrap();
    let (code, _, error) = convert(&ordinary, &unknown, "pvt");
    assert_eq!(code, 1, "{error}");
    assert!(error.contains("requires Motion Photo input"), "{error}");
    assert_eq!(fs::read(unknown.join("keep")).unwrap(), b"unknown bytes");
    assert!(
        fs::read(&ordinary).unwrap() == ordinary_source,
        "ordinary source changed"
    );
}

#[test]
fn input_scanning_prunes_packages_and_output_planning_preserves_unknown_package_members() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source.heic");
    let original = fs::read(fixture()).unwrap();
    fs::write(&source, &original).unwrap();
    let package = root.path().join("subdir/existing.pvt");
    fs::create_dir_all(&package).unwrap();
    let member = package.join("member.heic");
    fs::write(&member, &original).unwrap();
    let output = root.path().join("out");
    let (code, stdout, stderr) = run(&[
        "batch".as_ref(),
        "--input-dir".as_ref(),
        root.path().as_os_str(),
        "--recursive".as_ref(),
        "--output-dir".as_ref(),
        output.as_os_str(),
        "--json".as_ref(),
    ]);
    assert_eq!(code, 0, "{stderr}: {stdout}");
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["processed"], 1);
    for input in [&package, &member] {
        let (code, _, stderr) = run(&["batch".as_ref(), "--input".as_ref(), input.as_os_str()]);
        assert_ne!(code, 0);
        assert!(stderr.contains("PVT"), "{stderr}");
        let (code, _, stderr) = convert(input, &root.path().join("must-not-extract.heic"), "pair");
        assert_eq!(code, 1);
        assert!(stderr.contains("PVT"), "{stderr}");
        assert!(!root.path().join("must-not-extract.heic").exists());
    }
    let (code, _, stderr) = run(&[
        "batch".as_ref(),
        "--input-dir".as_ref(),
        package.as_os_str(),
        "--recursive".as_ref(),
    ]);
    assert_ne!(code, 0);
    assert!(stderr.contains("PVT"), "{stderr}");
    for (format, destination) in [
        ("pair", package.join("nested/capture.heic")),
        ("pvt", package.join("nested/capture.pvt")),
    ] {
        let (code, _, stderr) = convert(&source, &destination, format);
        assert_eq!(code, 1);
        assert!(stderr.contains("inside an existing PVT"), "{stderr}");
    }
    let (code, _, stderr) = run(&[
        "batch".as_ref(),
        "--input".as_ref(),
        source.as_os_str(),
        "--output-dir".as_ref(),
        package.as_os_str(),
    ]);
    assert_ne!(code, 0);
    assert!(stderr.contains("inside an existing PVT"), "{stderr}");
    assert_eq!(fs::read_dir(&package).unwrap().count(), 1);
    assert!(
        fs::read(&member).unwrap() == original,
        "package member changed"
    );
    assert!(fs::read(source).unwrap() == original, "source changed");
}

#[test]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn unknown_valid_packages_are_never_adopted_and_same_stems_get_distinct_outputs() {
    let root = tempfile::tempdir().unwrap();
    let left = root.path().join("left/source.heic");
    let right = root.path().join("right/source.heic");
    let original = fs::read(fixture()).unwrap();
    for source in [&left, &right] {
        fs::create_dir(source.parent().unwrap()).unwrap();
        fs::write(source, &original).unwrap();
    }
    let output = root.path().join("out");
    fs::create_dir(&output).unwrap();
    let unknown = output.join("source.xdremux.pvt");
    let (code, _, error) = convert(&left, &unknown, "pvt");
    assert_eq!(code, 0, "{error}");
    let members = fs::read_dir(&unknown)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (entry.file_name(), fs::read(entry.path()).unwrap())
        })
        .collect::<Vec<_>>();
    let (code, stdout, stderr) = run(&[
        "batch".as_ref(),
        "--input".as_ref(),
        left.as_os_str(),
        "--input".as_ref(),
        right.as_os_str(),
        "--output-dir".as_ref(),
        output.as_os_str(),
        "--live-photo-output".as_ref(),
        "pvt".as_ref(),
        "--resume".as_ref(),
        "--jobs".as_ref(),
        "2".as_ref(),
        "--json".as_ref(),
    ]);
    assert_eq!(code, 0, "{stderr}: {stdout}");
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["succeeded"], 2);
    assert_eq!(report["skipped_existing"], 0);
    let paths = report["successes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["outputs"][0].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(paths.len(), 2);
    assert!(paths.iter().all(|path| Path::new(path) != unknown));
    for (name, bytes) in members {
        assert!(
            fs::read(unknown.join(name)).unwrap() == bytes,
            "unknown package member changed"
        );
    }
}

#[test]
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn pvt_conversion_on_an_unsupported_publisher_preserves_source_and_reports_recovery() {
    let root = tempfile::tempdir().unwrap();
    let source = fixture();
    let bytes = fs::read(&source).unwrap();
    let output = root.path().join("capture.pvt");
    let (code, _, stderr) = convert(&source, &output, "pvt");
    assert_eq!(code, 1);
    assert!(stderr.contains("unavailable on this platform"), "{stderr}");
    assert!(
        stderr.contains("complete staged package retained at"),
        "{stderr}"
    );
    assert!(!output.exists());
    assert!(fs::read(source).unwrap() == bytes, "source bytes changed");
    let staging = fs::read_dir(root.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.is_dir())
        .unwrap();
    assert_eq!(fs::read_dir(staging).unwrap().count(), 3);
}
