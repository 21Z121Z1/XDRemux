use xdremux_format::isobmff::{IlocEntry, IlocExtent};
use xdremux_metadata::{
    adjusted_extent_for_oppo_user_comment_patch, adjusted_oppo_user_comment,
    apply_oppo_user_comment_patch, make_apple_tmap_payload, make_hdrgm_xmp,
    make_imageio_native_tmap_payload, make_strict_tmap_payload, target_oppo_tag_flags,
    OppoCompatibility, Result, ISO_ULTRA_HDR_FLAG, LOCAL_HDR_FLAG, OPPO_ULTRA_HDR_FLAG,
};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn push_u16_le(value: u16, output: &mut Vec<u8>) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_u32_le(value: u32, output: &mut Vec<u8>) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn synthetic_exif(comment: &str) -> Vec<u8> {
    let user_comment = [b"ASCII\0\0\0".as_slice(), comment.as_bytes()].concat();
    let mut tiff = Vec::new();
    tiff.extend_from_slice(b"II");
    push_u16_le(42, &mut tiff);
    push_u32_le(8, &mut tiff);
    push_u16_le(1, &mut tiff);
    push_u16_le(0x8769, &mut tiff);
    push_u16_le(4, &mut tiff);
    push_u32_le(1, &mut tiff);
    push_u32_le(26, &mut tiff);
    push_u32_le(0, &mut tiff);
    push_u16_le(1, &mut tiff);
    push_u16_le(0x9286, &mut tiff);
    push_u16_le(7, &mut tiff);
    push_u32_le(user_comment.len() as u32, &mut tiff);
    push_u32_le(44, &mut tiff);
    push_u32_le(0, &mut tiff);
    tiff.extend_from_slice(&user_comment);
    let mut exif = 0u32.to_be_bytes().to_vec();
    exif.extend_from_slice(&tiff);
    exif
}

fn emit_extent(name: &str, value: Option<(u64, u64)>) {
    match value {
        Some((offset, length)) => println!("extent\t{name}\t{offset}\t{length}"),
        None => println!("extent\t{name}\tnil"),
    }
}

fn run() -> Result<()> {
    let routing_sources = [
        ("clear", LOCAL_HDR_FLAG | 0x1234),
        (
            "all",
            OPPO_ULTRA_HDR_FLAG | ISO_ULTRA_HDR_FLAG | LOCAL_HDR_FLAG | 0x1234,
        ),
    ];
    for (source_name, source) in routing_sources {
        for mode in OppoCompatibility::ALL {
            println!(
                "routing\t{source_name}\t{}\t{}",
                mode.name(),
                target_oppo_tag_flags(source, mode)
            );
        }
    }

    for mode in OppoCompatibility::ALL {
        let adjusted = adjusted_oppo_user_comment(b"ASCIIOplus_00000001", mode)?;
        println!(
            "comment\t{}\t{}",
            mode.name(),
            adjusted.as_deref().unwrap_or("nil")
        );
    }

    let canonical_ratio = 4.926108360290527;
    let canonical = [
        1.0,
        1.0,
        1.0,
        1.0,
        canonical_ratio,
        canonical_ratio,
        canonical_ratio,
        1.0,
        1.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        canonical_ratio,
        canonical_ratio,
        0.0,
    ];
    let distinct = [
        1.25, 1.5, 1.75, 1.0, 4.0, 5.0, 6.0, 0.8, 1.1, 1.2, 0.01, 0.02, 0.03,
        0.04, 0.05, 0.06, 1.5, 6.5, 2.0, 0.0,
    ];
    for (name, info) in [("canonical", canonical), ("distinct", distinct)] {
        let apple = make_apple_tmap_payload(&info)?;
        let native = make_imageio_native_tmap_payload(&info)?;
        println!("metadata\t{name}\tapple-tmap\t{}", hex(&apple));
        println!("metadata\t{name}\tnative-tmap\t{}", hex(&native));
        println!(
            "metadata\t{name}\tstrict-apple-tmap\t{}",
            hex(&make_strict_tmap_payload(&apple)?)
        );
        println!(
            "metadata\t{name}\tstrict-native-tmap\t{}",
            hex(&make_strict_tmap_payload(&native)?)
        );
        println!("metadata\t{name}\thdrgm-xmp\t{}", hex(&make_hdrgm_xmp(&info)?));
    }

    let exif = synthetic_exif("Oplus_00000001");
    let prefix = vec![0x55; 13];
    let suffix = vec![0x77; 11];
    let mut mdat = [prefix, exif.clone(), suffix].concat();
    let entry = IlocEntry {
        item_id: 7,
        construction_method: 0,
        data_reference_index: 0,
        base_offset: 0,
        extents: vec![IlocExtent {
            index: None,
            offset: 1013,
            length: exif.len() as u64,
        }],
    };
    let patched_comment = adjusted_oppo_user_comment(&exif, OppoCompatibility::On)?
        .expect("synthetic OPPO comment must require activation");
    let patch = apply_oppo_user_comment_patch(&mut mdat, 1000, &entry, &patched_comment)?
        .expect("synthetic Exif patch must succeed");
    println!(
        "patch\t{}\t{}\t{}\t{}",
        patch.source_start,
        patch.source_end,
        patch.delta,
        hex(&mdat)
    );
    emit_extent(
        "before",
        adjusted_extent_for_oppo_user_comment_patch(900, 20, Some(patch))?,
    );
    emit_extent(
        "contains",
        adjusted_extent_for_oppo_user_comment_patch(1000, 200, Some(patch))?,
    );
    emit_extent(
        "after",
        adjusted_extent_for_oppo_user_comment_patch(1200, 20, Some(patch))?,
    );
    emit_extent(
        "partial",
        adjusted_extent_for_oppo_user_comment_patch(patch.source_start + 1, 20, Some(patch))?,
    );

    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
