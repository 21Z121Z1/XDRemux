use std::{env, fs, process::ExitCode};

use serde_json::{json, Value};
use xdremux_motion_photo::{
    enrich_oppo_video_range, ftyp_box_offsets, parse_android_motion_photo, parse_first_lpex_object,
    parse_oppo_motion_photo, resolve_heif_motion_photo_ranges, resolve_video_stream_layout,
    ByteRange, MotionPhotoAsset, MotionPhotoItem, OppoMetadata, VideoStreamRole,
};

fn parse_u64(text: &str) -> Result<u64, String> {
    text.parse::<u64>()
        .map_err(|error| format!("invalid integer {text:?}: {error}"))
}

fn parse_i64(text: &str) -> Result<i64, String> {
    text.parse::<i64>()
        .map_err(|error| format!("invalid integer {text:?}: {error}"))
}

fn range_json(range: ByteRange) -> Value {
    json!({"lower": range.lower_bound, "upper": range.upper_bound})
}

fn oppo_metadata_json(metadata: OppoMetadata) -> Value {
    let matrices = metadata
        .matrices
        .into_iter()
        .map(|(key, matrix)| (key, json!(matrix)))
        .collect::<serde_json::Map<String, Value>>();
    json!({
        "coverFramePtsUs": metadata.cover_frame_pts_us,
        "version": metadata.version,
        "matrixCount": metadata.matrix_count,
        "photoCropMatrix": metadata.photo_crop_matrix,
        "photoEisMatrix": metadata.photo_eis_matrix,
        "matrices": matrices,
        "videoWidth": metadata.video_width,
        "videoHeight": metadata.video_height,
        "originPhotoWidth": metadata.origin_photo_width,
        "originPhotoHeight": metadata.origin_photo_height,
        "photoEisCropFactor": metadata.photo_eis_crop_factor,
        "eisCropFactor": metadata.eis_crop_factor,
        "photoCropFactor": metadata.photo_crop_factor,
        "streamCount": metadata.stream_count,
    })
}

fn asset_json(asset: MotionPhotoAsset) -> Value {
    let vendor_metadata = asset
        .vendor_metadata
        .map(oppo_metadata_json)
        .unwrap_or(Value::Null);
    let items = asset
        .items
        .into_iter()
        .map(|item| {
            json!({
                "mime": item.mime,
                "semantic": item.semantic,
                "length": item.length,
                "padding": item.padding,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "status": "asset",
        "sourceKind": asset.source_kind.as_str(),
        "items": items,
        "still": range_json(asset.still_resource_range),
        "video": range_json(asset.video_resource_range),
        "presentationTimestampUs": asset.presentation_timestamp_us,
        "presentationSource": asset.presentation_source.map(|value| value.as_str()),
        "vendorMetadata": vendor_metadata,
    })
}

fn metadata_json(data: &[u8]) -> Value {
    let Some(metadata) = parse_first_lpex_object(data) else {
        return Value::Null;
    };
    oppo_metadata_json(metadata)
}

fn run() -> Result<Value, String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let mode = args.first().map(String::as_str).ok_or("missing mode")?;
    match mode {
        "android" => {
            if args.len() != 2 {
                return Err("usage: motion_photo_conformance android <file>".into());
            }
            let data = fs::read(&args[1]).map_err(|error| error.to_string())?;
            Ok(match parse_android_motion_photo(&data) {
                Ok(Some(asset)) => asset_json(asset),
                Ok(None) => json!({"status": "none"}),
                Err(error) => json!({"status": "error", "code": error.code()}),
            })
        }
        "oppo" => {
            if args.len() != 2 {
                return Err("usage: motion_photo_conformance oppo <file>".into());
            }
            let data = fs::read(&args[1]).map_err(|error| error.to_string())?;
            Ok(match parse_oppo_motion_photo(&data) {
                Ok(Some(asset)) => asset_json(asset),
                Ok(None) => json!({"status": "none"}),
                Err(error) => json!({"status": "error", "code": error.code()}),
            })
        }
        "lpex" => {
            if args.len() != 2 {
                return Err("usage: motion_photo_conformance lpex <file>".into());
            }
            let data = fs::read(&args[1]).map_err(|error| error.to_string())?;
            Ok(metadata_json(&data))
        }
        "scan" => {
            if args.len() != 4 {
                return Err("usage: motion_photo_conformance scan <file> <lower> <upper>".into());
            }
            let data = fs::read(&args[1]).map_err(|error| error.to_string())?;
            let range = ByteRange::new(parse_u64(&args[2])?, parse_u64(&args[3])?)
                .map_err(|error| error.to_string())?;
            let offsets = ftyp_box_offsets(&data, range, 64).map_err(|error| error.to_string())?;
            Ok(json!({"offsets": offsets}))
        }
        "heif" => {
            if args.len() != 3 {
                return Err("usage: motion_photo_conformance heif <file> <motion-length>".into());
            }
            let data = fs::read(&args[1]).map_err(|error| error.to_string())?;
            let motion_length = parse_u64(&args[2])?;
            let items = [
                MotionPhotoItem {
                    mime: "image/heic".into(),
                    semantic: "Primary".into(),
                    length: 0,
                    padding: 8,
                },
                MotionPhotoItem {
                    mime: "video/mp4".into(),
                    semantic: "MotionPhoto".into(),
                    length: motion_length,
                    padding: 0,
                },
            ];
            let (still, video) = resolve_heif_motion_photo_ranges(&data, &items)
                .map_err(|error| error.to_string())?;
            Ok(json!({"still": range_json(still), "video": range_json(video)}))
        }
        "topology" => {
            if args.len() != 6 {
                return Err("usage: motion_photo_conformance topology <file> <still-upper> <video-lower> <video-upper> <lpex-version>".into());
            }
            let data = fs::read(&args[1]).map_err(|error| error.to_string())?;
            let declared_still =
                ByteRange::new(0, parse_u64(&args[2])?).map_err(|error| error.to_string())?;
            let declared_video = ByteRange::new(parse_u64(&args[3])?, parse_u64(&args[4])?)
                .map_err(|error| error.to_string())?;
            let version = parse_i64(&args[5])?;
            let (still, video, stream_count) =
                enrich_oppo_video_range(&data, declared_still, declared_video, version)
                    .map_err(|error| error.to_string())?;
            let layout = resolve_video_stream_layout(&data, video, true, stream_count)
                .map_err(|error| error.to_string())?;
            let role = match layout.primary.role {
                VideoStreamRole::Primary => "primary",
                VideoStreamRole::AuxiliaryGeometry => "auxiliaryGeometry",
            };
            let auxiliary = layout
                .auxiliary_geometry
                .into_iter()
                .map(|stream| {
                    let role = match stream.role {
                        VideoStreamRole::Primary => "primary",
                        VideoStreamRole::AuxiliaryGeometry => "auxiliaryGeometry",
                    };
                    json!({
                        "index": stream.index,
                        "role": role,
                        "range": range_json(stream.range),
                    })
                })
                .collect::<Vec<_>>();
            Ok(json!({
                "still": range_json(still),
                "video": range_json(video),
                "streamCount": stream_count,
                "primary": {
                    "index": layout.primary.index,
                    "role": role,
                    "range": range_json(layout.primary.range),
                },
                "auxiliaryGeometry": auxiliary,
            }))
        }
        _ => Err(format!("unknown mode: {mode}")),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(value) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
