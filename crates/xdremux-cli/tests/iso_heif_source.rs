//! The convert entry point must accept a standalone ISO dual-layer HEIF and
//! publish the requested profile without relabeling the source encoding.
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(path)
}

fn unique_dir() -> PathBuf {
    static UNIQUE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after epoch")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "xdremux-cli-iso-heif-{}-{stamp}-{}",
        std::process::id(),
        UNIQUE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::create_dir_all(&directory).expect("temporary directory");
    directory
}

fn path_arg(path: &Path) -> OsString {
    path.as_os_str().to_owned()
}

fn run(args: Vec<OsString>) -> (u8, String, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = xdremux_cli::run_from(args, &mut stdout, &mut stderr);
    (
        code,
        String::from_utf8_lossy(&stdout).into_owned(),
        String::from_utf8_lossy(&stderr).into_owned(),
    )
}

fn convert(input: &Path, output: &Path, extra: &[&str]) -> (u8, String, String) {
    let mut args = vec![
        OsString::from("convert"),
        OsString::from("--input"),
        path_arg(input),
        OsString::from("--output"),
        path_arg(output),
    ];
    args.extend(extra.iter().map(OsString::from));
    run(args)
}

#[test]
fn a_published_iso_heif_reconverts_to_another_profile() {
    let work = unique_dir();
    let input = fixture("proxdr/oppo/find-x7-ultra/lhdr-v2-05.heic");
    let pq = work.join("pq.heic");
    let hlg = work.join("hlg.heic");

    let (code, stdout, stderr) = convert(&input, &pq, &["--hdr-profile", "bt2020-pq"]);
    assert_eq!(code, 0, "PQ conversion failed: {stderr}");
    assert!(stdout.contains("converted:"), "{stdout}");

    let (code, stdout, stderr) = convert(&pq, &hlg, &["--hdr-profile", "bt2020-hlg"]);
    assert_eq!(
        code, 0,
        "the published file must convert through the ISO graph path: {stderr}"
    );
    assert!(stdout.contains("converted:"), "{stdout}");

    let (code, stdout, stderr) = run(vec![
        OsString::from("validate"),
        path_arg(&hlg),
        OsString::from("--hdr-profile"),
        OsString::from("bt2020-hlg"),
        OsString::from("--semantic"),
        OsString::from("--decode-hdr"),
        OsString::from("--json"),
    ]);
    assert_eq!(code, 0, "validation failed: {stderr}");
    let document: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(document["valid"], true);
    assert_eq!(document["kind"], "iso-hdr-heif");
    assert_eq!(
        document["details"]["value"]["semantics"]["hdr_profile"],
        "bt2020-hlg"
    );
    assert!(document["details"]["value"]["semantics"]["decoded_hdr"].is_object());

    fs::remove_dir_all(work).ok();
}

#[test]
fn a_non_pq_iso_source_needs_a_matching_profile() {
    let work = unique_dir();
    let input = fixture("proxdr/oppo/find-x7-ultra/lhdr-v2-05.heic");
    let hlg = work.join("hlg.heic");
    let plain = work.join("plain.heic");

    let (code, _, stderr) = convert(&input, &hlg, &["--hdr-profile", "bt2020-hlg"]);
    assert_eq!(code, 0, "HLG conversion failed: {stderr}");

    let (code, _, stderr) = convert(&hlg, &plain, &[]);
    assert_eq!(code, 1, "the compatible layout must not relabel HLG");
    assert!(
        stderr.contains("compatible layout declares BT.2020 PQ"),
        "{stderr}"
    );
    assert!(!plain.exists(), "a failed conversion must not publish");

    fs::remove_dir_all(work).ok();
}
