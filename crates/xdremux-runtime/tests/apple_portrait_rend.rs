#![cfg(target_os = "macos")]

use std::fs;
use std::path::PathBuf;

use xdremux_container::{select_oppo_portrait_focus, OppoPortraitFocusRegion};
use xdremux_engine::{
    build_apple_portrait_rendering_parameters, AppleRendDocument, ConversionRequest,
};
use xdremux_runtime::PortableRuntime;

const ADAPTER_TEST_EXECUTABLE: &str = "XDREMUX_APPLE_ADAPTER_TEST_EXECUTABLE";

fn adapter_executable() -> Option<PathBuf> {
    let executable = PathBuf::from(std::env::var_os(ADAPTER_TEST_EXECUTABLE)?);
    assert!(
        executable.is_file(),
        "missing Apple adapter: {}",
        executable.display()
    );
    Some(executable)
}

#[test]
fn source_derived_rend_round_trips_through_the_rust_owned_portrait_manifest() {
    let Some(executable) = adapter_executable() else {
        // Normal workspace tests do not build Swift. The completion gate runs
        // this again with the minimal Apple adapter supplied.
        return;
    };

    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/proxdr/oppo/find-x9-ultra/uhdr-portrait-01.heic");
    let source = fs::read(&fixture).expect("read committed OPPO portrait fixture");
    let runtime = PortableRuntime::new();
    let prepared = runtime
        .analyze_proxdr(&source)
        .expect("Rust HDR analysis must expose source headroom");
    let preflight = runtime
        .preflight_apple_portrait_source(&executable, &source)
        .expect("Rust-owned Portrait preflight must accept the committed fixture");

    // RearDepthStruct records the producer focus point in src.image storage
    // pixels. Keep that source fact in Rust; the Apple adapter must not choose
    // focus, REND controls, or any other product policy.
    assert!(preflight.config.focus_x >= 0);
    assert!(preflight.config.focus_y >= 0);
    let focus_x = u32::try_from(preflight.config.focus_x).expect("non-negative focus x");
    let focus_y = u32::try_from(preflight.config.focus_y).expect("non-negative focus y");
    assert!(focus_x < preflight.base_width);
    assert!(focus_y < preflight.base_height);
    let focus = OppoPortraitFocusRegion {
        x: f64::from(focus_x) / f64::from(preflight.base_width),
        y: f64::from(focus_y) / f64::from(preflight.base_height),
        width: 0.12,
        height: 0.12,
    };
    let selection = select_oppo_portrait_focus(
        &preflight.depth,
        &preflight.config,
        preflight.base_width,
        preflight.base_height,
        focus,
    )
    .expect("Rust must resolve OPPO Portrait focus from producer facts");

    let disparity_span = 255.0 * f64::from(preflight.depth.header.rank_disparity_scale);
    let normalized_focus_rank = (selection.selected_rank / 255.0)
        .clamp(0.0, 1.0)
        .powi(i32::from(preflight.depth.header.disparity_exponentiation));
    let focus_disparity = (1.0 - normalized_focus_rank) * disparity_span;
    let rendering_parameters = build_apple_portrait_rendering_parameters(
        preflight.camera_calibration.profile,
        focus_disparity,
        disparity_span,
        prepared.scale.alternate_headroom,
        preflight.config.aec_lux_index.map(f64::from),
        preflight.depth.header.near_object_detected,
    )
    .expect("Rust must build source-derived Apple Portrait REND");
    let rend = AppleRendDocument::parse(&rendering_parameters)
        .expect("Rust-generated Portrait REND must parse");
    assert_eq!(
        rend.serialized(true).expect("serialize Rust-generated REND"),
        rendering_parameters
    );

    let payloads = preflight
        .into_auxiliary_payloads(&rendering_parameters)
        .expect("Rust must assemble Portrait auxiliaries with the real REND");

    // Attach the Rust-owned Portrait resources to the canonical Rust HDR base,
    // then ask ImageIO only to perform the platform write and factual probe.
    let temporary = tempfile::tempdir().expect("create Portrait output directory");
    let iso_base = temporary.path().join("portrait-rust-base.heic");
    runtime
        .convert_proxdr_file(&source, &iso_base, ConversionRequest::default(), |_| {})
        .expect("Rust runtime must build the ISO HDR base");
    let output = temporary.path().join("portrait-rust-rend.heic");
    runtime
        .apple_write_auxiliary_payloads(&executable, &iso_base, &output, &payloads)
        .expect("minimal Apple adapter must write the Rust-owned manifest");
    let facts = runtime
        .apple_image_auxiliary_facts(&executable, &output)
        .expect("ImageIO must report the final auxiliary resources");

    assert!(facts.iso_gain_map, "facts: {facts:?}");
    assert!(facts.satisfies_portrait_editing(), "facts: {facts:?}");
}
