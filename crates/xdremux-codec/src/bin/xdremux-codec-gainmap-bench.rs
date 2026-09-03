#![forbid(unsafe_code)]

use std::env;
use std::process::ExitCode;
use std::time::Instant;

use xdremux_codec::{GainMapTileEncodeRequest, LibHeifProvider, Raster8, RasterPixelFormat};
use xdremux_engine::{
    GainMapChannels, GainMapCodecLayout, GainMapEncodeProfile, GainMapTileEncoder,
};
use xdremux_format::ChromaSampling;

#[derive(Debug, Clone, Copy)]
struct Settings {
    width: u32,
    height: u32,
    warmup: usize,
    iterations: usize,
}

#[derive(Debug)]
struct Measurement {
    name: &'static str,
    channels: &'static str,
    chroma: &'static str,
    raw_bytes: usize,
    tile_count: usize,
    encoded_bytes: usize,
    median_seconds: f64,
    p95_seconds: f64,
}

fn usage() -> &'static str {
    "usage: xdremux-codec-gainmap-bench [--width N] [--height N] [--warmup N] [--iterations N]"
}

fn parse_positive_u32(value: Option<String>, flag: &str) -> Result<u32, String> {
    let raw = value.ok_or_else(|| format!("{flag} requires a value"))?;
    let parsed = raw
        .parse::<u32>()
        .map_err(|_| format!("{flag} must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{flag} must be greater than zero"));
    }
    Ok(parsed)
}

fn parse_usize(value: Option<String>, flag: &str, allow_zero: bool) -> Result<usize, String> {
    let raw = value.ok_or_else(|| format!("{flag} requires a value"))?;
    let parsed = raw
        .parse::<usize>()
        .map_err(|_| format!("{flag} must be an integer"))?;
    if !allow_zero && parsed == 0 {
        return Err(format!("{flag} must be greater than zero"));
    }
    Ok(parsed)
}

fn settings() -> Result<Settings, String> {
    let mut result = Settings {
        width: 1024,
        height: 768,
        warmup: 1,
        iterations: 5,
    };
    let mut args = env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--width" => result.width = parse_positive_u32(args.next(), "--width")?,
            "--height" => result.height = parse_positive_u32(args.next(), "--height")?,
            "--warmup" => result.warmup = parse_usize(args.next(), "--warmup", true)?,
            "--iterations" => result.iterations = parse_usize(args.next(), "--iterations", false)?,
            "-h" | "--help" => return Err(usage().to_owned()),
            other => return Err(format!("unknown argument {other:?}\n{}", usage())),
        }
    }
    if result.iterations < 3 {
        return Err("--iterations must be at least 3".to_owned());
    }
    Ok(result)
}

fn percentile(values: &[f64], fraction: f64) -> f64 {
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    if ordered.len() == 1 {
        return ordered[0];
    }
    let position = fraction * (ordered.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        ordered[lower]
    } else {
        let weight = position - lower as f64;
        ordered[lower] * (1.0 - weight) + ordered[upper] * weight
    }
}

fn median(values: &[f64]) -> f64 {
    percentile(values, 0.5)
}

fn smooth_raster(width: u32, height: u32, format: RasterPixelFormat) -> Result<Raster8, String> {
    let width_usize = usize::try_from(width).map_err(|_| "width exceeds usize")?;
    let height_usize = usize::try_from(height).map_err(|_| "height exceeds usize")?;
    let bytes_per_pixel = format.bytes_per_pixel();
    let bytes_per_row = width_usize
        .checked_mul(bytes_per_pixel)
        .ok_or("raster row size overflows")?;
    let byte_count = bytes_per_row
        .checked_mul(height_usize)
        .ok_or("raster byte count overflows")?;
    let mut data = vec![0_u8; byte_count];

    let x_denominator = width.saturating_sub(1).max(1);
    let y_denominator = height.saturating_sub(1).max(1);
    for y in 0..height {
        let y_component = y.saturating_mul(255) / y_denominator;
        for x in 0..width {
            let x_component = x.saturating_mul(255) / x_denominator;
            let base = ((x_component + y_component) / 2) as u8;
            let offset = usize::try_from(y)
                .ok()
                .and_then(|row| row.checked_mul(bytes_per_row))
                .and_then(|row| {
                    usize::try_from(x)
                        .ok()
                        .and_then(|column| column.checked_mul(bytes_per_pixel))
                        .and_then(|column| row.checked_add(column))
                })
                .ok_or("raster pixel offset overflows")?;
            match format {
                RasterPixelFormat::Mono8 => data[offset] = base,
                RasterPixelFormat::Rgb8 => {
                    data[offset] = base;
                    data[offset + 1] = base.wrapping_add(11);
                    data[offset + 2] = base.wrapping_add(23);
                }
            }
        }
    }

    Raster8::new(width, height, bytes_per_row, format, data).map_err(|error| error.to_string())
}

