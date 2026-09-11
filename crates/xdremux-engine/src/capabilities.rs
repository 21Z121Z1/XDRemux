use std::collections::BTreeSet;
use std::error::Error;

use crate::{GainMapCodec, GainMapCodecLayout, GainMapEncoderCapabilities};

/// One operation the conversion planner may require from the composed runtime.
///
/// This is a fact set, not an execution interface. Add a new execution port only
/// when a real request/result contract exists; do not model hypothetical
/// backends in advance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperationCapability {
    RasterDecoder(GainMapCodec),
    GainMapTileEncoder(GainMapCodecLayout),
    PhotographicStylesAdapter,
    PortraitAdapter,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RasterDecoderCapabilities {
    codecs: BTreeSet<GainMapCodec>,
}

impl RasterDecoderCapabilities {
    pub fn new(codecs: impl IntoIterator<Item = GainMapCodec>) -> Self {
        Self {
            codecs: codecs.into_iter().collect(),
        }
    }

    pub fn supports(&self, codec: GainMapCodec) -> bool {
        self.codecs.contains(&codec)
    }

    pub fn iter(&self) -> impl Iterator<Item = GainMapCodec> + '_ {
        self.codecs.iter().copied()
    }
}

/// Planner-facing inventory of operations available at the composition root.
///
/// It contains facts only: no adapter pointers and no execution state. Concrete
/// providers expose their capabilities, and the runtime composes this inventory.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CapabilityInventory {
    operations: BTreeSet<OperationCapability>,
}

impl CapabilityInventory {
    pub fn new(capabilities: impl IntoIterator<Item = OperationCapability>) -> Self {
        Self {
            operations: capabilities.into_iter().collect(),
        }
    }

    pub fn supports(&self, capability: OperationCapability) -> bool {
        self.operations.contains(&capability)
    }

    pub fn iter(&self) -> impl Iterator<Item = OperationCapability> + '_ {
        self.operations.iter().copied()
    }

    pub fn missing(
        &self,
        required: impl IntoIterator<Item = OperationCapability>,
    ) -> Vec<OperationCapability> {
        required
            .into_iter()
            .filter(|capability| !self.supports(*capability))
            .collect()
    }

    pub fn gain_map_encoder_capabilities(&self) -> GainMapEncoderCapabilities {
        GainMapEncoderCapabilities::new(self.operations.iter().filter_map(|capability| {
            if let OperationCapability::GainMapTileEncoder(layout) = capability {
                Some(*layout)
            } else {
                None
            }
        }))
    }

    pub fn raster_decoder_capabilities(&self) -> RasterDecoderCapabilities {
        RasterDecoderCapabilities::new(self.operations.iter().filter_map(|capability| {
            if let OperationCapability::RasterDecoder(codec) = capability {
                Some(*codec)
            } else {
                None
            }
        }))
    }
}

/// Outgoing port for encoding normalized Gain Map raster data into HEVC tiles.
///
/// Request/output payloads stay associated with the provider so the engine does
/// not prematurely standardize a file IPC or in-process pixel ABI. The planner
/// depends only on advertised codec layouts.
pub trait GainMapTileEncoder {
    type Request;
    type Output;
    type Error: Error;

    fn gain_map_encoder_capabilities(&self) -> GainMapEncoderCapabilities;

    fn encode_gain_map_tiles(
        &self,
        request: &Self::Request,
    ) -> std::result::Result<Self::Output, Self::Error>;
}

/// Outgoing port for decoding an encoded image representation into a raster.
pub trait RasterDecoder {
    type Request;
    type Output;
    type Error: Error;

    fn raster_decoder_capabilities(&self) -> RasterDecoderCapabilities;

    fn decode_raster(
        &self,
        request: &Self::Request,
    ) -> std::result::Result<Self::Output, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;

    use xdremux_format::ChromaSampling;

    #[derive(Debug)]
    struct MockError;

    impl fmt::Display for MockError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("mock capability error")
        }
    }

    impl Error for MockError {}

    struct MockCodecProvider;

    impl GainMapTileEncoder for MockCodecProvider {
        type Request = ();
        type Output = ();
        type Error = MockError;

        fn gain_map_encoder_capabilities(&self) -> GainMapEncoderCapabilities {
            GainMapEncoderCapabilities::new([GainMapCodecLayout {
                chroma: ChromaSampling::Yuv420,
                luma_bit_depth: 8,
                chroma_bit_depth: 8,
            }])
        }

        fn encode_gain_map_tiles(
            &self,
            _request: &Self::Request,
        ) -> std::result::Result<Self::Output, Self::Error> {
            Ok(())
        }
    }

    impl RasterDecoder for MockCodecProvider {
        type Request = ();
        type Output = ();
        type Error = MockError;

        fn raster_decoder_capabilities(&self) -> RasterDecoderCapabilities {
            RasterDecoderCapabilities::new([GainMapCodec::Jpeg])
        }

        fn decode_raster(
            &self,
            _request: &Self::Request,
        ) -> std::result::Result<Self::Output, Self::Error> {
            Ok(())
        }
    }

    #[test]
    fn inventory_is_only_a_set_of_composed_operation_facts() {
        let layout = GainMapCodecLayout {
            chroma: ChromaSampling::Yuv420,
            luma_bit_depth: 8,
            chroma_bit_depth: 8,
        };
        let inventory = CapabilityInventory::new([
            OperationCapability::GainMapTileEncoder(layout),
            OperationCapability::RasterDecoder(GainMapCodec::Jpeg),
            OperationCapability::PhotographicStylesAdapter,
        ]);

        assert!(inventory.supports(OperationCapability::GainMapTileEncoder(layout)));
        assert!(inventory.supports(OperationCapability::RasterDecoder(GainMapCodec::Jpeg)));
        assert!(inventory.supports(OperationCapability::PhotographicStylesAdapter));
        assert!(!inventory.supports(OperationCapability::PortraitAdapter));
    }

    #[test]
    fn concrete_codec_ports_remain_individually_dyn_compatible() {
        fn accept_encoder(
            _port: &dyn GainMapTileEncoder<Request = (), Output = (), Error = MockError>,
        ) {
        }
        fn accept_decoder(_port: &dyn RasterDecoder<Request = (), Output = (), Error = MockError>) {
        }

        let provider = MockCodecProvider;
        accept_encoder(&provider);
        accept_decoder(&provider);
    }
}
