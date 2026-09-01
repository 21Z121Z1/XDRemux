use std::fs;
use std::path::PathBuf;

use xdremux_motion_photo::{
    parse_oppo_motion_photo, read_apple_content_identifier, write_live_photo_heif_still,
    MotionPhotoSourceKind,
};

const CONTENT_IDENTIFIER: &str = "DF64C2AE-ED3C-4778-BFCA-C15277E521D2";

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/motion-photo/samsung/heif-ultrahdr-01.heic")
}

#[test]
fn samsung_heif_motion_photo_becomes_portable_apple_live_photo_still() {
    let source = fs::read(fixture_path()).expect("versioned Samsung HEIF fixture must be readable");
    let asset = parse_oppo_motion_photo(&source)
        .expect("Samsung HEIF Motion Photo parsing must succeed")
        .expect("fixture must remain a Motion Photo before conversion");
    assert_eq!(asset.source_kind, MotionPhotoSourceKind::AndroidHeifMotionPhotoV1);

    let still_start = usize::try_from(asset.still_resource_range.lower_bound)
        .expect("still start must fit usize");
    let still_end = usize::try_from(asset.still_resource_range.upper_bound)
        .expect("still end must fit usize");
    let static_heif = source
        .get(still_start..still_end)
        .expect("parsed static HEIF range must be in bounds");

    let output = write_live_photo_heif_still(static_heif, CONTENT_IDENTIFIER)
        .expect("pure-Rust HEIF Live Photo still writing must succeed");
    assert_eq!(
        read_apple_content_identifier(&output)
            .expect("written Apple asset identifier must be readable")
            .as_deref(),
        Some(CONTENT_IDENTIFIER)
    );
    assert!(
        parse_oppo_motion_photo(&output)
            .expect("written still must remain structurally parseable")
            .is_none(),
        "the generated still must no longer advertise itself as an Android Motion Photo"
    );
    assert!(output.windows(4).any(|window| window == b"ftyp"));
    assert!(output.windows(4).any(|window| window == b"meta"));
}
