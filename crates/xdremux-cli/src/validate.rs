use crate::i18n::{message, Language};
use std::io::{self, Write};
use std::path::PathBuf;

use clap::Args;
use serde_json::json;
use xdremux_runtime::{
    validate_media_file_with_profile, GainMapValidationProfile, ValidationReport,
};

const VALIDATE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Args)]
pub(crate) struct ValidateArgs {
    /// ISO HDR HEIC/HEIF, an Apple Live Photo pair member, or a PVT directory.
    #[arg(value_name = "INPUT")]
    input: PathBuf,
    /// Emit one stable machine-readable validation document.
    #[arg(long)]
    json: bool,
    /// Also require generated Photographic Styles and Texture data (macOS 27).
    #[arg(long)]
    apple_styles: bool,
    /// Check numeric tone-map metadata and derived HDR colorimetry as well as the graph.
    #[arg(long)]
    semantic: bool,
    /// Validate a selected BT.2020 HDR profile, orientation, color and optional metadata.
    #[arg(long, value_name = "bt2020-pq|bt2020-hlg", value_parser = ["bt2020-pq", "bt2020-hlg"], conflicts_with = "apple_styles")]
    hdr_profile: Option<String>,
    /// Decode final-file samples and reconstruct relative linear BT.2020 pixels; requires an HDR profile.
    #[arg(long, requires = "hdr_profile")]
    decode_hdr: bool,
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
            )?;
            if let Some(semantics) = &value.semantics {
                writeln!(
                    output,
                    "{}",
                    message!(
                        language,
                        "semantics: metadata-channels={} color={} base-stops={} alternate-stops={}",
                        "语义：元数据通道={} 色彩={} 基础档位={} 增强档位={}",
                        semantics.metadata_channel_count,
                        semantics.output_color,
                        semantics.base_headroom_stops,
                        semantics.alternate_headroom_stops
                    )
                )?;
                if let Some(profile) = semantics.hdr_profile {
                    writeln!(
                        output,
                        "{}",
                        message!(language, "hdr-profile: {}", "HDR 配置：{}", profile)
                    )?;
                }
                if let Some(registered) = &semantics.registered_metadata {
                    write_registered(language, registered, output)?;
                }
                if let Some(decoded) = &semantics.decoded_hdr {
                    writeln!(
                        output,
                        "{}",
                        message!(
                            language,
                            "decoded-hdr: {}x{} linear-bt2020 min={:?} max={:?}",
                            "HDR 重建：{}x{} linear-bt2020 最小值={:?} 最大值={:?}",
                            decoded.width,
                            decoded.height,
                            decoded.minimum_rgb,
                            decoded.maximum_rgb
                        )
                    )?;
                }
            }
            Ok(())
        }
        ValidationReport::DualLayerJpeg(value) => {
            writeln!(
                output,
                "{}",
                message!(
                    language,
                    "valid: dual-layer-jpeg",
                    "验证通过：dual-layer-jpeg"
                )
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
                    "base: {}x{} components={}",
                    "基本图像：{}x{} 分量数={}",
                    value.base_width,
                    value.base_height,
                    value.base_components
                )
            )?;
            writeln!(
                output,
                "{}",
                message!(
                    language,
                    "gain-map: {}x{} components={}",
                    "增强图像：{}x{} 分量数={}",
                    value.gain_map_width,
                    value.gain_map_height,
                    value.gain_map_components
                )
            )?;
            writeln!(
                output,
                "{}",
                message!(language, "images: {}", "图像数：{}", value.mp_image_count)
            )?;
            writeln!(
                output,
                "{}",
                message!(
                    language,
                    "metadata: parameter-sets={} base-space={} base-stops={} alternate-stops={}",
                    "元数据：参数组={} 基础色域={} 基础档位={} 增强档位={}",
                    value.parameter_sets,
                    value.use_base_color_space,
                    value.base_headroom_stops,
                    value.alternate_headroom_stops
                )
            )?;
            writeln!(
                output,
                "{}",
                message!(
                    language,
                    "carriers: iso-declaration={} iso-parameters={} base-icc={}",
                    "载体：ISO 声明={} ISO 参数={} 基本图像 ICC={}",
                    value.base_iso_declaration,
                    value.gain_map_iso_parameters,
                    value.base_icc_present
                )
            )?;
            if let Some(registered) = &value.registered_metadata {
                write_registered(language, registered, output)?;
            }
            if let Some(decoded) = &value.decoded_hdr {
                writeln!(
                    output,
                    "{}",
                    message!(
                        language,
                        "decoded-hdr: {}x{} linear-bt2020 min={:?} max={:?}",
                        "HDR 重建：{}x{} linear-bt2020 最小值={:?} 最大值={:?}",
                        decoded.width,
                        decoded.height,
                        decoded.minimum_rgb,
                        decoded.maximum_rgb
                    )
                )?;
            }
            Ok(())
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

