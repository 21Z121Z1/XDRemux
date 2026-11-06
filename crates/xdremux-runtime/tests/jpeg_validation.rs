//! The validate entry point must read a published two-image JPEG, report its
//! carriers, and reconstruct the same HDR image as the HEIF export of the same
//! source.
use std::fs;
use std::path::PathBuf;

use xdremux_engine::{ConversionRequest, HdrExportProfile, OutputContainer};
use xdremux_runtime::{
    reconstruct_dual_layer_jpeg_hdr, reconstruct_hdr_gain_map, validate_media_file_with_profile,
    GainMapValidationProfile, HdrTransfer, PortableRuntime, ValidationReport,
};

fn fixture(path: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(path),
    )
    .unwrap()
}

fn unique_dir() -> PathBuf {
    static UNIQUE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "xdremux-jpeg-validation-{}-{stamp}-{}",
        std::process::id(),
        UNIQUE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::create_dir_all(&directory).unwrap();
    directory
}

#[test]
fn jpeg_validation_reports_carriers_and_reconstructs_the_exported_hdr() {
    let source = fixture("proxdr/oppo/find-x7-ultra/lhdr-v2-05.heic");
    let runtime = PortableRuntime::new();
    let jpeg = runtime
        .convert_proxdr_bytes(
            &source,
            ConversionRequest {
                container: OutputContainer::Jpeg,
                ..ConversionRequest::default()
            },
            |_| {},
        )
        .expect("JPEG conversion must succeed");
    let heif = runtime
        .convert_proxdr_bytes(
            &source,
            ConversionRequest {
                hdr_profile: HdrExportProfile::Bt2020Pq,
                ..ConversionRequest::default()
            },
            |_| {},
        )
        .expect("HEIF conversion must succeed");

    let work = unique_dir();
    let path = work.join("output.jpg");
    fs::write(&path, &jpeg.bytes).unwrap();

    let report = validate_media_file_with_profile(&path, GainMapValidationProfile::Semantics)
        .expect("validation must succeed");
    let ValidationReport::DualLayerJpeg(report) = report else {
        panic!("expected a dual-layer JPEG report, got {}", report.kind());
    };
    assert_eq!(report.mp_image_count, 2);
    assert!(report.base_iso_declaration);
    assert!(report.gain_map_iso_parameters);
    assert!(report.base_icc_present);
    assert!(report.use_base_color_space);
    assert!(report.alternate_headroom_stops > 0.0);
    let decoded = report.decoded_hdr.expect("--semantic must decode pixels");
    assert_eq!(decoded.width, usize::from(report.base_width));
    assert_eq!(decoded.height, usize::from(report.base_height));

    // The JPEG export re-encodes both images. Its reconstruction must still
    // match the HEIF export of the same source within the JPEG loss budget.
    let jpeg_hdr = reconstruct_dual_layer_jpeg_hdr(&jpeg.bytes).expect("JPEG reconstruction");
    let heif_hdr =
        reconstruct_hdr_gain_map(&heif.bytes, HdrTransfer::Pq).expect("HEIF reconstruction");
    assert_eq!(jpeg_hdr.width, heif_hdr.width);
    assert_eq!(jpeg_hdr.height, heif_hdr.height);
    let mut total_relative = 0.0_f64;
    let mut total_absolute = 0.0_f64;
    let mut maximum_absolute = 0.0_f64;
    let mut relative_errors = Vec::with_capacity(jpeg_hdr.samples.len());
    for (a, b) in jpeg_hdr.samples.iter().zip(&heif_hdr.samples) {
        let difference = f64::from((a - b).abs());
        // Relative error uses a quarter of SDR white as its floor: near-black
        // samples would otherwise dominate with an insignificant absolute
        // difference.
        let scale = f64::from(a.abs().max(b.abs())).max(0.25);
        total_relative += difference / scale;
        total_absolute += difference;
        maximum_absolute = maximum_absolute.max(difference);
        relative_errors.push(difference / scale);
    }
    let count = jpeg_hdr.samples.len() as f64;
    let mean_relative = total_relative / count;
    let mean_absolute = total_absolute / count;
    relative_errors.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p99_relative = relative_errors[relative_errors.len() * 99 / 100];
    eprintln!(
        "jpeg-vs-heif reconstruction: mean-rel={mean_relative:.5} p99-rel={p99_relative:.5} \
         mean-abs={mean_absolute:.5} max-abs={maximum_absolute:.5}"
    );
    assert!(
        mean_relative < 0.01,
        "mean relative reconstruction error {mean_relative}"
    );
    assert!(
        p99_relative < 0.10,
        "p99 relative reconstruction error {p99_relative}"
    );
    assert!(
        maximum_absolute < 0.20,
        "maximum absolute reconstruction error {maximum_absolute}"
    );

    fs::remove_dir_all(work).ok();
}

