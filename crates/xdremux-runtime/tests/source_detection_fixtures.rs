use std::fs;
use std::path::{Path, PathBuf};

use xdremux_container::ExtractionMode;
use xdremux_engine::{GainMapChannels, GainMapCodec};
use xdremux_format::ChromaSampling;
use xdremux_runtime::{PortableRuntime, PreparedProXdr};

fn fixture(relative: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/proxdr/oppo")
        .join(relative)
}

fn analyze(relative: impl AsRef<Path>) -> PreparedProXdr {
    let path = fixture(relative);
    let source = fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "real ProXDR fixture {} should exist: {error}",
            path.display()
        )
    });
    PortableRuntime::new()
        .analyze_proxdr(&source)
        .unwrap_or_else(|error| panic!("fixture {} should analyze: {error}", path.display()))
}

fn assert_jpeg8(prepared: &PreparedProXdr) {
    assert_eq!(prepared.analysis.gain_map.storage.codec, GainMapCodec::Jpeg);
    assert_eq!(prepared.analysis.gain_map.storage.luma_bit_depth, 8);
    assert_eq!(prepared.analysis.gain_map.storage.chroma_bit_depth, 8);
}

#[test]
fn find_x6_lhdr_source_is_monochrome_jpeg8() {
    let prepared = analyze("find-x6-pro/lhdr-v1-01.heic");

    assert_eq!(prepared.extracted.mode, ExtractionMode::Lhdr);
    assert_eq!(prepared.analysis.gain_map.channels, GainMapChannels::Mono);
    assert_eq!(
        prepared.analysis.gain_map.storage.chroma,
        Some(ChromaSampling::Mono400)
    );
    assert_jpeg8(&prepared);
}

#[test]
fn find_x7_lhdr_source_is_monochrome_jpeg8() {
    let prepared = analyze("find-x7-ultra/lhdr-v2-01.heic");

    assert_eq!(prepared.extracted.mode, ExtractionMode::Lhdr);
    assert_eq!(prepared.analysis.gain_map.channels, GainMapChannels::Mono);
    assert_eq!(
        prepared.analysis.gain_map.storage.chroma,
        Some(ChromaSampling::Mono400)
    );
    assert_jpeg8(&prepared);
}

#[test]
fn find_x9_uhdr_source_is_rgb_jpeg8() {
    let prepared = analyze("find-x9-ultra/uhdr-hr-01.heic");

    assert_eq!(prepared.extracted.mode, ExtractionMode::Uhdr);
    assert_eq!(prepared.analysis.gain_map.channels, GainMapChannels::Rgb);
    assert_ne!(
        prepared.analysis.gain_map.storage.chroma,
        Some(ChromaSampling::Mono400)
    );
    assert_jpeg8(&prepared);
}
