#![cfg(target_os = "macos")]

use std::fs;
use std::path::PathBuf;

use xdremux_engine::OperationCapability;
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
fn rust_owns_oppo_portrait_source_preflight_around_imageio_observations() {
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
}
