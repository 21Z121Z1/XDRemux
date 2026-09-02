#![cfg(target_os = "macos")]

use std::path::PathBuf;

use xdremux_engine::OperationCapability;
use xdremux_runtime::PortableRuntime;

const ADAPTER_TEST_EXECUTABLE: &str = "XDREMUX_APPLE_ADAPTER_TEST_EXECUTABLE";

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

    let inventory = PortableRuntime::new()
        .capability_inventory_with_apple_adapter(&executable)
        .expect("Rust runtime must complete the versioned Swift adapter handshake");

    assert!(inventory.supports(OperationCapability::PhotographicStylesAdapter));
    assert!(inventory.supports(OperationCapability::PortraitAdapter));
    assert!(inventory.supports(OperationCapability::RasterDecoder(
        xdremux_engine::GainMapCodec::Jpeg,
    )));
}
