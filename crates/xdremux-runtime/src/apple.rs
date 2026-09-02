use std::path::Path;

use xdremux_engine::{AppleImageAuxiliaryFacts, CapabilityInventory};

use crate::{apple_adapter::AppleAdapterClient, PortableRuntime, Result};

impl PortableRuntime {
    /// Merge Apple-only capability facts into the portable runtime inventory.
    ///
    /// Transport stays private to the runtime so replacing the CLI helper with
    /// XPC for a sandboxed app does not change engine or CLI contracts.
    pub fn capability_inventory_with_apple_adapter(
        &self,
        executable: &Path,
    ) -> Result<CapabilityInventory> {
        let apple = AppleAdapterClient::new(executable)
            .capabilities()?
            .operation_capabilities();
        let mut operations = self.capability_inventory()?.iter().collect::<Vec<_>>();
        operations.extend(apple);
        Ok(CapabilityInventory::new(operations))
    }

    /// Ask ImageIO for observations only; product validity remains Rust policy.
    pub fn apple_image_auxiliary_facts(
        &self,
        executable: &Path,
        input: &Path,
    ) -> Result<AppleImageAuxiliaryFacts> {
        AppleAdapterClient::new(executable).imageio_auxiliary_facts(input)
    }
}
