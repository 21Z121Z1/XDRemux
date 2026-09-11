use std::io::{self, Write};
use std::path::PathBuf;

use clap::Args;
use serde_json::json;
use xdremux_runtime::{validate_media_file, ValidationReport};

const VALIDATE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Args)]
pub(crate) struct ValidateArgs {
    /// ISO HDR HEIC/HEIF or one resource of an Apple Live Photo pair.
    #[arg(value_name = "INPUT")]
    input: PathBuf,
    /// Emit one stable machine-readable validation document.
    #[arg(long)]
    json: bool,
}

fn write_human(report: &ValidationReport, output: &mut impl Write) -> io::Result<()> {
    match report {
        ValidationReport::IsoHdrHeif(value) => {
            writeln!(output, "valid: iso-hdr-heif")?;
            writeln!(output, "input: {}", value.input.display())?;
            writeln!(output, "gain-map: {}x{}", value.width, value.height)?;
            writeln!(output, "grid: {}x{}", value.rows, value.columns)?;
            writeln!(output, "tiles: {}", value.tile_item_ids.len())?;
            writeln!(output, "channels: {}", value.channel_count)?;
            writeln!(output, "chroma: {}", value.chroma_sampling)?;
            writeln!(
                output,
                "bit-depth: luma={} chroma={}",
                value.luma_bit_depth, value.chroma_bit_depth
            )
        }
        ValidationReport::LivePhoto(value) => {
            writeln!(output, "valid: live-photo")?;
            writeln!(output, "input: {}", value.input.display())?;
            writeln!(output, "still: {}", value.image.display())?;
            writeln!(output, "movie: {}", value.video.display())?;
            writeln!(output, "content-identifier: {}", value.content_identifier)?;
            writeln!(
                output,
                "still-time-seconds: {:.6}",
                value.still_time_seconds
            )
        }
    }
}

fn write_json(value: &serde_json::Value, output: &mut impl Write) -> io::Result<()> {
    serde_json::to_writer_pretty(&mut *output, value)
        .map_err(io::Error::other)
        .and_then(|()| writeln!(output))
}

pub(crate) fn run(arguments: ValidateArgs, stdout: &mut impl Write, stderr: &mut impl Write) -> u8 {
    match validate_media_file(&arguments.input) {
        Ok(report) => {
            let result = if arguments.json {
                write_json(
                    &json!({
                        "schema_version": VALIDATE_SCHEMA_VERSION,
                        "command": "validate",
                        "valid": true,
                        "kind": report.kind(),
                        "details": report,
                    }),
                    stdout,
                )
            } else {
                write_human(&report, stdout)
            };
            if let Err(error) = result {
                let _ = writeln!(stderr, "error: could not write validation output: {error}");
                return 1;
            }
            0
        }
        Err(error) if arguments.json => {
            let value = json!({
                "schema_version": VALIDATE_SCHEMA_VERSION,
                "command": "validate",
                "valid": false,
                "input": arguments.input.to_string_lossy(),
                "error": error.to_string(),
            });
            if let Err(write_error) = write_json(&value, stdout) {
                let _ = writeln!(
                    stderr,
                    "error: could not write validation failure: {write_error}"
                );
            }
            1
        }
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            1
        }
    }
}
