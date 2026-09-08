from pathlib import Path


def read(path: str) -> str:
    return Path(path).read_text()


def write(path: str, text: str) -> None:
    Path(path).write_text(text)


def replace_once(path: str, label: str, old: str, new: str) -> None:
    text = read(path)
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: {label}: expected one match, found {count}")
    write(path, text.replace(old, new, 1))


def splice(path: str, label: str, start: str, end: str, replacement: str) -> None:
    text = read(path)
    start_index = text.find(start)
    if start_index < 0:
        raise SystemExit(f"{path}: {label}: start marker not found")
    if text.find(start, start_index + 1) >= 0:
        raise SystemExit(f"{path}: {label}: start marker is ambiguous")
    end_index = text.find(end, start_index + len(start))
    if end_index < 0:
        raise SystemExit(f"{path}: {label}: end marker not found")
    write(path, text[:start_index] + replacement + text[end_index:])


# Engine consumer policy: keep the complete producer contract and add an explicit
# C contract that rejects synthesized semantics.
replace_once(
    "crates/xdremux-engine/src/apple.rs",
    "C consumer policy",
    '''    pub const fn satisfies_portrait_editing(self) -> bool {
        self.iso_gain_map
            && self.disparity
            && self.portrait_effects_matte
            && self.skin_matte
            && self.hair_matte
            && self.teeth_matte
            && self.glasses_matte
            && self.focus_metadata
    }
''',
    '''    pub const fn satisfies_portrait_editing(self) -> bool {
        self.iso_gain_map
            && self.disparity
            && self.portrait_effects_matte
            && self.skin_matte
            && self.hair_matte
            && self.teeth_matte
            && self.glasses_matte
            && self.focus_metadata
    }

    /// Resource contract for OPPO-native Portrait profile C.
    ///
    /// C intentionally publishes only producer-backed resources. Skin, teeth
    /// and glasses are absent instead of being invented when Vision is not used.
    pub const fn satisfies_oppo_native_portrait_editing(self) -> bool {
        self.iso_gain_map
            && self.disparity
            && self.portrait_effects_matte
            && self.hair_matte
            && self.focus_metadata
            && !self.skin_matte
            && !self.teeth_matte
            && !self.glasses_matte
    }
''',
)
replace_once(
    "crates/xdremux-engine/src/apple.rs",
    "C consumer policy regression",
    '''    #[test]
    fn portrait_semantic_contract_is_explicit_and_stable() {''',
    '''    #[test]
    fn oppo_native_portrait_contract_accepts_only_producer_backed_resources() {
        let c = AppleImageAuxiliaryFacts {
            iso_gain_map: true,
            disparity: true,
            portrait_effects_matte: true,
            skin_matte: false,
            hair_matte: true,
            teeth_matte: false,
            glasses_matte: false,
            focus_metadata: true,
        };
        assert!(c.satisfies_oppo_native_portrait_editing());
        assert!(!c.satisfies_portrait_editing());
        assert!(!AppleImageAuxiliaryFacts { skin_matte: true, ..c }
            .satisfies_oppo_native_portrait_editing());
        assert!(!AppleImageAuxiliaryFacts { hair_matte: false, ..c }
            .satisfies_oppo_native_portrait_editing());
    }

    #[test]
    fn portrait_semantic_contract_is_explicit_and_stable() {''',
)

