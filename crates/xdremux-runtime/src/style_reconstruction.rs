use std::fs;
use std::io::{BufWriter, Read, Write};
use std::path::Path;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use xdremux_codec::{GainMapTileEncodeRequest, LibHeifProvider, Raster8, RasterPixelFormat};
use xdremux_engine::{
    plan_style_reconstruction, GainMapCodecLayout, GainMapTileEncoder,
    STYLE_RECONSTRUCTION_METADATA_VERSIONS,
};
use xdremux_format::ChromaSampling;
use xdremux_hdr::{
    display_p3_luminance, make_scene_gain_map, reconstruct_relative_scene, LinearPixel,
};
use xdremux_heif::{
    assemble_iso_gain_map_heif, extract_style_reconstruction_resources, primary_image_signature,
    validate_gain_map_structure, DirectHevcGainMap, GainMapChannels, GainMapEncodeProfile,
    GainMapTile, IsoGainMapAssembly,
};
use xdremux_metadata::make_scene_gain_map_metadata;

use crate::apple_adapter::AppleAdapterClient;
use crate::{Result, RuntimeError};

fn error(detail: impl Into<String>) -> RuntimeError {
    RuntimeError::new("Styles reconstruction", detail)
}
fn io(e: std::io::Error) -> RuntimeError {
    RuntimeError::external("Styles reconstruction filesystem", e)
}

fn read_pixels(path: &Path, count: usize) -> Result<Vec<LinearPixel>> {
    let bytes = fs::read(path)
        .map_err(|e| error(format!("cannot read native pixels {}: {e}", path.display())))?;
    if bytes.len()
        != count
            .checked_mul(16)
            .ok_or_else(|| error("pixel count overflow"))?
    {
        return Err(error(
            "native raster byte length does not match its declared geometry",
        ));
    }
    let mut pixels = Vec::with_capacity(count);
    for p in bytes.chunks_exact(16) {
        let mut v = [0.; 4];
        for c in 0..4 {
            v[c] = f32::from_le_bytes(p[c * 4..c * 4 + 4].try_into().expect("four bytes"));
        }
        if v.iter().any(|x| !x.is_finite()) {
            return Err(error("non-finite native raster"));
        }
        pixels.push(v);
    }
    Ok(pixels)
}
fn write_pixels(path: &Path, pixels: &[LinearPixel]) -> Result<()> {
    let mut file = BufWriter::new(fs::File::create(path).map_err(io)?);
    for pixel in pixels {
        for v in pixel {
            file.write_all(&v.to_le_bytes()).map_err(io)?;
        }
    }
    file.flush().map_err(io)
}
fn geometry(value: &Value) -> Result<(usize, usize)> {
    let w = value["width"]
        .as_u64()
        .ok_or_else(|| error("missing native width"))? as usize;
    let h = value["height"]
        .as_u64()
        .ok_or_else(|| error("missing native height"))? as usize;
    if w == 0
        || h == 0
        || w > 32768
        || h > 32768
        || w.checked_mul(h).is_none_or(|n| n > 100_000_000)
    {
        return Err(error("unsupported native geometry"));
    }
    Ok((w, h))
}
fn hash_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).map_err(io)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer).map_err(io)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}
fn encode(
    client: &AppleAdapterClient,
    pixels: &Path,
    output: &Path,
    work: &Path,
    size: (usize, usize),
    orientation: u32,
    expected: &[LinearPixel],
) -> Result<Value> {
    let (w, h) = size;
    let readback = work.join("readback");
    let result=client.styles_primitive(&json!({"schema_version":2,"operation":"styles-encode-linear",
        "input_path":pixels,"output_path":output,"width":w,"height":h,"orientation":orientation,"readback_path":readback}))?;
    if geometry(&result["readback"])? != (w, h) {
        return Err(error("encoded image geometry changed"));
    }
    let decoded = read_pixels(&work.join("readback.f32"), w * h)?;
    let mut sum = 0.;
    let mut count = 0;
    let mut max_rgb = 0f32;
    for (a, b) in expected.iter().zip(&decoded) {
        for c in 0..3 {
            max_rgb = max_rgb.max((a[c] - b[c]).abs());
        }
        let ya = display_p3_luminance(a);
        let yb = display_p3_luminance(b);
        if ya > 1e-4 && yb > 1e-4 {
            sum += (ya / yb).log2().abs() as f64;
            count += 1;
        }
    }
    if count == 0 {
        return Err(error("encoded image has no usable luminance"));
    }
    let mae = sum / count as f64;
    if mae > 0.005 {
        return Err(error(format!(
            "encoded image luminance roundtrip error {mae:.6} EV exceeds the limit"
        )));
    }
    Ok(
        json!({"width":if orientation>=5 {h} else {w},"height":if orientation>=5 {w} else {h},"readback_mae_ev":mae,"readback_max_rgb_error":max_rgb,"sha256":hash_file(output)?}),
    )
}

