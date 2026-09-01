use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use jpeg_encoder::{ColorType, Encoder};
use xdremux_format::ChromaSampling;
use xdremux_heif::validate_gain_map_structure;

const JXRS_MARKER: &[u8] = b"\0jxrs";

fn tiny_rgb_jpeg() -> Vec<u8> {
    let mut pixels = Vec::with_capacity(8 * 8 * 3);
    for y in 0_u8..8 {
        for x in 0_u8..8 {
            pixels.extend_from_slice(&[
                x.saturating_mul(32),
                y.saturating_mul(32),
                x.saturating_add(y).saturating_mul(16),
            ]);
        }
    }

    let mut jpeg = Vec::new();
    Encoder::new(&mut jpeg, 100)
        .encode(&pixels, 8, 8, ColorType::Rgb)
        .expect("synthetic JPEG encoding must succeed");
    jpeg
}

fn canonical_uhdr_info() -> [f32; 20] {
    [
        1.0, 1.0, 1.0, 1.0, 4.926, 4.926, 4.926, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        4.926, 4.926, 0.0,
    ]
}

fn synthetic_uhdr_source(base: &[u8]) -> Vec<u8> {
    let mut source = base.to_vec();

    let info_start = source.len();
    for value in canonical_uhdr_info() {
        source.extend_from_slice(&value.to_le_bytes());
    }

    let jpeg = tiny_rgb_jpeg();
    let data_start = source.len();
    source.extend_from_slice(&jpeg);

    let json_start = source.len();
    let info_offset = json_start - info_start;
    let data_offset = json_start - data_start;
    let manifest = format!(
        "[{{\"name\":\"local.uhdr.gainmap.info\",\"offset\":{info_offset},\"length\":80}},{{\"name\":\"local.uhdr.gainmap.data\",\"offset\":{data_offset},\"length\":{}}}]",
        jpeg.len()
    );
    source.extend_from_slice(manifest.as_bytes());
    source.extend_from_slice(JXRS_MARKER);
    let footer_length = u32::try_from(manifest.len() + JXRS_MARKER.len() + 4)
        .expect("synthetic footer length must fit u32");
    source.extend_from_slice(&footer_length.to_le_bytes());
    source
}

fn public_heif_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/20260312_135609..heic")
}

fn unique_dir() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "xdremux-cli-convert-{}-{stamp}",
        std::process::id()
    ))
}

fn path_arg(path: &Path) -> OsString {
    path.as_os_str().to_owned()
}

#[test]
fn convert_command_executes_full_rust_proxdr_pipeline() {
    let work = unique_dir();
    fs::create_dir_all(&work).expect("temporary test directory must be created");
    let input = work.join("input.heic");
    let output = work.join("output.heic");

    let base = fs::read(public_heif_fixture()).expect("versioned HEIF fixture must be readable");
    fs::write(&input, synthetic_uhdr_source(&base)).expect("synthetic input must be written");

    let args = vec![
        OsString::from("convert"),
        OsString::from("--input"),
        path_arg(&input),
        OsString::from("--output"),
        path_arg(&output),
    ];
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = xdremux_cli::run_from(args, &mut stdout, &mut stderr);

    assert_eq!(
        code,
        0,
        "CLI conversion failed: {}",
        String::from_utf8_lossy(&stderr)
    );
    assert!(stderr.is_empty());
    assert!(String::from_utf8(stdout)
        .expect("CLI stdout must be UTF-8")
        .contains("converted:"));

    let converted = fs::read(&output).expect("CLI must atomically publish the output file");
    let structure =
        validate_gain_map_structure(&converted).expect("CLI output Gain Map graph must validate");
    assert_eq!(structure.channel_count, 3);
    assert_eq!(structure.chroma_sampling, ChromaSampling::Yuv444);
    assert_eq!(structure.luma_bit_depth, 8);
    assert_eq!(structure.chroma_bit_depth, 8);
    assert_eq!(structure.width, 8);
    assert_eq!(structure.height, 8);
    assert!(!converted
        .windows(b"local.uhdr.gainmap".len())
        .any(|window| window == b"local.uhdr.gainmap"));

    fs::remove_dir_all(work).expect("temporary test directory must be removed");
}
