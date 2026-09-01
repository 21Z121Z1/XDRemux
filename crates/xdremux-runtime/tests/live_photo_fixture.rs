use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use xdremux_motion_photo::{
    read_apple_content_identifier, read_live_photo_content_identifier, read_live_photo_still_time,
    validate_live_photo_movie,
};
use xdremux_runtime::PortableRuntime;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/motion-photo/samsung/heif-ultrahdr-01.heic")
}

#[test]
fn real_samsung_heif_motion_photo_becomes_valid_live_photo_pair() {
    let source_path = fixture();
    let source = fs::read(&source_path).expect("versioned Samsung fixture must be readable");
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "xdremux-runtime-live-photo-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    let image = dir.join("capture.heic");

    let receipt = PortableRuntime::new()
        .convert_motion_photo_file(&source, &source_path, &image)
        .expect("Rust runtime must convert the real HEIF Motion Photo");
    let still = fs::read(&receipt.image).unwrap();
    let movie = fs::read(&receipt.video).unwrap();
    let still_id = read_apple_content_identifier(&still).unwrap().unwrap();
    let movie_id = read_live_photo_content_identifier(&movie).unwrap().unwrap();
    assert_eq!(still_id, receipt.content_identifier);
    assert_eq!(movie_id, receipt.content_identifier);
    let still_time = read_live_photo_still_time(&movie).unwrap().unwrap();
    assert!((still_time - receipt.still_time_seconds).abs() < 1.0 / 600.0);
    validate_live_photo_movie(&movie, &movie_id, receipt.still_time_seconds).unwrap();
    assert_eq!(receipt.source_kind, "androidHeifMotionPhotoV1");

    fs::remove_dir_all(dir).unwrap();
}
