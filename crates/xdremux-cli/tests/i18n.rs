//! Exercise the process boundary without mutating the test process environment.
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn run(args: &[&OsStr], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_xdremux"));
    for key in ["XDREMUX_LANG", "LC_ALL", "LC_MESSAGES", "LANG"] {
        command.env_remove(key);
    }
    command
        .args(args)
        .env("NO_COLOR", "1")
        .envs(env.iter().copied());
    command.output().expect("run CLI")
}
fn strings(args: &[&str], env: &[(&str, &str)]) -> Output {
    run(&args.iter().map(OsStr::new).collect::<Vec<_>>(), env)
}
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("xdremux-i18n-{}-{stamp} 空格", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/motion-photo/samsung/heif-ultrahdr-01.heic")
}

#[test]
fn every_help_page_supports_both_languages() {
    for name in ["", "inspect", "convert", "batch", "categorize", "validate"] {
        for language in ["en", "zh-CN"] {
            let mut args = vec!["--lang", language];
            if !name.is_empty() {
                args.push(name);
            }
            args.push("--help");
            let output = strings(&args, &[]);
            assert!(output.status.success(), "{output:?}");
            assert!(output.stderr.is_empty());
            let help = String::from_utf8(output.stdout).unwrap();
            assert!(
                help.contains(if language == "en" {
                    "Usage:"
                } else {
                    "用法："
                }),
                "{help}"
            );
            assert!(help.contains("--lang"));
            assert!(help.contains("--help"));
            if name == "convert" || name == "batch" {
                assert!(help.contains("--apple-styles"));
                assert!(help.contains("--apple-cinematic"));
                assert!(help.contains("--focal-length-35mm"));
                assert!(help.contains("--aperture"));
            }
        }
    }
}
#[test]
fn environment_precedence_and_global_option_are_observable() {
    for (environment, expected) in [
        (vec![("LANG", "zh_CN.UTF-8")], true),
        (
            vec![("LANG", "zh_CN.UTF-8"), ("LC_MESSAGES", "en_US.UTF-8")],
            false,
        ),
        (vec![("LC_ALL", "C"), ("LANG", "zh_CN.UTF-8")], false),
        (vec![("XDREMUX_LANG", "zh-CN"), ("LC_ALL", "POSIX")], true),
        (vec![("XDREMUX_LANG", ""), ("LC_ALL", "zh-Hans")], true),
        (vec![("XDREMUX_LANG", "fr"), ("LANG", "zh_CN.UTF-8")], false),
    ] {
        let output = strings(&["--help"], &environment);
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().contains("用法："),
            expected
        );
    }
    for args in [
        vec!["--lang=zh-CN", "convert", "--help"],
        vec!["convert", "--lang", "zh_CN.UTF-8", "--help"],
    ] {
        let output = strings(&args, &[("XDREMUX_LANG", "en")]);
        assert!(output.status.success());
        assert!(String::from_utf8(output.stdout).unwrap().contains("用法："));
    }
    let no_args = strings(&[], &[("LANG", "zh_CN.UTF-8")]);
    assert!(no_args.status.success());
    assert!(String::from_utf8(no_args.stdout).unwrap().contains("命令"));
}
#[test]
fn parse_errors_keep_stdout_empty_and_exit_code_two() {
    for language in ["en", "zh-CN"] {
        for args in [
            vec!["batch", "--json", "--jobs", "0"],
            vec!["convert", "--unknown"],
            vec!["inspect", "--json"],
        ] {
            let output = strings(&args, &[("XDREMUX_LANG", language)]);
            assert_eq!(output.status.code(), Some(2), "{output:?}");
            assert!(output.stdout.is_empty());
            assert!(!output.stderr.is_empty());
            if language == "zh-CN" {
                assert!(String::from_utf8(output.stderr)
                    .unwrap()
                    .starts_with("错误："));
            }
        }
    }
    let bad_language = strings(&["--lang", "fr", "inspect", "--json", "missing.heic"], &[]);
    assert_eq!(bad_language.status.code(), Some(2));
    assert!(bad_language.stdout.is_empty());
}
#[test]
fn json_inspection_and_validation_failure_are_byte_identical() {
    let input = fixture();
    assert!(
        input.is_file(),
        "accepted real fixture missing: {}",
        input.display()
    );
    for args in [
        vec![
            OsStr::new("inspect"),
            input.as_os_str(),
            OsStr::new("--json"),
        ],
        vec![
            OsStr::new("validate"),
            OsStr::new("nonexistent converted error.heic"),
            OsStr::new("--json"),
        ],
    ] {
        let english = run(&args, &[("XDREMUX_LANG", "en")]);
        let chinese = run(&args, &[("XDREMUX_LANG", "zh-CN")]);
        assert_eq!(english.status.code(), chinese.status.code());
        assert_eq!(english.stdout, chinese.stdout);
        assert!(english.stderr.is_empty());
        assert!(chinese.stderr.is_empty());
        let json: serde_json::Value = serde_json::from_slice(&english.stdout).unwrap();
        assert_eq!(json["schema_version"], 1);
    }
}
#[test]
fn successful_batch_validation_and_categorization_json_are_locale_independent() {
    let scratch = Scratch::new();
    let input = fixture();
    assert!(input.is_file());
    let out = scratch.0.join("output");
    let batch = [
        OsStr::new("batch"),
        OsStr::new("--input"),
        input.as_os_str(),
        OsStr::new("--output-dir"),
        out.as_os_str(),
        OsStr::new("--jobs"),
        OsStr::new("1"),
        OsStr::new("--json"),
    ];
    let english = run(&batch, &[("XDREMUX_LANG", "en")]);
    assert!(english.status.success(), "{english:?}");
    fs::remove_dir_all(&out).unwrap();
    let chinese = run(&batch, &[("XDREMUX_LANG", "zh-CN")]);
    assert!(chinese.status.success(), "{chinese:?}");
    assert_eq!(english.stdout, chinese.stdout);
    assert!(chinese.stderr.is_empty());
    let receipt: serde_json::Value = serde_json::from_slice(&chinese.stdout).unwrap();
    assert_eq!(receipt["successes"][0]["status"], "converted");
    let output = PathBuf::from(receipt["successes"][0]["outputs"][0].as_str().unwrap());
    let members = receipt["successes"][0]["outputs"].as_array().unwrap();
    assert_eq!(members.len(), 2);
    for member in members {
        let path = PathBuf::from(member.as_str().unwrap());
        let validation = [
            OsStr::new("validate"),
            path.as_os_str(),
            OsStr::new("--json"),
        ];
        let english = run(&validation, &[("XDREMUX_LANG", "en")]);
        let chinese = run(&validation, &[("XDREMUX_LANG", "zh-CN")]);
        assert!(english.status.success(), "{english:?}");
        assert!(chinese.status.success(), "{chinese:?}");
        assert_eq!(english.stdout, chinese.stdout);
        let result: serde_json::Value = serde_json::from_slice(&english.stdout).unwrap();
        assert_eq!(result["valid"], true);
        assert_eq!(result["kind"], "live-photo");
    }
    let categories = scratch.0.join("categories");
    let categorize = [
        OsStr::new("categorize"),
        OsStr::new("--input"),
        output.as_os_str(),
        OsStr::new("--output-dir"),
        categories.as_os_str(),
        OsStr::new("--dry-run"),
        OsStr::new("--json"),
    ];
    let english = run(&categorize, &[("XDREMUX_LANG", "en")]);
    let chinese = run(&categorize, &[("XDREMUX_LANG", "zh-CN")]);
    assert!(english.status.success(), "{english:?}");
    assert_eq!(english.stdout, chinese.stdout);
}
#[test]
fn human_output_translates_labels_but_not_user_paths() {
    let scratch = Scratch::new();
    // These real Unicode/special-character names work on APFS, NTFS and the
    // Linux runner filesystem. Keep translation-like words inside the path.
    let name = scratch.0.join("原片 error converted '[]#🟦.heic");
    let original = fs::read(fixture()).unwrap();
    fs::copy(fixture(), &name).unwrap();
    for (language, label) in [("en", "input:"), ("zh-CN", "输入：")] {
        let output = run(
            &[OsStr::new("inspect"), name.as_os_str()],
            &[("XDREMUX_LANG", language)],
        );
        assert!(output.status.success(), "{output:?}");
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.starts_with(label), "{text}");
        assert!(text.contains(name.to_str().unwrap()), "{text}");
    }
    let args = [
        OsStr::new("inspect"),
        name.as_os_str(),
        OsStr::new("--json"),
    ];
    let english = run(&args, &[("XDREMUX_LANG", "en")]);
    let chinese = run(&args, &[("XDREMUX_LANG", "zh-CN")]);
    assert!(english.status.success() && chinese.status.success());
    assert_eq!(english.stdout, chinese.stdout);
    let json: serde_json::Value = serde_json::from_slice(&english.stdout).unwrap();
    assert_eq!(json["input"].as_str(), name.to_str());
    assert_eq!(fs::read(&name).unwrap(), original);
    let output = strings(
        &["inspect", "--", "--lang=zh-CN"],
        &[("XDREMUX_LANG", "en")],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .starts_with("error:"));
}

