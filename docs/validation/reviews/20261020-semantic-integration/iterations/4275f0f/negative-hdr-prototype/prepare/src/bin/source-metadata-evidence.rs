use std::fs;
fn main() {
    let source=fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let blocks=xdremux_container::portrait_blocks(&source).unwrap();
    let extracted=xdremux_container::extract(&source).unwrap();
    let info=blocks.get("local.uhdr.gainmap.info").unwrap();
    let gain=blocks.get("local.uhdr.gainmap.data").unwrap();
    assert_eq!(info,&extracted.meta_bytes,"extracted metadata differs from original source block");
    assert_eq!(gain,&extracted.mask_jpeg_data,"extracted gain map differs from original source block");
    assert!(extracted.meta_floats.iter().all(|v|v.is_finite()));
    assert!([0,4,7,16,17].iter().all(|&i|extracted.meta_floats[i]>0.0));
    println!("source_info_bytes_equal_extracted=true source_gain_bytes_equal_extracted=true metadata_bytes={} float_count={} fallback_conditions_false=true",info.len(),extracted.meta_floats.len());
}
