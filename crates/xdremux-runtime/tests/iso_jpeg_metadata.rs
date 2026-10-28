//! Synthetic headers exercise the public conversion transaction. Real fixture
//! bytes are only read; the small JPEGs and mutated copies live in memory/tempdir.
use std::{fs, path::PathBuf};

use xdremux_format::{
    exif::read_item_payload,
    isobmff::{parse_meta_box, scan_top_level_boxes, ParsedMeta, META},
    jpeg_icc_profile, jpeg_image_end, FourCC,
};
use xdremux_heif::validate_gain_map_structure;
use xdremux_motion_photo::{Input, MotionPhoto, ParseOptions};
use xdremux_runtime::PortableRuntime;

const ISO: &[u8] = b"urn:iso:std:iso:ts:21496:-1\0";
const XMP: &[u8] = br#"<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:hdrgm="http://ns.adobe.com/hdr-gain-map/1.0/"><description hdrgm:GainMapMax="4" hdrgm:HDRCapacityMax="4"/></x:xmpmeta>"#;

fn fixture() -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/motion-photo/samsung/jpeg-ultrahdr-01.jpg"),
    )
    .unwrap()
}

fn app(jpeg: &mut Vec<u8>, marker: u8, bytes: &[u8]) {
    jpeg.extend_from_slice(&[0xff, marker]);
    jpeg.extend_from_slice(&u16::try_from(bytes.len() + 2).unwrap().to_be_bytes());
    jpeg.extend_from_slice(bytes);
}

fn small_jpeg() -> Vec<u8> {
    let mut jpeg = Vec::new();
    jpeg_encoder::Encoder::new(&mut jpeg, 100)
        .encode(&[128; 64 * 64 * 3], 64, 64, jpeg_encoder::ColorType::Rgb)
        .unwrap();
    jpeg
}

fn iso_payload() -> Vec<u8> {
    // Three equal channel records must remain three records in the HEIF.
    let mut payload = vec![0, 0, 0, 0, 0x80];
    for value in [0_i32, 1, 2, 1]
        .into_iter()
        .chain([0, 1, 1, 1, 1, 1, -1, 8, 1, 16].repeat(3))
    {
        payload.extend_from_slice(&value.to_be_bytes());
    }
    payload
}

fn gain_jpeg(iso: Option<&[u8]>, xmp: &[u8], alternate: Option<&[u8]>) -> Vec<u8> {
    let jpeg = small_jpeg();
    let mut gain = jpeg[..2].to_vec();
    app(
        &mut gain,
        0xe1,
        &[b"http://ns.adobe.com/xap/1.0/\0", xmp].concat(),
    );
    if let Some(payload) = iso {
        app(&mut gain, 0xe2, &[ISO, payload].concat());
    }
    if let Some(icc) = alternate {
        app(&mut gain, 0xe2, &[b"ICC_PROFILE\0\x01\x01", icc].concat());
    }
    gain.extend_from_slice(&jpeg[2..]);
    gain
}

fn motion_source(gain: &[u8], mapped: bool) -> Vec<u8> {
    let source = fixture();
    let photo = MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
    let movie = photo.motion_video_bytes().unwrap();
    let gain_entry = if mapped {
        format!(
            r#"<rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="image/jpeg" Item:Semantic="GainMap" Item:Length="{}"/></rdf:li>"#,
            gain.len()
        )
    } else {
        String::new()
    };
    let xmp = format!(
        r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/" xmlns:Container="http://ns.google.com/photos/1.0/container/" xmlns:Item="http://ns.google.com/photos/1.0/container/item/" Camera:MotionPhoto="1" Camera:MotionPhotoVersion="1" Camera:MotionPhotoPresentationTimestampUs="1000000"><Container:Directory><rdf:Seq><rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="image/jpeg" Item:Semantic="Primary" Item:Length="0" Item:Padding="0"/></rdf:li>{gain_entry}<rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="video/mp4" Item:Semantic="MotionPhoto" Item:Length="{}" Item:Padding="0"/></rdf:li></rdf:Seq></Container:Directory></rdf:Description></rdf:RDF></x:xmpmeta>"#,
        movie.len()
    );
    let jpeg = small_jpeg();
    let mut result = jpeg[..2].to_vec();
    app(
        &mut result,
        0xe1,
        &[b"http://ns.adobe.com/xap/1.0/\0", xmp.as_bytes()].concat(),
    );
    // A baseline ISO APP2 carries only the version, never gain parameters.
    app(&mut result, 0xe2, &[ISO, &[0; 4]].concat());
    // Exif ColorSpace=1 must win over the deliberately different base ICC.
    let exif = [
        b"Exif\0\0".as_slice(),
        &[
            b'I', b'I', 42, 0, 8, 0, 0, 0, 1, 0, 0x69, 0x87, 4, 0, 1, 0, 0, 0, 26, 0, 0, 0, 0, 0,
            0, 0, 1, 0, 1, 0xa0, 3, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0,
        ],
    ]
    .concat();
    app(&mut result, 0xe1, &exif);
    app(
        &mut result,
        0xe2,
        &[
            b"ICC_PROFILE\0\x01\x01".as_slice(),
            &jpeg_icc_profile(&source).unwrap().unwrap(),
        ]
        .concat(),
    );
    result.extend_from_slice(&jpeg[2..]);
    result.extend_from_slice(gain);
    result.extend_from_slice(movie);
    result
}