# Runtime semantic profile. The established API continues to default to Complete.
replace_once(
    "crates/xdremux-runtime/src/oppo_portrait.rs",
    "semantic profile enum",
    '''#[cfg(target_os = "macos")]
const OPPO_PORTRAIT_PRIOR_LUMA_SIGMA: f32 = 0.15;

#[derive(Debug, Clone, PartialEq)]
pub struct ApplePortraitSourcePreflight {''',
    '''#[cfg(target_os = "macos")]
const OPPO_PORTRAIT_PRIOR_LUMA_SIGMA: f32 = 0.15;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ApplePortraitSemanticProfile {
    #[default]
    Complete,
    OppoNative,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApplePortraitSourcePreflight {''',
)
replace_once(
    "crates/xdremux-runtime/src/oppo_portrait.rs",
    "optional complete-only mattes",
    '''    pub portrait_effects_matte: AppleL8Mask,
    pub subject_prior_used: bool,
    pub skin_matte: AppleL8Mask,
    pub hair_matte: AppleL8Mask,
    pub hair_prior_added_high_confidence: bool,
    pub teeth_matte: AppleL8Mask,
    pub glasses_matte: AppleL8Mask,
    pub simulated_aperture: f64,
}''',
    '''    pub portrait_effects_matte: AppleL8Mask,
    pub subject_prior_used: bool,
    pub semantic_profile: ApplePortraitSemanticProfile,
    pub skin_matte: Option<AppleL8Mask>,
    pub hair_matte: AppleL8Mask,
    pub hair_prior_added_high_confidence: bool,
    pub teeth_matte: Option<AppleL8Mask>,
    pub glasses_matte: Option<AppleL8Mask>,
    pub simulated_aperture: f64,
}''',
)

splice(
    "crates/xdremux-runtime/src/oppo_portrait.rs",
    "profile-aware auxiliary payloads",
    '''impl ApplePortraitSourcePreflight {
''',
    '''#[cfg(any(target_os = "macos", test))]
fn producer_focus_region''',
    '''impl ApplePortraitSourcePreflight {
    /// Consume a completed Rust-owned Portrait preflight into the resource set
    /// selected by `semantic_profile`.
    pub fn into_auxiliary_payloads(self) -> Result<Vec<AppleAuxiliaryPayload>> {
        let disparity_span = f64::from(self.disparity.near - self.disparity.far);
        let focus_disparity = self
            .disparity
            .focus_disparity(
                self.focus.selected_rank,
                self.depth.header.disparity_exponentiation,
            )
            .map_err(|error| RuntimeError::external("Apple Portrait focus disparity", error))?;
        let gain_map_headroom = private_gain_map_headroom(self.private_gain_map_info.as_deref())?;
        let rendering_parameters = build_apple_portrait_rendering_parameters(
            self.camera_calibration.profile,
            focus_disparity,
            disparity_span,
            gain_map_headroom,
            self.config.aec_lux_index.map(f64::from),
            self.depth.header.near_object_detected,
        )
        .map_err(|error| RuntimeError::external("Apple Portrait REND", error))?;

        let mut payloads = Vec::with_capacity(match self.semantic_profile {
            ApplePortraitSemanticProfile::Complete => 6,
            ApplePortraitSemanticProfile::OppoNative => 3,
        });
        payloads.push(
            build_apple_portrait_disparity_payload(
                self.disparity,
                self.base_orientation,
                &self.camera_calibration,
                &rendering_parameters,
                self.simulated_aperture,
            )
            .map_err(|error| RuntimeError::external("Apple Portrait disparity payload", error))?,
        );
        payloads.push(
            build_apple_portrait_effects_matte_payload(
                self.portrait_effects_matte.width,
                self.portrait_effects_matte.height,
                self.portrait_effects_matte.pixels,
            )
            .map_err(|error| {
                RuntimeError::external("Apple Portrait effects matte payload", error)
            })?,
        );

        let mut push_semantic = |role: AppleSemanticRole, matte: AppleL8Mask| -> Result<()> {
            payloads.push(
                build_apple_semantic_matte_payload(role, matte.width, matte.height, matte.pixels)
                    .map_err(|error| {
                        RuntimeError::external("Apple Portrait semantic matte payload", error)
                    })?,
            );
            Ok(())
        };

        match self.semantic_profile {
            ApplePortraitSemanticProfile::Complete => {
                let skin = self.skin_matte.ok_or_else(|| {
                    RuntimeError::new(
                        "Apple Portrait semantic matte",
                        "complete profile is missing skin matte",
                    )
                })?;
                let teeth = self.teeth_matte.ok_or_else(|| {
                    RuntimeError::new(
                        "Apple Portrait semantic matte",
                        "complete profile is missing teeth matte",
                    )
                })?;
                let glasses = self.glasses_matte.ok_or_else(|| {
                    RuntimeError::new(
                        "Apple Portrait semantic matte",
                        "complete profile is missing glasses matte",
                    )
                })?;
                push_semantic(AppleSemanticRole::Skin, skin)?;
                push_semantic(AppleSemanticRole::Hair, self.hair_matte)?;
                push_semantic(AppleSemanticRole::Teeth, teeth)?;
                push_semantic(AppleSemanticRole::Glasses, glasses)?;
            }
            ApplePortraitSemanticProfile::OppoNative => {
                if self.skin_matte.is_some()
                    || self.teeth_matte.is_some()
                    || self.glasses_matte.is_some()
                {
                    return Err(RuntimeError::new(
                        "OPPO-native Portrait C",
                        "profile must not synthesize skin, teeth or glasses mattes",
                    ));
                }
                push_semantic(AppleSemanticRole::Hair, self.hair_matte)?;
            }
        }
        Ok(payloads)
    }
}

#[cfg(any(target_os = "macos", test))]
fn producer_focus_region''',
)

