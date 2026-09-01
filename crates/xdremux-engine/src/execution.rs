use std::error::Error;
use std::fmt;

use crate::{
    plan_conversion, CapabilityInventory, ConversionAnalysis, ConversionPlan, ConversionRequest,
    PlannerError,
};

/// Stable engine-owned phases for one conversion.
///
/// Front ends may render these as progress, logs, or telemetry, but they do not
/// decide ordering or skip phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStage {
    Plan,
    Build,
    Validate,
    Publish,
}

/// Port that materializes an unpublished conversion artifact from a resolved
/// product plan.
pub trait ArtifactBuilder {
    type Artifact;
    type Error: Error;

    fn build_artifact(
        &mut self,
        plan: &ConversionPlan,
    ) -> std::result::Result<Self::Artifact, Self::Error>;
}

/// Port that validates a fully materialized artifact before publication.
pub trait ArtifactValidator<Artifact> {
    type Error: Error;

    fn validate_artifact(
        &mut self,
        plan: &ConversionPlan,
        artifact: &Artifact,
    ) -> std::result::Result<(), Self::Error>;
}

/// Port that makes a validated artifact visible at its final destination.
pub trait ArtifactPublisher<Artifact> {
    type Output;
    type Error: Error;

    fn publish_artifact(
        &mut self,
        plan: &ConversionPlan,
        artifact: Artifact,
    ) -> std::result::Result<Self::Output, Self::Error>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionReceipt<Published> {
    pub plan: ConversionPlan,
    pub published: Published,
}

#[derive(Debug)]
pub enum ExecutionError<BuildError, ValidationError, PublicationError> {
    Plan(PlannerError),
    Build(BuildError),
    Validate(ValidationError),
    Publish(PublicationError),
}

impl<BuildError, ValidationError, PublicationError> fmt::Display
    for ExecutionError<BuildError, ValidationError, PublicationError>
where
    BuildError: fmt::Display,
    ValidationError: fmt::Display,
    PublicationError: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plan(error) => write!(f, "conversion planning failed: {error}"),
            Self::Build(error) => write!(f, "conversion build failed: {error}"),
            Self::Validate(error) => write!(f, "conversion validation failed: {error}"),
            Self::Publish(error) => write!(f, "conversion publication failed: {error}"),
        }
    }
}

impl<BuildError, ValidationError, PublicationError> Error
    for ExecutionError<BuildError, ValidationError, PublicationError>
where
    BuildError: Error + 'static,
    ValidationError: Error + 'static,
    PublicationError: Error + 'static,
{
}

