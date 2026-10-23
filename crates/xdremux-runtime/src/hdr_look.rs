//! Discovery and exclusive publication for the user-facing Styles HDR look.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{Result, RuntimeError};

#[derive(Debug, Clone, Serialize)]
pub struct HdrLookItem {
    pub input: PathBuf,
    pub output: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct HdrLookPlan {
    pub schema_version: u32,
    pub output_dir: PathBuf,
    pub headroom: u32,
    pub max_edge: u32,
    pub items: Vec<HdrLookItem>,
}

#[derive(Debug, Serialize)]
pub struct HdrLookResult {
    #[serde(flatten)]
    pub item: HdrLookItem,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HdrLookReceipt {
    pub schema_version: u32,
    pub command: &'static str,
    pub output_dir: PathBuf,
    pub headroom: u32,
    pub max_edge: u32,
    pub results: Vec<HdrLookResult>,
    pub failures: usize,
}

fn failure(message: impl Into<String>) -> RuntimeError {
    RuntimeError::new("HDR look", message)
}
fn io(error: std::io::Error) -> RuntimeError {
    RuntimeError::external("HDR look filesystem", error)
}
fn supported(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("heic") || s.eq_ignore_ascii_case("heif"))
}
fn generated(path: &Path) -> bool {
    path.file_name().and_then(|s| s.to_str()).is_some_and(|s| {
        let lower = s.to_ascii_lowercase();
        s.starts_with('.')
            || lower.ends_with(".reconstructed")
            || lower.ends_with(".hdr-look")
            || lower.ends_with(".hdr-look.heic")
    })
}
fn discover(
    directory: &Path,
    recursive: bool,
    output: &Path,
    inputs: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(directory).map_err(io)? {
        let entry = entry.map_err(io)?;
        let kind = entry.file_type().map_err(io)?;
        let path = entry.path();
        if kind.is_symlink() || generated(&path) || (path == output && path != directory) {
            continue;
        }
        if kind.is_file() && supported(&path) {
            inputs.insert(fs::canonicalize(path).map_err(io)?);
        } else if kind.is_dir() && recursive {
            discover(&path, true, output, inputs)?;
        }
    }
    Ok(())
}

/// Read-only discovery. No native adapter, output reservation or media write.
pub fn plan_hdr_look(
    paths: &[PathBuf],
    output: Option<&Path>,
    recursive: bool,
    headroom: u32,
    max_edge: u32,
) -> Result<HdrLookPlan> {
    xdremux_engine::plan_style_display(headroom).map_err(failure)?;
    xdremux_engine::plan_style_reconstruction(max_edge).map_err(failure)?;
    if paths.is_empty() {
        return Err(failure("provide a photo or directory"));
    }
    let roots = paths
        .iter()
        .map(|p| fs::canonicalize(p).map_err(io))
        .collect::<Result<Vec<_>>>()?;
    let output_dir = match output {
        Some(path) if path.exists() => {
            if !path.is_dir() {
                return Err(failure("output directory is an existing file"));
            }
            fs::canonicalize(path).map_err(io)?
        }
        Some(path) if path.is_absolute() => path.to_owned(),
        Some(path) => std::env::current_dir().map_err(io)?.join(path),
        None => roots[0]
            .parent()
            .ok_or_else(|| failure("choose an output directory for a filesystem root"))?
            .join(format!(
                "HDR-Look-{}",
                &Uuid::new_v4().simple().to_string()[..8]
            )),
    };
    let mut inputs = BTreeSet::new();
    if output_dir.to_str().is_none() {
        return Err(failure("output path is not UTF-8"));
    }
    for root in roots {
        if root.is_file() && supported(&root) {
            inputs.insert(root);
        } else if root.is_dir() {
            discover(&root, recursive, &output_dir, &mut inputs)?;
        } else {
            return Err(failure(format!(
                "expected a HEIC/HEIF photo or directory: {}",
                root.display()
            )));
        }
    }
    if inputs.is_empty() {
        return Err(failure("no HEIC/HEIF photos found"));
    }
    let mut destinations = BTreeMap::new();
    let mut items = Vec::new();
    for input in inputs {
        let name = input
            .file_name()
            .and_then(|p| p.to_str())
            .ok_or_else(|| failure("input file name is not UTF-8"))?;
        let path = input
            .to_str()
            .ok_or_else(|| failure("input path is not UTF-8"))?;
        let hash = Sha256::digest(path.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let output = output_dir.join(format!("{name}.{}.{headroom}x.hdr-look.heic", &hash[..12]));
        if destinations.insert(output.clone(), input.clone()).is_some() {
            return Err(failure("output naming collision"));
        }
        items.push(HdrLookItem { input, output });
    }
    Ok(HdrLookPlan {
        schema_version: 1,
        output_dir,
        headroom,
        max_edge,
        items,
    })
}

#[cfg(target_os = "macos")]
pub fn render_hdr_look(
    plan: &HdrLookPlan,
    adapter: &Path,
    progress: impl FnMut(usize, &HdrLookResult),
) -> Result<HdrLookReceipt> {
    execute(
        plan,
        |input, output| {
            crate::reconstruct_styles_hdr_with_display(
                adapter,
                input,
                output,
                plan.max_edge,
                Some(plan.headroom),
            )
            .map(|_| ())
        },
        progress,
    )
}

#[cfg(any(target_os = "macos", all(test, target_os = "linux")))]
fn execute(
    plan: &HdrLookPlan,
    mut renderer: impl FnMut(&Path, &Path) -> Result<()>,
    mut progress: impl FnMut(usize, &HdrLookResult),
) -> Result<HdrLookReceipt> {
    fs::create_dir_all(&plan.output_dir).map_err(io)?;
    let mut results = Vec::new();
    for (index, item) in plan.items.iter().enumerate() {
        let result = publish_item(item, &plan.output_dir, &mut renderer);
        let row = HdrLookResult {
            item: item.clone(),
            status: if result.is_ok() { "success" } else { "failed" }.into(),
            error: result.err().map(|e| e.to_string()),
        };
        progress(index, &row);
        results.push(row);
    }
    Ok(HdrLookReceipt {
        schema_version: 1,
        command: "hdr-look",
        output_dir: plan.output_dir.clone(),
        headroom: plan.headroom,
        max_edge: plan.max_edge,
        failures: results.iter().filter(|r| r.status == "failed").count(),
        results,
    })
}

#[cfg(any(target_os = "macos", all(test, target_os = "linux")))]
fn publish_item(
    item: &HdrLookItem,
    parent: &Path,
    renderer: &mut impl FnMut(&Path, &Path) -> Result<()>,
) -> Result<()> {
    if fs::symlink_metadata(&item.output).is_ok() {
        return Err(failure(format!(
            "output already exists: {}",
            item.output.display()
        )));
    }
    let scratch = tempfile::Builder::new()
        .prefix(".hdr-look-")
        .tempdir_in(parent)
        .map_err(io)?;
    let render = scratch.path().join("render");
    renderer(&item.input, &render)?;
    let image = render.join("hdr.heic");
    fs::File::open(&image).map_err(io)?.sync_all().map_err(io)?;
    crate::live_photo_package::rename_exclusive(&image, &item.output).map_err(io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_is_read_only_deduplicated_recursive_and_avoids_outputs() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("原图 with spaces");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("same.HEIC"), b"source").unwrap();
        fs::write(root.join("ignore.jpg"), b"jpeg").unwrap();
        fs::create_dir(root.join("nested")).unwrap();
        fs::write(root.join("nested/same.HEIC"), b"other").unwrap();
        fs::create_dir(root.join("old.hdr-look")).unwrap();
        fs::write(root.join("old.hdr-look/hdr.heic"), b"generated").unwrap();
        fs::write(root.join("old.8x.HDR-LOOK.HEIC"), b"generated").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&root, root.join("loop")).unwrap();
        let out = t.path().join("new");
        let shallow = plan_hdr_look(std::slice::from_ref(&root), Some(&out), false, 8, 0).unwrap();
        assert_eq!(shallow.items.len(), 1);
        let plan = plan_hdr_look(
            &[root.clone(), root.join("same.HEIC")],
            Some(&out),
            true,
            8,
            0,
        )
        .unwrap();
        assert_eq!(plan.items.len(), 2);
        assert_ne!(plan.items[0].output, plan.items[1].output);
        assert!(!out.exists());
        assert!(plan_hdr_look(&[root], Some(&out), true, 3, 0).is_err());
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn batch_continues_after_failure_and_retains_only_photos() {
        let t = tempfile::tempdir().unwrap();
        let inputs = t.path().join("inputs");
        fs::create_dir(&inputs).unwrap();
        fs::write(inputs.join("a.heic"), b"a").unwrap();
        fs::write(inputs.join("b.heic"), b"b").unwrap();
        let out = t.path().join("results");
        let plan = plan_hdr_look(std::slice::from_ref(&inputs), Some(&out), true, 8, 0).unwrap();
        let receipt = execute(
            &plan,
            |input, dest| {
                fs::create_dir(dest).unwrap();
                fs::write(dest.join("partial.exr"), b"temporary").unwrap();
                if input.file_name().unwrap() == "a.heic" {
                    return Err(failure("injected renderer failure"));
                }
                fs::write(dest.join("hdr.heic"), b"encoded").unwrap();
                fs::write(dest.join("report.json"), b"temporary report").unwrap();
                Ok(())
            },
            |_, _| {},
        )
        .unwrap();
        assert_eq!(receipt.failures, 1);
        assert!(!plan.items[0].output.exists());
        assert_eq!(fs::read(&plan.items[1].output).unwrap(), b"encoded");
        assert_eq!(fs::read_dir(&out).unwrap().count(), 1);
        fs::write(&plan.items[0].output, b"existing").unwrap();
        let again = execute(
            &plan,
            |_, _| panic!("existing output must not reach the renderer"),
            |_, _| {},
        )
        .unwrap();
        assert_eq!(again.failures, 2);
        assert_eq!(fs::read(&plan.items[1].output).unwrap(), b"encoded");
        assert_eq!(fs::read(&plan.items[0].output).unwrap(), b"existing");
        assert_eq!(fs::read_dir(&out).unwrap().count(), 2);
        assert_eq!(fs::read(inputs.join("a.heic")).unwrap(), b"a");
        assert_eq!(fs::read(inputs.join("b.heic")).unwrap(), b"b");
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn publication_race_preserves_destination_and_cleans_temporary_files() {
        let t = tempfile::tempdir().unwrap();
        let input = t.path().join("source.heic");
        fs::write(&input, b"source").unwrap();
        let out = t.path().join("out");
        let plan = plan_hdr_look(&[input], Some(&out), false, 8, 0).unwrap();
        let receipt = execute(
            &plan,
            |_, dest| {
                fs::create_dir(dest).unwrap();
                fs::write(dest.join("hdr.heic"), b"new").unwrap();
                fs::write(&plan.items[0].output, b"concurrent output").unwrap();
                Ok(())
            },
            |_, _| {},
        )
        .unwrap();
        assert_eq!(receipt.failures, 1);
        assert_eq!(
            fs::read(&plan.items[0].output).unwrap(),
            b"concurrent output"
        );
        assert_eq!(fs::read_dir(out).unwrap().count(), 1);
    }
}