replace_once(
    "crates/xdremux-runtime/src/oppo_portrait.rs",
    "profile-aware preflight entry",
    '''#[cfg(target_os = "macos")]
pub(crate) fn prepare_apple_portrait_source(
    adapter: &AppleAdapterClient,
    input: &[u8],
) -> Result<ApplePortraitSourcePreflight> {
    let source = extract_oppo_portrait_source(input)''',
    '''#[cfg(target_os = "macos")]
pub(crate) fn prepare_apple_portrait_source(
    adapter: &AppleAdapterClient,
    input: &[u8],
) -> Result<ApplePortraitSourcePreflight> {
    prepare_apple_portrait_source_with_profile(
        adapter,
        input,
        ApplePortraitSemanticProfile::Complete,
    )
}

#[cfg(target_os = "macos")]
pub(crate) fn prepare_apple_portrait_source_with_profile(
    adapter: &AppleAdapterClient,
    input: &[u8],
    semantic_profile: ApplePortraitSemanticProfile,
) -> Result<ApplePortraitSourcePreflight> {
    let source = extract_oppo_portrait_source(input)''',
)

splice(
    "crates/xdremux-runtime/src/oppo_portrait.rs",
    "Vision vs producer-only semantic branch",
    '''    // Vision reports native semantic observations. Rust chooses the complete
''',
    '''    let simulated_aperture = resolve_simulated_aperture(
''',
    '''    let (
        portrait_effects_matte,
        subject_prior_used,
        skin_matte,
        hair_matte,
        hair_prior_added_high_confidence,
        teeth_matte,
        glasses_matte,
    ) = match semantic_profile {
        ApplePortraitSemanticProfile::Complete => {
            // Vision reports native semantic observations only for the complete
            // profile. Rust retains all product policy and OPPO-prior fusion.
            let mut vision_masks = adapter.vision_semantic_mattes(
                source_image_file.path(),
                &APPLE_PORTRAIT_SEMANTIC_ROLES,
                Some(u32::from(base_orientation)),
            )?;
            let native_person = vision_masks
                .remove(&AppleSemanticRole::Person)
                .ok_or_else(|| {
                    RuntimeError::new(
                        "Apple Portrait person matte",
                        "Vision omitted the requested person matte",
                    )
                })?;
            if !native_person.has_credible_foreground() {
                return Err(RuntimeError::new(
                    "Apple Portrait unavailable",
                    "Vision returned no credible person foreground",
                ));
            }
            let rendered_person = adapter.coreimage_render_l8(
                &native_person,
                target_width,
                target_height,
                base_orientation,
            )?;

            let mut render_role = |role| -> Result<AppleL8Mask> {
                let native = vision_masks.remove(&role).ok_or_else(|| {
                    RuntimeError::new(
                        "Apple Portrait semantic matte",
                        format!("Vision omitted the requested {role:?} matte"),
                    )
                })?;
                adapter.coreimage_render_l8(
                    &native,
                    target_width,
                    target_height,
                    base_orientation,
                )
            };
            let rendered_skin = render_role(AppleSemanticRole::Skin)?;
            let rendered_hair = render_role(AppleSemanticRole::Hair)?;
            let rendered_teeth = render_role(AppleSemanticRole::Teeth)?;
            let rendered_glasses = render_role(AppleSemanticRole::Glasses)?;

            let person_fusion =
                fuse_apple_portrait_person_mask(&rendered_person, subject_prior.as_ref())
                    .map_err(|error| RuntimeError::external("Apple Portrait person fusion", error))?;
            let hair_fusion = fuse_apple_portrait_hair_mask(
                &rendered_hair,
                hair_prior.as_ref(),
                &person_fusion.mask,
            )
            .map_err(|error| RuntimeError::external("Apple Portrait hair fusion", error))?;
            (
                person_fusion.mask,
                person_fusion.used_prior,
                Some(rendered_skin),
                hair_fusion.mask,
                hair_fusion.prior_added_high_confidence,
                Some(rendered_teeth),
                Some(rendered_glasses),
            )
        }
        ApplePortraitSemanticProfile::OppoNative => {
            let subject = subject_prior.ok_or_else(|| {
                RuntimeError::new(
                    "OPPO-native Portrait C",
                    "OPPO portrait plane is unavailable; source only supports a lower profile",
                )
            })?;
            if !subject.has_credible_foreground() {
                return Err(RuntimeError::new(
                    "OPPO-native Portrait C",
                    "OPPO portrait plane has no credible foreground",
                ));
            }
            let hair = hair_prior.ok_or_else(|| {
                RuntimeError::new(
                    "OPPO-native Portrait C",
                    "OPPO hair plane is unavailable; source only supports profile B",
                )
            })?;
            if hair.width != subject.width || hair.height != subject.height {
                return Err(RuntimeError::new(
                    "OPPO-native Portrait C",
                    "OPPO hair and portrait planes do not share geometry",
                ));
            }
            let hair = AppleL8Mask::new(
                hair.width,
                hair.height,
                hair.pixels
                    .iter()
                    .zip(&subject.pixels)
                    .map(|(&hair_pixel, &subject_pixel)| hair_pixel.min(subject_pixel))
                    .collect(),
            )
            .map_err(|error| RuntimeError::external("OPPO-native hair matte", error))?;
            let hair_high_confidence = hair.has_credible_foreground();
            (
                subject,
                true,
                None,
                hair,
                hair_high_confidence,
                None,
                None,
            )
        }
    };

    let simulated_aperture = resolve_simulated_aperture(
''',
)

