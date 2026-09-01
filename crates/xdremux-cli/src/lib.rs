#![forbid(unsafe_code)]

mod categorize;
mod validate;

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::{error::ErrorKind, Args, CommandFactory, Parser, Subcommand};
use serde_json::json;
use xdremux_engine::ConversionRequest;
use xdremux_runtime::{
    motion_photo_checkpoint_path, plan_batch_items, BatchAssetKind, BatchExecutionOptions,
    BatchPlanOptions, BatchSuccessDisposition, PortableRuntime,
};
use xdremux_source::{inspect_path, probe_bytes, SourceAsset, SourceInspection};

#[derive(Debug, Parser)]
#[command(
    name = "xdremux",
    version,
    about = "Convert and inspect ProXDR and Motion Photo assets with the canonical Rust runtime.",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: RootCommand,
}

#[derive(Debug, Subcommand)]
enum RootCommand {
    /// Inspect one input without converting it.
    Inspect(InspectArgs),
    /// Convert one supported source with the unified Rust engine/runtime.
    Convert(ConvertArgs),
    /// Convert a deterministic batch of supported assets.
    Batch(BatchArgs),
    /// Classify photo assets and publish them into deterministic folders.
    Categorize(categorize::CategorizeArgs),
    /// Validate one canonical output without converting it.
    Validate(validate::ValidateArgs),
}

#[derive(Debug, Args)]
struct InspectArgs {
    /// Input ProXDR HEIC or supported Motion Photo.
    #[arg(value_name = "INPUT")]
    input: PathBuf,
    /// Emit the stable machine-readable inspection schema.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ConvertArgs {
    /// Input ProXDR HEIC or supported Motion Photo.
    #[arg(long, value_name = "INPUT")]
    input: PathBuf,
    /// Output HEIC; ProXDR defaults in-place, Motion Photo chooses a new pair.
    #[arg(long, value_name = "OUTPUT")]
    output: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct BatchArgs {
    /// Explicit input file. Repeat to add multiple files.
    #[arg(long = "input", value_name = "FILE")]
    inputs: Vec<PathBuf>,
    /// Discover HEIC/HEIF/JPEG inputs in a directory. Repeat to add multiple roots.
    #[arg(long = "input-dir", value_name = "DIR")]
    input_dirs: Vec<PathBuf>,
    /// Recurse below each input directory. Symlinks are never followed.
    #[arg(long)]
    recursive: bool,
    /// Place generated HEIC outputs in this directory.
    ///
    /// When omitted, each output is written beside its source with an .xdremux suffix.
    #[arg(long, value_name = "DIR")]
    output_dir: Option<PathBuf>,
    /// Reuse a completed Live Photo pair only when durable source provenance matches.
    #[arg(long)]
    skip_existing: bool,
    /// Resume completed Live Photo work from the durable checkpoint and retry remaining items.
    #[arg(long)]
    resume: bool,
    /// Shared Swift/Python/Rust Motion Photo checkpoint base path.
    ///
    /// The compatibility state is stored at this path with `.motion-photo` appended.
    #[arg(long, value_name = "FILE")]
    checkpoint: Option<PathBuf>,
    /// Emit one machine-readable JSON receipt instead of human progress.
    #[arg(long)]
    json: bool,
}

fn write_clap_error(error: clap::Error, stdout: &mut impl Write, stderr: &mut impl Write) -> u8 {
    let kind = error.kind();
    let code = u8::try_from(error.exit_code()).unwrap_or(2);
    let rendered = error.to_string();
    let result = if matches!(kind, ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
        write!(stdout, "{rendered}")
    } else {
        write!(stderr, "{rendered}")
    };
    if result.is_err() {
        return 1;
    }
    code
}

fn parse_cli(args: impl IntoIterator<Item = OsString>) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once(OsString::from("xdremux")).chain(args))
}

