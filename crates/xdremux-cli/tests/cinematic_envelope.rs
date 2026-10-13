#[test]
fn cinematic_command_exposes_pending_envelope_and_requires_both_paths() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = xdremux_cli::run_from(["cinematic", "--help"], &mut stdout, &mut stderr);
    assert_eq!(status, 0);
    let help = String::from_utf8(stdout).unwrap();
    for option in ["--input", "--output", "--focal-length-35mm", "--aperture"] {
        assert!(help.contains(option), "missing {option}");
    }
    stdout = Vec::new();
    stderr = Vec::new();
    let status = xdremux_cli::run_from(
        ["cinematic", "--input", "input.mov"],
        &mut stdout,
        &mut stderr,
    );
    assert_ne!(status, 0);
    assert!(String::from_utf8(stderr).unwrap().contains("--output"));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn cinematic_command_rejects_unsupported_platform_before_opening_input() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = xdremux_cli::run_from(
        [
            "cinematic",
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