fn measure(
    provider: &LibHeifProvider,
    settings: Settings,
    name: &'static str,
    channels: GainMapChannels,
    chroma: ChromaSampling,
) -> Result<Measurement, String> {
    let format = match channels {
        GainMapChannels::Mono => RasterPixelFormat::Mono8,
        GainMapChannels::Rgb => RasterPixelFormat::Rgb8,
    };
    let raster = smooth_raster(settings.width, settings.height, format)?;
    let raw_bytes = raster.data.len();
    let request = GainMapTileEncodeRequest::reference_compatible(
        raster,
        GainMapEncodeProfile {
            width: settings.width,
            height: settings.height,
            channels,
            layout: GainMapCodecLayout {
                chroma,
                luma_bit_depth: 8,
                chroma_bit_depth: 8,
            },
        },
    );

    let mut samples = Vec::with_capacity(settings.iterations);
    let mut tile_count = 0;
    let mut encoded_bytes = 0;
    for index in 0..settings.warmup + settings.iterations {
        let started = Instant::now();
        let encoded = provider
            .encode_gain_map_tiles(&request)
            .map_err(|error| format!("{name} encode failed: {error}"))?;
        let elapsed = started.elapsed().as_secs_f64();
        tile_count = encoded.tiles.len();
        encoded_bytes = encoded.hvcc.len()
            + encoded
                .tiles
                .iter()
                .map(|tile| tile.payload.len())
                .sum::<usize>();
        if tile_count == 0 || encoded_bytes == 0 {
            return Err(format!("{name} produced an empty encoded resource"));
        }
        if index >= settings.warmup {
            samples.push(elapsed);
        }
    }

    Ok(Measurement {
        name,
        channels: match channels {
            GainMapChannels::Mono => "mono",
            GainMapChannels::Rgb => "rgb",
        },
        chroma: match chroma {
            ChromaSampling::Mono400 => "400",
            ChromaSampling::Yuv420 => "420",
            ChromaSampling::Yuv422 => "422",
            ChromaSampling::Yuv444 => "444",
        },
        raw_bytes,
        tile_count,
        encoded_bytes,
        median_seconds: median(&samples),
        p95_seconds: percentile(&samples, 0.95),
    })
}

fn run() -> Result<(), String> {
    let settings = settings()?;
    let provider = LibHeifProvider::new();
    let cases = [
        measure(
            &provider,
            settings,
            "mono400",
            GainMapChannels::Mono,
            ChromaSampling::Mono400,
        )?,
        measure(
            &provider,
            settings,
            "rgb444",
            GainMapChannels::Rgb,
            ChromaSampling::Yuv444,
        )?,
        measure(
            &provider,
            settings,
            "rgb420",
            GainMapChannels::Rgb,
            ChromaSampling::Yuv420,
        )?,
    ];

    println!("{{");
    println!("  \"schema_version\": 1,");
    println!("  \"metric\": \"libheif_gain_map_tile_encode_primitive\",");
    println!("  \"width\": {},", settings.width);
    println!("  \"height\": {},", settings.height);
    println!("  \"warmup\": {},", settings.warmup);
    println!("  \"iterations\": {},", settings.iterations);
    println!("  \"cases\": [");
    for (index, case) in cases.iter().enumerate() {
        let comma = if index + 1 == cases.len() { "" } else { "," };
        println!(
            "    {{\"name\":\"{}\",\"channels\":\"{}\",\"chroma\":\"{}\",\"raw_raster_bytes\":{},\"tile_count\":{},\"encoded_resource_bytes\":{},\"median_seconds\":{:.9},\"p95_seconds\":{:.9}}}{}",
            case.name,
            case.channels,
            case.chroma,
            case.raw_bytes,
            case.tile_count,
            case.encoded_bytes,
            case.median_seconds,
            case.p95_seconds,
            comma
        );
    }
    println!("  ]");
    println!("}}");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
