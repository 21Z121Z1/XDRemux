use std::error::Error;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use xdremux_format::isobmff::{make_box, make_full_box, parse_boxes, BoxHeader};
use xdremux_format::FourCC;

use crate::OppoMetadata;

const FTYP: FourCC = FourCC::new(*b"ftyp");
const MOOV: FourCC = FourCC::new(*b"moov");
const MVHD: FourCC = FourCC::new(*b"mvhd");
const TRAK: FourCC = FourCC::new(*b"trak");
const TKHD: FourCC = FourCC::new(*b"tkhd");
const MDIA: FourCC = FourCC::new(*b"mdia");
const MDHD: FourCC = FourCC::new(*b"mdhd");
const HDLR: FourCC = FourCC::new(*b"hdlr");
const MINF: FourCC = FourCC::new(*b"minf");
const STBL: FourCC = FourCC::new(*b"stbl");
const STTS: FourCC = FourCC::new(*b"stts");
const CTTS: FourCC = FourCC::new(*b"ctts");
const EDTS: FourCC = FourCC::new(*b"edts");
const ELST: FourCC = FourCC::new(*b"elst");
const META: FourCC = FourCC::new(*b"meta");
const KEYS: FourCC = FourCC::new(*b"keys");
const ILST: FourCC = FourCC::new(*b"ilst");
const DATA: FourCC = FourCC::new(*b"data");
const MDTA: FourCC = FourCC::new(*b"mdta");
const MDAT: FourCC = FourCC::new(*b"mdat");
const FREE: FourCC = FourCC::new(*b"free");
const GMHD: FourCC = FourCC::new(*b"gmhd");
const GMIN: FourCC = FourCC::new(*b"gmin");
const DINF: FourCC = FourCC::new(*b"dinf");
const DREF: FourCC = FourCC::new(*b"dref");
const ALIS: FourCC = FourCC::new(*b"alis");
const MEBX: FourCC = FourCC::new(*b"mebx");
const STSD: FourCC = FourCC::new(*b"stsd");
const STSC: FourCC = FourCC::new(*b"stsc");
const STSZ: FourCC = FourCC::new(*b"stsz");
const STCO: FourCC = FourCC::new(*b"stco");
const CO64: FourCC = FourCC::new(*b"co64");

const QUICKTIME_EPOCH_OFFSET: u64 = 2_082_844_800;
const METADATA_TIMESCALE: u32 = 600;
const CONTENT_IDENTIFIER_KEY: &[u8] = b"com.apple.quicktime.content.identifier";
const STILL_IMAGE_KEY: &[u8] = b"com.apple.quicktime.still-image-time";
const TRANSFORM_KEY: &[u8] = b"com.apple.quicktime.live-photo-still-image-transform";
const REFERENCE_DIMENSIONS_KEY: &[u8] =
    b"com.apple.quicktime.live-photo-still-image-transform-reference-dimensions";
const LEGACY_COLOROS16_EIS_COMPENSATION_SCALE: f64 = 0.90;
const MAX_VIDEO_SAMPLES: usize = 10_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LivePhotoMovieError(String);

impl LivePhotoMovieError {
    fn invalid(detail: impl Into<String>) -> Self {
        Self(detail.into())
    }

    fn format(context: &'static str, error: impl fmt::Display) -> Self {
        Self(format!("{context}: {error}"))
    }
}

impl fmt::Display for LivePhotoMovieError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for LivePhotoMovieError {}

pub type LivePhotoMovieResult<T> = std::result::Result<T, LivePhotoMovieError>;

fn boxes(data: &[u8], start: usize, end: usize) -> LivePhotoMovieResult<Vec<BoxHeader>> {
    parse_boxes(data, start..end)
        .map_err(|error| LivePhotoMovieError::format("QuickTime box parse", error))
}

fn top_level(data: &[u8]) -> LivePhotoMovieResult<Vec<BoxHeader>> {
    boxes(data, 0, data.len())
}

fn one<'a>(
    items: &'a [BoxHeader],
    kind: FourCC,
    context: &str,
) -> LivePhotoMovieResult<&'a BoxHeader> {
    let mut matches = items.iter().filter(|item| item.kind == kind);
    let first = matches
        .next()
        .ok_or_else(|| LivePhotoMovieError::invalid(format!("missing {context}")))?;
    if matches.next().is_some() {
        return Err(LivePhotoMovieError::invalid(format!(
            "{context} appears more than once"
        )));
    }
    Ok(first)
}

fn child(
    data: &[u8],
    parent: &BoxHeader,
    kind: FourCC,
    context: &str,
) -> LivePhotoMovieResult<BoxHeader> {
    boxes(data, parent.data_start, parent.data_end)?
        .into_iter()
        .find(|item| item.kind == kind)
        .ok_or_else(|| LivePhotoMovieError::invalid(format!("missing {context}")))
}

fn read_u32(data: &[u8], offset: usize, context: &str) -> LivePhotoMovieResult<u32> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| LivePhotoMovieError::invalid(format!("truncated {context}")))?;
    Ok(u32::from_be_bytes(
        bytes.try_into().expect("four-byte slice"),
    ))
}

fn read_i32(data: &[u8], offset: usize, context: &str) -> LivePhotoMovieResult<i32> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| LivePhotoMovieError::invalid(format!("truncated {context}")))?;
    Ok(i32::from_be_bytes(
        bytes.try_into().expect("four-byte slice"),
    ))
}

fn read_u64(data: &[u8], offset: usize, context: &str) -> LivePhotoMovieResult<u64> {
    let bytes = data
        .get(offset..offset + 8)
        .ok_or_else(|| LivePhotoMovieError::invalid(format!("truncated {context}")))?;
    Ok(u64::from_be_bytes(
        bytes.try_into().expect("eight-byte slice"),
    ))
}

