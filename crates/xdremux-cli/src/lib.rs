#![forbid(unsafe_code)]

use std::ffi::{OsStr, OsString};
use std::io::{self, Write};
use std::path::PathBuf;

use xdremux_source::{inspect_path, SourceAsset, SourceInspection};

const HELP: &str = "XDRemux Rust CLI\n\nUsage:\n  xdremux <COMMAND>\n\nCommands:\n  inspect <INPUT> [--json]  Inspect the canonical Rust input route\n\nOptions:\n  -h, --help     Print help\n  -V, --version  Print version\n";
const INSPECT_HELP: &str = "Inspect one input without converting it.\n\nUsage:\n  xdremux inspect <INPUT> [--json]\n\nOptions:\n      --json  Emit the stable machine-readable inspection schema\n  -h, --help  Print help\n";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Help,
    Version,
    InspectHelp,
    Inspect { input: PathBuf, json: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliError(String);

fn is(arg: &OsStr, expected: &str) -> bool {
    arg == OsStr::new(expected)
}

fn parse_inspect(args: impl IntoIterator<Item = OsString>) -> Result<Command, CliError> {
    let mut input = None;
    let mut json = false;
    let mut options = true;

    for arg in args {
        if options && is(&arg, "--") {
            options = false;
            continue;
        }
        if options && (is(&arg, "--help") || is(&arg, "-h")) {
            return Ok(Command::InspectHelp);
        }
        if options && is(&arg, "--json") {
            json = true;
            continue;
        }
        if options && arg.to_string_lossy().starts_with('-') {
            return Err(CliError(format!(
                "unknown inspect option: {}",
                arg.to_string_lossy()
            )));
        }
        if input.replace(PathBuf::from(arg)).is_some() {
            return Err(CliError("inspect accepts exactly one input path".to_owned()));
        }
    }

    let input = input.ok_or_else(|| CliError("inspect requires an input path".to_owned()))?;
    Ok(Command::Inspect { input, json })
}

fn parse_command(args: impl IntoIterator<Item = OsString>) -> Result<Command, CliError> {
    let mut args = args.into_iter();
    let Some(command) = args.next() else {
        return Ok(Command::Help);
    };
    if is(&command, "--help") || is(&command, "-h") {
        return Ok(Command::Help);
    }
    if is(&command, "--version") || is(&command, "-V") {
        return Ok(Command::Version);
    }
    if is(&command, "inspect") {
        return parse_inspect(args);
    }
    Err(CliError(format!(
        "unknown command: {}",
        command.to_string_lossy()
    )))
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
            writeln!(output, "still: offset={} length={}", still.offset, still.length)?;
            writeln!(output, "video: offset={} length={}", video.offset, video.length)?;
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

fn run_inspect(
    input: PathBuf,
    json: bool,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
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

pub fn run_from<I, S>(args: I, stdout: &mut impl Write, stderr: &mut impl Write) -> u8
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let args = args.into_iter().map(Into::into);
    match parse_command(args) {
        Ok(Command::Help) => match write!(stdout, "{HELP}") {
            Ok(()) => 0,
            Err(_) => 1,
        },
        Ok(Command::Version) => match writeln!(stdout, "xdremux {}", env!("CARGO_PKG_VERSION")) {
            Ok(()) => 0,
            Err(_) => 1,
        },
        Ok(Command::InspectHelp) => match write!(stdout, "{INSPECT_HELP}") {
            Ok(()) => 0,
            Err(_) => 1,
        },
        Ok(Command::Inspect { input, json }) => run_inspect(input, json, stdout, stderr),
        Err(error) => {
            let _ = writeln!(stderr, "error: {}\n\nTry 'xdremux --help' for usage.", error.0);
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_arguments_prints_help() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(run_from(Vec::<&str>::new(), &mut stdout, &mut stderr), 0);
        assert!(String::from_utf8(stdout).unwrap().contains("xdremux <COMMAND>"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn inspect_requires_exactly_one_input() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(run_from(["inspect"], &mut stdout, &mut stderr), 2);
        assert!(String::from_utf8(stderr)
            .unwrap()
            .contains("inspect requires an input path"));
    }

    #[test]
    fn double_dash_allows_paths_that_start_with_dash() {
        let command = parse_command(
            ["inspect", "--", "-capture.jpg"]
                .into_iter()
                .map(OsString::from),
        )
        .unwrap();
        assert_eq!(
            command,
            Command::Inspect {
                input: PathBuf::from("-capture.jpg"),
                json: false,
            }
        );
    }

    #[test]
    fn unknown_command_is_a_usage_error() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(run_from(["convert"], &mut stdout, &mut stderr), 2);
        assert!(stdout.is_empty());
        assert!(String::from_utf8(stderr).unwrap().contains("unknown command: convert"));
    }
}
