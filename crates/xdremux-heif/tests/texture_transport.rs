use xdremux_format::isobmff::{
    make_box, make_full_box, make_iinf_box, make_iloc_box, make_infe_box, make_ipma_box,
    make_iref_box, make_pitm_box, IlocEntry, IlocExtent, IpmaAssociation, IpmaEntry, IrefEntry,
    IDAT, IPRP, MDAT, META,
};
use xdremux_format::FourCC;
use xdremux_heif::{
    augment_texture_style_heif, inspect_texture_style_heif, preserve_texture_style_heif,
};

const TEXTURE: &[u8] = b"tag:apple.com,2026:photo:metadata:texture_styles";

fn uri_infe(id: u16, name: &[u8], uri: &[u8]) -> Vec<u8> {
    let mut p = id.to_be_bytes().to_vec();
    p.extend_from_slice(&[0, 0]);
    p.extend_from_slice(b"uri ");
    p.extend_from_slice(name);
    p.push(0);
    p.extend_from_slice(uri);
    p.push(0);
    make_full_box(FourCC::new(*b"infe"), 2, 1, &p).unwrap()
}

// A transport fixture, not a photograph or an Apple-native positive image.
// It also meets the old probe's input preconditions, so the generation
// rejection check catches the original fixed-MakerNote/12-role fabrication.
fn fixture(
    texture_uri: Option<&[u8]>,
    texture_name: &[u8],
    escape_idat: bool,
    target: u32,
) -> Vec<u8> {
    let ftyp = make_box(FourCC::new(*b"ftyp"), b"heic\0\0\0\0mif1heic").unwrap();
    let mut items = vec![
        make_infe_box(1, FourCC::new(*b"hvc1"), 0).unwrap(),
        make_infe_box(2, FourCC::new(*b"tmap"), 0).unwrap(),
        make_infe_box(3, FourCC::new(*b"hvc1"), 0).unwrap(),
        make_infe_box(4, FourCC::new(*b"mime"), 0).unwrap(),
        uri_infe(5, b"metadata", b"tag:apple.com,2023:photo:metadata:styles"),
        make_infe_box(6, FourCC::new(*b"Exif"), 0).unwrap(),
    ];
    if let Some(uri) = texture_uri {
        items.push(uri_infe(7, texture_name, uri));
    }
    let payloads: Vec<&[u8]> = vec![
        b"compressed-primary-sentinel",
        b"gain-map-sentinel",
        b"own-skin-matte",
        b"<rdf>own-descriptor</rdf>",
        b"bplist00opaque-styles",
        b"\0\0\0\0II\x2a\0\x08\0\0\0\0\0\0\0\0\0",
        b"bplist00opaque-texture",
    ];
    let mut idat = Vec::new();
    let mut locations = Vec::new();
    for (i, bytes) in payloads.iter().take(items.len()).enumerate() {
        let offset = idat.len();
        idat.extend_from_slice(bytes);
        locations.push(IlocEntry {
            item_id: (i + 1) as u32,
            construction_method: 1,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset: offset as u64,
                length: bytes.len() as u64,
            }],
        });
    }
    if escape_idat {
        locations.last_mut().unwrap().extents[0].length += 10;
    }
    let mut properties = make_full_box(
        FourCC::new(*b"auxC"),
        0,
        0,
        b"urn:com:apple:photo:2019:aux:semanticskinmatte\0",
    )
    .unwrap();
    properties.extend(make_box(FourCC::new(*b"irot"), &[3]).unwrap());
    properties.extend(make_box(FourCC::new(*b"colr"), b"profkeep-ICC-exact").unwrap());
    properties.extend(make_box(FourCC::new(*b"imir"), &[1]).unwrap());
    let mut iprp = make_box(FourCC::new(*b"ipco"), &properties).unwrap();
    iprp.extend(
        make_ipma_box(
            0,
            0,
            &[
                IpmaEntry {
                    item_id: 3,
                    associations: vec![IpmaAssociation {
                        essential: false,
                        property_index: 1,
                    }],
                },
                IpmaEntry {
                    item_id: 1,
                    associations: vec![2, 3, 4]
                        .into_iter()
                        .map(|property_index| IpmaAssociation {
                            essential: false,
                            property_index,
                        })
                        .collect(),
                },
            ],
        )
        .unwrap(),
    );
    let mut refs = vec![
        IrefEntry {
            kind: FourCC::new(*b"cdsc"),
            from_item_id: 5,
            to_item_ids: vec![1, 2],
        },
        IrefEntry {
            kind: FourCC::new(*b"cdsc"),
            from_item_id: 4,
            to_item_ids: vec![3],
        },
        IrefEntry {
            kind: FourCC::new(*b"auxl"),
            from_item_id: 3,
            to_item_ids: vec![1],
        },
    ];
    if texture_uri.is_some() {
        refs.push(IrefEntry {
            kind: FourCC::new(*b"cdsc"),
            from_item_id: 7,
            to_item_ids: vec![target],
        });
    }
    let mut meta = make_pitm_box(0, 1).unwrap();
    meta.extend(make_iinf_box(0, &items).unwrap());
    meta.extend(make_iloc_box(1, 4, 4, 0, 0, &locations).unwrap());
    meta.extend(make_box(IPRP, &iprp).unwrap());
    meta.extend(make_iref_box(0, &refs).unwrap());
    meta.extend(make_box(IDAT, &idat).unwrap());
    let mut out = ftyp;
    out.extend(make_full_box(META, 0, 0, &meta).unwrap());
    out.extend(make_box(MDAT, b"compressed-media-tail-do-not-touch").unwrap());
    out.extend(make_box(FourCC::new(*b"uuid"), b"unrelated-metadata-exact").unwrap());
    out.extend(b"TAIL");
    out
}