replace_once(
    "crates/xdremux-runtime/src/oppo_portrait.rs",
    "profile preflight result",
    '''        portrait_effects_matte: person_fusion.mask,
        subject_prior_used: person_fusion.used_prior,
        skin_matte: rendered_skin,
        hair_matte: hair_fusion.mask,
        hair_prior_added_high_confidence: hair_fusion.prior_added_high_confidence,
        teeth_matte: rendered_teeth,
        glasses_matte: rendered_glasses,
        simulated_aperture,''',
    '''        portrait_effects_matte,
        subject_prior_used,
        semantic_profile,
        skin_matte,
        hair_matte,
        hair_prior_added_high_confidence,
        teeth_matte,
        glasses_matte,
        simulated_aperture,''',
)
replace_once(
    "crates/xdremux-runtime/src/oppo_portrait.rs",
    "fixture producer-plane regression",
    '''    #[test]
    fn private_gain_map_headroom_matches_the_swift_stop_mapping() {''',
    '''    #[test]
    fn committed_portrait_fixture_supports_oppo_native_profile_c() {
        let source = portrait_fixture();
        let source = extract_oppo_portrait_source(&source).expect("extract OPPO portrait source");
        let depth = decode_oppo_portrait_depth(&source.compressed_depth)
            .expect("decode and parse OPPO Portrait depth");
        assert!(
            depth.portrait.as_ref().is_some_and(|plane| plane.iter().any(|&value| value != 0)),
            "profile C requires a non-empty OPPO portrait plane"
        );
        assert!(
            depth.hair.as_ref().is_some_and(|plane| plane.iter().any(|&value| value != 0)),
            "profile C requires a non-empty OPPO hair plane"
        );
    }

    #[test]
    fn private_gain_map_headroom_matches_the_swift_stop_mapping() {''',
)

