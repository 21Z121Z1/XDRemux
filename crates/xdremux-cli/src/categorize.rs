use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::Args;
use serde_json::json;
use xdremux_runtime::PortableRuntime;

const CATEGORIZE_RECEIPT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Args)]
pub(crate) struct CategorizeArgs {
    /// Input image or directory. Repeat to add multiple roots. Directories are recursive.
    #[arg(long = "input", value_name = "PATH")]
    inputs: Vec<PathBuf>,
    /// Root directory for asset-type and capture-mode folders.
    #[arg(long, value_name = "DIR")]
    output_dir: PathBuf,
    /// Plan and report categorization without publishing files.
    #[arg(long)]
    dry_run: bool,
    /// Emit one stable machine-readable JSON receipt.
    #[arg(long)]
    json: bool,
}

fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "heic" | "heif" | "jpg" | "jpeg"
            )
        })
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| name.starts_with('.'))
}

fn is_category_root(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(OsStr::to_str),
        Some("静态照片" | "实况照片")
    )
}

fn nested_output_should_be_skipped(
    path: &Path,
    output_dir: &Path,
    output_identity: Option<&Path>,
    in_place: bool,
) -> bool {
    if in_place {
        return false;
    }
    if path == output_dir || path.starts_with(output_dir) {
        return true;
    }
    let Some(output_identity) = output_identity else {
        return false;
    };
    fs::canonicalize(path)
        .ok()
        .is_some_and(|identity| identity == output_identity || identity.starts_with(output_identity))
}

fn discover_directory(
    root: &Path,
    output_dir: &Path,
    inputs: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let root_identity = fs::canonicalize(root)
        .map_err(|error| format!("could not resolve input directory {}: {error}", root.display()))?;
    let output_identity = fs::canonicalize(output_dir).ok();
    let in_place = root == output_dir
        || output_identity
            .as_deref()
            .is_some_and(|output| output == root_identity);
    let mut pending = vec![(root.to_path_buf(), 0_usize)];

    while let Some((directory, depth)) = pending.pop() {
        let entries = fs::read_dir(&directory).map_err(|error| {
            format!(
                "could not read input directory {}: {error}",
                directory.display()
            )
        })?;
        let mut entries = entries
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| {
                format!(
                    "could not enumerate input directory {}: {error}",
                    directory.display()
                )
            })?;
        entries.sort_by_key(|entry| entry.path());

        for entry in entries {
            let file_type = entry.file_type().map_err(|error| {
                format!(
                    "could not inspect directory entry {}: {error}",
                    entry.path().display()
                )
            })?;
            let path = entry.path();
            if file_type.is_symlink() || is_hidden(&path) {
                continue;
            }
            if file_type.is_file() {
                if is_supported_image(&path) {
                    inputs.push(path);
                }
                continue;
            }
            if !file_type.is_dir() {
                continue;
            }
            if depth == 0 && in_place && is_category_root(&path) {
                continue;
            }
            if nested_output_should_be_skipped(
                &path,
                output_dir,
                output_identity.as_deref(),
                in_place,
            ) {
                continue;
            }
            pending.push((path, depth + 1));
        }
    }
    Ok(())
}

fn discover_inputs(arguments: &CategorizeArgs) -> Result<Vec<PathBuf>, String> {
    if arguments.inputs.is_empty() {
        return Err("categorize requires at least one --input".to_owned());
    }

    let mut collected = Vec::new();
    for input in &arguments.inputs {
        let metadata = fs::symlink_metadata(input)
            .map_err(|error| format!("input not found {}: {error}", input.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "categorize refuses explicit symlink input: {}",
                input.display()
            ));
        }
        if metadata.is_file() {
            if !is_supported_image(input) {
                return Err(format!(
                    "unsupported categorize input extension: {}",
                    input.display()
                ));
            }
            collected.push(input.clone());
        } else if metadata.is_dir() {
            discover_directory(input, &arguments.output_dir, &mut collected)?;
        } else {
            return Err(format!(
                "categorize input is neither a regular file nor directory: {}",
                input.display()
            ));
        }
    }

    collected.sort();
    let mut identities = BTreeSet::new();
    let mut unique = Vec::with_capacity(collected.len());
    for path in collected {
        let identity = fs::canonicalize(&path)
            .map_err(|error| format!("could not resolve input {}: {error}", path.display()))?;
        if identities.insert(identity) {
            unique.push(path);
        }
    }
    if unique.is_empty() {
        return Err("categorize discovery found no supported image files".to_owned());
    }
    Ok(unique)
}

fn path_json(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub(crate) fn run(
    arguments: CategorizeArgs,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
    let inputs = match discover_inputs(&arguments) {
        Ok(inputs) => inputs,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            return 2;
        }
    };

    let receipt = match PortableRuntime::new().categorize_files(
        &inputs,
        &arguments.output_dir,
        arguments.dry_run,
    ) {
        Ok(receipt) => receipt,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            return 1;
        }
    };

    if arguments.json {
        let items = receipt
            .items
            .iter()
            .map(|item| {
                json!({
                    "asset_id": item.asset_id,
                    "source": path_json(&item.source),
                    "destination": path_json(&item.destination),
                    "role": item.role,
                    "disposition": item.disposition.as_str(),
                    "classification": item.classification,
                    "error": item.error,
                })
            })
            .collect::<Vec<_>>();
        let value = json!({
            "schema_version": CATEGORIZE_RECEIPT_SCHEMA_VERSION,
            "command": "categorize",
            "processed": receipt.processed(),
            "copied": receipt.copied(),
            "duplicates": receipt.duplicates(),
            "dry_run": receipt.dry_run(),
            "failed": receipt.failed(),
            "items": items,
        });
        if let Err(error) = serde_json::to_writer_pretty(&mut *stdout, &value)
            .map_err(io::Error::other)
            .and_then(|()| writeln!(stdout))
        {
            let _ = writeln!(stderr, "error: could not write categorize JSON: {error}");
            return 1;
        }
    } else {
        for item in &receipt.items {
            let line = format!(
                "{}: {} -> {}",
                item.disposition.as_str(),
                item.source.display(),
                item.destination.display()
            );
            if item.error.is_some() {
                let _ = writeln!(stderr, "error: {line}: {}", item.error.as_deref().unwrap_or(""));
            } else {
                let _ = writeln!(stdout, "{line}");
            }
        }
        let _ = writeln!(
            stdout,
            "categorize: {} resources, {} copied, {} duplicates, {} dry-run, {} failed",
            receipt.processed(),
            receipt.copied(),
            receipt.duplicates(),
            receipt.dry_run(),
            receipt.failed()
        );
    }

    u8::from(!receipt.is_success())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_dir() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time must be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "xdremux-cli-categorize-{}-{stamp}",
            std::process::id()
        ))
    }

    #[test]
    fn in_place_discovery_skips_existing_category_roots() {
        let root = unique_dir();
        let categorized = root.join("静态照片").join("人像");
        fs::create_dir_all(&categorized).unwrap();
        fs::write(root.join("source.heic"), b"source").unwrap();
        fs::write(categorized.join("already.heic"), b"output").unwrap();
        let arguments = CategorizeArgs {
            inputs: vec![root.clone()],
            output_dir: root.clone(),
            dry_run: true,
            json: false,
        };
        assert_eq!(discover_inputs(&arguments).unwrap(), vec![root.join("source.heic")]);
        fs::remove_dir_all(root).unwrap();
    }
}