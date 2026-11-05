//! Source HDR metadata must come from the item graph, not from the first
//! matching box. These cases build fixed container bytes by hand so the reader
//! is exercised independently of the assembler that writes production output.
use xdremux_format::isobmff::{
    make_box, make_full_box, make_iinf_box, make_iloc_box, make_infe_box, make_ipma_box,
    make_iref_box, make_ispe_box, IlocEntry, IlocExtent, IpmaAssociation, IpmaEntry, IrefEntry,
};
use xdremux_format::FourCC;
use xdremux_heif::extract_hdr_metadata;
use xdremux_metadata::{ContentLightLevel, HdrStaticMetadata, MasteringDisplay};

const BASE: FourCC = FourCC::new(*b"hvc1");
const TMAP: FourCC = FourCC::new(*b"tmap");
const IT35: FourCC = FourCC::new(*b"it35");
const DIMG: FourCC = FourCC::new(*b"dimg");
const CDSC: FourCC = FourCC::new(*b"cdsc");
const MDCV: FourCC = FourCC::new(*b"mdcv");
const CLLI: FourCC = FourCC::new(*b"clli");

fn mastering() -> MasteringDisplay {
    MasteringDisplay {
        primaries: [[8500, 39850], [6550, 2300], [35400, 14600]],
        white_point: [15635, 16450],
        maximum_luminance: 10_000_000,
        minimum_luminance: 50,
    }
}

fn light() -> ContentLightLevel {
    ContentLightLevel {
        maximum_content_light_level: 1000,
        maximum_frame_average_light_level: 400,
    }
}

/// Registered metadata payload with a fixed dynamic section.
fn registered_payload(with_static: bool) -> Vec<u8> {
    let mut section = Vec::new();
    if with_static {
        section.extend_from_slice(&28_u16.to_be_bytes());
        section.extend_from_slice(&mastering().to_mdcv_payload().unwrap());
        section.extend_from_slice(&light().to_clli_payload().unwrap());
    } else {
        section.extend_from_slice(&0_u16.to_be_bytes());
    }
    let dynamic = [1_u8, 0, 0, 0, 0, 0, 0, 0];
    section.extend_from_slice(&(dynamic.len() as u16).to_be_bytes());
    section.extend_from_slice(&dynamic);
    let mut payload = vec![0x26, 0, 4, 0, 7, 2];
    payload.extend_from_slice(&0x02010000_u32.to_be_bytes());
    payload.extend_from_slice(&(section.len() as u16).to_be_bytes());
    payload.extend(section);
    payload
}

#[derive(Clone, Copy, Default)]
struct Carriers {
    mastering: bool,
    content_light: bool,
    /// Registered metadata target: `None` disables the carrier.
    registered: Option<RegisteredTarget>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RegisteredTarget {
    Derived,
    Primary,
    Duplicate,
}

struct Built {
    bytes: Vec<u8>,
    derived_id: u32,
}

fn build(carriers: Carriers, with_derived: bool) -> Built {
    let base_id = 1_u32;
    let derived_id = 3_u32;
    let registered_id = 4_u32;
    let base_bytes = b"base-image-bytes";

    let mut properties = vec![make_ispe_box(64, 64).unwrap()];
    if carriers.mastering {
        properties.push(make_box(MDCV, &mastering().to_mdcv_payload().unwrap()).unwrap());
    }
    if carriers.content_light {
        properties.push(make_box(CLLI, &light().to_clli_payload().unwrap()).unwrap());
    }
    let mut associations = vec![IpmaAssociation {
        property_index: 1,
        essential: true,
    }];
    for index in 2..=u16::try_from(properties.len()).unwrap() {
        associations.push(IpmaAssociation {
            property_index: index,
            essential: false,
        });
    }

    let mut entries = vec![make_infe_box(base_id, BASE, 0).unwrap()];
    let mut iloc = Vec::new();
    let mut idat = Vec::new();
    let mut iref = Vec::new();
    let mut ipma = vec![IpmaEntry {
        item_id: base_id,
        associations: vec![IpmaAssociation {
            property_index: 1,
            essential: true,
        }],
    }];

    if with_derived {
        entries.push(make_infe_box(derived_id, TMAP, 0).unwrap());
        ipma.push(IpmaEntry {
            item_id: derived_id,
            associations,
        });
        iref.push(IrefEntry {
            kind: DIMG,
            from_item_id: derived_id,
            to_item_ids: vec![base_id],
        });
        iloc.push(IlocEntry {
            item_id: derived_id,
            construction_method: 1,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset: u64::try_from(idat.len()).unwrap(),
                length: 4,
            }],
        });
        idat.extend_from_slice(b"tmap");
    }

    if let Some(target) = carriers.registered {
        entries.push(make_infe_box(registered_id, IT35, 1).unwrap());
        let payload = registered_payload(true);
        iloc.push(IlocEntry {
            item_id: registered_id,
            construction_method: 1,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset: u64::try_from(idat.len()).unwrap(),
                length: u64::try_from(payload.len()).unwrap(),
            }],
        });
        idat.extend_from_slice(&payload);
        iref.push(IrefEntry {
            kind: CDSC,
            from_item_id: registered_id,
            to_item_ids: vec![match target {
                RegisteredTarget::Derived | RegisteredTarget::Duplicate => derived_id,
                RegisteredTarget::Primary => base_id,
            }],
        });
        if target == RegisteredTarget::Duplicate {
            entries.push(make_infe_box(registered_id + 1, IT35, 1).unwrap());
            let offset = u64::try_from(idat.len()).unwrap();
            let length = u64::try_from(payload.len()).unwrap();
            iloc.push(IlocEntry {
                item_id: registered_id + 1,
                construction_method: 1,
                data_reference_index: 0,
                base_offset: 0,
                extents: vec![IlocExtent {
                    index: None,
                    offset,
                    length,
                }],
            });
            idat.extend_from_slice(&payload);
            iref.push(IrefEntry {
                kind: CDSC,
                from_item_id: registered_id + 1,
                to_item_ids: vec![derived_id],
            });
        }
    }

    iloc.insert(
        0,
        IlocEntry {
            item_id: base_id,
            construction_method: 1,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset: u64::try_from(idat.len()).unwrap(),
                length: u64::try_from(base_bytes.len()).unwrap(),
            }],
        },
    );
    idat.extend_from_slice(base_bytes);

    let ftyp = make_box(FourCC::new(*b"ftyp"), b"heic\0\0\0\0mif1heic").unwrap();
    let pitm = make_full_box(FourCC::new(*b"pitm"), 0, 0, &1_u16.to_be_bytes()).unwrap();
    let iinf = make_iinf_box(0, &entries).unwrap();
    let iloc = make_iloc_box(1, 4, 4, 0, 0, &iloc).unwrap();
    let ipco = make_box(FourCC::new(*b"ipco"), &properties.concat()).unwrap();
    let ipma = make_ipma_box(0, 0, &ipma).unwrap();
    let iprp = make_box(FourCC::new(*b"iprp"), &[ipco, ipma].concat()).unwrap();
    let iref = make_iref_box(0, &iref).unwrap();
    let idat_box = make_box(FourCC::new(*b"idat"), &idat).unwrap();
    let meta_payload = [pitm, iinf, iloc, iprp, iref, idat_box].concat();
    let meta = make_full_box(FourCC::new(*b"meta"), 0, 0, &meta_payload).unwrap();

    Built {
        bytes: [ftyp, meta].concat(),
        derived_id,
    }
}

