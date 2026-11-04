//! Exercise explicit HDR profiles through the conversion transaction and decode
//! the final bytes. Fixtures and any previously published output stay intact.
use std::{fs, path::PathBuf};

use xdremux_codec::{LibHeifProvider, PrimaryHeifEncodeRequest, Raster8, RasterPixelFormat};
use xdremux_engine::{ConversionRequest, HdrExportProfile};
use xdremux_format::{
    exif::read_item_payload,
    isobmff::{parse_meta_box, scan_top_level_boxes, META},
    FourCC,
};
use xdremux_heif::{validate_gain_map_structure, validate_hdr_gain_map_profile};
use xdremux_runtime::{reconstruct_hdr_gain_map, HdrTransfer, PortableRuntime};

fn fixture(path: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(path),
    )
    .unwrap()
}

fn body(source: &[u8]) -> &[u8] {
    let boundary = xdremux_container::extract(source)
        .unwrap()
        .manifest_info
        .extension_start;
    &source[..boundary]
}

fn primary_resources(source: &[u8]) -> (Vec<Vec<u8>>, std::collections::BTreeSet<Vec<u8>>) {
    let top = scan_top_level_boxes(source).unwrap();
    let meta = parse_meta_box(source, top.boxes.iter().find(|b| b.kind == META).unwrap()).unwrap();
    let mut ids = vec![meta.primary_item_id];
    let mut cursor = 0;
    while cursor < ids.len() {
        let id = ids[cursor];
        cursor += 1;
        if let Some(iref) = &meta.iref {
            for r in iref
                .entries
                .iter()
                .filter(|r| r.kind == FourCC::new(*b"dimg") && r.from_item_id == id)
            {
                ids.extend(&r.to_item_ids);
            }
        }
    }
    let mut properties = std::collections::BTreeSet::new();
    for entry in meta
        .ipma
        .entries
        .iter()
        .filter(|e| ids.contains(&e.item_id))
    {
        for association in &entry.associations {
            let property = meta
                .properties
                .iter()
                .find(|p| p.index == u32::from(association.property_index))
                .unwrap();
            if matches!(
                property.kind.as_bytes(),
                b"colr" | b"hvcC" | b"ispe" | b"pixi"
            ) {
                properties.insert(source[property.box_range.clone()].to_vec());
            }
        }
    }
    let payloads = ids
        .into_iter()
        .map(|id| {
            read_item_payload(
                source,
                meta.iloc.entries.iter().find(|e| e.item_id == id).unwrap(),
                meta.idat.as_ref(),
            )
            .unwrap()
        })
        .collect();
    (payloads, properties)
}

fn synthetic_source() -> Vec<u8> {
    let raster =
        Raster8::new(64, 64, 192, RasterPixelFormat::Rgb8, vec![128; 64 * 64 * 3]).unwrap();
    let mut source = LibHeifProvider::new()
        .encode_primary_heif(&PrimaryHeifEncodeRequest::live_photo(raster, None))
        .unwrap();
    let info_start = source.len();
    // SDR base, alternate +3 stops, min gain -1, gamma 2, nonzero offsets.
    let info = [
        0.5_f32,
        0.5,
        0.5,
        1.0,
        8.0,
        8.0,
        8.0,
        2.0,
        2.0,
        2.0,
        1.0 / 64.0,
        1.0 / 64.0,
        1.0 / 64.0,
        1.0 / 64.0,
        1.0 / 64.0,
        1.0 / 64.0,
        1.0,
        8.0,
        8.0,
        0.0,
    ];
    for v in info {
        source.extend_from_slice(&v.to_le_bytes());
    }
    let mut gain = Vec::new();
    jpeg_encoder::Encoder::new(&mut gain, 100)
        .encode(&[255; 16 * 16], 16, 16, jpeg_encoder::ColorType::Luma)
        .unwrap();
    let gain_start = source.len();
    source.extend_from_slice(&gain);
    let json_start = source.len();
    let manifest=format!("[{{\"name\":\"local.uhdr.gainmap.info\",\"offset\":{},\"length\":80}},{{\"name\":\"local.uhdr.gainmap.data\",\"offset\":{},\"length\":{}}}]",json_start-info_start,json_start-gain_start,gain.len());
    source.extend_from_slice(manifest.as_bytes());
    source.extend_from_slice(b"\0jxrs");
    source.extend_from_slice(&u32::try_from(manifest.len() + 9).unwrap().to_le_bytes());
    source
}

