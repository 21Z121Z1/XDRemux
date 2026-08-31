use std::{env, fs, process};

use xdremux_hdr::{get_knee_point_result, resolve, ExtractionMode, ResolvedScale};

fn bits(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

fn bits_list(values: &[f64]) -> String {
    values.iter().map(|value| bits(*value)).collect::<Vec<_>>().join(",")
}

fn emit_resolved(name: &str, resolved: &ResolvedScale) {
    println!(
        "resolve\t{name}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        bits(resolved.edr_scale),
        bits(resolved.ratio_min),
        bits(resolved.ratio_max),
        bits(resolved.gamma),
        bits(resolved.epsilon_sdr),
        bits(resolved.epsilon_hdr),
        bits(resolved.display_ratio_sdr),
        bits(resolved.display_ratio_hdr),
        bits(resolved.scale),
        bits(resolved.gain_map_min),
        bits(resolved.gain_map_max),
        bits(resolved.base_headroom),
        bits(resolved.alternate_headroom),
        resolved.source,
        resolved.channel_count,
        bits_list(&resolved.per_channel_gain_map_min),
        bits_list(&resolved.per_channel_gain_map_max),
        bits_list(&resolved.per_channel_gamma),
        bits_list(&resolved.per_channel_base_offset),
        bits_list(&resolved.per_channel_alternate_offset),
    );
}

fn run(path: &str) -> Result<(), String> {
    let input = fs::read_to_string(path).map_err(|error| format!("unable to read {path}: {error}"))?;
    for (line_index, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let name = fields.next().ok_or_else(|| format!("line {} missing name", line_index + 1))?;
        let mode_name = fields.next().ok_or_else(|| format!("line {} missing mode", line_index + 1))?;
        let encoded = fields.next().ok_or_else(|| format!("line {} missing values", line_index + 1))?;
        if fields.next().is_some() {
            return Err(format!("line {} has extra fields", line_index + 1));
        }
        let mode = match mode_name {
            "lhdr" => ExtractionMode::Lhdr,
            "uhdr" => ExtractionMode::Uhdr,
            _ => return Err(format!("line {} has unknown mode {mode_name}", line_index + 1)),
        };
        let values = encoded
            .split(',')
            .map(|word| {
                u32::from_str_radix(word, 16)
                    .map(|value| f64::from(f32::from_bits(value)))
                    .map_err(|error| format!("line {} invalid float bits {word}: {error}", line_index + 1))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let resolved = resolve(&values, mode).map_err(|error| format!("{name}: {error}"))?;
        emit_resolved(name, &resolved);
        if mode == ExtractionMode::Lhdr && values[0] < 3.0 {
            let knee = get_knee_point_result(resolved.edr_scale);
            println!("knee\t{name}\t{}\t{}", bits(knee.value), knee.source);
        }
    }
    Ok(())
}

fn main() {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: xdremux-hdr-vectors <hdr_edr_cases.tsv>");
        process::exit(2);
    };
    if args.next().is_some() {
        eprintln!("usage: xdremux-hdr-vectors <hdr_edr_cases.tsv>");
        process::exit(2);
    }
    if let Err(error) = run(&path) {
        eprintln!("{error}");
        process::exit(1);
    }
}
