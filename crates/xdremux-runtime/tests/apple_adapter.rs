#![cfg(target_os = "macos")]

use std::path::PathBuf;

use xdremux_engine::OperationCapability;
use xdremux_runtime::PortableRuntime;

const ADAPTER_TEST_EXECUTABLE: &str = "XDREMUX_APPLE_ADAPTER_TEST_EXECUTABLE";
const ADAPTER_TEST_INPUT: &str = "XDREMUX_APPLE_ADAPTER_TEST_INPUT";

#[test]
fn swift_adapter_advertises_only_apple_operation_facts() {
    let Some(executable) = std::env::var_os(ADAPTER_TEST_EXECUTABLE) else {
        // The normal workspace test pass does not build Swift. The completion
        // gate runs this test again with the built adapter path supplied.
        return;
    };
    let executable = PathBuf::from(executable);
    assert!(
        executable.is_file(),
        "missing Apple adapter: {}",
        executable.display()
    );

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
