//! The convert entry point must publish a two-image JPEG when the output
//! extension asks for one, and must refuse options that container cannot carry.
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use xdremux_format::dual_jpeg::index_dual_layer_jpeg;

fn fixture(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(path)
}

fn unique_dir() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("xdremux-cli-jpeg-{}-{stamp}", std::process::id()))
}

fn path_arg(path: &Path) -> OsString {
    path.as_os_str().to_owned()
}

fn convert(input: &Path, output: &Path, extra: &[&str]) -> (u8, String, String) {
    let mut args = vec![
        OsString::from("convert"),
        OsString::from("--input"),
        path_arg(input),
        OsString::from("--output"),
        path_arg(output),
    ];
    args.extend(extra.iter().map(OsString::from));
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = xdremux_cli::run_from(args, &mut stdout, &mut stderr);
    (
        code,
        String::from_utf8_lossy(&stdout).into_owned(),
        String::from_utf8_lossy(&stderr).into_owned(),
    )
}

#[test]
fn a_jpeg_destination_publishes_a_two_image_jpeg() {
    let work = unique_dir();
    fs::create_dir_all(&work).expect("temporary directory");
    let input = fixture("proxdr/oppo/find-x7-ultra/lhdr-v2-05.heic");
    let output = work.join("converted.jpg");

    let (code, stdout, stderr) = convert(&input, &output, &[]);
    assert_eq!(code, 0, "conversion failed: {stderr}");
    assert!(stdout.contains("converted:"), "{stdout}");

    let bytes = fs::read(&output).expect("the CLI must publish the JPEG");
    let index = index_dual_layer_jpeg(&bytes).expect("published file must be a two-image JPEG");
    assert!(index.base_has_iso_declaration);
    assert!(index.gain_map_has_iso_parameters);
    assert!(index.base_has_icc, "the Display P3 base must keep its ICC");

    // An independent ISO/JPEG gain-map reader must accept the published file.
    match Command::new("ultrahdr_app").arg("-h").arg("1").output() {
        Ok(_) => {
            let probe = Command::new("ultrahdr_app")
                .args(["-m", "1", "-j"])
                .arg(&output)
                .arg("-P")
                .output()
                .expect("probe must run");
            let text = String::from_utf8_lossy(&probe.stdout);
            assert!(
                probe.status.success() && text.contains("Ultra HDR Image: Yes"),
                "independent reader rejected the published file: {text}"
            );
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipped independent reader: ultrahdr_app is not installed");
        }
        Err(error) => panic!("cannot run ultrahdr_app: {error}"),
    }

    fs::remove_dir_all(work).ok();
}

#[test]
fn a_jpeg_destination_refuses_an_explicit_bt2020_profile() {
    let work = unique_dir();
    fs::create_dir_all(&work).expect("temporary directory");
    let input = fixture("proxdr/oppo/find-x7-ultra/lhdr-v2-05.heic");
    let output = work.join("converted.jpg");

    let (code, _, stderr) = convert(&input, &output, &["--hdr-profile", "bt2020-pq"]);
    assert_eq!(code, 1, "the conversion must fail: {stderr}");
    assert!(stderr.contains("JPEG output"), "{stderr}");
    assert!(
        !output.exists(),
        "a failed conversion must not publish an output"
    );
    fs::remove_dir_all(work).ok();
}