#[test]
fn static_carriers_of_the_derived_image_are_read_with_their_source() {
    let built = build(
        Carriers {
            mastering: true,
            content_light: true,
            registered: None,
        },
        true,
    );
    let observed = extract_hdr_metadata(&built.bytes).unwrap();
    assert_eq!(
        observed.provenance.derived_hdr_item_id,
        Some(built.derived_id)
    );
    assert_eq!(
        observed.static_metadata.mastering_display,
        Some(mastering())
    );
    assert_eq!(observed.static_metadata.content_light_level, Some(light()));
    assert!(observed.registered_metadata.is_none());
    assert_eq!(observed.provenance.mastering_display_property, Some(2));
    assert_eq!(observed.provenance.content_light_level_property, Some(3));
    assert_eq!(observed.provenance.ignored_carrier_count, 0);
}

#[test]
fn absent_carriers_stay_absent_instead_of_being_invented() {
    let built = build(Carriers::default(), true);
    let observed = extract_hdr_metadata(&built.bytes).unwrap();
    assert!(observed.is_empty());
    assert_eq!(observed.static_metadata, HdrStaticMetadata::default());
    assert_eq!(
        observed.provenance.derived_hdr_item_id,
        Some(built.derived_id)
    );
}

#[test]
fn a_file_without_a_derived_image_reports_its_carriers_as_not_applicable() {
    let built = build(
        Carriers {
            mastering: true,
            content_light: true,
            registered: None,
        },
        false,
    );
    let observed = extract_hdr_metadata(&built.bytes).unwrap();
    assert!(observed.is_empty());
    assert_eq!(observed.provenance.derived_hdr_item_id, None);
    assert_eq!(observed.provenance.ignored_carrier_count, 2);
}

#[test]
fn registered_metadata_that_describes_another_image_is_not_applied() {
    let built = build(
        Carriers {
            mastering: false,
            content_light: false,
            registered: Some(RegisteredTarget::Primary),
        },
        true,
    );
    let observed = extract_hdr_metadata(&built.bytes).unwrap();
    assert!(observed.is_empty());
    assert_eq!(observed.provenance.ignored_carrier_count, 1);
}

#[test]
fn registered_metadata_for_the_derived_image_is_retained_with_its_dynamic_payload() {
    let built = build(
        Carriers {
            mastering: false,
            content_light: false,
            registered: Some(RegisteredTarget::Derived),
        },
        true,
    );
    let observed = extract_hdr_metadata(&built.bytes).unwrap();
    assert!(observed.has_dynamic_metadata());
    assert_eq!(
        observed.static_metadata.mastering_display,
        Some(mastering())
    );
    assert!(observed.provenance.properties_from_registered);
    assert_eq!(observed.provenance.registered_item_id, Some(4));
}

#[test]
fn two_registered_items_for_the_same_image_are_ambiguous() {
    let built = build(
        Carriers {
            mastering: false,
            content_light: false,
            registered: Some(RegisteredTarget::Duplicate),
        },
        true,
    );
    let error = extract_hdr_metadata(&built.bytes).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("more than one registered metadata"),
        "{error}"
    );
}

#[test]
fn a_derived_image_that_describes_a_missing_primary_is_rejected() {
    let mut built = build(Carriers::default(), true);
    // Break the dimg reference target so no derived image describes the primary.
    let position = built
        .bytes
        .windows(4)
        .position(|w| w == b"dimg")
        .expect("dimg reference present");
    built.bytes[position + 16..position + 18].copy_from_slice(&9_u16.to_be_bytes());
    assert!(extract_hdr_metadata(&built.bytes).is_err());
}
