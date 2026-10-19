//! CC0 analytical charts. Values have no camera or sensor provenance.
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use sha2::{Digest, Sha256};

fn put(root: &Path, name: &str, bytes: &[u8]) -> std::io::Result<String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))?;
    file.write_all(bytes)?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let dir = env::args_os()
        .nth(1)
        .ok_or("usage: texture_synthetic_inputs NEW_DIRECTORY")?;
    let root = Path::new(&dir);
    fs::create_dir(root)?;
    let mut entries = Vec::new();
    for (name, width, height, orientation, hdr, pattern) in [
        ("landscape-gradient", 384_u32, 256_u32, 1, false, 0),
        ("portrait-checker", 256, 384, 6, false, 1),
        ("landscape-hdr-edge", 384, 256, 1, true, 2),
        ("portrait-hdr-impulse", 256, 384, 8, true, 3),
    ] {
        let mut rgba = Vec::with_capacity((width * height * 16) as usize);
        let mut sdr = Vec::with_capacity(rgba.capacity());
        for y in 0..height {
            for x in 0..width {
                let fx = x as f32 / (width - 1) as f32;
                let fy = y as f32 / (height - 1) as f32;
                let v = match pattern {
                    0 => [0.02 + 0.8 * fx, 0.04 + 0.65 * fy, 0.1 + 0.4 * fx * fy],
                    1 => {
                        let a = if (x / 13 + y / 17) % 2 == 0 { 0.1 } else { 0.8 };
                        [a, 0.2 + 0.5 * fx, 0.1 + 0.5 * fy]
                    }
                    2 => [
                        if x > width / 2 { 4.0 } else { 0.02 },
                        0.05 + 1.5 * fy,
                        0.1 + fx,
                    ],
                    _ => [
                        if x == width / 2 && y == height / 2 {
                            8.0
                        } else {
                            0.05 + fx
                        },
                        0.1 + 0.5 * fy,
                        0.02 + 0.4 * fx,
                    ],
                };
                for a in v.into_iter().chain([1.0]) {
                    rgba.extend_from_slice(&a.to_le_bytes());
                }
                let v = if hdr { v.map(|a| a / (1.0 + a)) } else { v };
                for a in v.into_iter().chain([1.0]) {
                    sdr.extend_from_slice(&a.to_le_bytes());
                }
            }
        }
        let file = format!("{name}.rgba32f");
        let hash = put(root, &file, &rgba)?;
        let sdr_file = format!("{name}-sdr.rgba32f");
        let sdr_hash = put(root, &sdr_file, &sdr)?;
        entries.push(format!(r#"{{"name":"{name}","file":"{file}","sha256":"{hash}","sdr_file":"{sdr_file}","sdr_sha256":"{sdr_hash}","width":{width},"height":{height},"orientation":{orientation},"hdr":{hdr}}}"#));
    }
    let manifest = format!(
        r#"{{"schema":"xdremux-texture-synthetic-inputs-v1","license":"CC0-1.0","domain":"analytical extended linear Display P3 float32 little endian RGBA","original_camera_information":false,"original_raw":false,"original_linear_thumbnail":false,"inputs":[{}]}}
"#,
        entries.join(",")
    );
    put(root, "manifest.json", manifest.as_bytes())?;
    println!("generated=4 copyright=CC0-1.0 camera_information=false");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