fn write_root_help(stdout: &mut impl Write) -> u8 {
    let mut command = Cli::command();
    match command
        .write_long_help(&mut *stdout)
        .and_then(|()| writeln!(stdout))
    {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

fn write_human(inspection: &SourceInspection, output: &mut impl Write) -> io::Result<()> {
    writeln!(output, "input: {}", inspection.input.display())?;
    writeln!(output, "kind: {}", inspection.asset.kind())?;
    match &inspection.asset {
        SourceAsset::MotionPhoto {
            source_kind,
            still,
            video,
            presentation_timestamp_us,
            presentation_source,
            stream_count,
        } => {
            writeln!(output, "source-kind: {source_kind}")?;
            writeln!(
                output,
                "still: offset={} length={}",
                still.offset, still.length
            )?;
            writeln!(
                output,
                "video: offset={} length={}",
                video.offset, video.length
            )?;
            writeln!(output, "streams: {stream_count}")?;
            if let Some(value) = presentation_timestamp_us {
                writeln!(output, "presentation-timestamp-us: {value}")?;
            }
            if let Some(value) = presentation_source {
                writeln!(output, "presentation-source: {value}")?;
            }
        }
        SourceAsset::ProXdr {
            hdr_mode,
            metadata_float_count,
            gain_map_bytes,
            manifest_entry_count,
            has_local_hdr_info,
        } => {
            writeln!(output, "hdr-mode: {hdr_mode}")?;
            writeln!(output, "metadata-floats: {metadata_float_count}")?;
            writeln!(output, "gain-map-bytes: {gain_map_bytes}")?;
            writeln!(output, "manifest-entries: {manifest_entry_count}")?;
            writeln!(output, "local-hdr-info: {has_local_hdr_info}")?;
        }
    }
    Ok(())
}

fn run_inspect(input: PathBuf, json: bool, stdout: &mut impl Write, stderr: &mut impl Write) -> u8 {
    let inspection = match inspect_path(&input) {
        Ok(value) => value,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            return 1;
        }
    };

    let result = if json {
        serde_json::to_writer_pretty(&mut *stdout, &inspection)
            .map_err(io::Error::other)
            .and_then(|()| writeln!(stdout))
    } else {
        write_human(&inspection, stdout)
    };
    if let Err(error) = result {
        let _ = writeln!(stderr, "error: could not write output: {error}");
        return 1;
    }
    0
}

fn default_motion_photo_output(input: &Path) -> PathBuf {
    let base = input.with_extension("heic");
    let mut sequence = 1_u32;
    loop {
        let candidate = if sequence == 1 {
            base.clone()
        } else {
            let stem = base
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("capture");
            base.with_file_name(format!("{stem} ({sequence}).heic"))
        };
        if candidate != input && !candidate.exists() && !candidate.with_extension("mov").exists() {
            return candidate;
        }
        sequence = sequence.saturating_add(1);
    }
}

fn run_convert(
    input: PathBuf,
    output: Option<PathBuf>,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
    let source = match fs::read(&input) {
        Ok(value) => value,
        Err(error) => {
            let _ = writeln!(stderr, "error: could not read {}: {error}", input.display());
            return 1;
        }
    };

    let asset = match probe_bytes(&source) {
        Ok(value) => value,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            return 1;
        }
    };
    let runtime = PortableRuntime::new();
    let result = match asset {
        SourceAsset::MotionPhoto { .. } => {
            let output = output.unwrap_or_else(|| default_motion_photo_output(&input));
            match runtime.convert_motion_photo_file(&source, &input, &output) {
                Ok(receipt) => writeln!(
                    stdout,
                    "converted: {} -> {} + {}",
                    input.display(),
                    receipt.image.display(),
                    receipt.video.display()
                ),
                Err(error) => {
                    let _ = writeln!(stderr, "error: {error}");
                    return 1;
                }
            }
        }
        SourceAsset::ProXdr { .. } => {
            let output = output.unwrap_or_else(|| input.clone());
            if let Err(error) =
                runtime.convert_proxdr_file(&source, &output, ConversionRequest::default(), |_| {})
            {
                let _ = writeln!(stderr, "error: {error}");
                return 1;
            }
            if output == input {
                writeln!(stdout, "converted: {} (in place)", input.display())
            } else {
                writeln!(
                    stdout,
                    "converted: {} -> {}",
                    input.display(),
                    output.display()
                )
            }
        }
    };
    if let Err(error) = result {
        let _ = writeln!(stderr, "error: could not write output: {error}");
        return 1;
    }
    0
}

fn is_hidden_batch_artifact(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| name.starts_with('.'))
}