# Runtime API. Existing complete methods remain compatibility wrappers.
replace_once(
    "crates/xdremux-runtime/src/lib.rs",
    "profile export",
    "pub use oppo_portrait::ApplePortraitSourcePreflight;",
    "pub use oppo_portrait::{ApplePortraitSemanticProfile, ApplePortraitSourcePreflight};",
)
replace_once(
    "crates/xdremux-runtime/src/lib.rs",
    "profile-aware preflight API",
    '''    #[cfg(target_os = "macos")]
    pub fn preflight_apple_portrait_source(
        &self,
        executable: impl AsRef<Path>,
        source: &[u8],
    ) -> Result<ApplePortraitSourcePreflight> {
        let adapter = apple_adapter::AppleAdapterClient::new(executable.as_ref().to_path_buf());
        oppo_portrait::prepare_apple_portrait_source(&adapter, source)
    }
''',
    '''    #[cfg(target_os = "macos")]
    pub fn preflight_apple_portrait_source(
        &self,
        executable: impl AsRef<Path>,
        source: &[u8],
    ) -> Result<ApplePortraitSourcePreflight> {
        let adapter = apple_adapter::AppleAdapterClient::new(executable.as_ref().to_path_buf());
        oppo_portrait::prepare_apple_portrait_source(&adapter, source)
    }

    #[cfg(target_os = "macos")]
    pub fn preflight_apple_portrait_source_with_profile(
        &self,
        executable: impl AsRef<Path>,
        source: &[u8],
        profile: ApplePortraitSemanticProfile,
    ) -> Result<ApplePortraitSourcePreflight> {
        let adapter = apple_adapter::AppleAdapterClient::new(executable.as_ref().to_path_buf());
        oppo_portrait::prepare_apple_portrait_source_with_profile(&adapter, source, profile)
    }
''',
)
replace_once(
    "crates/xdremux-runtime/src/lib.rs",
    "profile-aware conversion API",
    '''    #[cfg(target_os = "macos")]
    pub fn convert_apple_portrait_file(
        &self,
        executable: impl AsRef<Path>,
        source: &[u8],
        output: impl AsRef<Path>,
    ) -> Result<ApplePortraitFileReceipt> {
        let output = output.as_ref();''',
    '''    #[cfg(target_os = "macos")]
    pub fn convert_apple_portrait_file(
        &self,
        executable: impl AsRef<Path>,
        source: &[u8],
        output: impl AsRef<Path>,
    ) -> Result<ApplePortraitFileReceipt> {
        self.convert_apple_portrait_file_with_profile(
            executable,
            source,
            output,
            ApplePortraitSemanticProfile::Complete,
        )
    }

    #[cfg(target_os = "macos")]
    pub fn convert_apple_portrait_file_with_profile(
        &self,
        executable: impl AsRef<Path>,
        source: &[u8],
        output: impl AsRef<Path>,
        profile: ApplePortraitSemanticProfile,
    ) -> Result<ApplePortraitFileReceipt> {
        let output = output.as_ref();''',
)
replace_once(
    "crates/xdremux-runtime/src/lib.rs",
    "profile-aware source preparation",
    "        let preflight = oppo_portrait::prepare_apple_portrait_source(&adapter, source)?;",
    '''        let preflight = oppo_portrait::prepare_apple_portrait_source_with_profile(
            &adapter, source, profile,
        )?;''',
)
replace_once(
    "crates/xdremux-runtime/src/lib.rs",
    "profile-aware consumer validation",
    '''        let facts = adapter.imageio_auxiliary_facts(&output_path)?;
        if !facts.satisfies_portrait_editing() {
            return Err(RuntimeError::new(
                "Apple Portrait consumer validation",
                format!("ImageIO did not expose the complete Portrait resource set: {facts:?}"),
            ));
        }''',
    '''        let facts = adapter.imageio_auxiliary_facts(&output_path)?;
        let accepted = match profile {
            ApplePortraitSemanticProfile::Complete => facts.satisfies_portrait_editing(),
            ApplePortraitSemanticProfile::OppoNative => {
                facts.satisfies_oppo_native_portrait_editing()
            }
        };
        if !accepted {
            return Err(RuntimeError::new(
                "Apple Portrait consumer validation",
                format!(
                    "ImageIO did not expose the requested {profile:?} Portrait resource set: {facts:?}"
                ),
            ));
        }''',
)

