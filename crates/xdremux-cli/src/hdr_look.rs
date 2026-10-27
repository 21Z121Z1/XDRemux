use std::io::Write;
use std::path::PathBuf;

use crate::i18n::Language;
use clap::Args;

#[derive(Debug, Args)]
pub(super) struct HdrLookArgs {
    /// Original Photographic Styles HEIC/HEIF photos or directories.
    #[arg(required = true, value_name = "INPUT")]
    inputs: Vec<PathBuf>,
    /// New output directory; default is a new HDR-Look folder beside the first input.
    #[arg(long, value_name = "DIR")]
    output_dir: Option<PathBuf>,
    /// Search subdirectories without following symlinks or processing generated outputs.
    #[arg(long)]
    recursive: bool,
    /// HDR display headroom, as a peak/reference-white ratio.
    #[arg(long,default_value_t=8,value_parser=clap::value_parser!(u32).range(2..=8))]
    headroom: u32,
    /// Zero uses original pixels; otherwise limit processing and Gain Map long edge.
    #[arg(long, default_value_t = 0, value_name = "PIXELS")]
    max_edge: u32,
    /// List planned inputs and outputs without writing files or calling the native adapter.
    #[arg(long)]
    dry_run: bool,
    /// Emit results as JSON on stdout; no report files are saved.
    #[arg(long)]
    json: bool,
}

pub(super) fn run(
    language: Language,
    args: HdrLookArgs,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
    let plan = match xdremux_runtime::plan_hdr_look(
        &args.inputs,
        args.output_dir.as_deref(),
        args.recursive,
        args.headroom,
        args.max_edge,
    ) {
        Ok(plan) => plan,
        Err(e) => {
            let _ = writeln!(stderr, "error: {e}");
            return 2;
        }
    };
    if args.dry_run {
        return u8::from(
            serde_json::to_writer_pretty(&mut *stdout, &plan).is_err() || writeln!(stdout).is_err(),
        );
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = writeln!(
            stderr,
            "error: {}",
            language.text(
                "HDR look requires Apple Silicon and macOS 27 or newer",
                "HDR look 需要 Apple Silicon 和 macOS 27 或更新版本"
            )
        );
        1
    }
    #[cfg(target_os = "macos")]
    {
        let adapter = match super::resolve_apple_adapter_executable() {
            Ok(path) => path,
            Err(e) => {
                let _ = writeln!(stderr, "error: {e}");
                return 1;
            }
        };
        let receipt = match xdremux_runtime::render_hdr_look(&plan, &adapter, |index, row| {
            if !args.json {
                let status = if row.status == "success" {
                    language.text("done", "完成")
                } else {
                    language.text("failed", "失败")
                };
                let _ = writeln!(
                    stderr,
                    "[{}/{}] {status}: {}{}",
                    index + 1,
                    plan.items.len(),
                    row.item.input.display(),
                    row.error
                        .as_ref()
                        .map(|e| format!(" — {e}"))
                        .unwrap_or_default()
                );
            }
        }) {
            Ok(receipt) => receipt,
            Err(e) => {
                let _ = writeln!(stderr, "error: {e}");
                return 1;
            }
        };
        let written = if args.json {
            serde_json::to_writer_pretty(&mut *stdout, &receipt)
                .map_err(std::io::Error::other)
                .and_then(|_| writeln!(stdout))
        } else {
            writeln!(
                stdout,
                "{}: {}\n{}: {}",
                language.text("Output directory", "输出目录"),
                receipt.output_dir.display(),
                language.text("Failed", "失败数量"),
                receipt.failures
            )
        };
        u8::from(receipt.failures > 0 || written.is_err())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn dry_run_discovers_files_without_native_or_writes_and_rejects_invalid_headroom() {
        let t = tempfile::tempdir().unwrap();
        let input = t.path().join("中文 source.HEIC");
        std::fs::write(&input, b"no decoding needed").unwrap();
        let output = t.path().join("new results");
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = crate::run_from(
            [
                "hdr-look",
                input.to_str().unwrap(),
                "--output-dir",
                output.to_str().unwrap(),
                "--dry-run",
                "--json",
            ],
            &mut out,
            &mut err,
        );
        assert_eq!(code, 0, "{}", String::from_utf8_lossy(&err));
        let receipt: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(receipt["headroom"], 8);
        assert_eq!(receipt["items"].as_array().unwrap().len(), 1);
        assert!(!output.exists());
        assert_eq!(
            crate::run_from(
                [
                    "hdr-look",
                    input.to_str().unwrap(),
                    "--headroom",
                    "3",
                    "--dry-run"
                ],
                &mut Vec::new(),
                &mut Vec::new()
            ),
            2
        );
    }
}
