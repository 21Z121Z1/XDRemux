//! Build a photo-free ProXDR test source from analytical Apple-encoded inputs.
//! The gain metadata is an invented test context, never camera provenance.
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args_os().collect();
    if args.len() != 4 {
        return Err("usage: texture_synthetic_uhdr BASE.heic GAIN.jpg NEW_OUTPUT.heic".into());
    }
    let mut source = fs::read(&args[1])?;
    let gain = fs::read(&args[2])?;
    if !gain.starts_with(&[0xff, 0xd8]) {
        return Err("gain input must be a generated JPEG".into());
    }
    let info_start = source.len();
    let values: [f32; 20] = [
        1., 1., 1., 1., 4.926, 4.926, 4.926, 1., 1., 1., 0., 0., 0., 0., 0., 0., 1., 4.926, 4.926,
        0.,
    ];
    for value in values {
        source.extend_from_slice(&value.to_le_bytes());
    }
    let data_start = source.len();
    source.extend_from_slice(&gain);
    let json_start = source.len();
    let manifest = format!(
        r#"[{{"name":"local.uhdr.gainmap.info","offset":{},"length":80}},{{"name":"local.uhdr.gainmap.data","offset":{},"length":{}}}]"#,
        json_start - info_start,
        json_start - data_start,
        gain.len()
    );
    source.extend_from_slice(manifest.as_bytes());
    source.extend_from_slice(b"\0jxrs");
    source.extend_from_slice(&u32::try_from(manifest.len() + 9)?.to_le_bytes());
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[3])?;
    output.write_all(&source)?;
    output.sync_all()?;
    println!("synthetic_uhdr_fixture=true camera_context=false");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
