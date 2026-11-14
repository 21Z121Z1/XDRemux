use crate::{ConversionRequest, MotionOutputTarget, OutputIntent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetAssetKind {
    MotionPhoto,
    ProXdr,
}

/// A destination chosen by the user, not a still codec or a conversion recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionTarget {
    Apple,
    AndroidStandard,
    Google,
    Samsung,
    Oppo,
    Vivo,
    Xiaomi,
    Huawei,
}

/// Available product routes. An Android standard Motion Photo writer is not
/// evidence of native gallery support for every vendor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetCapabilities {
    pub motion: Option<MotionOutputTarget>,
    pub proxdr: Option<OutputIntent>,
}

impl ConversionTarget {
    pub const fn capabilities(self) -> TargetCapabilities {
        match self {
            Self::Apple => TargetCapabilities {
                motion: Some(MotionOutputTarget::ApplePair),
                proxdr: Some(OutputIntent::Standard),
            },
            Self::AndroidStandard
            | Self::Google
            | Self::Samsung
            | Self::Vivo
            | Self::Xiaomi
            | Self::Huawei => TargetCapabilities {
                motion: Some(MotionOutputTarget::GoogleAuto),
                proxdr: Some(OutputIntent::Standard),
            },
            Self::Oppo => TargetCapabilities {
                motion: Some(MotionOutputTarget::GoogleAuto),
                proxdr: Some(OutputIntent::OppoGallery),
            },
        }
    }

    pub const fn is_apple(self) -> bool {
        matches!(self, Self::Apple)
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Apple => "apple",
            Self::AndroidStandard => "android-standard",
            Self::Google => "google",
            Self::Samsung => "samsung",
            Self::Oppo => "oppo",
            Self::Vivo => "vivo",
            Self::Xiaomi => "xiaomi",
            Self::Huawei => "huawei",
        }
    }

    /// Resolve the still-image policy only after source identification. OPPO's
    /// ProXDR intent must never leak into its standard Android Motion writer.
    pub fn apply_request(
        self,
        kind: TargetAssetKind,
        mut request: ConversionRequest,
    ) -> Result<ConversionRequest, &'static str> {
        let capabilities = self.capabilities();
        match kind {
            TargetAssetKind::MotionPhoto if capabilities.motion.is_none() => {
                Err("this vendor target has no verified Motion Photo route yet")
            }
            TargetAssetKind::ProXdr if capabilities.proxdr.is_none() => {
                Err("this vendor target has no verified ProXDR route yet")
            }
            TargetAssetKind::MotionPhoto => {
                request.output = OutputIntent::Standard;
                Ok(request)
            }
            TargetAssetKind::ProXdr => {
                let Some(output) = capabilities.proxdr else {
                    return Err("this vendor target has no verified ProXDR route yet");
                };
                request.output = output;
                Ok(request)
            }
        }
    }
}

impl std::str::FromStr for ConversionTarget {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "apple" => Ok(Self::Apple),
            "android-standard" | "standard" => Ok(Self::AndroidStandard),
            "google" => Ok(Self::Google),
            "samsung" => Ok(Self::Samsung),
            "oppo" => Ok(Self::Oppo),
            "vivo" => Ok(Self::Vivo),
            "xiaomi" => Ok(Self::Xiaomi),
            "huawei" => Ok(Self::Huawei),
            _ => Err(
                "target must be apple, android-standard (standard), google, samsung, oppo, vivo, xiaomi or huawei",
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destination_profiles_map_to_existing_supported_routes() {
        assert_eq!(
            ConversionTarget::Apple.capabilities().motion,
            Some(MotionOutputTarget::ApplePair)
        );
        for target in [
            ConversionTarget::AndroidStandard,
            ConversionTarget::Google,
            ConversionTarget::Samsung,
            ConversionTarget::Oppo,
            ConversionTarget::Vivo,
            ConversionTarget::Xiaomi,
            ConversionTarget::Huawei,
        ] {
            assert_eq!(
                target.capabilities().motion,
                Some(MotionOutputTarget::GoogleAuto)
            );
            assert!(target.capabilities().proxdr.is_some());
        }
        assert_eq!(
            ConversionTarget::Oppo.capabilities().proxdr,
            Some(OutputIntent::OppoGallery)
        );
        assert_eq!(
            ConversionTarget::Vivo.capabilities().proxdr,
            Some(OutputIntent::Standard)
        );
        assert_eq!(
            "standard".parse::<ConversionTarget>().unwrap(),
            ConversionTarget::AndroidStandard
        );
        assert_eq!(ConversionTarget::AndroidStandard.name(), "android-standard");
        assert!("android".parse::<ConversionTarget>().is_err());
    }

    #[test]
    fn oppo_still_policy_does_not_leak_into_android_motion_output() {
        let target = ConversionTarget::Oppo;
        let request = ConversionRequest::default();
        let still = target
            .apply_request(TargetAssetKind::ProXdr, request)
            .unwrap();
        assert_eq!(still.output, OutputIntent::OppoGallery);
        let motion = target
            .apply_request(TargetAssetKind::MotionPhoto, still)
            .unwrap();
        assert_eq!(motion.output, OutputIntent::Standard);
        let huawei = ConversionTarget::Huawei
            .apply_request(TargetAssetKind::MotionPhoto, request)
            .unwrap();
        assert_eq!(huawei.output, OutputIntent::Standard);
        let huawei_still = ConversionTarget::Huawei
            .apply_request(TargetAssetKind::ProXdr, request)
            .unwrap();
        assert_eq!(huawei_still.output, OutputIntent::Standard);
    }
}
