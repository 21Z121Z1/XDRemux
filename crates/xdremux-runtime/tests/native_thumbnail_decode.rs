//! Replay retained compiled-consumer rasters through the current Rust decoder.
use std::{fs, path::Path};
use xdremux_hdr::{decode_styles_thumbnail, LinearPixel, LinearThumbnailEncoding};

fn pixels(path: &Path) -> Vec<LinearPixel> {
    let bytes = fs::read(path).unwrap();
    assert!(bytes.len().is_multiple_of(16));
    bytes
        .as_chunks::<16>()
        .0
        .iter()
        .map(|p| {
            std::array::from_fn(|c| f32::from_le_bytes(p[c * 4..c * 4 + 4].try_into().unwrap()))
        })
        .collect()
}

#[test]
#[ignore = "requires an explicit manifest of retained native thumbnail outputs"]
fn rust_thumbnail_decode_matches_compiled_native_outputs() {
    let manifest =
        std::env::var_os("XDREMUX_THUMBNAIL_NATIVE_CASES").expect("native cases required");
    let cases: serde_json::Value = serde_json::from_slice(&fs::read(manifest).unwrap()).unwrap();
    let cases = cases.as_array().unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        let input = pixels(Path::new(case["codes"].as_str().unwrap()));
        let expected = pixels(Path::new(case["native"].as_str().unwrap()));
        let encoding = LinearThumbnailEncoding {
            gain: case["gain"].as_f64().unwrap() as f32,
            minimum: case["minimum"].as_f64().unwrap() as f32,
            maximum: case["maximum"].as_f64().unwrap() as f32,
        };
        let actual = decode_styles_thumbnail(&input, encoding).unwrap();
        assert_eq!(actual.len(), expected.len());
        let mut maximum = 0f32;
        for (a, b) in actual.iter().zip(&expected) {
            for c in 0..3 {
                assert!(b[c].is_finite());
                maximum = maximum.max((a[c] - b[c]).abs());
            }
        }
        assert!(
            maximum < 3e-6,
            "{} native max RGB error {maximum}",
            case["name"]
        );
        println!(
            "{}: {} pixels, native max RGB error {maximum}",
            case["name"],
            actual.len()
        );
    }
}