#[test]
fn current_feature_conflicts_remain_usage_errors_in_both_languages() {
    for language in ["en", "zh-CN"] {
        for args in [
            vec![
                "convert",
                "--input",
                "missing.mp4",
                "--apple-cinematic",
                "--apple-styles",
            ],
            vec!["convert", "--input", "missing.mp4", "--aperture", "2.8"],
            vec![
                "batch",
                "--input",
                "missing.mp4",
                "--apple-cinematic",
                "--checkpoint",
                "checkpoint.json",
            ],
        ] {
            let result = strings(&args, &[("XDREMUX_LANG", language)]);
            assert_eq!(result.status.code(), Some(2), "{result:?}");
            assert!(result.stdout.is_empty());
        }
    }
    let input = fixture();
    let arguments = [
        OsStr::new("validate"),
        input.as_os_str(),
        OsStr::new("--apple-styles"),
        OsStr::new("--json"),
    ];
    let english = run(&arguments, &[("XDREMUX_LANG", "en")]);
    let chinese = run(&arguments, &[("XDREMUX_LANG", "zh-CN")]);
    assert_eq!(english.status.code(), chinese.status.code());
    assert_eq!(english.stdout, chinese.stdout);
    assert!(english.stderr.is_empty() && chinese.stderr.is_empty());
}

