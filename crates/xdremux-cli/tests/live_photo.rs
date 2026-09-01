use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use xdremux_motion_photo::{
    read_apple_content_identifier, read_live_photo_content_identifier, read_live_photo_still_time,
    validate_live_photo_movie,
};

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/motion-photo/samsung/heif-ultrahdr-01.heic")
}

fn arg(path: &Path) -> OsString {
    path.as_os_str().to_owned()
}

#[test]
fn convert_command_executes_full_rust_live_photo_pipeline() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "xdremux-cli-live-photo-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    let input = fixture();
    let image = dir.join("capture.heic");
    let movie = dir.join("capture.mov");
    let args = vec![
        OsString::from("convert"),
        OsString::from("--input"),
        arg(&input),
        OsString::from("--output"),
        arg(&image),
    ];
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = xdremux_cli::run_from(args, &mut stdout, &mut stderr);
    assert_eq!(
        code,
        0,
        "CLI Live Photo conversion failed: {}",
        String::from_utf8_lossy(&stderr)
    );
    assert!(stderr.is_empty());
    assert!(image.is_file());
    assert!(movie.is_file());

    let still = fs::read(&image).unwrap();
    let video = fs::read(&movie).unwrap();
    let still_id = read_apple_content_identifier(&still).unwrap().unwrap();
    let movie_id = read_live_photo_content_identifier(&video).unwrap().unwrap();
    assert_eq!(still_id, movie_id);
    let still_time = read_live_photo_still_time(&video).unwrap().unwrap();
    validate_live_photo_movie(&video, &movie_id, still_time).unwrap();
    assert!(String::from_utf8(stdout).unwrap().contains(" + "));
    fs::remove_dir_all(dir).unwrap();
}
