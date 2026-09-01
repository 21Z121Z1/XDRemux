#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::{error::ErrorKind, Args, CommandFactory, Parser, Subcommand};
use xdremux_engine::ConversionRequest;
use xdremux_runtime::PortableRuntime;
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
    match command.write_long_help(&mut *stdout).and_then(|()| writeln!(stdout)) {
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
        Err(error) => write_clap_error(error, stdout, stderr),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Cli {
        parse_cli(args.iter().map(OsString::from)).expect("arguments should parse")
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
        assert!(String::from_utf8(stderr).unwrap().contains("--input <INPUT>"));
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