// Linux's runner filesystem accepts opaque byte names. APFS rejects these at
// fixture creation (EILSEQ), before the CLI can run; its real path coverage is
// the Unicode/special-character process regression above.
#[cfg(target_os = "linux")]
#[test]
fn non_unicode_input_arguments_remain_opaque_and_locale_independent() {
    use std::os::unix::ffi::OsStringExt;
    let scratch = Scratch::new();
    let path = scratch
        .0
        .join(std::ffi::OsString::from_vec(b"source-\xff.heic".to_vec()));
    let original = fs::read(fixture()).unwrap();
    fs::write(&path, &original).unwrap();
    for language in ["en", "zh-CN"] {
        let output = run(
            &[OsStr::new("inspect"), path.as_os_str()],
            &[("XDREMUX_LANG", language)],
        );
        assert!(output.status.success(), "{output:?}");
    }
    let args = [
        OsStr::new("inspect"),
        path.as_os_str(),
        OsStr::new("--json"),
    ];
    let english = run(&args, &[("XDREMUX_LANG", "en")]);
    let chinese = run(&args, &[("XDREMUX_LANG", "zh-CN")]);
    // The existing JSON PathBuf serializer rejects non-Unicode paths. Language
    // selection must preserve that failure and payload, not reinterpret bytes.
    assert_eq!(english.status.code(), Some(1), "{english:?}");
    assert_eq!(english.status.code(), chinese.status.code());
    assert_eq!(english.stdout, chinese.stdout);
    assert_eq!(fs::read(path).unwrap(), original);
}