fn colr<'a>(bytes: &'a [u8], meta: &ParsedMeta, item: u32) -> &'a [u8] {
    let associations = &meta
        .ipma
        .entries
        .iter()
        .find(|e| e.item_id == item)
        .unwrap()
        .associations;
    let property = associations
        .iter()
        .find_map(|a| {
            meta.properties
                .iter()
                .find(|p| p.index == u32::from(a.property_index) && p.kind == FourCC::new(*b"colr"))
        })
        .unwrap();
    &bytes[property.box_range.start + 8..property.box_range.end]
}

#[test]
fn selected_iso_or_xmp_controls_the_published_metadata_and_color_properties() {
    let iso = iso_payload();
    let source_icc = jpeg_icc_profile(&fixture()).unwrap().unwrap();
    for (payload, xmp) in [
        (Some(iso.as_slice()), XMP),
        (Some(iso.as_slice()), b"<x:xmpmeta malformed".as_slice()),
        (None, XMP),
    ] {
        // XMP-only ignores even a damaged gain ICC. ISO with no alternate ICC
        // applies gains in base primaries despite the source flag being zero.
        let gain = gain_jpeg(payload, xmp, payload.is_none().then_some(b"broken ICC"));
        let source = motion_source(&gain, true);
        let work = tempfile::tempdir().unwrap();
        let input = work.path().join("source.jpg");
        fs::write(&input, &source).unwrap();
        let receipt = PortableRuntime::new()
            .convert_motion_photo_file(&source, &input, work.path().join("capture.heic"))
            .unwrap();
        assert!(receipt.source_had_gain_map);
        let output = fs::read(receipt.image).unwrap();
        let structure = validate_gain_map_structure(&output).unwrap();
        let top = scan_top_level_boxes(&output).unwrap();
        let meta =
            parse_meta_box(&output, top.boxes.iter().find(|b| b.kind == META).unwrap()).unwrap();
        let location = meta
            .iloc
            .entries
            .iter()
            .find(|e| e.item_id == structure.tmap_item_id)
            .unwrap();
        let tmap = read_item_payload(&output, location, meta.idat.as_ref()).unwrap();
        let is_iso = payload.is_some();
        assert_eq!(tmap[5], if is_iso { 0xc0 } else { 0x40 });
        assert_eq!(tmap.len(), if is_iso { 142 } else { 62 });
        // Read output rationals independently of the product metadata writer.
        let rational = |offset| {
            f64::from(i32::from_be_bytes(
                tmap[offset..offset + 4].try_into().unwrap(),
            )) / f64::from(u32::from_be_bytes(
                tmap[offset + 4..offset + 8].try_into().unwrap(),
            ))
        };
        assert_eq!(rational(14), if is_iso { 2.0 } else { 4.0 });
        assert_eq!(rational(30), if is_iso { 1.0 } else { 4.0 });
        if is_iso {
            for c in 0..3 {
                assert_eq!(rational(46 + 40 * c), -0.125);
                assert_eq!(rational(54 + 40 * c), 0.0625);
            }
        }
        let base_colr = colr(&output, &meta, structure.primary_item_id);
        assert_eq!(&base_colr[..4], b"prof");
        if is_iso {
            assert_eq!(
                &base_colr[4..],
                include_bytes!("../../xdremux-codec/src/portable/srgb.icc")
            );
        } else {
            assert_eq!(&base_colr[4..], source_icc);
        }
        assert_eq!(
            colr(&output, &meta, structure.gain_map_item_id),
            b"nclx\0\x02\0\x02\0\x06\x80"
        );
        assert_eq!(
            colr(&output, &meta, structure.tmap_item_id),
            b"nclx\0\x09\0\x10\0\x09\x80"
        );
        assert_eq!(fs::read(input).unwrap(), source);
    }
}

#[test]
fn invalid_iso_with_valid_xmp_fails_before_publication_even_without_a_gain_map_mapping() {
    let valid = iso_payload();
    let mut unknown = valid.clone();
    unknown[1] = 1;
    let mut reserved = valid.clone();
    reserved[4] |= 1;
    let mut inverted = valid.clone();
    inverted[21..25].copy_from_slice(&2_i32.to_be_bytes());
    let mut equal_headrooms = valid.clone();
    equal_headrooms[13..17].fill(0);
    for payload in [
        &valid[..4],
        &unknown,
        &reserved,
        &inverted,
        &equal_headrooms,
    ] {
        for mapped in [true, false] {
            let source = motion_source(&gain_jpeg(Some(payload), XMP, None), mapped);
            let work = tempfile::tempdir().unwrap();
            let input = work.path().join("source.jpg");
            let output = work.path().join("capture.heic");
            fs::write(&input, &source).unwrap();
            let error = PortableRuntime::new()
                .convert_motion_photo_file(&source, &input, &output)
                .expect_err("invalid ISO cannot be published through XMP or SDR fallback");
            assert!(error.to_string().contains("ISO 21496-1"), "{error}");
            assert!(!output.exists());
            assert!(!output.with_extension("mov").exists());
            assert_eq!(fs::read(input).unwrap(), source);
            fs::write(&output, b"existing still").unwrap();
            fs::write(output.with_extension("mov"), b"existing movie").unwrap();
            assert!(PortableRuntime::new()
                .convert_motion_photo_file(&source, work.path().join("source.jpg"), &output)
                .is_err());
            assert_eq!(fs::read(&output).unwrap(), b"existing still");
            assert_eq!(
                fs::read(output.with_extension("mov")).unwrap(),
                b"existing movie"
            );
        }
    }
}

