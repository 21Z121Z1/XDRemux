use std::collections::BTreeSet;
use std::error::Error;

use crate::{GainMapCodec, GainMapCodecLayout, GainMapEncoderCapabilities};

/// One operation the conversion planner may require from an external adapter.
///
/// This is deliberately operation-scoped rather than backend-scoped. A single
/// concrete adapter may implement several ports, and one conversion may combine
/// capabilities from several adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperationCapability {
    RasterDecoder(GainMapCodec),
    GainMapTileEncoder(GainMapCodecLayout),
    RawProcessor,
    ConsumerValidator,
    PhotographicStylesAdapter,
    PortraitAdapter,
}

/// Compatibility name for callers that still use the earlier planner term.
/// New code should prefer `OperationCapability`: these requirements are not
/// inherently platform-specific.
pub type PlatformCapability = OperationCapability;

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

/// Planner-facing inventory of capabilities available at the composition root.
///
/// It contains facts only: no adapter pointers and no execution state. This
/// keeps policy deterministic and prevents the inventory from becoming a
/// disguised monolithic `Backend` object.
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

    pub fn advertise_gain_map_tile_encoder<T>(&mut self, encoder: &T)
    where
        T: GainMapTileEncoder + ?Sized,
    {
        let capabilities = encoder.gain_map_encoder_capabilities();
        self.operations.extend(
            capabilities
                .iter()
                .map(OperationCapability::GainMapTileEncoder),
        );
    }

    pub fn advertise_raster_decoder<T>(&mut self, decoder: &T)
    where
        T: RasterDecoder + ?Sized,
    {
        let capabilities = decoder.raster_decoder_capabilities();
        self.operations.extend(
            capabilities
                .iter()
                .map(OperationCapability::RasterDecoder),
        );
    }

    pub fn advertise_raw_processor<T>(&mut self, _processor: &T)
    where
        T: RawProcessor + ?Sized,
    {
        self.operations.insert(OperationCapability::RawProcessor);
    }

    pub fn advertise_consumer_validator<T>(&mut self, _validator: &T)
    where
        T: ConsumerValidator + ?Sized,
    {
        self.operations
            .insert(OperationCapability::ConsumerValidator);
    }

    pub fn advertise_photographic_styles_adapter<T>(&mut self, _adapter: &T)
    where
        T: PhotographicStylesAdapter + ?Sized,
    {
        self.operations
            .insert(OperationCapability::PhotographicStylesAdapter);
    }

    pub fn advertise_portrait_adapter<T>(&mut self, _adapter: &T)
    where
        T: PortraitAdapter + ?Sized,
    {
        self.operations
            .insert(OperationCapability::PortraitAdapter);
    }
}

/// Outgoing port for encoding normalized Gain Map raster data into HEVC tiles.
///
/// Request/output payloads stay associated with the adapter for now so the
/// engine does not prematurely standardize a file IPC or in-process pixel ABI.
/// The planner depends only on the advertised codec layouts.
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

/// Outgoing port for platform or library RAW processing.
pub trait RawProcessor {
    type Request;
    type Output;
    type Error: Error;

    fn process_raw(
        &self,
        request: &Self::Request,
    ) -> std::result::Result<Self::Output, Self::Error>;
}

/// Outgoing port for consumer-specific validation such as ImageIO or Photos.
pub trait ConsumerValidator {
    type Request;
    type Output;
    type Error: Error;

    fn validate_consumer(
        &self,
        request: &Self::Request,
    ) -> std::result::Result<Self::Output, Self::Error>;
}

/// Apple-only outgoing port for Photographic Styles behavior.
pub trait PhotographicStylesAdapter {
    type Request;
    type Output;
    type Error: Error;

    fn apply_photographic_styles(
        &self,
        request: &Self::Request,
    ) -> std::result::Result<Self::Output, Self::Error>;
}

/// Apple-only outgoing port for Portrait behavior.
pub trait PortraitAdapter {
    type Request;
    type Output;
    type Error: Error;

    fn apply_portrait(
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

    struct MockAppleAdapter;

    impl GainMapTileEncoder for MockAppleAdapter {
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

    impl RasterDecoder for MockAppleAdapter {
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

    impl PhotographicStylesAdapter for MockAppleAdapter {
        type Request = ();
        type Output = ();
        type Error = MockError;

        fn apply_photographic_styles(
            &self,
            _request: &Self::Request,
        ) -> std::result::Result<Self::Output, Self::Error> {
            Ok(())
        }
    }

    struct MockValidator;

    impl ConsumerValidator for MockValidator {
        type Request = ();
        type Output = ();
        type Error = MockError;

        fn validate_consumer(
            &self,
            _request: &Self::Request,
        ) -> std::result::Result<Self::Output, Self::Error> {
            Ok(())
        }
    }

    #[test]
    fn independent_ports_compose_into_one_inventory_without_backend_trait() {
        let apple = MockAppleAdapter;
        let validator = MockValidator;
        let mut inventory = CapabilityInventory::default();
        inventory.advertise_gain_map_tile_encoder(&apple);
        inventory.advertise_raster_decoder(&apple);
        inventory.advertise_photographic_styles_adapter(&apple);
        inventory.advertise_consumer_validator(&validator);

        let layout = GainMapCodecLayout {
            chroma: ChromaSampling::Yuv420,
            luma_bit_depth: 8,
            chroma_bit_depth: 8,
        };
        assert!(inventory.supports(OperationCapability::GainMapTileEncoder(layout)));
        assert!(inventory.supports(OperationCapability::RasterDecoder(GainMapCodec::Jpeg)));
        assert!(inventory.supports(OperationCapability::PhotographicStylesAdapter));
        assert!(inventory.supports(OperationCapability::ConsumerValidator));
        assert!(!inventory.supports(OperationCapability::PortraitAdapter));
        assert!(!inventory.supports(OperationCapability::RawProcessor));
    }

    #[test]
    fn operation_ports_remain_individually_dyn_compatible() {
        fn accept_encoder(
            _port: &dyn GainMapTileEncoder<Request = (), Output = (), Error = MockError>,
        ) {
        }
        fn accept_decoder(
            _port: &dyn RasterDecoder<Request = (), Output = (), Error = MockError>,
        ) {
        }
        fn accept_styles(
            _port: &dyn PhotographicStylesAdapter<Request = (), Output = (), Error = MockError>,
        ) {
        }

        let apple = MockAppleAdapter;
        accept_encoder(&apple);
        accept_decoder(&apple);
        accept_styles(&apple);
    }
}