# Batch: keep the existing API as Complete and expose an opt-in profile-aware
# wrapper so unrelated callers do not gain a new required option field.
replace_once(
    "crates/xdremux-runtime/src/batch.rs",
    "batch profile import",
    "use crate::{PortableRuntime, Result, RuntimeError};",
    "use crate::{ApplePortraitSemanticProfile, PortableRuntime, Result, RuntimeError};",
)
replace_once(
    "crates/xdremux-runtime/src/batch.rs",
    "batch worker profile argument",
    '''    reuse_existing: bool,
    apple_adapter_executable: Option<&Path>,
) -> BatchWorkResult {''',
    '''    reuse_existing: bool,
    apple_adapter_executable: Option<&Path>,
    apple_portrait_profile: ApplePortraitSemanticProfile,
) -> BatchWorkResult {''',
)
replace_once(
    "crates/xdremux-runtime/src/batch.rs",
    "batch profile conversion",
    '''                        let receipt =
                            runtime.convert_apple_portrait_file(adapter, &source, &item.output)?;''',
    '''                        let receipt = runtime.convert_apple_portrait_file_with_profile(
                            adapter,
                            &source,
                            &item.output,
                            apple_portrait_profile,
                        )?;''',
)
# Rename the current implementation and put the old method back as a wrapper.
replace_once(
    "crates/xdremux-runtime/src/batch.rs",
    "batch profile-aware API",
    '''    pub fn convert_batch_with_options<I>(
        &self,
        items: I,
        request: ConversionRequest,
        options: &BatchExecutionOptions,
    ) -> BatchReceipt
    where
        I: IntoIterator<Item = BatchItem>,
    {
''',
    '''    pub fn convert_batch_with_options<I>(
        &self,
        items: I,
        request: ConversionRequest,
        options: &BatchExecutionOptions,
    ) -> BatchReceipt
    where
        I: IntoIterator<Item = BatchItem>,
    {
        self.convert_batch_with_options_and_portrait_profile(
            items,
            request,
            options,
            ApplePortraitSemanticProfile::Complete,
        )
    }

    pub fn convert_batch_with_options_and_portrait_profile<I>(
        &self,
        items: I,
        request: ConversionRequest,
        options: &BatchExecutionOptions,
        apple_portrait_profile: ApplePortraitSemanticProfile,
    ) -> BatchReceipt
    where
        I: IntoIterator<Item = BatchItem>,
    {
''',
)
# Two worker call sites: serial and scoped parallel.
batch = read("crates/xdremux-runtime/src/batch.rs")
needle = '''                    options.apple_adapter_executable.as_deref(),
                );'''