#[test]
fn samsung_dual_metadata_fixtures_select_iso_on_the_located_gain_jpeg() {
    for number in ["01", "02"] {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../fixtures/motion-photo/samsung/jpeg-ultrahdr-{number}.jpg"
        ));
        let source = fs::read(path).unwrap();
        let photo =
            MotionPhoto::parse(Input::SingleFile(&source), ParseOptions::compatible()).unwrap();
        let still = photo.still_bytes().unwrap();
        let start = jpeg_image_end(still, 0).unwrap();
        let end = jpeg_image_end(still, start).unwrap();
        let metadata = xdremux_metadata::parse_ultrahdr_gain_map_metadata(&still[start..end])
            .unwrap()
            .unwrap();
        assert!(metadata.uses_iso_colorimetry);
        assert!(metadata.use_base_color_space);
        // This exact rational differs from XMP's rounded decimal 2.300000.
        assert_eq!(metadata.gain_map_max, [2.299999952316284; 3]);
    }
}

#[test]
fn damaged_iso_alternate_icc_cannot_be_published_as_a_profile_or_replaced_with_a_guessed_gamut() {
    let iso = iso_payload();
    for icc in [
        b"broken ICC".as_slice(),
        &jpeg_icc_profile(&fixture()).unwrap().unwrap(),
    ] {
        let source = motion_source(&gain_jpeg(Some(&iso), XMP, Some(icc)), true);
        let work = tempfile::tempdir().unwrap();
        let input = work.path().join("source.jpg");
        let output = work.path().join("capture.heic");
        fs::write(&input, &source).unwrap();
        let error = PortableRuntime::new()
            .convert_motion_photo_file(&source, &input, &output)
            .expect_err("a present damaged ICC or HDR alternate ICC without CICP must fail");
        assert!(error.to_string().contains("alternate ICC"), "{error}");
        assert!(!output.exists());
        assert!(!output.with_extension("mov").exists());
        assert_eq!(fs::read(input).unwrap(), source);
    }
}

#[test]
fn iso_cicp_admission_profile_reaches_tmap_without_changing_gain_compression() {
    // A serialized admission-test header with a single CICP tag. It has no
    // transform tables and establishes byte routing, not native ICC validity.
    let mut profile = vec![0; 156];
    profile[..4].copy_from_slice(&156_u32.to_be_bytes());
    profile[8..10].copy_from_slice(&[4, 0x40]);
    profile[12..24].copy_from_slice(b"mntrRGB XYZ ");
    profile[36..40].copy_from_slice(b"acsp");
    profile[128..144].copy_from_slice(&[
        0, 0, 0, 1, b'c', b'i', b'c', b'p', 0, 0, 0, 144, 0, 0, 0, 12,
    ]);
    profile[144..].copy_from_slice(b"cicp\0\0\0\0\x09\x10\0\x01");
    let source = motion_source(&gain_jpeg(Some(&iso_payload()), XMP, Some(&profile)), true);
    let work = tempfile::tempdir().unwrap();
    let input = work.path().join("source.jpg");
    fs::write(&input, &source).unwrap();
    let receipt = PortableRuntime::new()
        .convert_motion_photo_file(&source, &input, work.path().join("capture.heic"))
        .unwrap();
    let output = fs::read(receipt.image).unwrap();
    let structure = validate_gain_map_structure(&output).unwrap();
    let top = scan_top_level_boxes(&output).unwrap();
    let meta = parse_meta_box(&output, top.boxes.iter().find(|b| b.kind == META).unwrap()).unwrap();
    assert_eq!(
        colr(&output, &meta, structure.tmap_item_id),
        [b"prof".as_slice(), &profile].concat()
    );
    assert_eq!(
        colr(&output, &meta, structure.gain_map_item_id),
        b"nclx\0\x02\0\x02\0\x06\x80"
    );
    let location = meta
        .iloc
        .entries
        .iter()
        .find(|e| e.item_id == structure.tmap_item_id)
        .unwrap();
    let payload = read_item_payload(&output, location, meta.idat.as_ref()).unwrap();
    assert_eq!(
        payload[5], 0x80,
        "alternate ICC preserves the zero base-space flag"
    );
    assert_eq!(fs::read(input).unwrap(), source);
}
