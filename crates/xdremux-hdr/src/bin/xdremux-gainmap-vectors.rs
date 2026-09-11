use std::{collections::HashMap, env, fs, process};

use xdremux_hdr::{
    reconstruct_gain_map, resolve, ExtractionMode, GainMapParams, GainMapRaster, ResolvedScale,
};

fn bits(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

fn hex(data: &[u8]) -> String {
    data.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn parse_cases(path: &str) -> Result<HashMap<String, (ExtractionMode, Vec<f64>)>, String> {
    let input =
        fs::read_to_string(path).map_err(|error| format!("unable to read {path}: {error}"))?;
    let mut cases = HashMap::new();
    for (line_index, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 3 {
            return Err(format!("line {} must have 3 fields", line_index + 1));
        }
        let mode = match fields[1] {
            "lhdr" => ExtractionMode::Lhdr,
            "uhdr" => ExtractionMode::Uhdr,
            other => return Err(format!("line {} has unknown mode {other}", line_index + 1)),
        };
        let values = fields[2]
            .split(',')
            .map(|word| {
                u32::from_str_radix(word, 16)
                    .map(|value| f64::from(f32::from_bits(value)))
                    .map_err(|error| {
                        format!("line {} invalid float bits {word}: {error}", line_index + 1)
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        cases.insert(fields[0].to_owned(), (mode, values));
    }
    Ok(cases)
}

fn scale_for(
    cases: &HashMap<String, (ExtractionMode, Vec<f64>)>,
    name: &str,
) -> Result<(ResolvedScale, Vec<f64>), String> {
    let (mode, values) = cases
        .get(name)
        .ok_or_else(|| format!("missing EDR case {name}"))?;
    if *mode != ExtractionMode::Lhdr {
        return Err(format!("gain map case {name} must be LHDR"));
    }
    let scale = resolve(values, *mode).map_err(|error| format!("{name}: {error}"))?;
    Ok((scale, values.clone()))
}

fn emit(name: &str, raster: &GainMapRaster, params: &GainMapParams) {
    println!(
        "gainmap\t{name}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        raster.width,
        raster.height,
        raster.bytes_per_row,
        raster.channel_count,
        bits(params.knee),
        bits(params.knee_range),
        bits(params.headroom_scale),
        bits(params.max_boost),
        bits(params.log2_scale),
        params.knee_source,
        hex(&raster.data),
    );
}

fn all_bytes_mask() -> GainMapRaster {
    GainMapRaster {
        width: 256,
        height: 1,
        bytes_per_row: 256,
        channel_count: 1,
        data: (0_u16..=255).map(|value| value as u8).collect(),
    }
}

fn padded_mask() -> GainMapRaster {
    let width = 257_usize;
    let height = 2_usize;
    let bytes_per_row = 300_usize;
    let mut data = vec![0xa5_u8; bytes_per_row * height];
    for x in 0..width {
        data[x] = ((x * 17 + 3) & 0xff) as u8;
        data[bytes_per_row + x] = (255_u16.wrapping_sub(((x * 11) & 0xff) as u16)) as u8;
    }
    GainMapRaster {
        width,
        height,
        bytes_per_row,
        channel_count: 1,
        data,
    }
}

fn run(path: &str) -> Result<(), String> {
    let cases = parse_cases(path)?;

    for (output_name, source_name) in [
        ("early-all-bytes", "early-face-mid-highlight"),
        ("modern-all-bytes", "modern-precomputed-f32-source"),
    ] {
        let (scale, meta) = scale_for(&cases, source_name)?;
        let (raster, params) = reconstruct_gain_map(&all_bytes_mask(), &scale, &meta)
            .map_err(|error| format!("{output_name}: {error}"))?;
        emit(output_name, &raster, &params);
    }

    let (scale, meta) = scale_for(&cases, "modern-precomputed-f32-source")?;
    let (raster, params) = reconstruct_gain_map(&padded_mask(), &scale, &meta)
        .map_err(|error| format!("padded-stride: {error}"))?;
    emit("padded-stride", &raster, &params);

    Ok(())
}

fn main() {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: xdremux-gainmap-vectors <hdr_edr_cases.tsv>");
        process::exit(2);
    };
    if args.next().is_some() {
        eprintln!("usage: xdremux-gainmap-vectors <hdr_edr_cases.tsv>");
        process::exit(2);
    }
    if let Err(error) = run(&path) {
        eprintln!("{error}");
        process::exit(1);
    }
}