fn read_i64(data: &[u8], offset: usize, context: &str) -> LivePhotoMovieResult<i64> {
    let bytes = data
        .get(offset..offset + 8)
        .ok_or_else(|| LivePhotoMovieError::invalid(format!("truncated {context}")))?;
    Ok(i64::from_be_bytes(
        bytes.try_into().expect("eight-byte slice"),
    ))
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn push_i16(output: &mut Vec<u8>, value: i16) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn push_i32(output: &mut Vec<u8>, value: i32) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn push_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn make_box_checked(kind: FourCC, payload: &[u8]) -> LivePhotoMovieResult<Vec<u8>> {
    make_box(kind, payload)
        .map_err(|error| LivePhotoMovieError::format("QuickTime box build", error))
}

fn make_full_box_checked(
    kind: FourCC,
    version: u8,
    flags: u32,
    payload: &[u8],
) -> LivePhotoMovieResult<Vec<u8>> {
    make_full_box(kind, version, flags, payload)
        .map_err(|error| LivePhotoMovieError::format("QuickTime full-box build", error))
}

fn movie_timescale(data: &[u8], moov: &BoxHeader) -> LivePhotoMovieResult<(u32, u64)> {
    let mvhd = child(data, moov, MVHD, "mvhd box")?;
    let version = *data
        .get(mvhd.data_start)
        .ok_or_else(|| LivePhotoMovieError::invalid("truncated mvhd version"))?;
    let (timescale_offset, duration_offset, duration_width) = match version {
        0 => (mvhd.box_start + 20, mvhd.box_start + 24, 4),
        1 => (mvhd.box_start + 28, mvhd.box_start + 32, 8),
        other => {
            return Err(LivePhotoMovieError::invalid(format!(
                "unsupported mvhd version {other}"
            )))
        }
    };
    let timescale = read_u32(data, timescale_offset, "mvhd timescale")?;
    if timescale == 0 {
        return Err(LivePhotoMovieError::invalid("invalid movie timescale"));
    }
    let duration = if duration_width == 4 {
        u64::from(read_u32(data, duration_offset, "mvhd duration")?)
    } else {
        read_u64(data, duration_offset, "mvhd duration")?
    };
    Ok((timescale, duration))
}

fn track_id(data: &[u8], track: &BoxHeader) -> LivePhotoMovieResult<u32> {
    let tkhd = child(data, track, TKHD, "tkhd box")?;
    let version = *data
        .get(tkhd.data_start)
        .ok_or_else(|| LivePhotoMovieError::invalid("truncated tkhd version"))?;
    let offset = match version {
        0 => tkhd.box_start + 20,
        1 => tkhd.box_start + 28,
        other => {
            return Err(LivePhotoMovieError::invalid(format!(
                "unsupported tkhd version {other}"
            )))
        }
    };
    read_u32(data, offset, "tkhd track id")
}

fn handler_type(data: &[u8], track: &BoxHeader) -> LivePhotoMovieResult<[u8; 4]> {
    let mdia = child(data, track, MDIA, "mdia box")?;
    let hdlr = child(data, &mdia, HDLR, "hdlr box")?;
    let bytes = data
        .get(hdlr.data_start + 8..hdlr.data_start + 12)
        .ok_or_else(|| LivePhotoMovieError::invalid("truncated media handler"))?;
    Ok(bytes.try_into().expect("four-byte handler type"))
}

fn media_timescale(data: &[u8], track: &BoxHeader) -> LivePhotoMovieResult<u32> {
    let mdia = child(data, track, MDIA, "mdia box")?;
    let mdhd = child(data, &mdia, MDHD, "mdhd box")?;
    let version = *data
        .get(mdhd.data_start)
        .ok_or_else(|| LivePhotoMovieError::invalid("truncated mdhd version"))?;
    let offset = match version {
        0 => mdhd.box_start + 20,
        1 => mdhd.box_start + 28,
        other => {
            return Err(LivePhotoMovieError::invalid(format!(
                "unsupported mdhd version {other}"
            )))
        }
    };
    let value = read_u32(data, offset, "mdhd timescale")?;
    if value == 0 {
        return Err(LivePhotoMovieError::invalid("invalid media timescale"));
    }
    Ok(value)
}

fn sample_pts_seconds(data: &[u8], track: &BoxHeader) -> LivePhotoMovieResult<Vec<f64>> {
    let timescale = media_timescale(data, track)?;
    let mdia = child(data, track, MDIA, "mdia box")?;
    let minf = child(data, &mdia, MINF, "minf box")?;
    let stbl = child(data, &minf, STBL, "stbl box")?;
    let stts = child(data, &stbl, STTS, "stts box")?;
    let mut cursor = stts.data_start + 4;
    let entry_count = usize::try_from(read_u32(data, cursor, "stts entry count")?)
        .map_err(|_| LivePhotoMovieError::invalid("stts entry count exceeds usize"))?;
    cursor += 4;
    let mut dts = Vec::new();
    let mut clock = 0_u64;
    for _ in 0..entry_count {
        let count = usize::try_from(read_u32(data, cursor, "stts sample count")?)
            .map_err(|_| LivePhotoMovieError::invalid("stts sample count exceeds usize"))?;
        let delta = u64::from(read_u32(data, cursor + 4, "stts sample delta")?);
        cursor += 8;
        if count > MAX_VIDEO_SAMPLES.saturating_sub(dts.len()) {
            return Err(LivePhotoMovieError::invalid(
                "video sample table exceeds safety limit",
            ));
        }
        for _ in 0..count {
            dts.push(clock);
            clock = clock
                .checked_add(delta)
                .ok_or_else(|| LivePhotoMovieError::invalid("video decode timeline overflows"))?;
        }
    }
    if dts.is_empty() {
        return Err(LivePhotoMovieError::invalid("video track has no samples"));
    }

    let mut offsets = vec![0_i64; dts.len()];
    if let Some(ctts) = boxes(data, stbl.data_start, stbl.data_end)?
        .into_iter()
        .find(|item| item.kind == CTTS)
    {
        let version = *data
            .get(ctts.data_start)
            .ok_or_else(|| LivePhotoMovieError::invalid("truncated ctts version"))?;
        let mut cursor = ctts.data_start + 4;
        let entry_count = usize::try_from(read_u32(data, cursor, "ctts entry count")?)
            .map_err(|_| LivePhotoMovieError::invalid("ctts entry count exceeds usize"))?;
        cursor += 4;
        let mut sample_index = 0_usize;
        for _ in 0..entry_count {
            let sample_count = usize::try_from(read_u32(data, cursor, "ctts sample count")?)
                .map_err(|_| LivePhotoMovieError::invalid("ctts sample count exceeds usize"))?;
            let value = if version == 1 {
                i64::from(read_i32(data, cursor + 4, "ctts sample offset")?)
            } else if version == 0 {
                i64::from(read_u32(data, cursor + 4, "ctts sample offset")?)
            } else {
                return Err(LivePhotoMovieError::invalid(format!(
                    "unsupported ctts version {version}"
                )));
            };
            cursor += 8;
            let end = sample_index
                .checked_add(sample_count)
                .ok_or_else(|| LivePhotoMovieError::invalid("ctts sample range overflows"))?;
            if end > offsets.len() {
                return Err(LivePhotoMovieError::invalid(
                    "ctts sample count exceeds stts",
                ));
            }
            offsets[sample_index..end].fill(value);
            sample_index = end;
        }
        if sample_index != offsets.len() {
            return Err(LivePhotoMovieError::invalid(
                "ctts/stts sample count mismatch",
            ));
        }
    }

    let scale = f64::from(timescale);
    dts.into_iter()
        .zip(offsets)
        .map(|(decode, offset)| {
            let presentation = i128::from(decode) + i128::from(offset);
            let value = presentation as f64 / scale;
            if value.is_finite() {
                Ok(value)
            } else {
                Err(LivePhotoMovieError::invalid(
                    "non-finite video presentation time",
                ))
            }
        })
        .collect()
}

pub fn resolve_live_photo_still_time(
    source: &[u8],
    requested_timestamp_us: Option<i64>,
) -> LivePhotoMovieResult<f64> {
    let top = top_level(source)?;
    let moov = one(&top, MOOV, "moov box")?;
    let (movie_timescale, movie_duration) = movie_timescale(source, moov)?;
    let tracks = boxes(source, moov.data_start, moov.data_end)?;
    let video_track = tracks
        .iter()
        .filter(|track| track.kind == TRAK)
        .find_map(|track| match handler_type(source, track) {
            Ok(kind) if kind == *b"vide" => Some(Ok(track)),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .transpose()?
        .ok_or_else(|| LivePhotoMovieError::invalid("embedded video contains no video track"))?;
    let pts = sample_pts_seconds(source, video_track)?;
    let duration_seconds = movie_duration as f64 / f64::from(movie_timescale);

    if let Some(requested_timestamp_us) = requested_timestamp_us {
        let requested = requested_timestamp_us as f64 / 1_000_000.0;
        if !requested.is_finite() || requested < 0.0 || requested > duration_seconds {
            return Err(LivePhotoMovieError::invalid(
                "Motion Photo still timestamp lies outside the video",
            ));
        }
        return pts
            .into_iter()
            .min_by(|left, right| {
                (left - requested)
                    .abs()
                    .total_cmp(&(right - requested).abs())
            })
            .ok_or_else(|| {
                LivePhotoMovieError::invalid("video track has no presentation samples")
            });
    }

    let midpoint = duration_seconds * 0.5;
    let closest = pts
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            (*left - midpoint)
                .abs()
                .total_cmp(&(*right - midpoint).abs())
        })
        .map(|(index, _)| index)
        .ok_or_else(|| LivePhotoMovieError::invalid("video track has no presentation samples"))?;
    Ok(pts[closest.saturating_sub(1)])
}

fn invert3(matrix: [f64; 9]) -> Option<[f64; 9]> {
    let [a, b, c, d, e, f, g, h, i] = matrix;
    let determinant = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    if !determinant.is_finite() || determinant.abs() <= 1e-10 {
        return None;
    }
    let inverse = 1.0 / determinant;
    Some([
        (e * i - f * h) * inverse,
        (c * h - b * i) * inverse,
        (b * f - c * e) * inverse,
        (f * g - d * i) * inverse,
        (a * i - c * g) * inverse,
        (c * d - a * f) * inverse,
        (d * h - e * g) * inverse,
        (b * g - a * h) * inverse,
        (a * e - b * d) * inverse,
    ])
}

fn multiply3(left: [f64; 9], right: [f64; 9]) -> [f64; 9] {
    let mut output = [0.0; 9];
    for row in 0..3 {
        for column in 0..3 {
            output[row * 3 + column] = (0..3)
                .map(|index| left[row * 3 + index] * right[index * 3 + column])
                .sum();
        }
    }
    output
}

fn normalized_axis_scale(value: f64) -> Option<f64> {
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    let scale = if value > 1.0 { 1.0 / value } else { value };
    (scale.is_finite() && scale > 0.0 && scale <= 1.0).then_some(scale)
}

fn normalized_scale(values: Option<&[f64]>) -> Option<(f64, f64)> {
    let values = values?;
    let first = *values.first()?;
    let second = values.get(1).copied().unwrap_or(first);
    Some((
        normalized_axis_scale(first)?,
        normalized_axis_scale(second)?,
    ))
}

fn normalize_homography(matrix: [f64; 9]) -> Option<[f64; 9]> {
    if matrix.iter().any(|value| !value.is_finite()) || matrix[8].abs() <= 1e-12 {
        return None;
    }
    let denominator = matrix[8];
    let mut output = matrix;
    for value in &mut output {
        *value /= denominator;
    }
    Some(output)
}

pub fn oppo_live_photo_transform(metadata: &OppoMetadata) -> Option<[f64; 9]> {
    let result = if metadata.version >= 1 {
        let (scale_x, scale_y) = normalized_scale(metadata.photo_eis_crop_factor.as_deref())
            .or_else(|| normalized_scale(metadata.eis_crop_factor.as_deref()))
            .unwrap_or((
                LEGACY_COLOROS16_EIS_COMPENSATION_SCALE,
                LEGACY_COLOROS16_EIS_COMPENSATION_SCALE,
            ));
        let mut result = [scale_x, 0.0, 0.0, 0.0, scale_y, 0.0, 0.0, 0.0, 1.0];
        if let Some(matrix) = metadata.photo_crop_matrix.and_then(invert3) {
            result = multiply3(result, matrix);
        }
        if let Some(matrix) = metadata.photo_eis_matrix.and_then(invert3) {
            result = multiply3(result, matrix);
        }
        result
    } else {
        if metadata.matrix_count <= 0 || metadata.matrices.is_empty() {
            return None;
        }
        let cover = metadata.cover_frame_pts_us?;
        let matrix = metadata
            .matrices
            .iter()
            .filter_map(|(key, matrix)| key.parse::<i64>().ok().map(|pts| (pts, *matrix)))
            .min_by_key(|(pts, _)| (i128::from(*pts) - i128::from(cover)).abs())?
            .1;
        invert3(matrix).unwrap_or(matrix)
    };

    let normalized = normalize_homography(result)?;
    let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    if normalized
        .iter()
        .zip(identity)
        .all(|(left, right)| (*left - right).abs() <= 1e-6)
    {
        None
    } else {
        Some(normalized)
    }
}

fn metadata_key_atom(local_id: u32, name: &[u8], type_code: u32) -> LivePhotoMovieResult<Vec<u8>> {
    let mut keyd_payload = b"mdta".to_vec();
    keyd_payload.extend_from_slice(name);
    let keyd = make_box_checked(FourCC::new(*b"keyd"), &keyd_payload)?;
    let mut dtyp_payload = Vec::with_capacity(8);
    push_u32(&mut dtyp_payload, 0);
    push_u32(&mut dtyp_payload, type_code);
    let dtyp = make_box_checked(FourCC::new(*b"dtyp"), &dtyp_payload)?;
    let mut payload = keyd;
    payload.extend_from_slice(&dtyp);
    make_box_checked(FourCC::new(local_id.to_be_bytes()), &payload)
}

fn metadata_sample(
    transform: Option<[f64; 9]>,
    dimensions: Option<(f32, f32)>,
) -> LivePhotoMovieResult<Vec<u8>> {
    let mut output = make_box_checked(FourCC::new(1_u32.to_be_bytes()), &[0])?;
    if let Some(transform) = transform {
        let mut payload = Vec::with_capacity(72);
        for value in transform {
            payload.extend_from_slice(&value.to_be_bytes());
        }
        output.extend_from_slice(&make_box_checked(
            FourCC::new(2_u32.to_be_bytes()),
            &payload,
        )?);
    }
    if let Some((width, height)) = dimensions {
        let mut payload = Vec::with_capacity(8);
        payload.extend_from_slice(&width.to_be_bytes());
        payload.extend_from_slice(&height.to_be_bytes());
        output.extend_from_slice(&make_box_checked(
            FourCC::new(3_u32.to_be_bytes()),
            &payload,
        )?);
    }
    Ok(output)
}

fn quicktime_timestamp() -> LivePhotoMovieResult<u32> {
    let unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let value = unix
        .checked_add(QUICKTIME_EPOCH_OFFSET)
        .ok_or_else(|| LivePhotoMovieError::invalid("QuickTime timestamp overflows"))?;
    u32::try_from(value)
        .map_err(|_| LivePhotoMovieError::invalid("QuickTime version-0 timestamp exceeds u32"))
}

fn rounded_u32(value: f64, context: &str) -> LivePhotoMovieResult<u32> {
    if !value.is_finite() || value < 0.0 || value.round() > f64::from(u32::MAX) {
        return Err(LivePhotoMovieError::invalid(format!(
            "{context} exceeds u32"
        )));
    }
    Ok(value.round() as u32)
}

fn metadata_track(
    track_id: u32,
    movie_timescale: u32,
    still_time_seconds: f64,
    chunk_offset: u64,
    transform: Option<[f64; 9]>,
    dimensions: Option<(f32, f32)>,
) -> LivePhotoMovieResult<(Vec<u8>, Vec<u8>)> {
    let sample = metadata_sample(transform, dimensions)?;
    let timestamp = quicktime_timestamp()?;
    let empty_duration = rounded_u32(
        still_time_seconds * f64::from(movie_timescale),
        "metadata empty duration",
    )?;
    let marker_duration = rounded_u32(
        f64::from(movie_timescale) / f64::from(METADATA_TIMESCALE),
        "metadata marker duration",
    )?
    .max(1);
    let track_duration = empty_duration
        .checked_add(marker_duration)
        .ok_or_else(|| LivePhotoMovieError::invalid("metadata track duration exceeds u32"))?;

    let mut tkhd_payload = Vec::new();
    for value in [timestamp, timestamp, track_id, 0, track_duration] {
        push_u32(&mut tkhd_payload, value);
    }
    tkhd_payload.extend_from_slice(&[0; 8]);
    for value in [0_i16; 4] {
        push_i16(&mut tkhd_payload, value);
    }
    for value in [0x10000_i32, 0, 0, 0, 0x10000, 0, 0, 0, 0x40000000] {
        push_i32(&mut tkhd_payload, value);
    }
    push_u32(&mut tkhd_payload, 0);
    push_u32(&mut tkhd_payload, 0);
    let tkhd = make_full_box_checked(TKHD, 0, 0x0f, &tkhd_payload)?;

    let mut edits = Vec::new();
    let mut edit_count = 1_u32;
    if empty_duration > 0 {
        push_u32(&mut edits, empty_duration);
        push_i32(&mut edits, -1);
        push_i16(&mut edits, 1);
        push_i16(&mut edits, 0);
        edit_count += 1;
    }
    push_u32(&mut edits, marker_duration);
    push_i32(&mut edits, 0);
    push_i16(&mut edits, 1);
    push_i16(&mut edits, 0);
    let mut elst_payload = Vec::new();
    push_u32(&mut elst_payload, edit_count);
    elst_payload.extend_from_slice(&edits);
    let elst = make_full_box_checked(ELST, 0, 0, &elst_payload)?;
    let edts = make_box_checked(EDTS, &elst)?;

    let mut mdhd_payload = Vec::new();
    for value in [timestamp, timestamp, METADATA_TIMESCALE, 1] {
        push_u32(&mut mdhd_payload, value);
    }
    push_u16(&mut mdhd_payload, 0x55c4);
    push_u16(&mut mdhd_payload, 0);
    let mdhd = make_full_box_checked(MDHD, 0, 0, &mdhd_payload)?;

    let media_name = b"Core Media Metadata";
    let mut media_handler_payload = b"mhlrmetaappl".to_vec();
    push_u32(&mut media_handler_payload, 1);
    push_u32(&mut media_handler_payload, 0);
    media_handler_payload.push(
        u8::try_from(media_name.len())
            .map_err(|_| LivePhotoMovieError::invalid("metadata handler name is too long"))?,
    );
    media_handler_payload.extend_from_slice(media_name);
    let media_handler = make_full_box_checked(HDLR, 0, 0, &media_handler_payload)?;

    let mut gmin_payload = Vec::new();
    for value in [0x40_u16, 0x8000, 0x8000, 0x8000] {
        push_u16(&mut gmin_payload, value);
    }
    push_i16(&mut gmin_payload, 0);
    push_u16(&mut gmin_payload, 0);
    let gmin = make_full_box_checked(GMIN, 0, 0, &gmin_payload)?;
    let gmhd = make_box_checked(GMHD, &gmin)?;

    let data_name = b"Core Media Data Handler";
    let mut data_handler_payload = b"dhlralisappl".to_vec();
    push_u32(&mut data_handler_payload, 0);
    push_u32(&mut data_handler_payload, 0);
    data_handler_payload.push(
        u8::try_from(data_name.len())
            .map_err(|_| LivePhotoMovieError::invalid("data handler name is too long"))?,
    );
    data_handler_payload.extend_from_slice(data_name);
    let data_handler = make_full_box_checked(HDLR, 0, 0, &data_handler_payload)?;

    let alis = make_full_box_checked(ALIS, 0, 1, &[])?;
    let mut dref_payload = Vec::new();
    push_u32(&mut dref_payload, 1);
    dref_payload.extend_from_slice(&alis);
    let dref = make_full_box_checked(DREF, 0, 0, &dref_payload)?;
    let dinf = make_box_checked(DINF, &dref)?;

    let mut key_atoms = metadata_key_atom(1, STILL_IMAGE_KEY, 65)?;
    if transform.is_some() {
        key_atoms.extend_from_slice(&metadata_key_atom(2, TRANSFORM_KEY, 79)?);
    }
    if dimensions.is_some() {
        key_atoms.extend_from_slice(&metadata_key_atom(3, REFERENCE_DIMENSIONS_KEY, 71)?);
    }
    let keys = make_box_checked(KEYS, &key_atoms)?;
    let mut mebx_payload = vec![0; 6];
    push_u16(&mut mebx_payload, 1);
    mebx_payload.extend_from_slice(&keys);
    let mebx = make_box_checked(MEBX, &mebx_payload)?;

    let mut stsd_payload = Vec::new();
    push_u32(&mut stsd_payload, 1);
    stsd_payload.extend_from_slice(&mebx);
    let stsd = make_full_box_checked(STSD, 0, 0, &stsd_payload)?;

    let mut stts_payload = Vec::new();
    for value in [1_u32, 1, 1] {
        push_u32(&mut stts_payload, value);
    }
    let stts = make_full_box_checked(STTS, 0, 0, &stts_payload)?;

    let mut stsc_payload = Vec::new();
    for value in [1_u32, 1, 1, 1] {
        push_u32(&mut stsc_payload, value);
    }
    let stsc = make_full_box_checked(STSC, 0, 0, &stsc_payload)?;

    let mut stsz_payload = Vec::new();
    push_u32(
        &mut stsz_payload,
        u32::try_from(sample.len())
            .map_err(|_| LivePhotoMovieError::invalid("metadata sample exceeds u32"))?,
    );
    push_u32(&mut stsz_payload, 1);
    let stsz = make_full_box_checked(STSZ, 0, 0, &stsz_payload)?;

    let chunk = if chunk_offset > u64::from(u32::MAX) {
        let mut payload = Vec::new();
        push_u32(&mut payload, 1);
        push_u64(&mut payload, chunk_offset);
        make_full_box_checked(CO64, 0, 0, &payload)?
    } else {
        let mut payload = Vec::new();
        push_u32(&mut payload, 1);
        push_u32(&mut payload, chunk_offset as u32);
        make_full_box_checked(STCO, 0, 0, &payload)?
    };

    let mut stbl_payload = stsd;
    stbl_payload.extend_from_slice(&stts);
    stbl_payload.extend_from_slice(&stsc);
    stbl_payload.extend_from_slice(&stsz);
    stbl_payload.extend_from_slice(&chunk);
    let stbl = make_box_checked(STBL, &stbl_payload)?;
    let mut minf_payload = gmhd;
    minf_payload.extend_from_slice(&data_handler);
    minf_payload.extend_from_slice(&dinf);
    minf_payload.extend_from_slice(&stbl);
    let minf = make_box_checked(MINF, &minf_payload)?;
    let mut mdia_payload = mdhd;
    mdia_payload.extend_from_slice(&media_handler);
    mdia_payload.extend_from_slice(&minf);
    let mdia = make_box_checked(MDIA, &mdia_payload)?;
    let mut trak_payload = tkhd;
    trak_payload.extend_from_slice(&edts);
    trak_payload.extend_from_slice(&mdia);
    Ok((make_box_checked(TRAK, &trak_payload)?, sample))
}

fn movie_metadata(content_identifier: &str) -> LivePhotoMovieResult<Vec<u8>> {
    if content_identifier.is_empty()
        || !content_identifier.is_ascii()
        || content_identifier.as_bytes().contains(&0)
    {
        return Err(LivePhotoMovieError::invalid(
            "invalid Live Photo content identifier",
        ));
    }
    let mut handler_payload = vec![0; 8];
    handler_payload.extend_from_slice(b"mdta");
    handler_payload.extend_from_slice(&[0; 14]);
    let handler = make_box_checked(HDLR, &handler_payload)?;

    let key = make_box_checked(MDTA, CONTENT_IDENTIFIER_KEY)?;
    let mut keys_payload = Vec::new();
    push_u32(&mut keys_payload, 1);
    keys_payload.extend_from_slice(&key);
    let keys = make_full_box_checked(KEYS, 0, 0, &keys_payload)?;

    let mut value_payload = Vec::new();
    push_u32(&mut value_payload, 1);
    push_u32(&mut value_payload, 0);
    value_payload.extend_from_slice(content_identifier.as_bytes());
    let value = make_box_checked(DATA, &value_payload)?;
    let item = make_box_checked(FourCC::new(1_u32.to_be_bytes()), &value)?;
    let ilst = make_box_checked(ILST, &item)?;

    let mut meta_payload = handler;
    meta_payload.extend_from_slice(&keys);
    meta_payload.extend_from_slice(&ilst);
    make_box_checked(META, &meta_payload)
}

fn patch_next_track_id(raw: &[u8], next_track_id: u32) -> LivePhotoMovieResult<Vec<u8>> {
    if raw.len() < 4 {
        return Err(LivePhotoMovieError::invalid("mvhd box is too small"));
    }
    let mut output = raw.to_vec();
    let start = output.len() - 4;
    output[start..].copy_from_slice(&next_track_id.to_be_bytes());
    Ok(output)
}

fn rebuild_moov(
    original: &[u8],
    metadata_track: &[u8],
    movie_metadata: &[u8],
    new_track_id: u32,
) -> LivePhotoMovieResult<Vec<u8>> {
    let root = one(&top_level(original)?, MOOV, "moov box")?.clone();
    let mut payload = Vec::new();
    for child in boxes(original, root.data_start, root.data_end)? {
        if child.kind == META {
            continue;
        }
        let raw = original
            .get(child.box_range())
            .ok_or_else(|| LivePhotoMovieError::invalid("moov child lies outside source"))?;
        if child.kind == MVHD {
            let next = new_track_id
                .checked_add(1)
                .ok_or_else(|| LivePhotoMovieError::invalid("QuickTime track id overflows"))?;
            payload.extend_from_slice(&patch_next_track_id(raw, next)?);
        } else {
            payload.extend_from_slice(raw);
        }
    }
    payload.extend_from_slice(metadata_track);
    payload.extend_from_slice(movie_metadata);
    make_box_checked(MOOV, &payload)
}

fn free_box_same_size(size: usize) -> LivePhotoMovieResult<Vec<u8>> {
    if size < 8 {
        return Err(LivePhotoMovieError::invalid(
            "cannot replace undersized moov box",
        ));
    }
    if size <= u32::MAX as usize {
        let mut output = Vec::with_capacity(size);
        push_u32(&mut output, size as u32);
        output.extend_from_slice(FREE.as_bytes());
        output.resize(size, 0);
        Ok(output)
    } else {
        let size64 = u64::try_from(size)
            .map_err(|_| LivePhotoMovieError::invalid("moov size exceeds u64"))?;
        let mut output = Vec::with_capacity(size);
        push_u32(&mut output, 1);
        output.extend_from_slice(FREE.as_bytes());
        push_u64(&mut output, size64);
        output.resize(size, 0);
        Ok(output)
    }
}

fn exact_video_dimension(value: Option<i64>) -> Option<f32> {
    let value = value?;
    const MAX_EXACT_F32_INTEGER: i64 = 1 << 24;
    if value <= 0 || value > MAX_EXACT_F32_INTEGER {
        return None;
    }
    Some(value as f32)
}

pub fn write_live_photo_movie(
    source: &[u8],
    content_identifier: &str,
    still_time_seconds: f64,
    oppo_metadata: Option<&OppoMetadata>,
) -> LivePhotoMovieResult<Vec<u8>> {
    if !still_time_seconds.is_finite() || still_time_seconds < 0.0 {
        return Err(LivePhotoMovieError::invalid(
            "invalid Live Photo still time",
        ));
    }
    let top = top_level(source)?;
    let ftyp = one(&top, FTYP, "ftyp box")?;
    let moov = one(&top, MOOV, "moov box")?;
    if ftyp.data_end.saturating_sub(ftyp.data_start) < 8 {
        return Err(LivePhotoMovieError::invalid("source ftyp is too small"));
    }

    let original_moov = source
        .get(moov.box_range())
        .ok_or_else(|| LivePhotoMovieError::invalid("moov lies outside source"))?;
    let original_root = one(&top_level(original_moov)?, MOOV, "moov box")?.clone();
    let (timescale, _) = movie_timescale(original_moov, &original_root)?;
    let tracks = boxes(
        original_moov,
        original_root.data_start,
        original_root.data_end,
    )?;
    let max_track_id = tracks
        .iter()
        .filter(|item| item.kind == TRAK)
        .map(|track| track_id(original_moov, track))
        .collect::<LivePhotoMovieResult<Vec<_>>>()?
        .into_iter()
        .max()
        .unwrap_or(0);
    let new_track_id = max_track_id
        .checked_add(1)
        .ok_or_else(|| LivePhotoMovieError::invalid("QuickTime track id overflows"))?;
    let transform = oppo_metadata.and_then(oppo_live_photo_transform);
    let dimensions = if transform.is_some() {
        oppo_metadata.and_then(|metadata| {
            Some((
                exact_video_dimension(metadata.video_width)?,
                exact_video_dimension(metadata.video_height)?,
            ))
        })
    } else {
        None
    };
    let marker_payload_offset = u64::try_from(source.len())
        .map_err(|_| LivePhotoMovieError::invalid("source movie exceeds u64"))?
        .checked_add(8)
        .ok_or_else(|| LivePhotoMovieError::invalid("metadata chunk offset overflows"))?;
    let (metadata_track, marker_sample) = metadata_track(
        new_track_id,
        timescale,
        still_time_seconds,
        marker_payload_offset,
        transform,
        dimensions,
    )?;
    let marker_mdat = make_box_checked(MDAT, &marker_sample)?;
    let new_moov = rebuild_moov(
        original_moov,
        &metadata_track,
        &movie_metadata(content_identifier)?,
        new_track_id,
    )?;

    let mut output = Vec::with_capacity(
        source
            .len()
            .saturating_add(marker_mdat.len())
            .saturating_add(new_moov.len()),
    );
    for item in &top {
        if item.kind == MOOV {
            output.extend_from_slice(&free_box_same_size(item.size)?);
            continue;
        }
        if item.kind == FTYP {
            let mut raw = source
                .get(item.box_range())
                .ok_or_else(|| LivePhotoMovieError::invalid("ftyp lies outside source"))?
                .to_vec();
            let local_payload = item.data_start - item.box_start;
            raw[local_payload..local_payload + 4].copy_from_slice(b"qt  ");
            raw[local_payload + 4..local_payload + 8].fill(0);
            output.extend_from_slice(&raw);
            continue;
        }
        let size32 = read_u32(source, item.box_start, "top-level box size")?;
        if size32 == 0 {
            let explicit_size = u32::try_from(item.size).map_err(|_| {
                LivePhotoMovieError::invalid("cannot append after >4GiB size==0 box")
            })?;
            push_u32(&mut output, explicit_size);
            output.extend_from_slice(item.kind.as_bytes());
            output.extend_from_slice(source.get(item.data_start..item.data_end).ok_or_else(
                || LivePhotoMovieError::invalid("top-level box lies outside source"),
            )?);
        } else {
            output.extend_from_slice(source.get(item.box_range()).ok_or_else(|| {
                LivePhotoMovieError::invalid("top-level box lies outside source")
            })?);
        }
    }
    output.extend_from_slice(&marker_mdat);
    output.extend_from_slice(&new_moov);
    validate_live_photo_movie(&output, content_identifier, still_time_seconds)?;
    Ok(output)
}

fn parse_meta_children(data: &[u8], meta: &BoxHeader) -> LivePhotoMovieResult<Vec<BoxHeader>> {
    match boxes(data, meta.data_start, meta.data_end) {
        Ok(children) if !children.is_empty() => Ok(children),
        _ if meta.data_start + 4 <= meta.data_end => {
            boxes(data, meta.data_start + 4, meta.data_end)
        }
        Ok(children) => Ok(children),
        Err(error) => Err(error),
    }
}

pub fn read_live_photo_content_identifier(data: &[u8]) -> LivePhotoMovieResult<Option<String>> {
    let top = top_level(data)?;
    let Some(moov) = top.iter().find(|item| item.kind == MOOV) else {
        return Ok(None);
    };
    let Some(meta) = boxes(data, moov.data_start, moov.data_end)?
        .into_iter()
        .find(|item| item.kind == META)
    else {
        return Ok(None);
    };
    let children = parse_meta_children(data, &meta)?;
    let Some(keys) = children.iter().find(|item| item.kind == KEYS) else {
        return Ok(None);
    };
    let Some(ilst) = children.iter().find(|item| item.kind == ILST) else {
        return Ok(None);
    };
    let count = usize::try_from(read_u32(data, keys.data_start + 4, "metadata key count")?)
        .map_err(|_| LivePhotoMovieError::invalid("metadata key count exceeds usize"))?;
    let key_start = keys.data_start + 8;
    let key_boxes = boxes(data, key_start, keys.data_end)?;
    if key_boxes.len() < count {
        return Ok(None);
    }
    let content_index = key_boxes
        .iter()
        .take(count)
        .position(|item| {
            item.kind == MDTA
                && data.get(item.data_start..item.data_end) == Some(CONTENT_IDENTIFIER_KEY)
        })
        .map(|index| index + 1);
    let Some(content_index) = content_index else {
        return Ok(None);
    };
    let wanted = FourCC::new(
        u32::try_from(content_index)
            .map_err(|_| LivePhotoMovieError::invalid("metadata key index exceeds u32"))?
            .to_be_bytes(),
    );
    for item in boxes(data, ilst.data_start, ilst.data_end)? {
        if item.kind != wanted {
            continue;
        }
        let Some(value) = boxes(data, item.data_start, item.data_end)?
            .into_iter()
            .find(|child| child.kind == DATA)
        else {
            return Ok(None);
        };
        if value.size < 16
            || read_u32(data, value.data_start, "metadata data type")? != 1
            || read_u32(data, value.data_start + 4, "metadata locale")? != 0
        {
            return Ok(None);
        }
        let raw = data
            .get(value.data_start + 8..value.data_end)
            .ok_or_else(|| LivePhotoMovieError::invalid("metadata value lies outside file"))?;
        return Ok(std::str::from_utf8(raw).ok().map(ToOwned::to_owned));
    }
    Ok(None)
}

pub fn read_live_photo_still_time(data: &[u8]) -> LivePhotoMovieResult<Option<f64>> {
    let top = top_level(data)?;
    let Some(moov) = top.iter().find(|item| item.kind == MOOV) else {
        return Ok(None);
    };
    let (timescale, _) = movie_timescale(data, moov)?;
    for track in boxes(data, moov.data_start, moov.data_end)? {
        if track.kind != TRAK {
            continue;
        }
        let raw = data
            .get(track.box_range())
            .ok_or_else(|| LivePhotoMovieError::invalid("track lies outside file"))?;
        if !raw.windows(4).any(|window| window == b"mebx")
            || !raw
                .windows(STILL_IMAGE_KEY.len())
                .any(|window| window == STILL_IMAGE_KEY)
        {
            continue;
        }
        let edts = match child(data, &track, EDTS, "edts box") {
            Ok(value) => value,
            Err(_) => return Ok(Some(0.0)),
        };
        let elst = match child(data, &edts, ELST, "elst box") {
            Ok(value) => value,
            Err(_) => return Ok(Some(0.0)),
        };
        let version = *data
            .get(elst.data_start)
            .ok_or_else(|| LivePhotoMovieError::invalid("truncated elst version"))?;
        let count = read_u32(data, elst.data_start + 4, "elst entry count")?;
        if count == 0 {
            return Ok(Some(0.0));
        }
        let cursor = elst.data_start + 8;
        let (duration, media_time) = match version {
            0 => (
                u64::from(read_u32(data, cursor, "elst duration")?),
                i64::from(read_i32(data, cursor + 4, "elst media time")?),
            ),
            1 => (
                read_u64(data, cursor, "elst duration")?,
                read_i64(data, cursor + 8, "elst media time")?,
            ),
            _ => return Ok(None),
        };
        return Ok(Some(if media_time == -1 {
            duration as f64 / f64::from(timescale)
        } else {
            0.0
        }));
    }
    Ok(None)
}

pub fn media_mdat_payloads(data: &[u8]) -> LivePhotoMovieResult<Vec<Vec<u8>>> {
    let mut payloads = Vec::new();
    for item in top_level(data)? {
        if item.kind != MDAT {
            continue;
        }
        let payload = data
            .get(item.data_start..item.data_end)
            .ok_or_else(|| LivePhotoMovieError::invalid("mdat lies outside file"))?;
        let metadata_marker = payload.len() <= 512
            && (payload
                .windows(STILL_IMAGE_KEY.len())
                .any(|window| window == STILL_IMAGE_KEY)
                || payload.starts_with(&[0, 0, 0, 9, 0, 0, 0, 1]));
        if !metadata_marker {
            payloads.push(payload.to_vec());
        }
    }
    Ok(payloads)
}

pub fn validate_live_photo_movie(
    data: &[u8],
    content_identifier: &str,
    still_time_seconds: f64,
) -> LivePhotoMovieResult<()> {
    if read_live_photo_content_identifier(data)?.as_deref() != Some(content_identifier) {
        return Err(LivePhotoMovieError::invalid(
            "MOV content identifier mismatch",
        ));
    }
    let actual = read_live_photo_still_time(data)?
        .ok_or_else(|| LivePhotoMovieError::invalid("MOV lacks still-image-time metadata track"))?;
    let top = top_level(data)?;
    let moov = one(&top, MOOV, "active moov box")?;
    let (timescale, _) = movie_timescale(data, moov)?;
    let tolerance = (1.0 / f64::from(timescale)).max(1e-6);
    if (actual - still_time_seconds).abs() > tolerance {
        return Err(LivePhotoMovieError::invalid(
            "MOV still-image-time mismatch",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn full(kind: FourCC, payload: &[u8], flags: u32) -> Vec<u8> {
        make_full_box(kind, 0, flags, payload).unwrap()
    }

    fn fake_video() -> Vec<u8> {
        let mut ftyp_payload = b"isom".to_vec();
        push_u32(&mut ftyp_payload, 0);
        ftyp_payload.extend_from_slice(b"isommp42");
        let ftyp = make_box(FTYP, &ftyp_payload).unwrap();

        let mut mvhd_payload = Vec::new();
        for value in [0_u32, 0, 1000, 300, 0x0001_0000] {
            push_u32(&mut mvhd_payload, value);
        }
        push_u16(&mut mvhd_payload, 0x0100);
        push_u16(&mut mvhd_payload, 0);
        mvhd_payload.extend_from_slice(&[0; 8]);
        for value in [0x10000_i32, 0, 0, 0, 0x10000, 0, 0, 0, 0x40000000] {
            push_i32(&mut mvhd_payload, value);
        }
        mvhd_payload.extend_from_slice(&[0; 24]);
        push_u32(&mut mvhd_payload, 2);
        let mvhd = full(MVHD, &mvhd_payload, 0);

        let mut tkhd_payload = Vec::new();
        for value in [0_u32, 0, 1, 0, 300] {
            push_u32(&mut tkhd_payload, value);
        }
        tkhd_payload.extend_from_slice(&[0; 8]);
        for value in [0_i16; 4] {
            push_i16(&mut tkhd_payload, value);
        }
        for value in [0x10000_i32, 0, 0, 0, 0x10000, 0, 0, 0, 0x40000000] {
            push_i32(&mut tkhd_payload, value);
        }
        push_u32(&mut tkhd_payload, 64 << 16);
        push_u32(&mut tkhd_payload, 48 << 16);
        let tkhd = full(TKHD, &tkhd_payload, 7);

        let mut mdhd_payload = Vec::new();
        for value in [0_u32, 0, 1000, 300] {
            push_u32(&mut mdhd_payload, value);
        }
        push_u16(&mut mdhd_payload, 0x55c4);
        push_u16(&mut mdhd_payload, 0);
        let mdhd = full(MDHD, &mdhd_payload, 0);
        let mut hdlr_payload = vec![0; 4];
        hdlr_payload.extend_from_slice(b"vide");
        hdlr_payload.extend_from_slice(&[0; 12]);
        hdlr_payload.extend_from_slice(b"Video\0");
        let hdlr = full(HDLR, &hdlr_payload, 0);
        let mut stts_payload = Vec::new();
        for value in [1_u32, 3, 100] {
            push_u32(&mut stts_payload, value);
        }
        let stts = full(STTS, &stts_payload, 0);
        let stbl = make_box(STBL, &stts).unwrap();
        let minf = make_box(MINF, &stbl).unwrap();
        let mut mdia_payload = mdhd;
        mdia_payload.extend_from_slice(&hdlr);
        mdia_payload.extend_from_slice(&minf);
        let mdia = make_box(MDIA, &mdia_payload).unwrap();
        let mut trak_payload = tkhd;
        trak_payload.extend_from_slice(&mdia);
        let trak = make_box(TRAK, &trak_payload).unwrap();
        let mut moov_payload = mvhd;
        moov_payload.extend_from_slice(&trak);
        let moov = make_box(MOOV, &moov_payload).unwrap();
        let mdat = make_box(MDAT, b"encoded-media-payload").unwrap();
        [ftyp, moov, mdat].concat()
    }

    #[test]
    fn writer_preserves_compressed_media_and_adds_required_metadata() {
        let source = fake_video();
        let source_media = media_mdat_payloads(&source).unwrap();
        let still_time = resolve_live_photo_still_time(&source, Some(120_000)).unwrap();
        assert!((still_time - 0.1).abs() < 1e-9);

        let output = write_live_photo_movie(&source, "ABC-123", still_time, None).unwrap();

        assert_eq!(
            read_live_photo_content_identifier(&output)
                .unwrap()
                .as_deref(),
            Some("ABC-123")
        );
        assert!((read_live_photo_still_time(&output).unwrap().unwrap() - 0.1).abs() < 1e-3);
        assert_eq!(media_mdat_payloads(&output).unwrap(), source_media);
        validate_live_photo_movie(&output, "ABC-123", still_time).unwrap();
    }

    #[test]
    fn coloros16_transform_matches_existing_product_policy() {
        let metadata = OppoMetadata {
            version: 1,
            photo_eis_crop_factor: Some(vec![1.11, 1.12]),
            video_width: Some(1728),
            video_height: Some(1296),
            ..OppoMetadata::default()
        };
        let transform = oppo_live_photo_transform(&metadata).unwrap();
        assert!((transform[0] - 1.0 / 1.11).abs() < 1e-12);
        assert!((transform[4] - 1.0 / 1.12).abs() < 1e-12);
        assert_eq!(transform[8], 1.0);

        let source = fake_video();
        let output = write_live_photo_movie(&source, "ABC-123", 0.1, Some(&metadata)).unwrap();
        let transform_bytes = transform
            .into_iter()
            .flat_map(f64::to_be_bytes)
            .collect::<Vec<_>>();
        assert!(output
            .windows(transform_bytes.len())
            .any(|window| window == transform_bytes));
        assert_eq!(
            media_mdat_payloads(&output).unwrap(),
            media_mdat_payloads(&source).unwrap()
        );
    }

    #[test]
    fn coloros15_uses_closest_cover_frame_and_inverts_matrix() {
        let mut matrices = BTreeMap::new();
        matrices.insert(
            "1000".to_owned(),
            [2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 1.0],
        );
        matrices.insert(
            "2000".to_owned(),
            [4.0, 0.0, 0.0, 0.0, 4.0, 0.0, 0.0, 0.0, 1.0],
        );
        let metadata = OppoMetadata {
            cover_frame_pts_us: Some(1100),
            matrix_count: 2,
            matrices,
            ..OppoMetadata::default()
        };
        let transform = oppo_live_photo_transform(&metadata).unwrap();
        assert!((transform[0] - 0.5).abs() < 1e-12);
        assert!((transform[4] - 0.5).abs() < 1e-12);
        assert_eq!(transform[8], 1.0);
    }

    #[test]
    fn requested_still_time_must_lie_inside_movie() {
        let source = fake_video();
        assert!(resolve_live_photo_still_time(&source, Some(-1)).is_err());
        assert!(resolve_live_photo_still_time(&source, Some(1_000_000)).is_err());
    }
}
