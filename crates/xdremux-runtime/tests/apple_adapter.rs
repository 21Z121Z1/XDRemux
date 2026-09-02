#![cfg(target_os = "macos")]

use std::fs;
use std::path::PathBuf;

use xdremux_engine::{build_apple_portrait_effects_matte_payload, OperationCapability};
use xdremux_runtime::PortableRuntime;

const ADAPTER_TEST_EXECUTABLE: &str = "XDREMUX_APPLE_ADAPTER_TEST_EXECUTABLE";
const ADAPTER_TEST_INPUT: &str = "XDREMUX_APPLE_ADAPTER_TEST_INPUT";

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
fn swift_adapter_advertises_only_apple_operation_facts() {
    let Some(executable) = adapter_executable() else {
        // The normal workspace test pass does not build Swift. The completion
        // gate runs this test again with the built adapter path supplied.
        return;
    };

    let runtime = PortableRuntime::new();
    let inventory = runtime
        .capability_inventory_with_apple_adapter(&executable)
        .expect("Rust runtime must complete the versioned Swift adapter handshake");

    assert!(inventory.supports(OperationCapability::PhotographicStylesAdapter));
    assert!(inventory.supports(OperationCapability::PortraitAdapter));
    assert!(inventory.supports(OperationCapability::RasterDecoder(
        xdremux_engine::GainMapCodec::Jpeg,
    )));

    let Some(input) = std::env::var_os(ADAPTER_TEST_INPUT) else {
        return;
    };
    let facts = runtime
        .apple_image_auxiliary_facts(&executable, PathBuf::from(input))
        .expect("Rust runtime must decode ImageIO auxiliary facts from the Swift adapter");

    // This public Samsung Ultra HDR Motion Photo is not an Apple Portrait asset.
    // ImageIO may or may not expose its HDR gain map as the ISO auxiliary type,
    // but the portrait-specific resources must not be invented by the adapter.
    assert!(!facts.disparity);
    assert!(!facts.portrait_effects_matte);
    assert!(!facts.skin_matte);
    assert!(!facts.hair_matte);
    assert!(!facts.teeth_matte);
    assert!(!facts.glasses_matte);
    assert!(!facts.satisfies_portrait_editing());
}

#[test]
fn rust_owned_auxiliary_manifest_round_trips_through_imageio() {
    let Some(executable) = adapter_executable() else {
        return;
    };
    let input = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/proxdr/oppo/find-x9-ultra/uhdr-portrait-01.heic");
    let temporary = tempfile::tempdir().expect("create ImageIO output directory");
    let output = temporary.path().join("auxiliary-round-trip.heic");
    let payload = build_apple_portrait_effects_matte_payload(2, 2, vec![0, 64, 128, 255])
        .expect("build Rust-owned portrait effects matte payload");

    let runtime = PortableRuntime::new();
    runtime
        .apple_write_auxiliary_payloads(&executable, &input, &output, &[payload])
        .expect("Swift adapter must execute the Rust-owned ImageIO auxiliary manifest");
    let facts = runtime
        .apple_image_auxiliary_facts(&executable, &output)
        .expect("ImageIO must expose the written auxiliary resource");

    assert!(facts.portrait_effects_matte);
}

#[test]
fn rust_owns_oppo_portrait_source_preflight_around_apple_framework_primitives() {
    let Some(executable) = adapter_executable() else {
        return;
    };
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/proxdr/oppo/find-x9-ultra/uhdr-portrait-01.heic");
    let source = fs::read(fixture).expect("read committed OPPO portrait fixture");

    let preflight = PortableRuntime::new()
        .preflight_apple_portrait_source(&executable, &source)
        .expect("Rust-owned Portrait preflight must accept the committed fixture");

    assert!(!preflight.base_jpeg.is_empty());
    assert!(!preflight.gain_map_jpeg.is_empty());
    assert!(preflight.depth.header.width > 0);
    assert!(preflight.depth.header.height > 0);
    assert!(!preflight.depth.ranks.is_empty());
    assert!(preflight.base_width > 0);
    assert!(preflight.base_height > 0);
    assert!(preflight.gain_map.supports_portrait_source());
    assert!((1.0..=4.0).contains(&preflight.config.version));

    assert!((1..=8).contains(&preflight.base_orientation));
    assert!(preflight.camera_calibration.reference_width > 0);
    assert!(preflight.camera_calibration.reference_height > 0);
    assert!(preflight.camera_calibration.focal_length_pixels > 0.0);
    assert_eq!(preflight.disparity.width, preflight.depth.header.width);
    assert_eq!(preflight.disparity.height, preflight.depth.header.height);
    assert_eq!(
        preflight.disparity.pixels_le_f16.len(),
        usize::try_from(preflight.depth.header.width).unwrap()
            * usize::try_from(preflight.depth.header.height).unwrap()
            * 2
    );
    assert!(preflight.disparity.near > preflight.disparity.far);

    let matte = &preflight.portrait_effects_matte;
    assert_eq!(matte.width, preflight.base_width / 2);
    assert_eq!(matte.height, preflight.base_height / 2);
    assert_eq!(
        matte.pixels.len(),
        usize::try_from(matte.width).unwrap() * usize::try_from(matte.height).unwrap()
    );
    assert!(!preflight.subject_prior_used || preflight.depth.portrait.is_some());
    build_apple_portrait_effects_matte_payload(
        matte.width,
        matte.height,
        matte.pixels.clone(),
    )
    .expect("Rust-owned fused person matte must be directly payload-ready");

    assert!((1.0..=64.0).contains(&preflight.simulated_aperture));
}
