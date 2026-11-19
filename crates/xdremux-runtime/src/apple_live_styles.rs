use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Seek, Write};
use std::path::Path;

use xdremux_engine::{apple_live_frame_style, apple_live_texture_metadata};
use xdremux_format::{parse_hvcc_profile, ChromaSampling};
use xdremux_motion_photo::{add_apple_live_style_tracks, AppleLiveStyleAac, AppleLiveStyleFrames};

use crate::apple_adapter::{AppleAdapterClient, AppleVideoTime, AppleVideoToolboxMain10Encode};
use crate::apple_styles::{build_live_still, scene_rgb_from_native, single_idr_payload};
use crate::{PortableRuntime, Result, RuntimeError};

const STYLES: &str = "Live Photo Styles";

pub(super) fn build_pair(
    runtime: &PortableRuntime,
    executable: &Path,
    still: &[u8],
    movie: &[u8],
) -> Result<(Vec<u8>, Vec<u8>)> {
    let staging = tempfile::Builder::new()
        .prefix("xdremux-live-styles-")
        .tempdir()
        .map_err(|error| RuntimeError::external(STYLES, error))?;
    let adapter = AppleAdapterClient::new(executable.to_path_buf());
    let movie = build_movie(&adapter, movie, staging.path())?;
    let still = build_live_still(runtime, &adapter, still, staging.path())?;
    Ok((still, movie))
}

