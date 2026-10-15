#[test]
fn cinematic_is_a_conversion_flag_on_both_entry_points() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = xdremux_cli::run_from(["convert", "--help"], &mut stdout, &mut stderr);
    assert_eq!(status, 0);
    let help = String::from_utf8(stdout).unwrap();
    for option in [
        "--input",
        "--output",
        "--apple-cinematic",
        "--focal-length-35mm",
        "--aperture",
    ] {
        assert!(help.contains(option), "missing {option}");
    }
    stdout = Vec::new();
    stderr = Vec::new();
    let status = xdremux_cli::run_from(["batch", "--help"], &mut stdout, &mut stderr);
    assert_eq!(status, 0);
    assert!(String::from_utf8(stdout)
        .unwrap()
        .contains("--apple-cinematic"));
    assert!(stderr.is_empty());
}

#[test]
fn cinematic_configuration_requires_the_feature_and_rejects_image_intents() {
    for command in ["convert", "batch"] {
        for other in [
            "--apple-styles",
            "--apple-portrait",
            "--apple-portrait-oppo",
            "--oppo-compatible",
        ] {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            assert_eq!(
                xdremux_cli::run_from(
                    [command, "--input", "clip.mov", "--apple-cinematic", other],
                    &mut stdout,
                    &mut stderr
                ),
                2
            );
            assert!(String::from_utf8(stderr)
                .unwrap()
                .contains("cannot be used"));
        }
        for parameter in ["--focal-length-35mm", "--aperture"] {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            assert_eq!(
                xdremux_cli::run_from(
                    [command, "--input", "clip.mov", parameter, "3"],
                    &mut stdout,
                    &mut stderr
                ),
                2
            );
            assert!(String::from_utf8(stderr)
                .unwrap()
                .contains("--apple-cinematic"));
        }
    }
}

#[cfg(not(target_os = "macos"))]
#[test]
fn cinematic_command_rejects_unsupported_platform_before_opening_input() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = xdremux_cli::run_from(
        [
            "convert",
            "--apple-cinematic",
            "--input",
            "missing.mp4",
            "--output",
            "pending.mov",
        ],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(status, 1);
    assert!(stdout.is_empty());
    assert!(String::from_utf8(stderr)
        .unwrap()
        .contains("requires macOS"));
}
