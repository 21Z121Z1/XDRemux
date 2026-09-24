use crate::i18n::{message, Language};
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

fn write_human(
    language: Language,
    report: &ValidationReport,
    output: &mut impl Write,
) -> io::Result<()> {
    match report {
        ValidationReport::IsoHdrHeif(value) => {
            writeln!(
                output,
                "{}",
                message!(language, "valid: iso-hdr-heif", "验证通过：iso-hdr-heif")
            )?;
            writeln!(
                output,
                "{}",
                message!(language, "input: {}", "输入：{}", value.input.display())
            )?;
            writeln!(
                output,
                "{}",
                message!(
                    language,
                    "gain-map: {}x{}",
                    "Gain Map：{}x{}",
                    value.width,
                    value.height
                )
            )?;
            writeln!(
                output,
                "{}",
                message!(
                    language,
                    "grid: {}x{}",
                    "网格：{}x{}",
                    value.rows,
                    value.columns
                )
            )?;
            writeln!(
                output,
                "{}",
                message!(
                    language,
                    "tiles: {}",
                    "分块数：{}",
                    value.tile_item_ids.len()
                )
            )?;
            writeln!(
                output,
                "{}",
                message!(language, "channels: {}", "通道数：{}", value.channel_count)
            )?;
            writeln!(
                output,
                "{}",
                message!(
                    language,
                    "chroma: {}",
                    "色度采样：{}",
                    value.chroma_sampling
                )
            )?;
            writeln!(
                output,
                "{}",
                message!(
                    language,
                    "bit-depth: luma={} chroma={}",
                    "位深：亮度={} 色度={}",
                    value.luma_bit_depth,
                    value.chroma_bit_depth
                )
            )
        }
        ValidationReport::LivePhoto(value) => {
            writeln!(
                output,
                "{}",
                message!(language, "valid: live-photo", "验证通过：live-photo")
            )?;
            writeln!(
                output,
                "{}",
                message!(language, "input: {}", "输入：{}", value.input.display())
            )?;
            writeln!(
                output,
                "{}",
                message!(language, "still: {}", "静态图像：{}", value.image.display())
            )?;
            writeln!(
                output,
                "{}",
                message!(language, "movie: {}", "视频：{}", value.video.display())
            )?;
            writeln!(
                output,
                "{}",
                message!(
                    language,
                    "content-identifier: {}",
                    "内容标识符：{}",
                    value.content_identifier
                )
            )?;
            writeln!(
                output,
                "{}",
                message!(
                    language,
                    "still-time-seconds: {:.6}",
                    "静态帧时间（秒）：{:.6}",
                    value.still_time_seconds
                )
            )
        }
    }
}

fn write_json(value: &serde_json::Value, output: &mut impl Write) -> io::Result<()> {
    serde_json::to_writer_pretty(&mut *output, value)
        .map_err(io::Error::other)
        .and_then(|()| writeln!(output))
}

pub(crate) fn run(
    language: Language,
    arguments: ValidateArgs,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
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
                write_human(language, &report, stdout)
            };
            if let Err(error) = result {
                let _ = writeln!(
                    stderr,
                    "{}",
                    message!(
                        language,
                        "error: could not write validation output: {error}",
                        "错误：无法写入验证结果：{error}"
                    )
                );
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
                    "{}",
                    message!(
                        language,
                        "error: could not write validation failure: {write_error}",
                        "错误：无法写入验证失败信息：{write_error}"
                    )
                );
            }
            1
        }
        Err(error) => {
            let _ = writeln!(
                stderr,
                "{}",
                message!(language, "error: {error}", "错误：{error}")
            );
            1
        }
    }
}
