//! CLI presentation only. Never pass a Language into product/runtime/format crates.
use clap::Command;
use std::ffi::{OsStr, OsString};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Language {
    #[default]
    En,
    ZhCn,
}

impl Language {
    pub(crate) fn parse(raw: &str) -> Result<Self, String> {
        let normalized = raw.trim().to_ascii_lowercase().replace('_', "-");
        let base = normalized.split(['.', '@']).next().unwrap_or("");
        match base {
            "c" | "posix" | "en" => Ok(Self::En),
            "zh" => Ok(Self::ZhCn),
            value if value.starts_with("en-") => Ok(Self::En),
            value if value.starts_with("zh-") => Ok(Self::ZhCn),
            _ => Err("supported languages: en, zh-CN".to_owned()),
        }
    }
    pub(crate) fn text(self, en: &'static str, zh: &'static str) -> &'static str {
        match self {
            Self::En => en,
            Self::ZhCn => zh,
        }
    }
    pub(crate) fn disposition(self, value: &str) -> &str {
        match (self, value) {
            (Self::ZhCn, "copied") => "已复制",
            (Self::ZhCn, "duplicate") => "重复",
            (Self::ZhCn, "dry-run") => "预演",
            (Self::ZhCn, "failed") => "失败",
            (Self::ZhCn, "converted") => "已转换",
            (Self::ZhCn, "skipped-existing") => "已跳过现有文件",
            _ => value,
        }
    }
}

/// Preselect help/error language without consuming or rewriting arguments.
/// Clap remains authoritative for syntax, conflicts, and unsupported values.
/// Arguments following `--` and non-Unicode file names stay opaque.
pub(crate) fn resolve(
    args: &[OsString],
    mut environment: impl FnMut(&str) -> Option<OsString>,
) -> Language {
    let mut explicit = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == OsStr::new("--") {
            break;
        }
        if arg == OsStr::new("--lang") {
            explicit = iter.next().and_then(|value| value.to_str());
        } else if let Some(value) = arg.to_str().and_then(|arg| arg.strip_prefix("--lang=")) {
            explicit = Some(value);
        }
    }
    if let Some(raw) = explicit {
        return Language::parse(raw).unwrap_or_default();
    }
    for name in ["XDREMUX_LANG", "LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Some(raw) = environment(name) {
            if raw.is_empty() {
                continue;
            }
            return raw
                .to_str()
                .and_then(|value| Language::parse(value).ok())
                .unwrap_or_default();
        }
    }
    Language::En
}

/// Translate descriptions before clap renders usage, never by replacing output
/// substrings (which could change a user's file name, an enum, or JSON).
pub(crate) fn command(mut command: Command, language: Language) -> Command {
    if language == Language::En {
        return command;
    }
    command.build();
    localize(&mut command);
    command
}

fn about(name: &str) -> &'static str {
    match name {
        "xdremux" => "使用统一的 Rust runtime 转换和检查 ProXDR 与 Motion Photo 资源。",
        "inspect" => "检查一个输入文件，不进行转换。",
        "convert" => "使用统一的 Rust engine/runtime 转换一个支持的输入文件。",
        "batch" => "以确定的顺序批量转换支持的资源。",
        "categorize" => "分类照片资源并发布到确定的目录中。",
        "validate" => "验证一个标准输出，不进行转换。",
        _ => "",
    }
}

fn description(command: &str, id: &str) -> Option<&'static str> {
    Some(match (command, id) {
        (_, "help") => "显示帮助信息",
        (_, "version") => "显示版本信息",
        (_, "lang") => "终端显示语言（en 或 zh-CN）；JSON 始终保持不变。",
        ("inspect", "input") | ("convert", "input") => "输入 ProXDR HEIC 或支持的 Motion Photo。",
        ("validate", "input") => "ISO HDR HEIC/HEIF，或 Apple Live Photo 文件对中的一个资源。",
        ("categorize", "inputs") => "输入图像或目录；可重复指定多个路径，目录会递归查找。",
        (_, "inputs") => "输入文件；可重复指定多个文件。",
        (_, "input_dirs") => "在目录中查找 HEIC/HEIF/JPEG；可重复指定多个目录。",
        ("inspect", "json") => "输出稳定的机器可读检查 schema。",
        ("validate", "json") => "输出一个稳定的机器可读验证文档。",
        (_, "json") => "输出一个稳定的 JSON 回执，而不是终端进度信息。",
        (_, "oppo_compatible") => "转换 ProXDR 静态照片时保留 OPPO 相册兼容性。",
        (_, "apple_portrait") => "利用 Vision 语义保留完整的 Apple Portrait 编辑资源。",
        (_, "apple_portrait_oppo") => {
            "使用 OPPO 景深及已有遮罩，不使用 Vision；人像效果受限（macOS）。"
        }
        (_, "apple_styles") => "为 ProXDR 静态照片附加 Rust 管理的 Photographic Styles 资源图。",
        (_, "output") => "输出 HEIC；ProXDR 默认原地替换，Motion Photo 默认生成新文件对。",
        (_, "recursive") => "递归查找输入目录；不跟随符号链接。",
        ("categorize", "output_dir") => "按资源类型和拍摄模式分类的根目录。",
        (_, "output_dir") => "输出目录；省略时写入源文件旁，使用 .xdremux 后缀。",
        (_, "skip_existing") => "仅在持久化来源记录匹配时复用已完成的 Live Photo 文件对。",
        (_, "resume") => "从持久化检查点恢复已完成的 Live Photo，并重试其余项目。",
        (_, "checkpoint") => "Motion Photo 检查点路径；兼容性状态保存到追加 .motion-photo 的路径。",
        (_, "categorize") => "按资源类型和主要拍摄模式归档转换后的资源。",
        (_, "jobs") => "最大并行转换数，必须大于零。",
        (_, "dry_run") => "仅规划并报告分类结果，不发布文件。",
        _ => return None,
    })
}

