//! A two-image JPEG assembled by XDRemux must keep both compressed images
//! decodable and must carry the ISO 21496-1 carriers that a gain-map reader
//! expects. The input is a real dual-layer JPEG from a public fixture; the
//! assembler rewrites only the metadata segments and the MPF index.
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use xdremux_codec::{JpegRasterDecodeRequest, RasterPixelFormat, ZuneJpegProvider};
use xdremux_engine::RasterDecoder;
use xdremux_format::dual_jpeg::{
    assemble_dual_layer_jpeg, index_dual_layer_jpeg, strip_assembly_owned_segments, AppSegment,
    DualLayerJpegAssembly, ISO_GAIN_MAP_APP2_IDENTIFIER,
};
use xdremux_format::scan_jpeg_image_at;
use xdremux_metadata::{iso_gain_map_base_app2_payload, parse_ultrahdr_gain_map_metadata};

fn fixture(path: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(path),
    )
    .unwrap()
}

fn decode(provider: &ZuneJpegProvider, data: &[u8], format: RasterPixelFormat) -> Vec<u8> {
    provider
        .decode_raster(&JpegRasterDecodeRequest {
            data: data.to_vec(),
            format,
        })
        .expect("JPEG must decode")
        .data
}

/// Repackage the public dual-layer JPEG fixture through the XDRemux assembler.
///
/// Returns the assembled file and the untouched fixture bytes.
fn repackage_fixture() -> (Vec<u8>, Vec<u8>) {
    let source = fixture("motion-photo/samsung/jpeg-ultrahdr-01.jpg");
    // The fixture appends a motion video after the two still images.
    let first = scan_jpeg_image_at(&source, 0).expect("fixture must start with a JPEG image");
    let second = scan_jpeg_image_at(&source, first.range.end)
        .expect("fixture must continue with the gain-map image");
    let raw_base = &source[first.range.clone()];
    let raw_gain = &source[second.range.clone()];
    let metadata = parse_ultrahdr_gain_map_metadata(raw_gain)
        .expect("fixture gain-map metadata must parse")
        .expect("fixture must carry ISO gain-map metadata");
    let parameters = metadata
        .to_iso_app2_payload(metadata.use_base_color_space)
        .expect("checked metadata must serialize as an APP2 payload");

    // Repackaging drops the carriers the assembler owns, then writes its own.
    let base_image = strip_assembly_owned_segments(raw_base).expect("primary image must strip");
    let gain_image = strip_assembly_owned_segments(raw_gain).expect("gain-map image must strip");

    let mut base_payload = ISO_GAIN_MAP_APP2_IDENTIFIER.to_vec();
    base_payload.extend_from_slice(&iso_gain_map_base_app2_payload());
    let mut gain_payload = ISO_GAIN_MAP_APP2_IDENTIFIER.to_vec();
    gain_payload.extend_from_slice(&parameters);

    let output = assemble_dual_layer_jpeg(&DualLayerJpegAssembly {
        base: &base_image,
        gain_map: &gain_image,
        base_segments: &[AppSegment {
            marker: 0xe2,
            payload: &base_payload,
        }],
        gain_segments: &[AppSegment {
            marker: 0xe2,
            payload: &gain_payload,
        }],
    })
    .expect("assembly must succeed");
    (output, source)
}

fn temporary_directory(context: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "xdremux-dual-jpeg-{context}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    directory
}

fn files_equal(first: &Path, second: &Path) -> std::io::Result<bool> {
    let mut a = fs::File::open(first)?;
    let mut b = fs::File::open(second)?;
    if a.metadata()?.len() != b.metadata()?.len() {
        return Ok(false);
    }
    let mut left = vec![0u8; 64 * 1024];
    let mut right = vec![0u8; 64 * 1024];
    loop {
        let read = a.read(&mut left)?;
        if read == 0 {
            return Ok(true);
        }
        b.read_exact(&mut right[..read])?;
        if left[..read] != right[..read] {
            return Ok(false);
        }
    }
}

