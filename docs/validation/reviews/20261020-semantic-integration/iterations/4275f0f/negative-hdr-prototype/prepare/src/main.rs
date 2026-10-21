use std::{fs, io::{BufWriter,Write},path::Path};
use xdremux_codec::{HeifRasterDecodeRequest,JpegRasterDecodeRequest,LibHeifProvider,ZuneJpegProvider,RasterPixelFormat};
use xdremux_engine::RasterDecoder;
use xdremux_hdr::{resolve,iso_gain_application_for_alternate,ExtractionMode};
fn srgb_decode(v:f32)->f32 {if v<=0.04045 {v/12.92} else {((v+0.055)/1.055).powf(2.4)}}
fn main() {
 let args:Vec<_>=std::env::args().collect();assert_eq!(args.len(),3);
 let source=fs::read(&args[1]).unwrap();
 let extracted=xdremux_container::extract(&source).unwrap();
 let mode=match extracted.mode {xdremux_container::ExtractionMode::Uhdr=>ExtractionMode::Uhdr,xdremux_container::ExtractionMode::Lhdr=>ExtractionMode::Lhdr};
 assert_eq!(mode,ExtractionMode::Uhdr,"this one-off input preparation expects the public UHDR fixture");
 let scale=resolve(&extracted.meta_floats,mode).unwrap();
 let application=iso_gain_application_for_alternate(&scale).unwrap();
 let raster=LibHeifProvider::new().decode_raster(&HeifRasterDecodeRequest{data:source,format:RasterPixelFormat::Rgb8}).unwrap();raster.validate().unwrap();
 let gain=ZuneJpegProvider::new().decode_raster(&JpegRasterDecodeRequest{data:extracted.mask_jpeg_data,format:RasterPixelFormat::Rgb8}).unwrap();gain.validate().unwrap();
 let path=Path::new(&args[2]);let mut f=BufWriter::new(fs::File::create(path).unwrap());f.write_all(b"HDRPROBE").unwrap();
 for v in [raster.width as u64,raster.height as u64,gain.width as u64,gain.height as u64,gain.bytes_per_row as u64,3,gain.data.len() as u64] {f.write_all(&v.to_le_bytes()).unwrap();}
 for c in application.channels {for v in [c.minimum_log2,c.maximum_log2,c.gamma,c.base_offset,c.alternate_offset] {f.write_all(&v.to_le_bytes()).unwrap();}}
 f.write_all(&application.weight.to_le_bytes()).unwrap();
 for y in 0..raster.height as usize {for v in &raster.data[y*raster.bytes_per_row..y*raster.bytes_per_row+raster.width as usize*3] {f.write_all(&srgb_decode(f32::from(*v)/255.0).to_le_bytes()).unwrap();}}
 f.write_all(&gain.data).unwrap();f.flush().unwrap();
 println!("public_source_primary={}x{} gain_private_jpeg={}x{} gain_channels=3 source_mode=UHDR scale_source={} application={:?}",raster.width,raster.height,gain.width,gain.height,scale.source,application);
 println!("Input uses source primary HEIF + source private JPEG gain codes, not the native semantic-base/HEVC-reencoded E2E raster. All decoding/preparation is outside timed reconstruction.");
}