#[test]
fn a_plain_single_image_jpeg_is_not_reported_as_dual_layer() {
    let work = unique_dir();
    let path = work.join("plain.jpg");
    // A one-image JPEG with no MPF index.
    let source = fixture("motion-photo/samsung/jpeg-ultrahdr-01.jpg");
    let first = xdremux_format::scan_jpeg_image_at(&source, 0).unwrap();
    fs::write(&path, &source[first.range]).unwrap();
    assert!(validate_media_file_with_profile(&path, GainMapValidationProfile::Structure).is_err());
    fs::remove_dir_all(work).ok();
}

#[test]
fn an_absent_alternate_icc_keeps_gains_in_the_base_primaries_for_either_flag() {
    use xdremux_format::dual_jpeg::{
        assemble_dual_layer_jpeg, AppSegment, DualLayerJpegAssembly, ISO_GAIN_MAP_APP2_IDENTIFIER,
    };
    let encode = |pixels: &[u8]| {
        let mut bytes = Vec::new();
        jpeg_encoder::Encoder::new(&mut bytes, 100)
            .encode(pixels, 1, 1, jpeg_encoder::ColorType::Rgb)
            .unwrap();
        bytes
    };
    let base = encode(&[128, 64, 32]);
    let gain = encode(&[0, 128, 255]);
    let info = [
        1.0, 1.0, 1.0, 1.0, 8.0, 8.0, 8.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 8.0,
    ];
    let tmap = xdremux_metadata::make_apple_tmap_payload(&info).unwrap();
    let metadata = xdremux_metadata::parse_heif_tmap_metadata(&tmap).unwrap();
    let mut base_iso = ISO_GAIN_MAP_APP2_IDENTIFIER.to_vec();
    base_iso.extend([0; 4]);
    let assemble = |use_base, invalid_icc| {
        let mut gain_iso = ISO_GAIN_MAP_APP2_IDENTIFIER.to_vec();
        gain_iso.extend(metadata.to_iso_app2_payload(use_base).unwrap());
        let mut gain_segments = vec![AppSegment {
            marker: 0xe2,
            payload: &gain_iso,
        }];
        let icc = b"ICC_PROFILE\0\x01\x01broken ICC";
        if invalid_icc {
            gain_segments.push(AppSegment {
                marker: 0xe2,
                payload: icc,
            });
        }
        assemble_dual_layer_jpeg(&DualLayerJpegAssembly {
            base: &base,
            gain_map: &gain,
            base_segments: &[AppSegment {
                marker: 0xe2,
                payload: &base_iso,
            }],
            gain_segments: &gain_segments,
        })
        .unwrap()
    };
    let with = reconstruct_dual_layer_jpeg_hdr(&assemble(true, false)).unwrap();
    let without = reconstruct_dual_layer_jpeg_hdr(&assemble(false, false)).unwrap();
    assert_eq!(
        with.samples, without.samples,
        "absence of alternate ICC makes the flag immaterial"
    );
    for use_base in [true, false] {
        assert!(reconstruct_dual_layer_jpeg_hdr(&assemble(use_base, true)).is_err());
    }
}
