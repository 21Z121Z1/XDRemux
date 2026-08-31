use super::*;
use crate::fourcc::FourCC;

fn header_for(data: &[u8]) -> BoxHeader {
    parse_boxes(data, 0..data.len()).unwrap().remove(0)
}

#[test]
fn box_parser_accepts_normal_size_zero_and_largesize() {
    let normal = make_box(FourCC::new(*b"free"), &[1, 2, 3, 4]).unwrap();
    let parsed = parse_boxes(&normal, 0..normal.len()).unwrap();
    assert_eq!(parsed[0].size, 12);
    assert_eq!(parsed[0].data_start, 8);

    let mut to_end = Vec::new();
    to_end.extend_from_slice(&0u32.to_be_bytes());
    to_end.extend_from_slice(b"free");
    to_end.extend_from_slice(&[1, 2, 3]);
    assert_eq!(
        parse_boxes(&to_end, 0..to_end.len()).unwrap()[0].size,
        to_end.len()
    );

    let mut large = Vec::new();
    large.extend_from_slice(&1u32.to_be_bytes());
    large.extend_from_slice(b"free");
    large.extend_from_slice(&19u64.to_be_bytes());
    large.extend_from_slice(&[1, 2, 3]);
    assert_eq!(
        parse_boxes(&large, 0..large.len()).unwrap()[0].data_start,
        16
    );
}

#[test]
fn zero_largesize_and_truncated_boxes_are_rejected() {
    let mut zero_large = Vec::new();
    zero_large.extend_from_slice(&1u32.to_be_bytes());
    zero_large.extend_from_slice(b"free");
    zero_large.extend_from_slice(&0u64.to_be_bytes());
    assert!(parse_boxes(&zero_large, 0..zero_large.len()).is_err());

    let truncated = [0, 0, 0, 20, b'f', b'r', b'e', b'e'];
    assert!(parse_boxes(&truncated, 0..truncated.len()).is_err());
    assert!(parse_boxes(&truncated[..4], 0..4).is_err());
}

#[test]
fn top_level_scan_preserves_oppo_tail_only_after_mdat() {
    let ftyp = make_box(FTYP, b"heic\0\0\0\0").unwrap();
    let mdat = make_box(MDAT, &[1, 2, 3]).unwrap();
    let mut file = [ftyp, mdat].concat();
    let tail_start = file.len();
    file.extend_from_slice(b"not-an-isobmff-box");
    let scan = scan_top_level_boxes(&file).unwrap();
    assert_eq!(scan.boxes.len(), 2);
    assert_eq!(scan.trailing_range, tail_start..file.len());

    assert!(scan_top_level_boxes(b"not-a-box").is_err());
}

#[test]
fn iloc_constructor_round_trips_and_resolves_base_offset() {
    let source = IlocEntry {
        item_id: 7,
        construction_method: 0,
        data_reference_index: 0,
        base_offset: 100,
        extents: vec![IlocExtent {
            index: Some(2),
            offset: 20,
            length: 30,
        }],
    };
    let bytes = make_iloc_box(1, 4, 4, 4, 2, std::slice::from_ref(&source)).unwrap();
    let header = header_for(&bytes);
    let parsed = parse_iloc(&bytes, &header).unwrap();
    assert_eq!(parsed.entries, vec![source]);
    assert_eq!(
        parsed.entries[0]
            .resolved_extent_offset(&parsed.entries[0].extents[0])
            .unwrap(),
        120
    );
}

#[test]
fn malformed_iloc_counts_and_prefix_truncations_fail_without_panicking() {
    let payload = vec![0, 0, 0, 0, 0x44, 0x40, 0xff, 0xff];
    let bytes = make_box(ILOC, &payload).unwrap();
    let header = header_for(&bytes);
    assert!(parse_iloc(&bytes, &header).is_err());

    let validish = make_box(ILOC, &[0, 0, 0, 0, 0x44, 0x40, 0, 1]).unwrap();
    for length in 0..validish.len() {
        let prefix = &validish[..length];
        let _ = parse_boxes(prefix, 0..prefix.len());
    }
}

#[test]
fn truncated_infe_ipma_and_pitm_are_rejected() {
    let truncated_infe = make_full_box(INFE, 3, 0, &[]).unwrap();
    let iinf = make_iinf_box(0, &[truncated_infe]).unwrap();
    assert!(parse_iinf(&iinf, &header_for(&iinf)).is_err());

    let ipma = make_full_box(IPMA, 0, 0, &[0, 0, 0, 2]).unwrap();
    assert!(parse_ipma(&ipma, &header_for(&ipma)).is_err());

    let pitm = make_full_box(PITM, 0, 0, &[]).unwrap();
    assert!(parse_pitm(&pitm, &header_for(&pitm)).is_err());
}

#[test]
fn infe_ipma_and_iref_constructors_round_trip() {
    let infe = make_infe_box(5, FourCC::new(*b"hvc1"), 0).unwrap();
    let iinf = make_iinf_box(0, &[infe]).unwrap();
    let parsed_iinf = parse_iinf(&iinf, &header_for(&iinf)).unwrap();
    assert_eq!(parsed_iinf.entries[0].item_id, 5);
    assert_eq!(
        parsed_iinf.entries[0].item_type,
        Some(FourCC::new(*b"hvc1"))
    );

    let ipma_entry = IpmaEntry {
        item_id: 5,
        associations: vec![IpmaAssociation {
            property_index: 3,
            essential: true,
        }],
    };
    let ipma = make_ipma_box(0, 0, std::slice::from_ref(&ipma_entry)).unwrap();
    let parsed_ipma = parse_ipma(&ipma, &header_for(&ipma)).unwrap();
    assert_eq!(parsed_ipma.entries, vec![ipma_entry]);

    let iref_entry = IrefEntry {
        kind: FourCC::new(*b"auxl"),
        from_item_id: 5,
        to_item_ids: vec![1],
    };
    let iref = make_iref_box(0, std::slice::from_ref(&iref_entry)).unwrap();
    let parsed_iref = parse_iref(&iref, &header_for(&iref)).unwrap();
    assert_eq!(parsed_iref.entries, vec![iref_entry]);
}

#[test]
fn ispe_and_irot_round_trip_without_string_conversions() {
    let ispe = make_ispe_box(4080, 3064).unwrap();
    assert_eq!(
        parse_ispe_dimensions(&ispe, &header_for(&ispe)).unwrap(),
        (4080, 3064)
    );

    let irot = make_irot_box(3).unwrap();
    assert_eq!(
        parse_irot_quarter_turns(&irot, &header_for(&irot)).unwrap(),
        3
    );
}

#[test]
fn high_byte_fourcc_is_preserved_by_constructor() {
    let code = FourCC::new([0xa9, b'T', b'O', b'O']);
    let bytes = make_box(code, &[1, 2, 3]).unwrap();
    assert_eq!(&bytes[4..8], code.as_bytes());
    assert_eq!(header_for(&bytes).kind, code);
}