fn build_movie(adapter: &AppleAdapterClient, movie: &[u8], staging: &Path) -> Result<Vec<u8>> {
    let movie_path = staging.join("source.mov");
    fs::write(&movie_path, movie).map_err(|error| RuntimeError::external(STYLES, error))?;
    let scene_path = staging.join("frames.rgba-f32");
    let video = adapter.observe_live_style_video(&movie_path, &scene_path)?;
    let (timescale, boundaries) = normalize_times(&video.frames, video.end)?;
    let width =
        usize::try_from(video.width).map_err(|error| RuntimeError::external(STYLES, error))?;
    let height =
        usize::try_from(video.height).map_err(|error| RuntimeError::external(STYLES, error))?;
    if !(2..=128).contains(&width)
        || !(2..=128).contains(&height)
        || !width.is_multiple_of(2)
        || !height.is_multiple_of(2)
    {
        return Err(RuntimeError::new(
            STYLES,
            "unexpected linear video observation dimensions",
        ));
    }
    let frame_size = width * height * 16;
    let expected_size = frame_size
        .checked_mul(video.frames.len())
        .ok_or_else(|| RuntimeError::new(STYLES, "linear video size overflows"))?;
    let scene = File::open(&scene_path).map_err(|error| RuntimeError::external(STYLES, error))?;
    if scene
        .metadata()
        .map_err(|error| RuntimeError::external(STYLES, error))?
        .len()
        != expected_size as u64
    {
        return Err(RuntimeError::new(
            STYLES,
            "decoded frames do not match their timestamps",
        ));
    }
    let mut scene = BufReader::new(scene);
    let mut bytes = vec![0; frame_size];
    let mut storage_exposure = 1.0f32;
    for _ in &video.frames {
        scene
            .read_exact(&mut bytes)
            .map_err(|error| RuntimeError::external(STYLES, error))?;
        let rgb = scene_rgb_from_native(&bytes, width, height)?;
        storage_exposure = rgb.samples.into_iter().fold(storage_exposure, f32::max);
    }
    scene
        .rewind()
        .map_err(|error| RuntimeError::external(STYLES, error))?;
    let log_path = staging.join("frames.log-rgb16");
    let mut log = BufWriter::new(
        File::create(&log_path).map_err(|error| RuntimeError::external(STYLES, error))?,
    );
    let mut style_properties = Vec::with_capacity(video.frames.len());
    for _ in &video.frames {
        scene
            .read_exact(&mut bytes)
            .map_err(|error| RuntimeError::external(STYLES, error))?;
        let rgb = scene_rgb_from_native(&bytes, width, height)?;
        let frame = apple_live_frame_style(&rgb.samples, storage_exposure)
            .map_err(|error| RuntimeError::external(STYLES, error))?;
        for code in frame.apple_log_codes {
            log.write_all(&code.to_le_bytes())
                .map_err(|error| RuntimeError::external(STYLES, error))?;
        }
        style_properties.push(frame.metadata);
    }
    log.flush()
        .map_err(|error| RuntimeError::external(STYLES, error))?;
    let annex_b_path = staging.join("frames.hevc");
    let hvcc_path = staging.join("frames.hvcc");
    let facts = adapter.videotoolbox_encode_main10(&AppleVideoToolboxMain10Encode {
        input: &log_path,
        output_annex_b: &annex_b_path,
        output_hvcc: &hvcc_path,
        width: video.width,
        height: video.height,
        bytes_per_row: video.width * 6,
        component_bits: 16,
        quality: 0.85,
        frame_count: Some(video.frames.len()),
    })?;
    let annex_b = fs::read(annex_b_path).map_err(|error| RuntimeError::external(STYLES, error))?;
    let hvcc = fs::read(hvcc_path).map_err(|error| RuntimeError::external(STYLES, error))?;
    let profile =
        parse_hvcc_profile(&hvcc).map_err(|error| RuntimeError::external(STYLES, error))?;
    if profile.chroma_sampling != ChromaSampling::Yuv420
        || profile.luma_bit_depth != 10
        || profile.chroma_bit_depth != 10
        || hvcc.len() != facts.hvcc_length
        || annex_b.len() != facts.annex_b_length
    {
        return Err(RuntimeError::new(
            STYLES,
            "codec output is not the required Main10 4:2:0 sequence",
        ));
    }
    let mut remaining = annex_b.as_slice();
    let mut thumbnails = Vec::with_capacity(video.frames.len());
    for length in facts
        .frame_lengths
        .ok_or_else(|| RuntimeError::new(STYLES, "missing encoded frame lengths"))?
    {
        let (packet, rest) = remaining.split_at_checked(length).ok_or_else(|| {
            RuntimeError::new(STYLES, "encoded frame extends beyond its resource")
        })?;
        thumbnails.push(single_idr_payload(packet)?);
        remaining = rest;
    }
    if !remaining.is_empty() {
        return Err(RuntimeError::new(
            STYLES,
            "unaccounted encoded video samples",
        ));
    }
    let texture =
        apple_live_texture_metadata().map_err(|error| RuntimeError::external(STYLES, error))?;
    let aac = video
        .aac
        .into_iter()
        .map(|format| AppleLiveStyleAac {
            track_id: format.track_id,
            frames_per_packet: format.frames_per_packet,
            channel_count: format.channel_count,
        })
        .collect::<Vec<_>>();
    add_apple_live_style_tracks(
        movie,
        &AppleLiveStyleFrames {
            track_id: video.track_id,
            timescale,
            boundaries: &boundaries,
            width: video.width as u16,
            height: video.height as u16,
            hvcc: &hvcc,
            thumbnail_samples: &thumbnails,
            style_properties: &style_properties,
            texture_properties: &texture,
            aac: &aac,
        },
    )
    .map_err(|error| RuntimeError::external(STYLES, error))
}

fn normalize_times(frames: &[AppleVideoTime], end: AppleVideoTime) -> Result<(u32, Vec<u64>)> {
    fn gcd(mut a: u32, mut b: u32) -> u32 {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    }
    let invalid = || {
        RuntimeError::new(
            STYLES,
            "video timestamps have no exact common positive timescale",
        )
    };
    let values = frames.iter().copied().chain(std::iter::once(end));
    let scale = values.clone().try_fold(1u32, |scale, timestamp| {
        if timestamp.timescale == 0 {
            return Err(invalid());
        }
        (scale / gcd(scale, timestamp.timescale))
            .checked_mul(timestamp.timescale)
            .ok_or_else(invalid)
    })?;
    let ticks = values
        .map(|time| {
            u64::try_from(time.value)
                .ok()
                .and_then(|value| value.checked_mul(u64::from(scale / time.timescale)))
                .ok_or_else(invalid)
        })
        .collect::<Result<_>>()?;
    Ok((scale, ticks))
}
