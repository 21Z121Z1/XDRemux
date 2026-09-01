#!/usr/bin/env python3
from pathlib import Path


categorize_path = Path("crates/xdremux-runtime/src/categorize.rs")
categorize = categorize_path.read_text()

projection = r'''pub(crate) fn classification_relative_directory(source: &Path) -> Result<PathBuf> {
    let bytes = fs::read(source)
        .map_err(|error| RuntimeError::external("batch categorization input read", error))?;
    let asset_type = inferred_asset_type(&bytes);
    let classification = classify_user_comment_with_context(
        extract_user_comment(&bytes).as_deref(),
        asset_type,
        detect_capabilities(&bytes),
    );
    let [asset_type, capture_mode] = classification.relative_directory_components();
    Ok(PathBuf::from(asset_type).join(capture_mode))
}

'''
if "pub(crate) fn classification_relative_directory(" not in categorize:
    anchor = "fn fingerprint_path(path: &Path) -> Result<ResourceFingerprint> {\n"
    if anchor not in categorize:
        raise SystemExit("categorize fingerprint anchor not found")
    categorize = categorize.replace(anchor, projection + anchor, 1)
categorize_path.write_text(categorize)

batch_path = Path("crates/xdremux-runtime/src/batch.rs")
batch = batch_path.read_text()

if "use crate::categorize::classification_relative_directory;" not in batch:
    anchor = "use crate::batch_checkpoint::{\n"
    if anchor not in batch:
        raise SystemExit("batch checkpoint import anchor not found")
    batch = batch.replace(
        anchor,
        "use crate::categorize::classification_relative_directory;\n" + anchor,
        1,
    )

old_options = """pub struct BatchPlanOptions {
    pub output_dir: Option<PathBuf>,
    pub checkpoint_path: Option<PathBuf>,
    pub reuse_existing: bool,
}
"""
new_options = """pub struct BatchPlanOptions {
    pub output_dir: Option<PathBuf>,
    pub checkpoint_path: Option<PathBuf>,
    pub reuse_existing: bool,
    pub categorize_output: bool,
}
"""
if old_options in batch:
    batch = batch.replace(old_options, new_options, 1)
elif "pub categorize_output: bool," not in batch:
    raise SystemExit("BatchPlanOptions anchor not found")

old_parent = """        let parent = match options.output_dir.as_deref() {
            Some(directory) => directory.to_path_buf(),
            None => input
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf(),
        };
"""
new_parent = """        let mut parent = match options.output_dir.as_deref() {
            Some(directory) => directory.to_path_buf(),
            None => input
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf(),
        };
        if options.categorize_output {
            parent.push(classification_relative_directory(input)?);
        }
"""
if old_parent in batch:
    batch = batch.replace(old_parent, new_parent, 1)
elif "parent.push(classification_relative_directory(input)?);" not in batch:
    raise SystemExit("batch parent planning anchor not found")

for marker in (
    "classification_relative_directory",
    "pub categorize_output: bool,",
    "parent.push(classification_relative_directory(input)?);",
):
    if marker not in batch:
        raise SystemExit(f"batch categorize runtime wiring missing marker: {marker}")
batch_path.write_text(batch)

cli_path = Path("crates/xdremux-cli/src/lib.rs")
cli = cli_path.read_text()

categorize_field = """    /// File converted assets by asset type and primary capture mode.
    #[arg(long)]
    categorize: bool,
"""
if "    categorize: bool," not in cli:
    anchor = """    /// Maximum number of concurrent conversions; must be greater than zero.
"""
    if anchor not in cli:
        # First jobs materializer may still expose its pre-contract help while the
        # contract materializer is queued. Both forms are accepted here.
        anchor = """    /// Maximum number of concurrent conversions. Zero is treated as one.
"""
    if anchor not in cli:
        raise SystemExit("BatchArgs jobs help anchor not found")
    cli = cli.replace(anchor, categorize_field + anchor, 1)

old_plan = """    let plan_options = BatchPlanOptions {
        output_dir: arguments.output_dir.clone(),
        checkpoint_path: checkpoint_path.clone(),
        reuse_existing,
    };
"""
new_plan = """    let plan_options = BatchPlanOptions {
        output_dir: arguments.output_dir.clone(),
        checkpoint_path: checkpoint_path.clone(),
        reuse_existing,
        categorize_output: arguments.categorize,
    };
"""
if old_plan in cli:
    cli = cli.replace(old_plan, new_plan, 1)
elif "categorize_output: arguments.categorize," not in cli:
    raise SystemExit("BatchPlanOptions CLI anchor not found")

# Every hand-written BatchArgs literal must specify the new flag. This replacement
# is deliberately broad but idempotent.
cli = cli.replace(
    "            checkpoint: None,\n            jobs:",
    "            checkpoint: None,\n            categorize: false,\n            jobs:",
)
if "assert!(!arguments.categorize);" not in cli:
    anchor = "        assert_eq!(arguments.checkpoint, None);\n"
    if anchor not in cli:
        raise SystemExit("batch parser assertion anchor not found")
    cli = cli.replace(anchor, anchor + "        assert!(!arguments.categorize);\n", 1)

for marker in (
    "    categorize: bool,",
    "categorize_output: arguments.categorize,",
    "assert!(!arguments.categorize);",
):
    if marker not in cli:
        raise SystemExit(f"batch categorize CLI wiring missing marker: {marker}")
cli_path.write_text(cli)
