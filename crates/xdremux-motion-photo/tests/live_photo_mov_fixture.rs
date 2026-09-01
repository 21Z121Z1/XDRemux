use std::fs;
use std::path::PathBuf;

use xdremux_motion_photo::{
    media_mdat_payloads, normalize_embedded_video, parse_oppo_motion_photo,
    resolve_live_photo_still_time, resolve_video_stream_layout, validate_live_photo_movie,
    write_live_photo_movie, MotionPhotoSourceKind,
};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/IMG20260710191114_ColorOS_16.jpg")
}

#[test]
fn real_coloros_motion_video_rewrap_preserves_compressed_media() {
    let path = fixture_path();
    assert!(
        path.is_file(),
        "missing versioned fixture {}",
        path.display()
    );
    let source = fs::read(&path).unwrap();
    let asset = parse_oppo_motion_photo(&source)
        .unwrap()
        .expect("fixture must remain a supported Motion Photo");
    let stream_count = asset
        .vendor_metadata
        .as_ref()
        .map_or(1, |metadata| metadata.stream_count.max(1));
    let layout = resolve_video_stream_layout(
        &source,
        asset.video_resource_range,
        asset.source_kind == MotionPhotoSourceKind::OppoLivePhoto,
        stream_count,
    )
    .unwrap();
    let start = usize::try_from(layout.primary.range.lower_bound).unwrap();
    let end = usize::try_from(layout.primary.range.upper_bound).unwrap();
    let embedded_video = &source[start..end];
    let normalized = normalize_embedded_video(embedded_video).unwrap();
    assert!(
        normalized.removed_vendor_bytes > 0,
        "ColorOS 16 fixture must keep exercising trailing vendor-byte normalization"
    );
    let video = normalized.data;
    let media_before = media_mdat_payloads(video).unwrap();
    assert!(
        !media_before.is_empty(),
        "fixture primary stream must contain media mdat"
    );

    let still_time = resolve_live_photo_still_time(video, asset.presentation_timestamp_us).unwrap();
    let output = write_live_photo_movie(
        video,
        "01234567-89AB-CDEF-0123-456789ABCDEF",
        still_time,
        asset.vendor_metadata.as_ref(),
    )
    .unwrap();

    validate_live_photo_movie(&output, "01234567-89AB-CDEF-0123-456789ABCDEF", still_time).unwrap();
    assert_eq!(media_mdat_payloads(&output).unwrap(), media_before);
}
