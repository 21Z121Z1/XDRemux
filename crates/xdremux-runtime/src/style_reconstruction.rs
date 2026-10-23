use std::fs;
use std::io::{BufWriter, Read, Write};
use std::path::Path;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use xdremux_codec::{GainMapTileEncodeRequest, LibHeifProvider, Raster8, RasterPixelFormat};
use xdremux_engine::{
    plan_style_display, plan_style_reconstruction, GainMapCodecLayout, GainMapTileEncoder,
    STYLE_RECONSTRUCTION_METADATA_VERSIONS,
};
use xdremux_format::ChromaSampling;
use xdremux_hdr::{
    display_p3_luminance, make_scene_gain_map, reconstruct_relative_scene, DisplayFamily,
    DisplayFamilyPolicy, DisplayMethods, LinearPixel, SceneGuidancePolicy,
};
use xdremux_heif::{
    assemble_iso_gain_map_heif, extract_style_display_mattes,
    extract_style_reconstruction_resources, primary_image_signature, validate_gain_map_structure,
    DirectHevcGainMap, GainMapChannels, GainMapEncodeProfile, GainMapTile, IsoGainMapAssembly,
    StyleDisplayMatte,
};
use xdremux_metadata::{make_scene_gain_map_metadata, make_scene_gain_map_metadata_with_capacity};

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