if batch.count(needle) != 1:
    raise SystemExit(
        f"crates/xdremux-runtime/src/batch.rs: serial profile call: expected one match, found {batch.count(needle)}"
    )
batch = batch.replace(
    needle,
    '''                    options.apple_adapter_executable.as_deref(),
                    apple_portrait_profile,
                );''',
    1,
)
needle = '''                            apple_adapter_executable.as_deref(),
                        );'''
if batch.count(needle) != 1:
    raise SystemExit(
        f"crates/xdremux-runtime/src/batch.rs: parallel profile call: expected one match, found {batch.count(needle)}"
    )
batch = batch.replace(
    needle,
    '''                            apple_adapter_executable.as_deref(),
                            apple_portrait_profile,
                        );''',
    1,
)
write("crates/xdremux-runtime/src/batch.rs", batch)

# CLI: explicit C flag; existing --apple-portrait remains the complete path.
replace_once(
    "crates/xdremux-cli/src/lib.rs",
    "runtime profile import",
    '''    motion_photo_checkpoint_path, plan_batch_items, BatchAssetKind, BatchExecutionOptions,
    BatchPlanOptions, BatchSuccessDisposition, PortableRuntime,
};''',
    '''    motion_photo_checkpoint_path, plan_batch_items, ApplePortraitSemanticProfile,
    BatchAssetKind, BatchExecutionOptions, BatchPlanOptions, BatchSuccessDisposition,
    PortableRuntime,
};''',
)
replace_once(
    "crates/xdremux-cli/src/lib.rs",
    "C CLI flag",
    '''    #[arg(long, conflicts_with_all = ["apple_portrait", "apple_styles"])]
    oppo_compatible: bool,
    /// Preserve Apple Portrait editing resources when converting an OPPO Portrait still.
    #[arg(long, conflicts_with_all = ["oppo_compatible", "apple_styles"])]
    apple_portrait: bool,
    /// Attach the Rust-owned Photographic Styles graph to a ProXDR still.
    #[arg(long, conflicts_with_all = ["oppo_compatible", "apple_portrait"])]
    apple_styles: bool,''',
    '''    #[arg(long, conflicts_with_all = ["apple_portrait", "apple_portrait_oppo", "apple_styles"])]
    oppo_compatible: bool,
    /// Preserve the complete Apple Portrait editing resource set using Vision semantics.
    #[arg(long, conflicts_with_all = ["oppo_compatible", "apple_portrait_oppo", "apple_styles"])]
    apple_portrait: bool,
    /// Build Portrait profile C only from OPPO depth/focus/aperture/portrait/hair data.
    #[arg(long, conflicts_with_all = ["oppo_compatible", "apple_portrait", "apple_styles"])]
    apple_portrait_oppo: bool,
    /// Attach the Rust-owned Photographic Styles graph to a ProXDR still.
    #[arg(long, conflicts_with_all = ["oppo_compatible", "apple_portrait", "apple_portrait_oppo"])]
    apple_styles: bool,''',
)
splice(
    "crates/xdremux-cli/src/lib.rs",
    "CLI product request and profile",
    '''    fn request(&self) -> ConversionRequest {
''',
    '''}

#[derive(Debug, Args)]
struct ConvertArgs''',
    '''    fn request(&self) -> ConversionRequest {
        if self.oppo_compatible {
            ConversionRequest::oppo_gallery_compatible()
        } else if self.apple_portrait || self.apple_portrait_oppo {
            ConversionRequest {
                apple_features: AppleFeatureRequest {
                    portrait: true,
                    ..AppleFeatureRequest::default()
                },
                ..ConversionRequest::default()
            }
        } else if self.apple_styles {
            ConversionRequest {
                apple_features: AppleFeatureRequest {
                    photographic_styles: true,
                    ..AppleFeatureRequest::default()
                },
                ..ConversionRequest::default()
            }
        } else {
            ConversionRequest::default()
        }
    }

    fn portrait_profile(&self) -> ApplePortraitSemanticProfile {
        if self.apple_portrait_oppo {
            ApplePortraitSemanticProfile::OppoNative
        } else {
            ApplePortraitSemanticProfile::Complete
        }
    }
}

#[derive(Debug, Args)]
struct ConvertArgs''',
)
replace_once(
    "crates/xdremux-cli/src/lib.rs",
    "single convert profile selection",
    '''    let apple_portrait = arguments.product.apple_portrait;
    let apple_styles = arguments.product.apple_styles;''',
    '''    let apple_portrait =
        arguments.product.apple_portrait || arguments.product.apple_portrait_oppo;
    let apple_portrait_profile = arguments.product.portrait_profile();
    let apple_styles = arguments.product.apple_styles;''',
)
replace_once(
    "crates/xdremux-cli/src/lib.rs",
    "single convert profile call",
    '''                    match runtime.convert_apple_portrait_file(&adapter, &source, &output) {''',
    '''                    match runtime.convert_apple_portrait_file_with_profile(
                        &adapter,
                        &source,
                        &output,
                        apple_portrait_profile,
                    ) {''',
)
replace_once(
    "crates/xdremux-cli/src/lib.rs",
    "batch adapter resolution for C",
    '''        if arguments.product.apple_portrait || arguments.product.apple_styles {''',
    '''        if arguments.product.apple_portrait
            || arguments.product.apple_portrait_oppo
            || arguments.product.apple_styles
        {''',
)
replace_once(
    "crates/xdremux-cli/src/lib.rs",
    "batch profile call",
    '''    let receipt =
        runtime.convert_batch_with_options(items, arguments.product.request(), &execution_options);''',
    '''    let receipt = runtime.convert_batch_with_options_and_portrait_profile(
        items,
        arguments.product.request(),
        &execution_options,
        arguments.product.portrait_profile(),
    );''',
)
replace_once(
    "crates/xdremux-cli/src/lib.rs",
    "default C flag assertion",
    '''        assert!(!arguments.product.apple_portrait);''',
    '''        assert!(!arguments.product.apple_portrait);
        assert!(!arguments.product.apple_portrait_oppo);''',
)
replace_once(
    "crates/xdremux-cli/src/lib.rs",
    "C CLI tests",
    '''    #[test]
    fn convert_accepts_explicit_output() {''',
    '''    #[test]
    fn convert_accepts_oppo_native_portrait_profile() {
        let command = parse(&[
            "convert",
            "--input",
            "portrait.heic",
            "--apple-portrait-oppo",
        ]);
        let RootCommand::Convert(arguments) = command.command else {
            panic!("expected convert command");
        };
        assert!(arguments.product.apple_portrait_oppo);
        assert_eq!(
            arguments.product.portrait_profile(),
            ApplePortraitSemanticProfile::OppoNative
        );
        assert!(arguments.product.request().apple_features.portrait);
    }

    #[test]
    fn convert_rejects_combining_complete_and_oppo_native_portrait() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(
            run_from(
                [
                    "convert",
                    "--input",
                    "portrait.heic",
                    "--apple-portrait",
                    "--apple-portrait-oppo",
                ],
                &mut stdout,
                &mut stderr,
            ),
            2
        );
        assert!(stdout.is_empty());
        assert!(String::from_utf8(stderr)
            .unwrap()
            .contains("cannot be used with"));
    }

    #[test]
    fn convert_accepts_explicit_output() {''',
)

print("OPPO-native Portrait C patch applied")
