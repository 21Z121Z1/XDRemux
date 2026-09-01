use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use xdremux_format::ChromaSampling;
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
    std::env::temp_dir().join(format!(
        "xdremux-cli-convert-{}-{stamp}",
        std::process::id()
    ))
}

fn path_arg(path: &Path) -> OsString {
    path.as_os_str().to_owned()
}

#[test]
fn convert_command_executes_full_rust_proxdr_pipeline() {
    let work = unique_dir();
    fs::create_dir_all(&work).expect("temporary test directory must be created");
    let input = public_proxdr_fixture();
    let output = work.join("output.heic");

    let args = vec![
        OsString::from("convert"),
        OsString::from("--input"),
        path_arg(&input),
        OsString::from("--output"),
        path_arg(&output),
    ];
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = xdremux_cli::run_from(args, &mut stdout, &mut stderr);

    assert_eq!(
        code,
        0,
        "CLI conversion failed: {}",
        String::from_utf8_lossy(&stderr)
    );
    assert!(stderr.is_empty());
    assert!(String::from_utf8(stdout)
        .expect("CLI stdout must be UTF-8")
        .contains("converted:"));

    let converted = fs::read(&output).expect("CLI must atomically publish the output file");
    let structure =
        validate_gain_map_structure(&converted).expect("CLI output Gain Map graph must validate");
    assert_eq!(structure.channel_count, 3);
    assert_eq!(structure.chroma_sampling, ChromaSampling::Yuv444);
    assert_eq!(structure.luma_bit_depth, 8);
    assert_eq!(structure.chroma_bit_depth, 8);
    assert!(structure.width > 0);
    assert!(structure.height > 0);
    assert!(!converted
        .windows(b"local.lhdr.gainmap".len())
        .any(|window| window == b"local.lhdr.gainmap"));

    fs::remove_dir_all(work).expect("temporary test directory must be removed");
}
