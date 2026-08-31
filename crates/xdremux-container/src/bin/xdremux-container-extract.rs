use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use xdremux_container::{extract, portrait_blocks};

fn hex(data: &[u8]) -> String {
    data.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn bits(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

fn reset_directory(path: &Path) -> Result<(), String> {
    match fs::remove_dir_all(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("unable to remove {}: {error}", path.display())),
    }
    fs::create_dir_all(path)
        .map_err(|error| format!("unable to create {}: {error}", path.display()))
}

fn write_snapshot(input_path: &Path, output_path: &Path) -> Result<(), String> {
    let data = fs::read(input_path)
        .map_err(|error| format!("unable to read {}: {error}", input_path.display()))?;
    let extracted = extract(&data).map_err(|error| error.to_string())?;
    let blocks = portrait_blocks(&data).map_err(|error| error.to_string())?;

    reset_directory(output_path)?;
    fs::write(output_path.join("meta.bin"), &extracted.meta_bytes)
        .map_err(|error| format!("unable to write meta.bin: {error}"))?;
    fs::write(output_path.join("mask.bin"), &extracted.mask_jpeg_data)
        .map_err(|error| format!("unable to write mask.bin: {error}"))?;

    let mut lines = Vec::new();
    lines.push(format!("mode\t{}", extracted.mode.as_str()));
    lines.push(format!("data-base\t{}", extracted.data_base));
    lines.push(format!(
        "manifest\t{}\t{}\t{}",
        extracted.manifest_info.extension_start,
        extracted.manifest_info.json_start,
        extracted.manifest_info.json_end
    ));
    if let Some(local) = extracted.local_hdr_info {
        lines.push(format!(
            "local-hdr\t{}\t{}\t{}\t{}",
            bits(local.version),
            bits(local.length),
            bits(local.meta_size),
            bits(local.offset)
        ));
    } else {
        lines.push("local-hdr\tnone".to_owned());
    }
    lines.push(format!(
        "meta-floats\t{}",
        extracted
            .meta_floats
            .iter()
            .copied()
            .map(bits)
            .collect::<Vec<_>>()
            .join(",")
    ));

    for entry in &extracted.manifest_info.entries {
        lines.push(format!(
            "entry\t{}\t{}\t{}\t{}\t{}\t{}",
            entry.json_order,
            hex(entry.name.as_bytes()),
            entry.offset,
            entry.length,
            entry.start,
            entry.end
        ));
    }

    for (index, (name, block)) in blocks.iter().enumerate() {
        let filename = format!("block-{index:04}.bin");
        fs::write(output_path.join(&filename), block)
            .map_err(|error| format!("unable to write {filename}: {error}"))?;
        lines.push(format!(
            "block\t{}\t{}\t{}",
            index,
            hex(name.as_bytes()),
            block.len()
        ));
    }

    let mut summary = lines.join("\n");
    summary.push('\n');
    fs::write(output_path.join("summary.tsv"), summary)
        .map_err(|error| format!("unable to write summary.tsv: {error}"))?;
    Ok(())
}

fn run() -> Result<(), String> {
    let mut arguments = env::args_os().skip(1);
    let input = arguments.next().map(PathBuf::from).ok_or_else(|| {
        "usage: xdremux-container-extract <input-file> <output-directory>".to_owned()
    })?;
    let output = arguments.next().map(PathBuf::from).ok_or_else(|| {
        "usage: xdremux-container-extract <input-file> <output-directory>".to_owned()
    })?;
    if arguments.next().is_some() {
        return Err("usage: xdremux-container-extract <input-file> <output-directory>".to_owned());
    }
    write_snapshot(&input, &output)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}