#[test]
fn final_file_reconstruction_preserves_sdr_and_matches_known_hdr() {
    let source = synthetic_source();
    let runtime = PortableRuntime::new();
    let original = primary_resources(body(&source));
    for (profile, transfer) in [
        (HdrExportProfile::Bt2020Pq, HdrTransfer::Pq),
        (HdrExportProfile::Bt2020Hlg, HdrTransfer::Hlg),
    ] {
        let output = runtime
            .convert_proxdr_bytes(
                &source,
                ConversionRequest {
                    hdr_profile: profile,
                    ..Default::default()
                },
                |_| {},
            )
            .unwrap()
            .bytes;
        assert_eq!(primary_resources(&output), original);
        let observed = validate_hdr_gain_map_profile(&output, transfer).unwrap();
        assert_eq!(
            observed.base_color_space,
            xdremux_metadata::SdrColorSpace::Srgb
        );
        assert!(observed.static_metadata.mastering_display.is_none());
        let hdr = reconstruct_hdr_gain_map(&output, transfer).unwrap();
        assert_eq!((hdr.width, hdr.height), (64, 64));
        // Read both encoded images back independently. Compare the full RGB
        // transform with a matrix independently derived from chromaticities.
        let base = LibHeifProvider::new()
            .decode_coded_item_normalized(&output, observed.semantics.structure.primary_item_id, 3)
            .unwrap();
        let gain = LibHeifProvider::new()
            .decode_coded_item_normalized(
                &output,
                observed.semantics.structure.gain_map_item_id,
                observed.semantics.structure.channel_count,
            )
            .unwrap();
        let gain_code = gain.samples[0];
        assert!(gain.samples.iter().all(|v| *v == gain_code));
        assert!(
            (gain_code - 1.0).abs() < 2.0 / 255.0,
            "encoded white Gain Map: {gain_code}"
        );
        let decoded_gain = 2.0_f64.powf(-1.0 + 4.0 * f64::from(gain_code).sqrt());
        let matrix = [
            [0.6274038959346991, 0.3292830383778837, 0.04331306568741723],
            [0.06909728935823208, 0.9195403950754587, 0.01136231556630918],
            [0.01639143887515028, 0.08801330787722576, 0.895595253247624],
        ];
        for (actual, base) in hdr
            .samples
            .as_chunks::<3>()
            .0
            .iter()
            .zip(base.samples.as_chunks::<3>().0)
        {
            let linear = base
                .iter()
                .map(|v| {
                    let code = f64::from(*v);
                    let value = if code <= 0.04045 {
                        code / 12.92
                    } else {
                        ((code + 0.055) / 1.055).powf(2.4)
                    };
                    (value + 1.0 / 64.0) * decoded_gain - 1.0 / 64.0
                })
                .collect::<Vec<_>>();
            for (actual, row) in actual.iter().zip(matrix) {
                let expected = row
                    .into_iter()
                    .zip(&linear)
                    .map(|(a, b)| a * b)
                    .sum::<f64>();
                assert!(
                    (f64::from(*actual) - expected).abs() < 3e-5,
                    "{actual} vs {expected}; base={base:?}, gain={gain_code}"
                );
            }
        }
        assert!(validate_hdr_gain_map_profile(
            &output,
            if transfer == HdrTransfer::Pq {
                HdrTransfer::Hlg
            } else {
                HdrTransfer::Pq
            }
        )
        .is_err());
    }
}

#[test]
fn real_p3_sources_keep_compressed_base_and_consistent_orientation() {
    for path in [
        "proxdr/oppo/find-x6-pro/lhdr-v1-01.heic",
        "proxdr/oppo/find-x9-ultra/uhdr-hr-01.heic",
    ] {
        let source = fixture(path);
        let original = primary_resources(body(&source));
        let output = PortableRuntime::new()
            .convert_proxdr_bytes(
                &source,
                ConversionRequest {
                    hdr_profile: HdrExportProfile::Bt2020Pq,
                    ..Default::default()
                },
                |_| {},
            )
            .unwrap()
            .bytes;
        let report = validate_hdr_gain_map_profile(&output, HdrTransfer::Pq).unwrap();
        assert_eq!(primary_resources(&output), original, "{path}");
        assert!(matches!(
            report.base_color_space,
            xdremux_metadata::SdrColorSpace::DisplayP3 | xdremux_metadata::SdrColorSpace::Srgb
        ));
        assert!(reconstruct_hdr_gain_map(&output, HdrTransfer::Pq)
            .unwrap()
            .samples
            .iter()
            .all(|v| v.is_finite()));
    }
}

#[test]
fn invalid_profile_source_preserves_existing_output_and_input() {
    let mut source = synthetic_source();
    // Make all embedded ICC profile signatures invalid. The ordinary parser
    // can still extract the private Gain Map, but profile admission must fail.
    let positions = source
        .windows(4)
        .enumerate()
        .filter(|(_, b)| *b == b"acsp")
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    assert!(!positions.is_empty());
    for i in positions {
        source[i..i + 4].copy_from_slice(b"bad!");
    }
    let before = source.clone();
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("existing.heic");
    fs::write(&output, b"previous output").unwrap();
    let result = PortableRuntime::new().convert_proxdr_file(
        &source,
        &output,
        ConversionRequest {
            hdr_profile: HdrExportProfile::Bt2020Pq,
            ..Default::default()
        },
        |_| {},
    );
    assert!(result.is_err());
    assert_eq!(source, before);
    assert_eq!(fs::read(&output).unwrap(), b"previous output");
}

#[test]
fn default_conversion_retains_its_existing_layout() {
    let source = synthetic_source();
    let runtime = PortableRuntime::new();
    let a = runtime
        .convert_proxdr_bytes(&source, ConversionRequest::default(), |_| {})
        .unwrap();
    let b = runtime
        .convert_proxdr_bytes(
            &source,
            ConversionRequest {
                hdr_profile: HdrExportProfile::Compatible,
                ..Default::default()
            },
            |_| {},
        )
        .unwrap();
    assert_eq!(a.bytes, b.bytes);
    validate_gain_map_structure(&a.bytes).unwrap();
}