/// Execute one conversion through the engine-owned lifecycle.
///
/// The artifact is never passed to the publisher until validation succeeds.
/// This is deliberately synchronous: codec/platform adapters may internally use
/// threads, but the product lifecycle remains deterministic and easy to audit.
pub fn execute_conversion<Builder, Validator, Publisher, Observe>(
    analysis: &ConversionAnalysis,
    request: ConversionRequest,
    capabilities: &CapabilityInventory,
    builder: &mut Builder,
    validator: &mut Validator,
    publisher: &mut Publisher,
    mut observe: Observe,
) -> std::result::Result<ExecutionReceipt<Publisher::Output>, ExecutionError<Builder::Error, Validator::Error, Publisher::Error>>
where
    Builder: ArtifactBuilder,
    Validator: ArtifactValidator<Builder::Artifact>,
    Publisher: ArtifactPublisher<Builder::Artifact>,
    Observe: FnMut(ExecutionStage),
{
    observe(ExecutionStage::Plan);
    let plan = plan_conversion(analysis, request, capabilities).map_err(ExecutionError::Plan)?;

    observe(ExecutionStage::Build);
    let artifact = builder
        .build_artifact(&plan)
        .map_err(ExecutionError::Build)?;

    observe(ExecutionStage::Validate);
    validator
        .validate_artifact(&plan, &artifact)
        .map_err(ExecutionError::Validate)?;

    observe(ExecutionStage::Publish);
    let published = publisher
        .publish_artifact(&plan, artifact)
        .map_err(ExecutionError::Publish)?;

    Ok(ExecutionReceipt { plan, published })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        GainMapChannels, GainMapCodec, GainMapCodecLayout, GainMapSourceProfile,
        GainMapStorageProfile, OperationCapability, SourceFamily, SourceHdrMode,
    };
    use std::cell::Cell;
    use std::io;
    use xdremux_format::ChromaSampling;

    fn layout() -> GainMapCodecLayout {
        GainMapCodecLayout {
            chroma: ChromaSampling::Yuv420,
            luma_bit_depth: 8,
            chroma_bit_depth: 8,
        }
    }

    fn analysis() -> ConversionAnalysis {
        ConversionAnalysis {
            source_family: SourceFamily::X7,
            hdr_mode: SourceHdrMode::Uhdr,
            gain_map: GainMapSourceProfile {
                width: 64,
                height: 32,
                channels: GainMapChannels::Rgb,
                storage: GainMapStorageProfile {
                    codec: GainMapCodec::Jpeg,
                    chroma: Some(ChromaSampling::Yuv420),
                    luma_bit_depth: 8,
                    chroma_bit_depth: 8,
                },
            },
        }
    }

    fn capabilities() -> CapabilityInventory {
        CapabilityInventory::new([
            OperationCapability::RasterDecoder(GainMapCodec::Jpeg),
            OperationCapability::GainMapTileEncoder(layout()),
        ])
    }

    struct Builder {
        calls: Cell<u32>,
    }

    impl ArtifactBuilder for Builder {
        type Artifact = Vec<u8>;
        type Error = io::Error;

        fn build_artifact(
            &mut self,
            _plan: &ConversionPlan,
        ) -> std::result::Result<Self::Artifact, Self::Error> {
            self.calls.set(self.calls.get() + 1);
            Ok(vec![1, 2, 3])
        }
    }

    struct Validator {
        calls: Cell<u32>,
        fail: bool,
    }

    impl ArtifactValidator<Vec<u8>> for Validator {
        type Error = io::Error;

        fn validate_artifact(
            &mut self,
            _plan: &ConversionPlan,
            artifact: &Vec<u8>,
        ) -> std::result::Result<(), Self::Error> {
            self.calls.set(self.calls.get() + 1);
            assert_eq!(artifact, &[1, 2, 3]);
            if self.fail {
                Err(io::Error::other("validator rejected artifact"))
            } else {
                Ok(())
            }
        }
    }

    struct Publisher {
        calls: Cell<u32>,
    }

    impl ArtifactPublisher<Vec<u8>> for Publisher {
        type Output = usize;
        type Error = io::Error;

        fn publish_artifact(
            &mut self,
            _plan: &ConversionPlan,
            artifact: Vec<u8>,
        ) -> std::result::Result<Self::Output, Self::Error> {
            self.calls.set(self.calls.get() + 1);
            Ok(artifact.len())
        }
    }

    #[test]
    fn lifecycle_is_plan_build_validate_publish() {
        let mut builder = Builder {
            calls: Cell::new(0),
        };
        let mut validator = Validator {
            calls: Cell::new(0),
            fail: false,
        };
        let mut publisher = Publisher {
            calls: Cell::new(0),
        };
        let mut stages = Vec::new();

        let receipt = execute_conversion(
            &analysis(),
            ConversionRequest::default(),
            &capabilities(),
            &mut builder,
            &mut validator,
            &mut publisher,
            |stage| stages.push(stage),
        )
        .unwrap();

        assert_eq!(receipt.published, 3);
        assert_eq!(builder.calls.get(), 1);
        assert_eq!(validator.calls.get(), 1);
        assert_eq!(publisher.calls.get(), 1);
        assert_eq!(
            stages,
            [
                ExecutionStage::Plan,
                ExecutionStage::Build,
                ExecutionStage::Validate,
                ExecutionStage::Publish,
            ]
        );
    }

    #[test]
    fn validation_failure_prevents_publication() {
        let mut builder = Builder {
            calls: Cell::new(0),
        };
        let mut validator = Validator {
            calls: Cell::new(0),
            fail: true,
        };
        let mut publisher = Publisher {
            calls: Cell::new(0),
        };
        let mut stages = Vec::new();

        let error = execute_conversion(
            &analysis(),
            ConversionRequest::default(),
            &capabilities(),
            &mut builder,
            &mut validator,
            &mut publisher,
            |stage| stages.push(stage),
        )
        .unwrap_err();

        assert!(matches!(error, ExecutionError::Validate(_)));
        assert_eq!(publisher.calls.get(), 0);
        assert_eq!(
            stages,
            [
                ExecutionStage::Plan,
                ExecutionStage::Build,
                ExecutionStage::Validate,
            ]
        );
    }

    #[test]
    fn planning_failure_prevents_all_side_effect_ports() {
        let mut builder = Builder {
            calls: Cell::new(0),
        };
        let mut validator = Validator {
            calls: Cell::new(0),
            fail: false,
        };
        let mut publisher = Publisher {
            calls: Cell::new(0),
        };
        let mut stages = Vec::new();

        let error = execute_conversion(
            &analysis(),
            ConversionRequest::default(),
            &CapabilityInventory::default(),
            &mut builder,
            &mut validator,
            &mut publisher,
            |stage| stages.push(stage),
        )
        .unwrap_err();

        assert!(matches!(error, ExecutionError::Plan(_)));
        assert_eq!(builder.calls.get(), 0);
        assert_eq!(validator.calls.get(), 0);
        assert_eq!(publisher.calls.get(), 0);
        assert_eq!(stages, [ExecutionStage::Plan]);
    }
}