fn is_supported_batch_candidate(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "heic" | "heif" | "jpg" | "jpeg"
            )
        })
}

fn is_generated_batch_output(path: &Path) -> bool {
    path.file_stem()
        .and_then(OsStr::to_str)
        .is_some_and(|stem| {
            stem.ends_with(".xdremux")
                || stem.rsplit_once(".xdremux (").is_some_and(|(_, suffix)| {
                    suffix
                        .strip_suffix(')')
                        .is_some_and(|value| value.parse::<u32>().is_ok())
                })
        })
}

fn discover_directory(
    root: &Path,
    recursive: bool,
    inputs: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
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
            if is_hidden_batch_artifact(&path) {
                continue;
            }
            if file_type.is_file() {
                if is_supported_batch_candidate(&path) && !is_generated_batch_output(&path) {
                    inputs.push(path);
                }
            } else if recursive && file_type.is_dir() {
                pending.push(path);
            }
        }
    }
    Ok(())
}

fn discover_batch_inputs(arguments: &BatchArgs) -> Result<Vec<PathBuf>, String> {
    if arguments.inputs.is_empty() && arguments.input_dirs.is_empty() {
        return Err("batch requires at least one --input or --input-dir".to_owned());
    }

    let mut inputs = Vec::new();
    for path in &arguments.inputs {
        if !path.is_file() {
            return Err(format!("input file not found: {}", path.display()));
        }
        inputs.push(path.clone());
    }

    for directory in &arguments.input_dirs {
        if !directory.is_dir() {
            return Err(format!(
                "input directory not found: {}",
                directory.display()
            ));
        }
        discover_directory(directory, arguments.recursive, &mut inputs)?;
    }

    inputs.sort();
    let mut seen = BTreeSet::new();
    let mut unique = Vec::with_capacity(inputs.len());
    for path in inputs {
        let identity = fs::canonicalize(&path)
            .map_err(|error| format!("could not resolve input {}: {error}", path.display()))?;
        if seen.insert(identity) {
            unique.push(path);
        }
    }
    if unique.is_empty() {
        return Err("batch discovery found no supported input files".to_owned());
    }
    Ok(unique)
}

fn batch_kind_name(kind: BatchAssetKind) -> &'static str {
    match kind {
        BatchAssetKind::ProXdr => "pro-xdr",
        BatchAssetKind::LivePhoto => "live-photo",
    }
}

fn batch_disposition_name(disposition: BatchSuccessDisposition) -> &'static str {
    disposition.as_str()
}

