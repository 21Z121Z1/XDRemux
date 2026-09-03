use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use crate::{PhotoAsset, PhotoAssetType, PhotoClassification, PhotoResource, PhotoResourceRole};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceFingerprint(Vec<u8>);

impl ResourceFingerprint {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhotoAssetPlanningInput {
    pub asset: PhotoAsset,
    pub classification: PhotoClassification,
    pub fingerprints: BTreeMap<PathBuf, ResourceFingerprint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategorizationDestinationRoot {
    Explicit(PathBuf),
    AlongsidePrimary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhotoAssetCategorizationDisposition {
    Copy,
    Duplicate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhotoAssetCategorizationItem {
    pub asset_id: String,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub role: PhotoResourceRole,
    pub disposition: PhotoAssetCategorizationDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PhotoAssetCategorizationPlan {
    pub items: Vec<PhotoAssetCategorizationItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhotoAssetPlanningError {
    MissingPrimaryImage {
        asset_id: String,
    },
    MultiplePrimaryImages {
        asset_id: String,
    },
    AssetTypeMismatch {
        asset_id: String,
        asset_type: PhotoAssetType,
        classification_asset_type: PhotoAssetType,
    },
    DuplicateResourcePath {
        path: PathBuf,
    },
    MissingFingerprint {
        path: PathBuf,
    },
    UnexpectedFingerprint {
        path: PathBuf,
    },
    InvalidResourceName {
        path: PathBuf,
    },
    DuplicateDestination {
        path: PathBuf,
    },
    SequenceOverflow {
        asset_id: String,
    },
}

impl std::fmt::Display for PhotoAssetPlanningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPrimaryImage { asset_id } => {
                write!(f, "photo asset {asset_id} has no primary image")
            }
            Self::MultiplePrimaryImages { asset_id } => {
                write!(f, "photo asset {asset_id} has more than one primary image")
            }
            Self::AssetTypeMismatch {
                asset_id,
                asset_type,
                classification_asset_type,
            } => write!(
                f,
                "photo asset {asset_id} is {asset_type:?} but its classification is {classification_asset_type:?}"
            ),
            Self::DuplicateResourcePath { path } => {
                write!(f, "photo asset repeats resource path {}", path.display())
            }
            Self::MissingFingerprint { path } => {
                write!(f, "photo resource {} has no content fingerprint", path.display())
            }
            Self::UnexpectedFingerprint { path } => write!(
                f,
                "content fingerprint {} does not belong to the photo asset",
                path.display()
            ),
            Self::InvalidResourceName { path } => {
                write!(f, "photo resource {} has no file name", path.display())
            }
            Self::DuplicateDestination { path } => write!(
                f,
                "two resources in one photo asset map to {}",
                path.display()
            ),
            Self::SequenceOverflow { asset_id } => {
                write!(f, "photo asset {asset_id} exhausted destination sequence numbers")
            }
        }
    }
}

impl std::error::Error for PhotoAssetPlanningError {}

pub type PlanningResult<T> = std::result::Result<T, PhotoAssetPlanningError>;

fn primary_image(asset: &PhotoAsset) -> PlanningResult<&Path> {
    let mut primary = asset
        .resources
        .iter()
        .filter(|resource| resource.role == PhotoResourceRole::PrimaryImage);
    let first = primary
        .next()
        .ok_or_else(|| PhotoAssetPlanningError::MissingPrimaryImage {
            asset_id: asset.id.clone(),
        })?;
    if primary.next().is_some() {
        return Err(PhotoAssetPlanningError::MultiplePrimaryImages {
            asset_id: asset.id.clone(),
        });
    }
    Ok(first.path.as_path())
}

fn is_mov(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case("mov"))
}

fn is_same_stem_sibling(image: &Path, video: &Path) -> bool {
    is_mov(video) && image.parent() == video.parent() && image.file_stem() == video.file_stem()
}

/// Resolves one user-visible photo asset from an image and an optional sibling MOV candidate.
///
/// A video is claimed only when it is a same-stem sibling MOV and the caller-supplied validator
/// proves the pair. The validator is deliberately injected: content-identifier parsing belongs to
/// the Motion Photo / Live Photo format layer, while classification owns the asset grouping policy.
pub fn resolve_photo_asset(
    primary_image: impl Into<PathBuf>,
    companion_video: Option<PathBuf>,
    inferred_asset_type: PhotoAssetType,
    pair_validator: impl FnOnce(&Path, &Path) -> bool,
) -> PhotoAsset {
    let image = primary_image.into();
    if let Some(video) = companion_video
        .filter(|video| is_same_stem_sibling(&image, video) && pair_validator(&image, video))
    {
        let id = image.to_string_lossy().into_owned();
        return PhotoAsset::live_photo(image, video, id);
    }

    match inferred_asset_type {
        PhotoAssetType::StaticPhoto => PhotoAsset::static_photo(image),
        PhotoAssetType::LivePhoto => PhotoAsset {
            id: image.to_string_lossy().into_owned(),
            asset_type: PhotoAssetType::LivePhoto,
            resources: vec![PhotoResource {
                path: image,
                role: PhotoResourceRole::PrimaryImage,
            }],
        },
    }
}

fn validate_input(input: &PhotoAssetPlanningInput) -> PlanningResult<&Path> {
    let primary = primary_image(&input.asset)?;
    if input.asset.asset_type != input.classification.asset_type {
        return Err(PhotoAssetPlanningError::AssetTypeMismatch {
            asset_id: input.asset.id.clone(),
            asset_type: input.asset.asset_type,
            classification_asset_type: input.classification.asset_type,
        });
    }

    let mut resource_paths = BTreeSet::new();
    for resource in &input.asset.resources {
        if !resource_paths.insert(resource.path.clone()) {
            return Err(PhotoAssetPlanningError::DuplicateResourcePath {
                path: resource.path.clone(),
            });
        }
        if !input.fingerprints.contains_key(&resource.path) {
            return Err(PhotoAssetPlanningError::MissingFingerprint {
                path: resource.path.clone(),
            });
        }
    }
    if let Some(path) = input
        .fingerprints
        .keys()
        .find(|path| !resource_paths.contains(*path))
    {
        return Err(PhotoAssetPlanningError::UnexpectedFingerprint { path: path.clone() });
    }
    Ok(primary)
}

fn sequenced_file_name(source: &Path, sequence: u32) -> PlanningResult<OsString> {
    let file_name =
        source
            .file_name()
            .ok_or_else(|| PhotoAssetPlanningError::InvalidResourceName {
                path: source.to_path_buf(),
            })?;
    if sequence == 1 {
        return Ok(file_name.to_os_string());
    }

    let stem = source
        .file_stem()
        .ok_or_else(|| PhotoAssetPlanningError::InvalidResourceName {
            path: source.to_path_buf(),
        })?;
    let mut name = OsString::from(stem);
    name.push(format!(" ({sequence})"));
    if let Some(extension) = source.extension() {
        name.push(".");
        name.push(extension);
    }
    Ok(name)
}

fn destination_directory(
    primary: &Path,
    classification: &PhotoClassification,
    root: &CategorizationDestinationRoot,
) -> PathBuf {
    let mut directory = match root {
        CategorizationDestinationRoot::Explicit(path) => path.clone(),
        CategorizationDestinationRoot::AlongsidePrimary => {
            primary.parent().unwrap_or(Path::new("")).to_path_buf()
        }
    };
    for component in classification.relative_directory_components() {
        directory.push(component);
    }
    directory
}

/// Plans categorization for already-resolved photo assets without touching the filesystem.
///
/// `existing_destinations` contains opaque content fingerprints for paths that already exist on
/// disk. The planner also reserves destinations selected for earlier assets. If any resource in a
/// multi-resource asset conflicts, the whole asset advances to the next sequence so a Live Photo
/// can never be split across `name.*` and `name (2).*`.
pub fn plan_photo_assets(
    inputs: &[PhotoAssetPlanningInput],
    root: CategorizationDestinationRoot,
    existing_destinations: &BTreeMap<PathBuf, ResourceFingerprint>,
) -> PlanningResult<PhotoAssetCategorizationPlan> {
    let mut ordered = inputs
        .iter()
        .map(|input| validate_input(input).map(|primary| (primary.to_path_buf(), input)))
        .collect::<PlanningResult<Vec<_>>>()?;
    ordered.sort_by(|left, right| left.0.cmp(&right.0));

    let mut reserved = BTreeMap::<PathBuf, ResourceFingerprint>::new();
    let mut items = Vec::new();

    for (primary, input) in ordered {
        let directory = destination_directory(&primary, &input.classification, &root);
        let mut sequence = 1u32;

        loop {
            let mut candidate_destinations = BTreeSet::new();
            let mut candidates = Vec::with_capacity(input.asset.resources.len());
            for resource in &input.asset.resources {
                let destination = directory.join(sequenced_file_name(&resource.path, sequence)?);
                if !candidate_destinations.insert(destination.clone()) {
                    return Err(PhotoAssetPlanningError::DuplicateDestination {
                        path: destination,
                    });
                }
                let fingerprint = input
                    .fingerprints
                    .get(&resource.path)
                    .expect("fingerprints validated above");
                candidates.push((resource, destination, fingerprint));
            }

            let mut dispositions = Vec::with_capacity(candidates.len());
            let mut conflict = false;
            for (_, destination, fingerprint) in &candidates {
                let prior = reserved
                    .get(destination)
                    .or_else(|| existing_destinations.get(destination));
                match prior {
                    Some(prior) if prior == *fingerprint => {
                        dispositions.push(PhotoAssetCategorizationDisposition::Duplicate);
                    }
                    Some(_) => {
                        conflict = true;
                        break;
                    }
                    None => dispositions.push(PhotoAssetCategorizationDisposition::Copy),
                }
            }

            if conflict {
                sequence = sequence.checked_add(1).ok_or_else(|| {
                    PhotoAssetPlanningError::SequenceOverflow {
                        asset_id: input.asset.id.clone(),
                    }
                })?;
                continue;
            }

            for ((resource, destination, fingerprint), disposition) in
                candidates.into_iter().zip(dispositions)
            {
                reserved
                    .entry(destination.clone())
                    .or_insert_with(|| fingerprint.clone());
                items.push(PhotoAssetCategorizationItem {
                    asset_id: input.asset.id.clone(),
                    source: resource.path.clone(),
                    destination,
                    role: resource.role,
                    disposition,
                });
            }
            break;
        }
    }

    Ok(PhotoAssetCategorizationPlan { items })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    use crate::classify_user_comment_with_context;

    fn fingerprint(value: &str) -> ResourceFingerprint {
        ResourceFingerprint::new(value.as_bytes().to_vec())
    }

    fn planning_input(
        asset: PhotoAsset,
        comment: &str,
        fingerprints: &[(&str, &str)],
    ) -> PhotoAssetPlanningInput {
        let classification =
            classify_user_comment_with_context(Some(comment), asset.asset_type, BTreeSet::new());
        PhotoAssetPlanningInput {
            asset,
            classification,
            fingerprints: fingerprints
                .iter()
                .map(|(path, value)| (PathBuf::from(path), fingerprint(value)))
                .collect(),
        }
    }

    #[test]
    fn resolve_asset_claims_only_validated_same_stem_sibling_mov() {
        let asset = resolve_photo_asset(
            "/input/pair.heic",
            Some(PathBuf::from("/input/pair.mov")),
            PhotoAssetType::StaticPhoto,
            |image, video| image.ends_with("pair.heic") && video.ends_with("pair.mov"),
        );
        assert_eq!(asset.asset_type, PhotoAssetType::LivePhoto);
        assert_eq!(
            asset
                .resources
                .iter()
                .map(|resource| resource.role)
                .collect::<Vec<_>>(),
            vec![
                PhotoResourceRole::PrimaryImage,
                PhotoResourceRole::PairedVideo
            ]
        );

        let rejected = resolve_photo_asset(
            "/input/pair.heic",
            Some(PathBuf::from("/input/other.mov")),
            PhotoAssetType::StaticPhoto,
            |_, _| panic!("different-stem candidate must not reach validator"),
        );
        assert_eq!(rejected.asset_type, PhotoAssetType::StaticPhoto);
        assert_eq!(rejected.resources.len(), 1);
    }

    #[test]
    fn resolve_asset_preserves_inferred_embedded_live_photo_without_companion() {
        let asset = resolve_photo_asset(
            "/input/embedded.heic",
            None,
            PhotoAssetType::LivePhoto,
            |_, _| false,
        );
        assert_eq!(asset.asset_type, PhotoAssetType::LivePhoto);
        assert_eq!(asset.resources.len(), 1);
        assert_eq!(asset.resources[0].role, PhotoResourceRole::PrimaryImage);
    }

    #[test]
    fn asset_planning_contract_keeps_validated_live_pair_on_shared_collision_sequence() {
        let asset = PhotoAsset::live_photo("/input/pair.heic", "/input/pair.mov", "pair");
        let input = planning_input(
            asset,
            "oplus_18",
            &[("/input/pair.heic", "image"), ("/input/pair.mov", "video")],
        );
        let existing = BTreeMap::from([(
            PathBuf::from("/output/实况照片/人像/pair.heic"),
            fingerprint("foreign-image"),
        )]);

        let plan = plan_photo_assets(
            &[input],
            CategorizationDestinationRoot::Explicit(PathBuf::from("/output")),
            &existing,
        )
        .unwrap();
        assert_eq!(plan.items.len(), 2);
        assert_eq!(
            plan.items
                .iter()
                .map(|item| item.destination.file_name().unwrap().to_owned())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                OsString::from("pair (2).heic"),
                OsString::from("pair (2).mov")
            ])
        );
        assert!(plan
            .items
            .iter()
            .all(|item| item.disposition == PhotoAssetCategorizationDisposition::Copy));
    }

