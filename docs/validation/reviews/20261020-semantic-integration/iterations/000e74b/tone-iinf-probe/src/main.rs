use xdremux_format::isobmff::*;
use xdremux_format::FourCC;

fn main() {
    let directory = std::env::args().nth(1).expect("output directory");
    let output = std::path::Path::new(&directory);
    std::fs::create_dir_all(output).unwrap();
    let uri = b"tag:apple.com,2023:photo:metadata:styles\0";
    let mut body = 2u32.to_be_bytes().to_vec();
    body.extend_from_slice(b"\0\0uri styleMetadata\0");
    body.extend_from_slice(uri);
    let infos = [make_infe_box(1, FourCC::new(*b"hvc1"), 0).unwrap(),
                 make_full_box(INFE, 3, 1, &body).unwrap()];
    let payload = b"basebplist00structural-probe";
    let locations = [IlocEntry { item_id: 1, construction_method: 1, data_reference_index: 0,
        base_offset: 0, extents: vec![IlocExtent { index: None, offset: 0, length: 4 }] },
        IlocEntry { item_id: 2, construction_method: 1, data_reference_index: 0,
        base_offset: 0, extents: vec![IlocExtent { index: None, offset: 4, length: (payload.len()-4) as u64 }] }];
    let refs = [IrefEntry { kind: FourCC::new(*b"cdsc"), from_item_id: 2, to_item_ids: vec![1] }];
    let children = [make_pitm_box(0, 1).unwrap(), make_iinf_box(0, &infos).unwrap(),
        make_iloc_box(1, 4, 4, 0, 0, &locations).unwrap(),
        make_box(IPRP, &[make_box(IPCO, &[]).unwrap(), make_ipma_box(0, 0, &[]).unwrap()].concat()).unwrap(),
        make_iref_box(0, &refs).unwrap(), make_box(IDAT, payload).unwrap()].concat();
    let source = [make_box(FTYP, b"mif1\0\0\0\0mif1heic").unwrap(),
        make_full_box(META, 0, 0, &children).unwrap()].concat();
    let texture = xdremux_heif::assemble_texture_styles_heif(&source, b"bplist00measured", &[]).unwrap();
    for (name, bytes) in [("before", source), ("after", texture)] {
        let top = scan_top_level_boxes(&bytes).unwrap();
        let meta = top.boxes.iter().find(|b| b.kind == META).unwrap();
        let parsed = parse_meta_box(&bytes, meta).unwrap();
        let refs = parsed.iref.as_ref().unwrap();
        println!("{name}: product items={}, cdsc_from2_to1={}, original_style_payload_bytes={}",
            parsed.iinf.entries.len(), refs.entries.iter().any(|r| r.kind==FourCC::new(*b"cdsc") && r.from_item_id==2 && r.to_item_ids==[1]),
            xdremux_format::exif::read_item_payload(&bytes, parsed.iloc.entries.iter().find(|e|e.item_id==2).unwrap(), parsed.idat.as_ref()).unwrap().len());
        std::fs::write(output.join(format!("{name}.heic")), bytes).unwrap();
    }
}
