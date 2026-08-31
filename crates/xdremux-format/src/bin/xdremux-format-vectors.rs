use xdremux_format::isobmff::{
    make_iinf_box, make_iloc_box, make_infe_box, make_ipma_box, make_iref_box, make_irot_box,
    make_ispe_box, make_pitm_box, IlocEntry, IlocExtent, IpmaAssociation, IpmaEntry, IrefEntry,
};
use xdremux_format::{FourCC, Result};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn emit(name: &str, bytes: &[u8]) {
    println!("vector\t{name}\t{}", hex(bytes));
}

fn run() -> Result<()> {
    let pitm_v0 = make_pitm_box(0, 0x1234)?;
    emit("pitm-v0", &pitm_v0);

    let pitm_v1 = make_pitm_box(1, 0x1234_5678)?;
    emit("pitm-v1", &pitm_v1);

    let infe = make_infe_box(0x1234, FourCC::new(*b"hvc1"), 0x01_02_03)?;
    emit("infe-v2", &infe);
    emit("iinf-v0", &make_iinf_box(0, std::slice::from_ref(&infe))?);
    emit("iinf-v1", &make_iinf_box(1, std::slice::from_ref(&infe))?);

    let iloc_entries = vec![
        IlocEntry {
            item_id: 7,
            construction_method: 0,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![
                IlocExtent {
                    index: None,
                    offset: 0x0102_0304,
                    length: 0x0506_0708,
                },
                IlocExtent {
                    index: None,
                    offset: 0x1112_1314,
                    length: 0x1516_1718,
                },
            ],
        },
        IlocEntry {
            item_id: 8,
            construction_method: 1,
            data_reference_index: 0,
            base_offset: 0,
            extents: vec![IlocExtent {
                index: None,
                offset: 0x2122_2324,
                length: 0x2526_2728,
            }],
        },
    ];
    emit(
        "iloc-v1-44",
        &make_iloc_box(1, 4, 4, 0, 0, &iloc_entries)?,
    );

    let narrow_ipma = vec![IpmaEntry {
        item_id: 0x1234,
        associations: vec![
            IpmaAssociation {
                property_index: 3,
                essential: true,
            },
            IpmaAssociation {
                property_index: 4,
                essential: false,
            },
        ],
    }];
    emit("ipma-v0-narrow", &make_ipma_box(0, 0, &narrow_ipma)?);

    let wide_ipma = vec![IpmaEntry {
        item_id: 0x1234_5678,
        associations: vec![
            IpmaAssociation {
                property_index: 0x0123,
                essential: true,
            },
            IpmaAssociation {
                property_index: 0x0456,
                essential: false,
            },
        ],
    }];
    emit("ipma-v1-wide", &make_ipma_box(1, 1, &wide_ipma)?);

    let refs_v0 = vec![IrefEntry {
        kind: FourCC::new(*b"auxl"),
        from_item_id: 0x1234,
        to_item_ids: vec![0x2345, 0x3456],
    }];
    emit("iref-v0", &make_iref_box(0, &refs_v0)?);

    let refs_v1 = vec![IrefEntry {
        kind: FourCC::new(*b"dimg"),
        from_item_id: 0x1234_5678,
        to_item_ids: vec![0x2345_6789, 0x3456_789a],
    }];
    emit("iref-v1", &make_iref_box(1, &refs_v1)?);

    emit("ispe", &make_ispe_box(4032, 3024)?);
    emit("irot", &make_irot_box(3)?);

    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