#[test]
fn missing_native_producer_rejects_legacy_fabrication() {
    let source = fixture(None, b"metadata", false, 1);
    let error = augment_texture_style_heif(&source).unwrap_err().to_string();
    assert!(error.contains("generation unsupported"), "{error}");
    assert!(!source.windows(9).any(|w| w == b"Apple iOS"));
}

#[test]
fn opaque_transport_preserves_media_gain_color_orientation_and_unknown_bytes() {
    let source = fixture(Some(TEXTURE), b"metadata", false, 1);
    let facts = inspect_texture_style_heif(&source).unwrap().unwrap();
    assert_eq!(facts.primary_item_id, 1);
    assert_eq!(facts.metadata_item_id, 7);
    assert!(facts.metadata_has_binary_plist_header);
    // There are no invented 2026 roles. The opaque marker is not a valid
    // plist; transport deliberately makes no native parser-success claim.
    assert!(facts.auxiliary_type_claims.is_empty());
    assert_eq!(preserve_texture_style_heif(&source).unwrap(), source);
    assert_eq!(augment_texture_style_heif(&source).unwrap(), source);
}

#[test]
fn texture_uri_in_the_item_name_does_not_authorize_transport() {
    let source = fixture(Some(b"tag:example.org,2026:unrelated"), TEXTURE, false, 1);
    assert!(inspect_texture_style_heif(&source).unwrap().is_none());
    assert!(preserve_texture_style_heif(&source).is_err());
}

#[test]
fn idat_extent_cannot_escape_into_the_following_media_box() {
    let source = fixture(Some(TEXTURE), b"metadata", true, 1);
    assert!(preserve_texture_style_heif(&source)
        .unwrap_err()
        .to_string()
        .contains("escapes"));
}

#[test]
fn dangling_or_wrong_image_metadata_references_fail_closed() {
    for target in [99, 3] {
        let source = fixture(Some(TEXTURE), b"metadata", false, target);
        assert!(preserve_texture_style_heif(&source).is_err());
    }
}

#[test]
fn jpeg_and_truncated_inputs_cannot_trigger_makernote_synthesis() {
    for source in [&b"\xff\xd8\xff\xd9"[..], &b"bplist00"[..], &b""[..]] {
        assert!(augment_texture_style_heif(source).is_err());
    }
}
