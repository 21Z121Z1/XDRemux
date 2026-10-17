use std::io::Write;
use std::path::PathBuf;

use clap::Args;

#[derive(Debug, Args)]
pub(super) struct ReconstructStylesArgs {
    /// Original HEIC/HEIF containing Styles metadata, delta map and Linear Thumbnail. Repeat for a batch.
    #[arg(long = "input", required = true, value_name = "FILE")]
    inputs: Vec<PathBuf>,
    /// Each source gets a new bundle inside this directory. Existing bundles are never replaced.
    #[arg(long, value_name = "DIR")]
    output_dir: PathBuf,
    /// Limit processing and Gain Map long edge; zero uses native pixels. SDR always retains source resolution.
    #[arg(long, default_value_t = 0, value_name = "PIXELS")]
    max_edge: u32,
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    #[test]
    fn public_command_accepts_multiple_sources_and_a_gain_map_resolution_limit() {
        let parsed = crate::Cli::try_parse_from([
            "xdremux",
            "reconstruct-styles",
            "--input",
            "photo.part1.heic",
            "--input",
            "other.heic",
            "--output-dir",
            "results",
            "--max-edge",
            "1024",
        ])
        .unwrap();
        let crate::RootCommand::ReconstructStyles(args) = parsed.command else {
            panic!("wrong public command")
        };
        assert_eq!(args.inputs.len(), 2);
        assert_eq!(args.max_edge, 1024);
        assert_eq!(args.output_dir, std::path::PathBuf::from("results"));
    }
}

pub(super) fn run(
    args: ReconstructStylesArgs,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (args, stdout);
        let _ = writeln!(
            stderr,
            "error: Photographic Styles reversal requires macOS and its native Apple adapter"
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
        let mut failures = 0;
        let mut results = Vec::new();
        for input in &args.inputs {
            let stem = input.file_stem().unwrap_or_default();
            let mut name = stem.to_os_string();
            name.push(".reconstructed");
            let output = args.output_dir.join(name);
            match xdremux_runtime::reconstruct_styles_hdr(&adapter,input,&output,args.max_edge) {
                Ok(report)=>results.push(serde_json::json!({"input":input,"output":output,"status":"success","report":report})),
                Err(e)=>{
                    failures+=1;
                    let _=writeln!(stderr,"{}: {e}",input.display());
                    results.push(serde_json::json!({"input":input,"status":"failed","error":e.to_string()}));
                }
            }
        }
        if serde_json::to_writer_pretty(
            &mut *stdout,
            &serde_json::json!({"schema_version":1,"results":results,"failures":failures}),
        )
        .is_err()
            || writeln!(stdout).is_err()
        {
            return 1;
        }
        u8::from(failures > 0)
    }
}