fn path_json(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

const BATCH_RECEIPT_SCHEMA_VERSION: u32 = 1;

fn run_batch(arguments: BatchArgs, stdout: &mut impl Write, stderr: &mut impl Write) -> u8 {
    let inputs = match discover_batch_inputs(&arguments) {
        Ok(inputs) => inputs,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            return 2;
        }
    };
    let reuse_existing = arguments.skip_existing || arguments.resume;
    let checkpoint_path = motion_photo_checkpoint_path(
        arguments.output_dir.as_deref(),
        arguments.checkpoint.as_deref(),
    );
    if reuse_existing && checkpoint_path.is_none() {
        let _ = writeln!(
            stderr,
            "error: --skip-existing/--resume requires --output-dir or --checkpoint for durable provenance"
        );
        return 2;
    }
    let plan_options = BatchPlanOptions {
        output_dir: arguments.output_dir.clone(),
        checkpoint_path: checkpoint_path.clone(),
        reuse_existing,
    };
    let items = match plan_batch_items(&inputs, &plan_options) {
        Ok(items) => items,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            return 2;
        }
    };

    let runtime = PortableRuntime::new();
    let execution_options = BatchExecutionOptions {
        checkpoint_path,
        reuse_existing,
    };
    let receipt =
        runtime.convert_batch_with_options(items, ConversionRequest::default(), &execution_options);

    if arguments.json {
        let successes = receipt
            .successes
            .iter()
            .map(|success| {
                json!({
                    "input": path_json(&success.input),
                    "kind": batch_kind_name(success.kind),
                    "status": batch_disposition_name(success.disposition),
                    "outputs": success.outputs.iter().map(|path| path_json(path)).collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>();
        let failures = receipt
            .failures
            .iter()
            .map(|failure| {
                json!({
                    "input": path_json(&failure.input),
                    "output": path_json(&failure.output),
                    "error": failure.error.as_str(),
                })
            })
            .collect::<Vec<_>>();
        let value = json!({
            "schema_version": BATCH_RECEIPT_SCHEMA_VERSION,
            "command": "batch",
            "processed": receipt.processed(),
            "succeeded": receipt.succeeded(),
            "skipped_existing": receipt.skipped_existing(),
            "failed": receipt.failed(),
            "successes": successes,
            "failures": failures,
        });
        match serde_json::to_writer_pretty(&mut *stdout, &value)
            .map_err(io::Error::other)
            .and_then(|()| writeln!(stdout))
        {
            Ok(()) => {}
            Err(error) => {
                let _ = writeln!(stderr, "error: could not write batch JSON: {error}");
                return 1;
            }
        }
    } else {
        for success in &receipt.successes {
            let outputs = success
                .outputs
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(" + ");
            let _ = writeln!(
                stdout,
                "converted: {} -> {outputs}",
                success.input.display()
            );
        }
        for failure in &receipt.failures {
            let _ = writeln!(
                stderr,
                "error: {} -> {}: {}",
                failure.input.display(),
                failure.output.display(),
                failure.error
            );
        }
        let _ = writeln!(
            stdout,
            "batch: {} processed, {} succeeded, {} failed",
            receipt.processed(),
            receipt.succeeded(),
            receipt.failed()
        );
    }

    u8::from(!receipt.is_success())
}

pub fn run_from<I, S>(args: I, stdout: &mut impl Write, stderr: &mut impl Write) -> u8
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    if args.is_empty() {
        return write_root_help(stdout);
    }

    match parse_cli(args) {
        Ok(Cli {
            command: RootCommand::Inspect(arguments),
        }) => run_inspect(arguments.input, arguments.json, stdout, stderr),
        Ok(Cli {
            command: RootCommand::Convert(arguments),
        }) => run_convert(arguments.input, arguments.output, stdout, stderr),
        Ok(Cli {
            command: RootCommand::Batch(arguments),
        }) => run_batch(arguments, stdout, stderr),
        Ok(Cli {
            command: RootCommand::Categorize(arguments),
        }) => categorize::run(arguments, stdout, stderr),
        Ok(Cli {
            command: RootCommand::Validate(arguments),
        }) => validate::run(arguments, stdout, stderr),
        Err(error) => write_clap_error(error, stdout, stderr),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn parse(args: &[&str]) -> Cli {
        parse_cli(args.iter().map(OsString::from)).expect("arguments should parse")
    }

    fn unique_dir(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time must be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "xdremux-cli-{label}-{}-{stamp}",
            std::process::id()
        ))
    }

    #[test]
    fn no_arguments_prints_help() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(run_from(Vec::<&str>::new(), &mut stdout, &mut stderr), 0);
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("Commands:"));
        assert!(output.contains("inspect"));
        assert!(output.contains("convert"));
        assert!(output.contains("batch"));
        assert!(output.contains("categorize"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn inspect_requires_exactly_one_input() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(run_from(["inspect"], &mut stdout, &mut stderr), 2);
        assert!(stdout.is_empty());
        assert!(String::from_utf8(stderr).unwrap().contains("<INPUT>"));
    }

    #[test]
    fn double_dash_allows_paths_that_start_with_dash() {
        let command = parse(&["inspect", "--", "-capture.jpg"]);
        let RootCommand::Inspect(arguments) = command.command else {
            panic!("expected inspect command");
        };
        assert_eq!(arguments.input, PathBuf::from("-capture.jpg"));
        assert!(!arguments.json);
    }

    #[test]
    fn convert_defaults_to_in_place_publication() {
        let command = parse(&["convert", "--input", "capture.heic"]);
        let RootCommand::Convert(arguments) = command.command else {
            panic!("expected convert command");
        };
        assert_eq!(arguments.input, PathBuf::from("capture.heic"));
        assert_eq!(arguments.output, None);
    }

    #[test]
    fn convert_accepts_explicit_output() {
        let command = parse(&[
            "convert",
            "--input",
            "capture.heic",
            "--output",
            "converted.heic",
        ]);
        let RootCommand::Convert(arguments) = command.command else {
            panic!("expected convert command");
        };
        assert_eq!(arguments.input, PathBuf::from("capture.heic"));
        assert_eq!(arguments.output, Some(PathBuf::from("converted.heic")));
    }

    #[test]
    fn convert_requires_named_input() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(run_from(["convert"], &mut stdout, &mut stderr), 2);
        assert!(stdout.is_empty());
        assert!(String::from_utf8(stderr)
            .unwrap()
            .contains("--input <INPUT>"));
    }

    #[test]
    fn batch_requires_a_source() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(run_from(["batch"], &mut stdout, &mut stderr), 2);
        assert!(stdout.is_empty());
        assert!(String::from_utf8(stderr)
            .unwrap()
            .contains("at least one --input or --input-dir"));
    }

    #[test]
    fn batch_accepts_repeatable_inputs() {
        let command = parse(&[
            "batch",
            "--input",
            "a.heic",
            "--input",
            "b.jpg",
            "--output-dir",
            "out",
        ]);
        let RootCommand::Batch(arguments) = command.command else {
            panic!("expected batch command");
        };
        assert_eq!(
            arguments.inputs,
            vec![PathBuf::from("a.heic"), PathBuf::from("b.jpg")]
        );
        assert_eq!(arguments.output_dir, Some(PathBuf::from("out")));
        assert!(!arguments.skip_existing);
        assert!(!arguments.resume);
        assert_eq!(arguments.checkpoint, None);
    }

    #[test]
    fn batch_discovery_is_non_recursive_by_default_and_recursive_on_request() {
        let root = unique_dir("discovery");
        let nested = root.join("nested");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.join("A.HEIC"), b"a").unwrap();
        fs::write(root.join("ignore.txt"), b"x").unwrap();
        fs::write(root.join(".hidden.heic"), b"x").unwrap();
        fs::write(root.join("old.xdremux.heic"), b"x").unwrap();
        fs::write(root.join("old.xdremux (2).heic"), b"x").unwrap();
        fs::write(nested.join("B.jpg"), b"b").unwrap();

        let flat = BatchArgs {
            inputs: Vec::new(),
            input_dirs: vec![root.clone()],
            recursive: false,
            output_dir: None,
            skip_existing: false,
            resume: false,
            checkpoint: None,
            json: false,
        };
        assert_eq!(
            discover_batch_inputs(&flat).unwrap(),
            vec![root.join("A.HEIC")]
        );

        let recursive = BatchArgs {
            recursive: true,
            ..flat
        };
        assert_eq!(
            discover_batch_inputs(&recursive).unwrap(),
            vec![root.join("A.HEIC"), nested.join("B.jpg")]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn batch_output_planning_is_source_safe_and_reserves_live_photo_companions() {
        let root = unique_dir("planning");
        fs::create_dir_all(&root).unwrap();
        let a = root.join("capture.heic");
        let b = root.join("capture.jpg");
        fs::write(&a, b"a").unwrap();
        fs::write(&b, b"b").unwrap();

        let plan = plan_batch_items(
            &[a.clone(), b.clone()],
            &BatchPlanOptions {
                output_dir: Some(root.clone()),
                ..BatchPlanOptions::default()
            },
        )
        .unwrap();
        assert_eq!(plan.len(), 2);
        assert_ne!(plan[0].output, a);
        assert_ne!(plan[1].output, b);
        assert_ne!(plan[0].output, plan[1].output);
        assert_ne!(
            plan[0].output.with_extension("mov"),
            plan[1].output.with_extension("mov")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn help_and_version_are_successful_control_flow() {
        for arguments in [["--help"], ["--version"]] {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            assert_eq!(run_from(arguments, &mut stdout, &mut stderr), 0);
            assert!(!stdout.is_empty());
            assert!(stderr.is_empty());
        }
    }

    #[test]
    fn unknown_command_is_a_usage_error() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(run_from(["frobnicate"], &mut stdout, &mut stderr), 2);
        assert!(stdout.is_empty());
        assert!(String::from_utf8(stderr).unwrap().contains("frobnicate"));
    }

    #[test]
    fn clap_command_definition_is_internally_consistent() {
        Cli::command().debug_assert();
    }
}