    #[test]
    fn identical_existing_resources_are_duplicates_without_renaming() {
        let asset = PhotoAsset::live_photo("/input/pair.heic", "/input/pair.mov", "pair");
        let input = planning_input(
            asset,
            "oplus_18",
            &[("/input/pair.heic", "image"), ("/input/pair.mov", "video")],
        );
        let existing = BTreeMap::from([
            (
                PathBuf::from("/output/实况照片/人像/pair.heic"),
                fingerprint("image"),
            ),
            (
                PathBuf::from("/output/实况照片/人像/pair.mov"),
                fingerprint("video"),
            ),
        ]);

        let plan = plan_photo_assets(
            &[input],
            CategorizationDestinationRoot::Explicit(PathBuf::from("/output")),
            &existing,
        )
        .unwrap();
        assert!(plan.items.iter().all(|item| {
            item.disposition == PhotoAssetCategorizationDisposition::Duplicate
                && !item
                    .destination
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .contains("(2)")
        }));
    }

    #[test]
    fn reservations_keep_cross_asset_collisions_deterministic() {
        let first = planning_input(
            PhotoAsset::static_photo("/a/same.heic"),
            "oplus_18",
            &[("/a/same.heic", "first")],
        );
        let second = planning_input(
            PhotoAsset::static_photo("/b/same.heic"),
            "oplus_18",
            &[("/b/same.heic", "second")],
        );

        let plan = plan_photo_assets(
            &[second, first],
            CategorizationDestinationRoot::Explicit(PathBuf::from("/output")),
            &BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(plan.items[0].source, PathBuf::from("/a/same.heic"));
        assert_eq!(plan.items[0].destination.file_name().unwrap(), "same.heic");
        assert_eq!(plan.items[1].source, PathBuf::from("/b/same.heic"));
        assert_eq!(
            plan.items[1].destination.file_name().unwrap(),
            "same (2).heic"
        );
    }

    #[test]
    fn planner_rejects_asset_classification_type_drift() {
        let asset = PhotoAsset::static_photo("/input/photo.heic");
        let classification = classify_user_comment_with_context(
            Some("oplus_18"),
            PhotoAssetType::LivePhoto,
            BTreeSet::new(),
        );
        let input = PhotoAssetPlanningInput {
            asset,
            classification,
            fingerprints: BTreeMap::from([(
                PathBuf::from("/input/photo.heic"),
                fingerprint("image"),
            )]),
        };
        assert!(matches!(
            plan_photo_assets(
                &[input],
                CategorizationDestinationRoot::Explicit(PathBuf::from("/output")),
                &BTreeMap::new(),
            ),
            Err(PhotoAssetPlanningError::AssetTypeMismatch { .. })
        ));
    }
}