fn hdr_gain_map(
    client: &AppleAdapterClient,
    source: &[u8],
    sdr: &[LinearPixel],
    target: &[LinearPixel],
    output: &Path,
    work: &Path,
    size: (usize, usize),
) -> Result<Value> {
    let (w, h) = size;
    let map = make_scene_gain_map(sdr, target, w, h)
        .map_err(|e| RuntimeError::external("HDR-only scene Gain Map", e))?;
    let raster = Raster8::new(
        w as u32,
        h as u32,
        w * 3,
        RasterPixelFormat::Rgb8,
        map.raster.data,
    )
    .map_err(|e| RuntimeError::external("scene Gain Map raster", e))?;
    let encoded = LibHeifProvider::new()
        .encode_gain_map_tiles(&GainMapTileEncodeRequest::reference_compatible(
            raster,
            xdremux_engine::GainMapEncodeProfile {
                width: w as u32,
                height: h as u32,
                channels: xdremux_engine::GainMapChannels::Rgb,
                layout: GainMapCodecLayout {
                    chroma: ChromaSampling::Yuv444,
                    luma_bit_depth: 8,
                    chroma_bit_depth: 8,
                },
            },
        ))
        .map_err(|e| RuntimeError::external("scene Gain Map HEVC encode", e))?;
    let tiles = encoded
        .tiles
        .iter()
        .map(|t| GainMapTile {
            payload: &t.payload,
            width: t.width,
            height: t.height,
        })
        .collect::<Vec<_>>();
    let (tmap, xmp) =
        make_scene_gain_map_metadata(map.min_ev as f64, map.max_ev as f64, map.offset as f64)
            .map_err(|e| RuntimeError::external("scene Gain Map metadata", e))?;
    let bytes = assemble_iso_gain_map_heif(
        source,
        &IsoGainMapAssembly {
            gain_map: DirectHevcGainMap {
                gain_map_width: w as u32,
                gain_map_height: h as u32,
                tile_width: encoded.tile_width,
                tile_height: encoded.tile_height,
                tiles: &tiles,
                hvcc: &encoded.hvcc,
                profile: GainMapEncodeProfile {
                    channels: GainMapChannels::Rgb,
                    chroma: ChromaSampling::Yuv444,
                    luma_bit_depth: 8,
                    chroma_bit_depth: 8,
                },
            },
            tmap_payload: &tmap,
            xmp_payload: &xmp,
        },
    )
    .map_err(|e| RuntimeError::external("source-preserving scene HEIF", e))?;
    validate_gain_map_structure(&bytes)
        .map_err(|e| RuntimeError::external("scene Gain Map graph", e))?;
    if primary_image_signature(source)
        .map_err(|e| RuntimeError::external("source SDR identity", e))?
        != primary_image_signature(&bytes)
            .map_err(|e| RuntimeError::external("output SDR identity", e))?
    {
        return Err(error(
            "compressed SDR image or its colour/geometry properties changed",
        ));
    }
    fs::write(output, &bytes).map_err(io)?;
    client.styles_primitive(&json!({"schema_version":2,"operation":"styles-readback","input_path":output,"output_path":work,"width":w,"height":h}))?;
    let published_sdr = read_pixels(&work.join("published_sdr.f32"), w * h)?;
    let sdr_max = published_sdr
        .iter()
        .zip(sdr)
        .flat_map(|(a, b)| (0..3).map(move |c| (a[c] - b[c]).abs()))
        .fold(0f32, f32::max);
    if sdr_max > 1e-6 {
        return Err(error(format!("SDR consumer readback changed by {sdr_max}")));
    }
    let hdr = read_pixels(&work.join("published_hdr.f32"), w * h)?;
    let mut residuals = Vec::new();
    for (a, b) in target.iter().zip(&hdr) {
        let ya = display_p3_luminance(a);
        let yb = display_p3_luminance(b);
        if ya > 1e-4 && yb > 1e-4 {
            residuals.push((ya / yb).log2().abs());
        }
    }
    if residuals.is_empty() {
        return Err(error("published Gain Map did not expand to HDR"));
    }
    let mae = residuals.iter().map(|v| *v as f64).sum::<f64>() / residuals.len() as f64;
    if mae > 0.15 {
        return Err(error(format!(
            "published HDR differs from scene target by {mae:.6} EV"
        )));
    }
    Ok(
        json!({"sha256":hash_file(output)?,"compressed_sdr_preserved":true,"sdr_readback_max_rgb_error":sdr_max,
        "hdr_target_mae_ev":mae,"gain_min_ev":map.min_ev,"gain_max_ev":map.max_ev,"offset":map.offset,
        "clipped_negative_channel_fraction":map.clipped_negative_fraction,"gain_map_width":w,"gain_map_height":h,"gain_map_channels":3}),
    )
}

