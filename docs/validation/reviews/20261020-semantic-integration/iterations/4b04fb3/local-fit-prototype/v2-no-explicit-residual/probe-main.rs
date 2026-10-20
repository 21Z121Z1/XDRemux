use std::{fs, hint::black_box, path::Path, time::Instant};
use sha2::{Digest, Sha256};
use xdremux_codec::{HeifRasterDecodeRequest, LibHeifProvider, RasterPixelFormat};
use xdremux_engine::{apple_style_revert_local_data, AppleStyleRevertRequest, RasterDecoder};
use xdremux_hdr::{area_resize_linear_rgb, SceneLinearRgb};

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}
fn srgb_decode(v: f32) -> f32 {
    if v <= 0.040_45 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}
fn prepare(fixture: &Path, output: &Path) {
    let source = fs::read(fixture).unwrap();
    let source_hash = hash(&source);
    let raster = LibHeifProvider::new().decode_raster(&HeifRasterDecodeRequest {
        data: source, format: RasterPixelFormat::Rgb8,
    }).unwrap();
    raster.validate().unwrap();
    let mut samples = Vec::with_capacity(raster.width as usize * raster.height as usize * 3);
    for row in 0..raster.height as usize {
        for &v in &raster.data[row * raster.bytes_per_row..row * raster.bytes_per_row + raster.width as usize * 3] {
            samples.push(srgb_decode(f32::from(v) / 255.0));
        }
    }
    let base = area_resize_linear_rgb(&SceneLinearRgb {
        width: raster.width as usize, height: raster.height as usize, samples,
    }, 1024).unwrap();
    // Deliberately synthetic target over the real decoded public primary.
    // This is not an HDR-derived Styles target, native conversion, or E2E.
    let target = base.samples.iter().map(|&v| v * (0.85 + 0.1*v)).collect::<Vec<_>>();
    let mut data = b"XDFIT001".to_vec();
    data.extend_from_slice(&(base.width as u64).to_le_bytes());
    data.extend_from_slice(&(base.height as u64).to_le_bytes());
    for &value in base.samples.iter().chain(&target) {
        data.extend_from_slice(&value.to_le_bytes());
    }
    fs::write(output, &data).unwrap();
    println!("{{\"kind\":\"prepared_public_primary_synthetic_target\",\"fixture_sha256\":\"{source_hash}\",\"source_width\":{},\"source_height\":{},\"width\":{},\"height\":{},\"request_sha256\":\"{}\"}}", raster.width, raster.height, base.width, base.height, hash(&data));
}
fn load(path: &Path) -> (usize, usize, Vec<f32>, Vec<f32>) {
    let data = fs::read(path).unwrap();
    assert_eq!(&data[..8], b"XDFIT001");
    let width = u64::from_le_bytes(data[8..16].try_into().unwrap()) as usize;
    let height = u64::from_le_bytes(data[16..24].try_into().unwrap()) as usize;
    let count = width.checked_mul(height).unwrap().checked_mul(3).unwrap();
    assert_eq!(data.len(), 24 + count*8);
    let values = data[24..].chunks_exact(4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).collect::<Vec<_>>();
    (width, height, values[..count].to_vec(), values[count..].to_vec())
}
fn invoke(name: &str, width: usize, height: usize, styled: &[f32], recovered: &[f32], output: &Path, expected_error: bool) {
    let request = AppleStyleRevertRequest { width, height, styled_rgb: styled, recovered_rgb: recovered };
    let started = Instant::now();
    let result = apple_style_revert_local_data(black_box(&request));
    let elapsed = started.elapsed().as_secs_f64();
    match result {
        Ok(data) => {
            assert!(!expected_error, "invalid request unexpectedly succeeded: {name}");
            fs::write(output.join(format!("{name}.bin")), &data).unwrap();
            println!("{{\"case\":\"{name}\",\"status\":\"ok\",\"width\":{width},\"height\":{height},\"wall_seconds\":{elapsed},\"payload_bytes\":{},\"payload_sha256\":\"{}\"}}",data.len(),hash(&data));
        }
        Err(error) => {
            assert!(expected_error, "valid request failed: {name}: {error:?}");
            let error = format!("{error:?}");
            fs::write(output.join(format!("{name}.error.txt")), &error).unwrap();
            println!("{{\"case\":\"{name}\",\"status\":\"error\",\"error\":\"{error}\"}}");
        }
    }
}
fn cases(input: &Path, output: &Path) {
    let (width, height, styled, recovered) = load(input);
    invoke("public-primary-synthetic-target",width,height,&styled,&recovered,output,false);
    let mut seed = 0x2137_f12b_6a90_c4d1_u64;
    for &(name,width,height,kind) in &[
        ("single-zero",1,1,0),("single-signed-zero",1,1,1),
        ("thin-column",1,37,2),("thin-row",41,1,2),("small-rank-deficient",2,2,3),
        ("grid-undersize",11,8,2),("grid-exact",12,9,2),("grid-oversize",13,10,2),
        ("odd-footprints",31,23,2),("even-footprints",32,24,2),
        ("identity-gradient",79,55,4),("random-wide",257,193,2),
        ("constant-rank-deficient",97,71,3),("mixed-negative-and-hdr",67,43,5),
        ("extreme-finite-underflow",23,19,6),
    ] {
        let mut source = Vec::new();
        let mut target = Vec::new();
        for i in 0..width*height*3 {
            seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17;
            let random = ((seed >> 32) as u32) as f64 / u32::MAX as f64;
            let v = match kind {
                0 => 0.0, 1 => if i%2==0 {-0.0} else {0.0},
                3 => 0.4375, 4 => ((i*17%1021) as f32)/1020.0,
                5 => (random*4.0-1.5) as f32,
                6 => if i%2==0 {f32::MAX} else {-f32::MAX},
                _ => random as f32,
            };
            source.push(v);
            target.push(if kind==4 {v} else if kind==6 {-v} else {v*(0.85+0.1*v)});
        }
        invoke(name,width,height,&source,&target,output,false);
    }
    invoke("zero-width",0,1,&[],&[],output,true);
    invoke("zero-height",1,0,&[],&[],output,true);
    invoke("pixel-count-overflow",usize::MAX,2,&[],&[],output,true);
    invoke("value-count-overflow",usize::MAX/3+1,1,&[],&[],output,true);
    invoke("short-source",1,1,&[0.0,1.0],&[0.0,1.0,0.0],output,true);
    invoke("short-target",1,1,&[0.0,1.0,0.0],&[0.0,1.0],output,true);
    for &(name,value,target) in &[("source-nan",f32::NAN,false),("source-inf",f32::INFINITY,false),
        ("target-nan",f32::NAN,true),("target-neg-inf",f32::NEG_INFINITY,true)] {
        let invalid = [0.5,value,0.25];
        let valid = [0.5,0.75,0.25];
        invoke(name,1,1,if target {&valid} else {&invalid},if target {&invalid} else {&valid},output,true);
    }
}
fn main() {
    let args = std::env::args_os().collect::<Vec<_>>();
    let mode = args[1].to_str().unwrap();
    let input = Path::new(&args[2]);
    let output = Path::new(&args[3]);
    match mode {
        "prepare" => prepare(input,output),
        "cases" => {fs::create_dir(output).unwrap(); cases(input,output);}
        "fit" => {
            fs::create_dir(output).unwrap();
            let (width,height,styled,recovered) = load(input);
            invoke("public-primary-synthetic-target",width,height,&styled,&recovered,output,false);
        }
        _ => panic!("unknown mode"),
    }
}
