use std::error::Error;

use xdremux_metadata::{adjusted_oppo_user_comment_in_heif, OppoCompatibility};

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args().skip(1);
    let path = arguments
        .next()
        .ok_or("usage: xdremux-metadata-fixture <heif-file>")?;
    if arguments.next().is_some() {
        return Err("usage: xdremux-metadata-fixture <heif-file>".into());
    }
    let data = std::fs::read(path)?;
    for mode in OppoCompatibility::ALL {
        let adjusted = adjusted_oppo_user_comment_in_heif(&data, mode)?;
        println!(
            "fixture\t{}\t{}",
            mode.name(),
            adjusted.as_deref().unwrap_or("nil")
        );
    }
    Ok(())
}