/// Create a new, manifest-last output bundle. Existing files/directories are
/// never replaced. A failed invocation removes only the directory it reserved.
pub fn reconstruct_styles_hdr(
    adapter: &Path,
    input: &Path,
    output: &Path,
    max_edge: u32,
) -> Result<Value> {
    let plan = plan_style_reconstruction(max_edge).map_err(error)?;
    let input = fs::canonicalize(input).map_err(io)?;
    let source = fs::read(&input).map_err(io)?;
    let source_hash = Sha256::digest(&source)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let resources = extract_style_reconstruction_resources(&source)
        .map_err(|e| RuntimeError::external("embedded Styles resources", e))?;
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(io)?;
    fs::create_dir(output).map_err(io)?;
    let run = (|| {
        let work = tempfile::Builder::new()
            .prefix(".styles-work-")
            .tempdir_in(output)
            .map_err(io)?;
        let dir = work.path();
        fs::write(dir.join("styles.plist"), &resources.metadata).map_err(io)?;
        fs::write(dir.join("linear.heic"), &resources.linear_thumbnail_heif).map_err(io)?;
        fs::write(dir.join("delta.heic"), &resources.delta_map_heif).map_err(io)?;
        let client = AppleAdapterClient::new(adapter);
        let native=client.styles_primitive(&json!({"schema_version":2,"operation":"styles-decode-reverse", "input_path":input,"output_path":dir,"max_edge":plan.max_edge,
            "supported_metadata_versions":STYLE_RECONSTRUCTION_METADATA_VERSIONS}))?;
        // No fallback to a guessed transfer or a generic inverse tone curve.
        let version = native["metadata_version"]
            .as_u64()
            .ok_or_else(|| error("unknown Styles metadata version"))?;
        if !STYLE_RECONSTRUCTION_METADATA_VERSIONS.contains(&(version as u32)) {
            return Err(error(format!(
                "unsupported Styles metadata version 0.{version}"
            )));
        }
        let orientation = native["orientation"]
            .as_u64()
            .ok_or_else(|| error("missing orientation"))? as u32;
        let (w, h) = geometry(&native["unstyled"])?;
        if geometry(&native["original_hdr"])? != (w, h) {
            return Err(error("native HDR and reverse extents differ"));
        }
        let (tw, th) = geometry(&native["thumbnail"])?;
        let unstyled = read_pixels(&dir.join("unstyled.f32"), w * h)?;
        let hdr = read_pixels(&dir.join("original_hdr.f32"), w * h)?;
        let codes = read_pixels(&dir.join("thumbnail_codes.f32"), tw * th)?;
        let sdr = read_pixels(&dir.join("original_sdr.f32"), w * h)?;
        let reconstructed = reconstruct_relative_scene(&unstyled, &hdr, w, h, &codes, tw, th)
            .map_err(|e| RuntimeError::external("relative scene reconstruction", e))?;
        drop(hdr);
        drop(codes);
        let scene_path = dir.join("scene.f32");
        write_pixels(&scene_path, &reconstructed.pixels)?;
        let scene = encode(
            &client,
            &scene_path,
            &output.join("scene_linear.exr"),
            dir,
            (w, h),
            orientation,
            &reconstructed.pixels,
        )?;
        let heic = hdr_gain_map(
            &client,
            &source,
            &sdr,
            &reconstructed.pixels,
            &output.join("hdr.heic"),
            dir,
            (w, h),
        )?;
        let reverse = encode(
            &client,
            &dir.join("unstyled.f32"),
            &output.join("unstyled.exr"),
            dir,
            (w, h),
            orientation,
            &unstyled,
        )?;
        for key in ["globalToneCurveData", "lightMapData", "linearLightMapData"] {
            let name = format!("{key}.bin");
            if dir.join(&name).exists() {
                fs::copy(dir.join(&name), output.join(&name)).map_err(io)?;
            }
        }
        if hash_file(&input)? != source_hash {
            return Err(error("source changed during processing"));
        }
        let report = json!({"schema_version":1,"algorithm":"embedded-styles-relative-scene-v1",
            "source":input,"source_sha256":source_hash,"source_preserved":true,
            "resources":{"metadata_item_id":resources.metadata_item_id,"linear_thumbnail_item_id":resources.linear_thumbnail_item_id,"delta_map_item_id":resources.delta_map_item_id},
            "native":native,"scene":{"width":w,"height":h,"color_space":"extended-linear Display P3","exposure_ev":reconstructed.exposure_ev,
                "guidance_valid_fraction":reconstructed.guidance_valid_fraction,"limited_fraction":reconstructed.limited_fraction,
                "runtime_raw_inputs":0,"transfer":"measured native Apple Log exponential inverse","absolute_nit_calibration":false},
            "sdr_policy":"original compressed primary and its colour/geometry properties preserved; only HDR Gain Map changes appearance",
            "outputs":{"scene_linear.exr":scene,"hdr.heic":heic,"unstyled.exr":reverse},
            "limitations":["Approximate relative scene luminance; not complete capture ISP inversion or RAW recovery.",
                "Private Apple ABI and embedded resource support are checked at runtime; unknown versions fail closed.",
                "Clipped colour and detail absent from the encoded primary cannot be restored.",
                "Global exposure preserves the source HDR midtones; diffuse white is not calibrated to 203 nits."]});
        fs::write(
            output.join("report.json"),
            serde_json::to_vec_pretty(&report)
                .map_err(|e| RuntimeError::external("Styles report", e))?,
        )
        .map_err(io)?;
        Ok(report)
    })();
    if run.is_err() {
        let _ = fs::remove_dir_all(output);
    }
    run
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_reverse_uses_source_coefficients_and_is_repeatable() {
        let Some(source) = std::env::var_os("XDREMUX_STYLES_TEST_INPUT") else {
            return;
        };
        let coefficient_path = std::env::var_os("XDREMUX_STYLES_TEST_COEFFICIENTS")
            .expect("coefficient control fixture required");
        let adapter = std::env::var_os("XDREMUX_APPLE_ADAPTER_TEST_EXECUTABLE")
            .expect("native adapter required");
        let original = fs::read(&source).unwrap();
        let resources = extract_style_reconstruction_resources(&original).unwrap();
        let coeff = fs::read(coefficient_path).unwrap();
        assert_eq!(
            coeff.len(),
            51840,
            "this native control uses the measured float16 fixture"
        );
        let locations = resources
            .metadata
            .windows(coeff.len())
            .enumerate()
            .filter(|(_, v)| *v == coeff)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        assert_eq!(
            locations.len(),
            1,
            "fixture coefficients must appear once in its bplist"
        );
        let mut identity = vec![0u8; coeff.len()];
        for block in identity.chunks_exact_mut(60) {
            for c in 0..3 {
                let i = ((c + 1) * 3 + c) * 2;
                block[i..i + 2].copy_from_slice(&0x3c00u16.to_le_bytes());
            }
        }
        let mut identity_metadata = resources.metadata.clone();
        identity_metadata[locations[0]..locations[0] + coeff.len()].copy_from_slice(&identity);
        let client = AppleAdapterClient::new(adapter);
        let root = tempfile::tempdir().unwrap();
        let mut outputs = Vec::new();
        for (name, metadata) in [
            ("source", &resources.metadata),
            ("identity", &identity_metadata),
            ("repeat", &resources.metadata),
        ] {
            let dir = root.path().join(name);
            fs::create_dir(&dir).unwrap();
            fs::write(dir.join("styles.plist"), metadata).unwrap();
            fs::write(dir.join("linear.heic"), &resources.linear_thumbnail_heif).unwrap();
            fs::write(dir.join("delta.heic"), &resources.delta_map_heif).unwrap();
            let facts = client
                .styles_primitive(
                    &json!({"schema_version":2,"operation":"styles-decode-reverse",
                "input_path":Path::new(&source),"output_path":dir,"max_edge":1024,
                "supported_metadata_versions":STYLE_RECONSTRUCTION_METADATA_VERSIONS}),
                )
                .unwrap();
            let (w, h) = geometry(&facts["unstyled"]).unwrap();
            outputs.push(read_pixels(&dir.join("unstyled.f32"), w * h).unwrap());
        }
        assert_eq!(
            outputs[0], outputs[2],
            "native repeat must be pixel-identical"
        );
        let difference = outputs[0]
            .iter()
            .zip(&outputs[1])
            .flat_map(|(a, b)| (0..3).map(move |c| (a[c] - b[c]).abs() as f64))
            .sum::<f64>()
            / (outputs[0].len() * 3) as f64;
        assert!(
            difference > 0.001,
            "embedded coefficients must materially affect the native reversal: {difference}"
        );
        assert_eq!(
            fs::read(source).unwrap(),
            original,
            "native controls must preserve the original"
        );
        println!("Native Styles control: repeat exact; source versus identity mean RGB difference {difference:.8}");
    }
}