#[test]
fn assembled_dual_layer_jpeg_keeps_both_images_decodable_and_its_iso_carriers() {
    let (output, source) = repackage_fixture();
    let first = scan_jpeg_image_at(&source, 0).unwrap();
    let second = scan_jpeg_image_at(&source, first.range.end).unwrap();
    let base_image = &source[first.range.clone()];
    let gain_image = &source[second.range.clone()];
    let metadata = parse_ultrahdr_gain_map_metadata(gain_image)
        .unwrap()
        .unwrap();

    let index = index_dual_layer_jpeg(&output).expect("assembled file must index consistently");
    assert!(index.base_has_iso_declaration);
    assert!(index.gain_map_has_iso_parameters);
    assert_eq!(index.mp.image_count, 2);
    assert_eq!(
        index.mpf_tiff_start + index.mp.entries[1].data_offset as usize,
        index.base.range.end,
        "the second MP entry must point at the real second Start Of Image"
    );

    // Both compressed images must decode, and copying them must not change a
    // single sample.
    let provider = ZuneJpegProvider::new();
    let base_samples = decode(&provider, base_image, RasterPixelFormat::Rgb8);
    let gain_samples = decode(&provider, gain_image, RasterPixelFormat::Mono8);
    let assembled_base = &output[index.base.range.clone()];
    let assembled_gain = &output[index.gain_map.range.clone()];
    assert_eq!(
        decode(&provider, assembled_base, RasterPixelFormat::Rgb8),
        base_samples
    );
    assert_eq!(
        decode(&provider, assembled_gain, RasterPixelFormat::Mono8),
        gain_samples
    );

    // The gain-map reader must find the same parameters in the new file.
    let observed = parse_ultrahdr_gain_map_metadata(assembled_gain)
        .expect("assembled metadata must parse")
        .expect("assembled file must carry ISO gain-map metadata");
    assert_eq!(observed, metadata);

    // The base image must stay a readable SDR JPEG on its own.
    assert_eq!(
        xdremux_format::probe_jpeg_frame_profile(assembled_base)
            .expect("base image must keep a frame header")
            .width,
        first.frame.width
    );
}

/// Run the installed independent ISO/JPEG gain-map implementation when it is
/// available. The check is skipped, with a message, when the tool is absent.
#[test]
fn an_independent_iso_gain_map_reader_accepts_the_assembled_file() {
    match Command::new("ultrahdr_app").arg("-h").arg("1").output() {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipped: ultrahdr_app is not installed");
            return;
        }
        Err(error) => panic!("cannot run ultrahdr_app: {error}"),
    }

    let (output, source) = repackage_fixture();
    let work = temporary_directory("ultrahdr");
    let assembled = work.join("assembled.jpg");
    let reference = work.join("reference.jpg");
    fs::write(&assembled, &output).unwrap();
    fs::write(&reference, &source).unwrap();

    let probe = |input: &Path| {
        let result = Command::new("ultrahdr_app")
            .args(["-m", "1", "-j"])
            .arg(input)
            .arg("-P")
            .output()
            .expect("probe must run");
        (
            result.status.success(),
            String::from_utf8_lossy(&result.stdout).into_owned(),
        )
    };
    let (ok, probe_text) = probe(&assembled);
    assert!(
        ok && probe_text.contains("Ultra HDR Image: Yes"),
        "independent reader rejected the assembled file: {probe_text}"
    );
    let (_, reference_text) = probe(&reference);
    let metadata_of = |text: &str| {
        text.lines()
            .skip_while(|line| !line.starts_with("GainMap Metadata:"))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        metadata_of(&probe_text),
        metadata_of(&reference_text),
        "independent reader must report the same gain-map parameters"
    );

    // Decode both files to linear RGBA half-float and compare every byte.
    let assembled_raw = work.join("assembled.raw");
    let reference_raw = work.join("reference.raw");
    for (input, raw) in [(&assembled, &assembled_raw), (&reference, &reference_raw)] {
        let decode = Command::new("ultrahdr_app")
            .args(["-m", "1", "-j"])
            .arg(input)
            .args(["-o", "0", "-O", "4", "-z"])
            .arg(raw)
            .output()
            .expect("decode must run");
        assert!(
            decode.status.success() && raw.is_file(),
            "independent decoder failed for {}: {}",
            input.display(),
            String::from_utf8_lossy(&decode.stderr)
        );
    }
    assert!(
        files_equal(&assembled_raw, &reference_raw).unwrap(),
        "independent decoder produced different linear output"
    );

    fs::remove_dir_all(&work).ok();
}

#[test]
fn assembly_refuses_a_source_that_already_has_an_mpf_index() {
    let source = fixture("motion-photo/samsung/jpeg-ultrahdr-01.jpg");
    let first = scan_jpeg_image_at(&source, 0).unwrap();
    let second = scan_jpeg_image_at(&source, first.range.end).unwrap();
    // The fixture's primary image already owns an MPF index and an ISO
    // declaration, so it cannot be a base input: the assembler would otherwise
    // write a second index.
    assert!(assemble_dual_layer_jpeg(&DualLayerJpegAssembly {
        base: &source[first.range.clone()],
        gain_map: &source[second.range.clone()],
        base_segments: &[],
        gain_segments: &[],
    })
    .is_err());
}