fn localize(command: &mut Command) {
    let name = command.get_name().to_owned();
    let arguments = command
        .get_arguments()
        .map(|arg| (arg.get_id().clone(), arg.is_positional()))
        .collect::<Vec<_>>();
    let mut localized = command.clone().about(about(&name)).long_about(about(&name))
        .subcommand_help_heading("命令")
        .help_template("{before-help}{name} {version}\n{about-with-newline}\n用法： {usage}\n\n{all-args}{after-help}");
    for (id, positional) in arguments {
        if let Some(text) = description(&name, id.as_str()) {
            localized = localized.mut_arg(id, |arg| {
                arg.help(text).long_help(text).help_heading(if positional {
                    "参数"
                } else {
                    "选项"
                })
            });
        }
    }
    for child in localized.get_subcommands_mut() {
        localize(child);
    }
    *command = localized;
}

// Both literals live at the call site, preserving Rust format! capture hygiene.
macro_rules! message {
    ($language:expr, $en:literal, $zh:literal $(, $arg:expr)* $(,)?) => {
        match $language {
            $crate::i18n::Language::En => format!($en $(, $arg)*),
            $crate::i18n::Language::ZhCn => format!($zh $(, $arg)*),
        }
    };
}
pub(crate) use message;

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    #[test]
    fn normalizes_supported_locales() {
        for raw in [
            "zh",
            "zh-CN",
            "zh_CN.UTF-8",
            "zh-Hans-CN",
            "zh_TW",
            "ZH_cn@Hans",
        ] {
            assert_eq!(Language::parse(raw), Ok(Language::ZhCn));
        }
        for raw in ["en", "en_US.UTF-8", "C", "C.UTF-8", "POSIX"] {
            assert_eq!(Language::parse(raw), Ok(Language::En));
        }
        assert!(Language::parse("fr").is_err());
        assert!(Language::parse("").is_err());
    }
    #[test]
    fn precedence_and_empty_values_are_deterministic() {
        let args = |values: &[&str]| values.iter().map(OsString::from).collect::<Vec<_>>();
        let env = |name: &str| match name {
            "XDREMUX_LANG" => Some("en".into()),
            _ => Some("zh_CN.UTF-8".into()),
        };
        assert_eq!(
            resolve(&args(&["convert", "--lang=zh-CN"]), env),
            Language::ZhCn
        );
        assert_eq!(resolve(&[], env), Language::En);
        assert_eq!(
            resolve(&args(&["inspect", "--", "--lang=zh-CN"]), env),
            Language::En
        );
        let keys = ["XDREMUX_LANG", "LC_ALL", "LC_MESSAGES", "LANG"];
        for (index, expected_key) in keys.iter().enumerate() {
            let selected = resolve(&[], |key| {
                let position = keys.iter().position(|candidate| *candidate == key).unwrap();
                Some(
                    if position < index {
                        ""
                    } else if key == *expected_key {
                        "zh_CN.UTF-8"
                    } else {
                        "en"
                    }
                    .into(),
                )
            });
            assert_eq!(selected, Language::ZhCn);
        }
        assert_eq!(resolve(&[], |_| None), Language::En);
        assert_eq!(resolve(&[], |_| Some("unsupported".into())), Language::En);
    }
    #[test]
    fn every_public_description_has_a_translation() {
        fn check(command: &Command) {
            assert!(!about(command.get_name()).is_empty());
            for arg in command.get_arguments() {
                assert!(
                    description(command.get_name(), arg.get_id().as_str()).is_some(),
                    "{}: {}",
                    command.get_name(),
                    arg.get_id()
                );
            }
            for child in command.get_subcommands() {
                check(child);
            }
        }
        let mut command = crate::Cli::command();
        command.build();
        check(&command);
    }
}
