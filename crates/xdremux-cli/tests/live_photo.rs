use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use xdremux_heif::validate_gain_map_structure;
use xdremux_motion_photo::{
    read_apple_content_identifier, read_live_photo_content_identifier, read_live_photo_still_time,
    validate_live_photo_movie,
};

static NEXT_TEMP_DIRECTORY: AtomicU64 = AtomicU64::new(0);

fn fixture(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/motion-photo")
        .join(relative)
}

fn arg(path: &Path) -> OsString {
    path.as_os_str().to_owned()
}

fn exercise(relative: &str, expect_gain_map: bool) {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let sequence = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "xdremux-cli-live-photo-{}-{stamp}-{sequence}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    let input = fixture(relative);
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
        "{relative}: CLI Live Photo conversion failed: {}",
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
    if expect_gain_map {
        validate_gain_map_structure(&still)
            .unwrap_or_else(|error| panic!("{relative}: final Gain Map graph invalid: {error}"));
    }
    assert!(String::from_utf8(stdout).unwrap().contains(" + "));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn convert_command_executes_full_rust_heif_live_photo_pipeline() {
    exercise("samsung/heif-ultrahdr-01.heic", false);
}

#[test]
fn convert_command_executes_full_rust_jpeg_ultrahdr_live_photo_pipeline() {
    exercise("samsung/jpeg-ultrahdr-01.jpg", true);
}
