#![cfg(target_os = "macos")]

use std::path::PathBuf;

use xdremux_runtime::PortableRuntime;

/// Core Image gain-map round trip for a tiny even raster.
///
/// Product CI does not set the adapter variable, so this test returns without
/// calling the encoder there. The system HEIF workflow sets the variable.
#[test]
fn system_encoder_round_trips_a_small_linear_hdr_pair() {
    let Some(executable) = std::env::var_os("XDREMUX_APPLE_ADAPTER_TEST_EXECUTABLE") else {
        return;
    };
    let executable = PathBuf::from(executable);
    assert!(
        executable.is_file(),
        "missing Apple adapter: {}",
        executable.display()
    );
    let width = 32_u32;
    let height = 32_u32;
    let mut sdr = Vec::with_capacity(32 * 32 * 3);
    let mut hdr = Vec::with_capacity(32 * 32 * 3);
    for y in 0..height {
        for x in 0..width {
            let code = (x + y) as f32 / 62.0;
            sdr.extend_from_slice(&[code, code * 0.8, code * 0.6]);
            let gain = if x >= 16 { 4.0 } else { 1.0 };
            hdr.extend_from_slice(&[code * gain, code * 0.8 * gain, code * 0.6 * gain]);
        }
    }
    let temporary = tempfile::tempdir().unwrap();
    let output = temporary.path().join("round-trip.heic");
    let report = PortableRuntime::new()
        .measure_system_heif_hdr_pair(&executable, &sdr, &hdr, width, height, &output)
        .expect("Core Image HEIF gain-map encode");
    assert!(output.is_file());
    assert!(output.metadata().unwrap().len() > 0);
    assert_eq!(report.width, width);
    assert_eq!(report.height, height);
    assert!(report.provisional);
    assert!(report.gain_map_as_rgb);
    assert!(report.content_headroom.is_finite() && report.content_headroom >= 1.0);
    assert_eq!(report.before_styles.decoded_width, width);
    assert_eq!(report.before_styles.decoded_height, height);
    let against_submitted = report
        .before_styles
        .error_vs_submitted_f16
        .expect("submitted HDR comparison");
    let against_linear = report
        .before_styles
        .error_vs_prequantized_f32
        .expect("prequantized HDR comparison");
    assert!(against_submitted.mean_abs.is_finite() && against_submitted.max_abs.is_finite());
    assert!(against_linear.mean_abs.is_finite() && against_linear.max_abs.is_finite());
    assert!(against_submitted.mean_abs >= 0.0 && against_submitted.max_abs >= 0.0);
}