fn write_registered(
    language: Language,
    report: &xdremux_runtime::RegisteredHdrProcessingReport,
    output: &mut impl Write,
) -> io::Result<()> {
    writeln!(
        output,
        "{}",
        message!(
            language,
            "registered-hdr: dynamic={} base-mapping={} combined-mapping={} display-adaptation={}",
            "注册 HDR：动态元数据={} 基本图像映射={} 合成图像映射={} 显示适配={}",
            report.dynamic_metadata,
            report.base_mapping,
            report.combined_mapping,
            report.display_adaptation
        )
    )
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
    let profile = if let Some(name) = &arguments.hdr_profile {
        GainMapValidationProfile::Bt2020 {
            transfer: if name == "bt2020-pq" {
                xdremux_runtime::HdrTransfer::Pq
            } else {
                xdremux_runtime::HdrTransfer::Hlg
            },
            decode_pixels: arguments.decode_hdr,
        }
    } else if arguments.semantic {
        GainMapValidationProfile::Semantics
    } else {
        GainMapValidationProfile::Structure
    };
    let result = validate_media_file_with_profile(&arguments.input, profile)
        .map_err(|e| e.to_string())
        .and_then(|report| {
            if arguments.apple_styles {
                #[cfg(target_os = "macos")]
                {
                    let adapter = super::resolve_apple_adapter_executable()?;
                    xdremux_runtime::validate_apple_styles_file(adapter, &arguments.input)
                        .map_err(|e| e.to_string())?;
                }
                #[cfg(not(target_os = "macos"))]
                return Err(
                    "native Photographic Styles and Texture validation requires macOS 27"
                        .to_owned(),
                );
            }
            Ok(report)
        });
    match result {
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn feature_validation_uses_the_same_styles_intent() {
        #[derive(clap::Parser)]
        struct TestCli {
            #[command(flatten)]
            args: ValidateArgs,
        }
        let cli = TestCli::try_parse_from(["validate", "result.heic", "--apple-styles"]).unwrap();
        assert!(cli.args.apple_styles);
        assert!(!cli.args.semantic);
        assert!(
            !TestCli::try_parse_from(["validate", "result.heic"])
                .unwrap()
                .args
                .apple_styles
        );
    }

    #[test]
    fn semantic_validation_is_explicit_opt_in() {
        #[derive(clap::Parser)]
        struct TestCli {
            #[command(flatten)]
            args: ValidateArgs,
        }
        assert!(
            TestCli::try_parse_from(["validate", "result.heic", "--semantic"])
                .unwrap()
                .args
                .semantic
        );
        assert!(
            !TestCli::try_parse_from(["validate", "result.heic"])
                .unwrap()
                .args
                .semantic
        );
    }

    #[test]
    fn decoded_validation_requires_an_explicit_supported_hdr_profile() {
        #[derive(clap::Parser)]
        struct TestCli {
            #[command(flatten)]
            args: ValidateArgs,
        }
        assert!(TestCli::try_parse_from(["validate", "result.heic", "--decode-hdr"]).is_err());
        assert!(TestCli::try_parse_from([
            "validate",
            "result.heic",
            "--hdr-profile",
            "compatible"
        ])
        .is_err());
        let value = TestCli::try_parse_from([
            "validate",
            "result.heic",
            "--hdr-profile",
            "bt2020-hlg",
            "--decode-hdr",
        ])
        .unwrap();
        assert!(value.args.decode_hdr);
        assert_eq!(value.args.hdr_profile.as_deref(), Some("bt2020-hlg"));
    }
}
