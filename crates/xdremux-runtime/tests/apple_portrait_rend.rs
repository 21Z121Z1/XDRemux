#![cfg(target_os = "macos")]

use std::fs;
use std::path::PathBuf;

use xdremux_engine::{AppleAuxiliaryKind, AppleMetadataValue, ConversionRequest};
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
    let preflight = runtime
        .preflight_apple_portrait_source(&executable, &source)
        .expect("Rust-owned Portrait preflight must accept the committed fixture");

    // The completed preflight now owns focus selection, Gain Map headroom and
    // the per-image REND. No caller-supplied rendering policy remains.
    let payloads = preflight
        .into_auxiliary_payloads()
        .expect("Rust must assemble Portrait auxiliaries with source-derived REND");
    let disparity = payloads
        .iter()
        .find(|payload| payload.kind == AppleAuxiliaryKind::Disparity)
        .expect("Rust-owned manifest must contain disparity");
    let rend = disparity
        .metadata
        .iter()
        .find(|tag| tag.path == "depthBlurEffect:RenderingParameters")
        .expect("Rust-owned disparity metadata must contain REND");
    let AppleMetadataValue::Text(encoded_rend) = &rend.value else {
        panic!("Portrait REND metadata must be text");
    };
    assert!(
        encoded_rend.starts_with("UkVORA"),
        "base64 REND must begin with the REND magic"
    );
    assert!(
        encoded_rend.len() > 100,
        "REND payload is unexpectedly short"
    );

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