fn hdr_transport_metrics(target: &[LinearPixel], actual: &[LinearPixel]) -> Result<Value> {
    let mut errors = Vec::new();
    let mut negative = 0usize;
    let mut nonpositive = 0usize;
    let mut maximum = f32::NEG_INFINITY;
    let mut rgb_sum = 0f64;
    let mut rgb_max = 0f32;
    for (a, b) in target.iter().zip(actual) {
        let ya = display_p3_luminance(a);
        let yb = display_p3_luminance(b);
        nonpositive += usize::from(yb <= 0.);
        for c in 0..3 {
            negative += usize::from(b[c] < 0.);
            maximum = maximum.max(b[c]);
            let e = (a[c] - b[c]).abs();
            rgb_sum += e as f64;
            rgb_max = rgb_max.max(e);
        }
        if ya > 1e-4 {
            errors.push((yb.max(1e-8) / ya).log2().abs());
        }
    }
    if errors.is_empty() {
        return Err(error(
            "no positive HDR target pixels for transport measurement",
        ));
    }
    errors.sort_by(f32::total_cmp);
    let mean = errors.iter().map(|v| *v as f64).sum::<f64>() / errors.len() as f64;
    let index = (errors.len() - 1) as f64 * 0.99;
    let lo = index.floor() as usize;
    let hi = index.ceil() as usize;
    let p99 = errors[lo] as f64 + (errors[hi] - errors[lo]) as f64 * (index - lo as f64);
    Ok(
        json!({"mean_abs_ev":mean,"p99_abs_ev":p99,"max_abs_ev":errors.last(),"pixels":errors.len(),
        "decoded_negative_rgb_components":negative,"decoded_nonpositive_y_pixels":nonpositive,"decoded_max_rgb":maximum,
        "mean_abs_rgb":rgb_sum/(target.len()*3)as f64,"max_abs_rgb":rgb_max,"nonfinite_rgb_components":0,
        "ev_valid_rule":"targetY>1e-4; decoded nonpositiveY counted and floored at1e-8, never silently excluded"}),
    )
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

#[allow(clippy::too_many_arguments)]
fn hdr_gain_map(
    client: &AppleAdapterClient,
    source: &[u8],
    sdr: &[LinearPixel],
    target: &[LinearPixel],
    output: &Path,
    work: &Path,
    size: (usize, usize),
    display_headroom: Option<u32>,
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
    let metadata = if let Some(h) = display_headroom {
        if target
            .iter()
            .any(|p| p[..3].iter().any(|v| *v > h as f32 + 2e-6))
        {
            return Err(error("display target exceeds declared headroom"));
        }
        make_scene_gain_map_metadata_with_capacity(
            map.min_ev as f64,
            map.max_ev as f64,
            map.offset as f64,
            (h as f64).log2(),
        )
    } else {
        make_scene_gain_map_metadata(map.min_ev as f64, map.max_ev as f64, map.offset as f64)
    };
    let (tmap, xmp) = metadata.map_err(|e| RuntimeError::external("scene Gain Map metadata", e))?;
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
    let consumer=client.styles_primitive(&json!({"schema_version":2,"operation":"styles-readback","input_path":output,"output_path":work,"width":w,"height":h}))?;
    if let Some(h) = display_headroom {
        if consumer["hdr_content_headroom"]
            .as_f64()
            .is_none_or(|v| (v - h as f64).abs() > 1e-5)
        {
            return Err(error(
                "native consumer headroom differs from display target",
            ));
        }
    }
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
    let transport = hdr_transport_metrics(target, &hdr)?;
    let mae = transport["mean_abs_ev"]
        .as_f64()
        .ok_or_else(|| error("invalid transport metrics"))?;
    if mae > 0.15 {
        return Err(error(format!(
            "published HDR differs from scene target by {mae:.6} EV"
        )));
    }
    Ok(
        json!({"sha256":hash_file(output)?,"compressed_sdr_preserved":true,"sdr_readback_max_rgb_error":sdr_max,
        "hdr_target_mae_ev":mae,"gain_min_ev":map.min_ev,"gain_max_ev":map.max_ev,"offset":map.offset,
        "clipped_negative_channel_fraction":map.clipped_negative_fraction,"gain_map_width":w,"gain_map_height":h,"gain_map_channels":3,
        "consumer_content_headroom":consumer["hdr_content_headroom"],"transport":transport}),
    )
}

fn display_matte(
    client: &AppleAdapterClient,
    matte: Option<StyleDisplayMatte>,
    dir: &Path,
    name: &str,
    w: usize,
    h: usize,
) -> Result<(Vec<f32>, Value)> {
    let Some(matte) = matte else {
        return Ok((
            vec![0.; w * h],
            json!({"role":name,"status":"missing-explicit-zero"}),
        ));
    };
    let input = dir.join(format!("{name}.heic"));
    let output = dir.join(format!("{name}.f32"));
    fs::write(&input, &matte.heif).map_err(io)?;
    let native=client.styles_primitive(&json!({"schema_version":2,"operation":"styles-decode-matte","input_path":input,"output_path":output,"width":w,"height":h}))?;
    let bytes = fs::read(&output).map_err(io)?;
    if bytes.len() != w * h * 4 {
        return Err(error("matte geometry mismatch"));
    }
    let pixels = bytes
        .chunks_exact(4)
        .map(|v| f32::from_le_bytes(v.try_into().unwrap()))
        .collect();
    Ok((
        pixels,
        json!({"role":name,"item_id":matte.item_id,"uri":matte.uri,"foreground_proxy":matte.foreground_proxy,
        "standalone_sha256":hash_file(&input)?,"pixels_sha256":hash_file(&output)?,"native":native}),
    ))
}

#[allow(clippy::too_many_arguments)]
fn verify_display_family(
    client: &AppleAdapterClient,
    input: &Path,
    work: &Path,
    scene: &[LinearPixel],
    sdr: &[LinearPixel],
    skin: &[f32],
    foreground: &[f32],
    size: (usize, usize),
    family: &DisplayFamily,
    methods: DisplayMethods,
    capacity: u32,
) -> Result<Value> {
    let (w, h) = size;
    let mut rows = Vec::new();
    for requested in [1f32, 1.5, 2., 3., 4., 6., 8.] {
        let target_headroom = requested.min(capacity as f32);
        let expected = family
            .render(scene, sdr, skin, foreground, w, h, target_headroom, methods)
            .map_err(|e| RuntimeError::external("display family target", e))?;
        let native=client.styles_primitive(&json!({"schema_version":2,"operation":"styles-readback",
            "input_path":input,"output_path":work,"width":w,"height":h,"target_headroom":requested}))?;
        if geometry(&native["target"])? != (w, h) {
            return Err(error("display family consumer geometry changed"));
        }
        let actual = read_pixels(&work.join("published_target.f32"), w * h)?;
        let metrics = hdr_transport_metrics(&expected.pixels, &actual)?;
        // Native log-gain application, codec transport, range projection and
        // clipped extended SDR all remain in these errors, including black.
        if metrics["mean_abs_ev"].as_f64().is_none_or(|e| e > 0.05) {
            return Err(error(format!(
                "same-file display family error exceeds 0.05 EV at {requested}x: {metrics}"
            )));
        }
        let schedule = family
            .schedule(target_headroom)
            .map_err(|e| RuntimeError::external("display schedule", e))?;
        rows.push(
            json!({"requested_headroom":requested,"target_headroom":target_headroom,
            "master_weight":schedule.master_weight,"carrier_weight":target_headroom.log2()/(capacity as f32).log2(),"retained_local":schedule.retained_local,
            "unconstrained_retained_local":schedule.unconstrained_retained_local,
            "target_range_limited_fraction":expected.range_limited_fraction,
            "negative_sdr_pixel_fraction":expected.negative_sdr_fraction,
            "native":native,"transport_and_projection":metrics}),
        );
    }
    Ok(
        json!({"same_encoded_file":true,"method":"native CIImage imageByApplyingGainMap:headroom: on encoded auxiliary",
        "mean_abs_ev_limit":0.05,"rows":rows,"physical_HDR_visual_acceptance":false}),
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
    reconstruct_styles_hdr_with_display(adapter, input, output, max_edge, None)
}

/// Optional display rendering uses independently callable methods on R, not
/// a native output ratio or Apple's editor parameter system.
pub fn reconstruct_styles_hdr_with_display(
    adapter: &Path,
    input: &Path,
    output: &Path,
    max_edge: u32,
    display_headroom: Option<u32>,
) -> Result<Value> {
    let plan = plan_style_reconstruction(max_edge).map_err(error)?;
    let display_plan = display_headroom
        .map(plan_style_display)
        .transpose()
        .map_err(error)?;
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
        let reconstructed = reconstruct_relative_scene(
            &unstyled,
            &hdr,
            &sdr,
            (w, h),
            &codes,
            (tw, th),
            SceneGuidancePolicy {
                analysis_edge: plan.guidance_analysis_edge,
                primary_span_ev: plan.guidance_primary_span_ev,
                minimum_region_pixels: plan.guidance_minimum_region_pixels,
                feather_radius: plan.guidance_feather_radius,
                merge_radius: plan.guidance_merge_radius,
                detail_radius: plan.guidance_detail_radius,
                detail_threshold_ev: plan.guidance_detail_threshold_ev,
                detail_limit_ev: plan.guidance_detail_limit_ev,
                minimum_gain_support: plan.guidance_minimum_gain_support,
                edge_ev: plan.guidance_edge_ev,
                flat_ev: plan.guidance_flat_ev,
                limit_ev: plan.guidance_limit_ev,
            },
        )
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
        let mut display_pixels = None;
        let mut display_context = None;
        let mut display_facts = Value::Null;
        if let Some(display) = display_plan {
            let (skin, foreground) = extract_style_display_mattes(&source)
                .map_err(|e| RuntimeError::external("same-photo semantic masks", e))?;
            let (skin, skin_facts) = display_matte(&client, skin, dir, "skin", w, h)?;
            let (foreground, foreground_facts) =
                display_matte(&client, foreground, dir, "foreground", w, h)?;
            let [mb, mf, ab, ask, sb, sh, cg, fp] = display.methods;
            let methods = DisplayMethods {
                mids_background: mb,
                mids_foreground: mf,
                anchor_background: ab,
                anchor_skin: ask,
                skin_brightness: sb,
                skin_highlight: sh,
                chroma_gain: cg,
                foreground_chroma_protection: fp,
            };
            let family = DisplayFamily::analyze(
                &reconstructed.pixels,
                &sdr,
                w,
                h,
                DisplayFamilyPolicy {
                    analysis_edge: display.analysis_edge,
                    local_limit_ev: display.local_limit_ev,
                },
            )
            .map_err(|e| RuntimeError::external("display family analysis", e))?;
            let result = family
                .render(
                    &reconstructed.pixels,
                    &sdr,
                    &skin,
                    &foreground,
                    w,
                    h,
                    display.headroom as f32,
                    methods,
                )
                .map_err(|e| RuntimeError::external("display family rendering", e))?;
            display_facts = json!({"algorithm":"scene-gap-coupled-log-family-v2","headroom":display.headroom,
                "family_master_headroom":8,"exposure_ev":family.exposure_ev,"target_max_rgb":result.maximum_rgb,
                "target_range_limited_fraction":result.range_limited_fraction,"methods":{
                    "mids_background":mb,"mids_foreground":mf,"anchor_background":ab,"anchor_skin":ask,
                    "skin_brightness":sb,"skin_highlight":sh,"chroma_gain":cg,"foreground_chroma_protection":fp},
                "masks":[skin_facts,foreground_facts],"runtime_raw_inputs":0,"native_editor_parameters_used":false,"native_output_ratio_used":false,
                "working_domain":"component-linear Display P3; offset-log RGB family, common 8x scene-derived global endpoint",
                "local_compensation":{"scene_range_ev":family.scene_range_ev,"sdr_global_range_ev":family.sdr_global_range_ev,
                    "range_gap_proxy_ev":family.range_gap_ev,"residual_rms_ev":family.local_rms_ev,
                    "residual_limit_ev":display.local_limit_ev,"limited_fraction":family.limited_fraction,
                    "analysis_edge":display.analysis_edge,"monotone_curve":family.curve,
                    "schedule":"1-log2(headroom)/max(3,range_gap_proxy_ev)",
                    "attribution":"bounded coarse SDR spatial residual against monotone scene-to-SDR fit; proxy, not capture-ISP inversion"},
                "guide":"area luminance; joint bilateral residual sigma spatial4/range0.5 at long edge up to512; scene-edge-aware upsampling",
                "color":"extended source SDR relative opponent chroma, fixed post-tone luminance and gamut fit",
                "texture":"source detail and original texture metadata retained; texture renderer not rerun",
                "physical_HDR_visual_acceptance":false});
            let path = dir.join("display.f32");
            write_pixels(&path, &result.pixels)?;
            let encoded = encode(
                &client,
                &path,
                &output.join("display_linear.exr"),
                dir,
                (w, h),
                orientation,
                &result.pixels,
            )?;
            display_facts["exr"] = encoded;
            display_pixels = Some(result.pixels);
            display_context = Some((family, skin, foreground, methods));
        }
        let heic = hdr_gain_map(
            &client,
            &source,
            &sdr,
            display_pixels.as_deref().unwrap_or(&reconstructed.pixels),
            &output.join("hdr.heic"),
            dir,
            (w, h),
            display_headroom,
        )?;
        if let Some((family, skin, foreground, methods)) = display_context {
            display_facts["headroom_validation"] = verify_display_family(
                &client,
                &output.join("hdr.heic"),
                dir,
                &reconstructed.pixels,
                &sdr,
                &skin,
                &foreground,
                (w, h),
                &family,
                methods,
                display_headroom.unwrap(),
            )?;
        }
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
        let report = json!({"schema_version":1,"algorithm":"embedded-styles-relative-scene-v4",
            "source":input,"source_sha256":source_hash,"source_preserved":true,
            "resources":{"metadata_item_id":resources.metadata_item_id,"linear_thumbnail_item_id":resources.linear_thumbnail_item_id,"delta_map_item_id":resources.delta_map_item_id},
            "native":native,"scene":{"width":w,"height":h,"color_space":"extended-linear Display P3","exposure_ev":reconstructed.exposure_ev,
                "guidance_valid_fraction":reconstructed.guidance_valid_fraction,"limited_fraction":reconstructed.limited_fraction,
                "guidance_stabilization":{"algorithm":"bounded-base-guided-region-gain-v3",
                    "width":reconstructed.guidance.width,"height":reconstructed.guidance.height,
                    "analysis_edge":plan.guidance_analysis_edge,
                    "guide":"full-resolution linear SDR base",
                    "primary_span_ev":plan.guidance_primary_span_ev,"minimum_region_pixels_at_analysis_edge":plan.guidance_minimum_region_pixels,
                    "feather_radius_at_analysis_edge":plan.guidance_feather_radius,"merge_radius_at_analysis_edge":plan.guidance_merge_radius,
                    "edge_ev":plan.guidance_edge_ev,"flat_ev":plan.guidance_flat_ev,"limit_ev":plan.guidance_limit_ev,
                    "regions":reconstructed.guidance.regions,"region_fraction":reconstructed.guidance.region_fraction,
                    "detail_radius_at_analysis_edge":plan.guidance_detail_radius,"detail_threshold_ev":plan.guidance_detail_threshold_ev,
                    "detail_limit_ev":plan.guidance_detail_limit_ev,"detail_fraction":reconstructed.guidance.detail_fraction,
                    "minimum_gain_support":plan.guidance_minimum_gain_support,
                    "adjustment_rms_ev":reconstructed.guidance.adjustment_rms_ev,"maximum_adjustment_ev":reconstructed.guidance.maximum_adjustment_ev},
                "runtime_raw_inputs":0,"transfer":"measured native Apple Log exponential inverse","absolute_nit_calibration":false},
            "sdr_policy":"original compressed primary and its colour/geometry properties preserved; only HDR Gain Map changes appearance",
            "outputs":{"scene_linear.exr":scene,"hdr.heic":heic,"unstyled.exr":reverse},"display":display_facts,
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
    fn negative_consumer_luminance_remains_in_hdr_error_tails() {
        let result =
            hdr_transport_metrics(&[[1., 1., 1., 1.]], &[[-0.01, -0.01, -0.01, 1.]]).unwrap();
        assert_eq!(result["pixels"], 1);
        assert_eq!(result["decoded_negative_rgb_components"], 3);
        assert_eq!(result["decoded_nonpositive_y_pixels"], 1);
        assert!(result["mean_abs_ev"].as_f64().unwrap() > 20.);
    }

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
