use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use xdremux_heif::validate_gain_map_structure;
use xdremux_motion_photo::{
    read_apple_content_identifier, read_live_photo_content_identifier, read_live_photo_still_time,
    validate_live_photo_movie,
};
use xdremux_runtime::PortableRuntime;

fn fixture(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/motion-photo")
        .join(relative)
}

fn unique_dir(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "xdremux-runtime-{label}-{}-{stamp}",
        std::process::id()
    ))
}

fn exercise(relative: &str, expect_gain_map: bool) {
    let input = fixture(relative);
    let source = fs::read(&input).expect("versioned Motion Photo fixture must be readable");
    let work = unique_dir(
        Path::new(relative)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("motion-photo"),
    );
    fs::create_dir_all(&work).expect("temporary output directory");
    let image = work.join("capture.heic");

    let receipt = PortableRuntime::new()
        .convert_motion_photo_file(&source, &input, &image)
        .unwrap_or_else(|error| panic!("{relative}: Rust runtime failed: {error}"));
    assert_eq!(
        receipt.source_had_gain_map, expect_gain_map,
        "{relative}: source gain-map classification"
    );

    let still = fs::read(&receipt.image).expect("published Live Photo still");
    let movie = fs::read(&receipt.video).expect("published Live Photo movie");
    let still_id = read_apple_content_identifier(&still)
        .expect("read still identifier")
        .expect("still identifier exists");
    let movie_id = read_live_photo_content_identifier(&movie)
        .expect("read movie identifier")
        .expect("movie identifier exists");
    assert_eq!(still_id, receipt.content_identifier);
    assert_eq!(movie_id, receipt.content_identifier);
    let still_time = read_live_photo_still_time(&movie)
        .expect("read still-time metadata")
        .expect("still-time metadata exists");
    assert!((still_time - receipt.still_time_seconds).abs() < 1.0 / 600.0);
    validate_live_photo_movie(&movie, &movie_id, receipt.still_time_seconds)
        .expect("published movie must satisfy Live Photo contract");

    if expect_gain_map {
        validate_gain_map_structure(&still).unwrap_or_else(|error| {
            panic!("{relative}: final HEIF Gain Map graph invalid: {error}")
        });
    } else {
        assert!(
            validate_gain_map_structure(&still).is_err(),
            "{relative}: SDR source unexpectedly gained an ISO Gain Map graph"
        );
    }

    fs::remove_dir_all(work).expect("remove temporary output directory");
}

#[test]
fn samsung_ultrahdr_jpeg_becomes_valid_hdr_live_photo() {
    exercise("samsung/jpeg-ultrahdr-01.jpg", true);
}

#[test]
#[ignore = "real-device vendor matrix; invoked explicitly by the Rust CLI gate"]
fn real_jpeg_motion_photo_vendor_matrix() {
    for (relative, expect_gain_map) in [
        ("vivo/android-v1-sdr-01.jpg", false),
        ("xiaomi/android-v1-ultrahdr-01.jpg", true),
        ("oppo/coloros15-ultrahdr-01.jpg", true),
        ("oppo/coloros16-dualstream-ultrahdr-01.jpg", true),
    ] {
        exercise(relative, expect_gain_map);
    }
}
