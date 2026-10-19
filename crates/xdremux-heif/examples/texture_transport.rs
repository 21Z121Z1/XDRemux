//! Research transport client. Never writes over an existing output.
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;

use xdremux_heif::{inspect_texture_style_heif, preserve_texture_style_heif};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args_os().collect();
    if !(2..=3).contains(&args.len()) {
        return Err("usage: texture_transport INPUT.heic [NEW_OUTPUT.heic]".into());
    }
    let source = fs::read(&args[1])?;
    let facts = inspect_texture_style_heif(&source)?;
    if let Some(facts) = &facts {
        println!(
            "texture=opaque-carrier primary={} metadata={} bytes={} auxiliary_type_claims={}",
            facts.primary_item_id,
            facts.metadata_item_id,
            facts.metadata_byte_count,
            facts.auxiliary_type_claims.len()
        );
    } else {
        println!("texture=absent generation=unsupported");
    }
    if args.len() == 3 {
        let output = preserve_texture_style_heif(&source)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&args[2])?;
        file.write_all(&output)?;
        file.sync_all()?;
        println!("transport=byte-identical");
    }
    println!("native_texture_recognition=unverified photos_activation=unverified original_capture_information=not-generated");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
