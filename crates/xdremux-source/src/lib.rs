#![forbid(unsafe_code)]

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use xdremux_container::{extract as extract_proxdr, ContainerError};
use xdremux_motion_photo::{
    product_source_kind, AssetFormat, ByteRange, ContainerKind, Input, MotionPhoto,
    MotionPhotoError, MotionPhotoReadError, ParseOptions, TimeSource,
};

pub const INSPECTION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ByteSpan {
    pub offset: u64,
    pub length: u64,
}

impl From<ByteRange> for ByteSpan {
    fn from(value: ByteRange) -> Self {
        Self {
            offset: value.lower_bound,
            length: value.length(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SourceAsset {
    MotionPhoto {
        source_kind: String,
        still: ByteSpan,
        video: ByteSpan,
        presentation_timestamp_us: Option<i64>,
        presentation_source: Option<String>,
        stream_count: usize,
    },
    ProXdr {
        hdr_mode: String,
        metadata_float_count: usize,
        gain_map_bytes: usize,
        manifest_entry_count: usize,
        has_local_hdr_info: bool,
    },
}

impl SourceAsset {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::MotionPhoto { .. } => "motion-photo",
            Self::ProXdr { .. } => "pro-xdr",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceInspection {
    pub schema_version: u32,
    pub input: PathBuf,
    #[serde(flatten)]
    pub asset: SourceAsset,
}

#[derive(Debug)]
pub enum SourceProbeError {
    Io { path: PathBuf, source: io::Error },
    MotionPhoto(MotionPhotoReadError),
    Unsupported(ContainerError),
}

impl fmt::Display for SourceProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(formatter, "could not read {}: {source}", path.display())
            }
            Self::MotionPhoto(source) => write!(formatter, "invalid Motion Photo: {source}"),
            Self::Unsupported(source) => {
                write!(
                    formatter,
                    "unsupported input: no Motion Photo or ProXDR payload ({source})"
                )
            }
        }
    }
}

impl std::error::Error for SourceProbeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::MotionPhoto(source) => Some(source),
            Self::Unsupported(source) => Some(source),
        }
    }
}

pub type Result<T> = std::result::Result<T, SourceProbeError>;

pub fn probe_bytes(data: &[u8]) -> Result<SourceAsset> {
    match MotionPhoto::parse(Input::SingleFile(data), ParseOptions::compatible()) {
        Ok(photo) => {
            let asset = photo.asset();
            return Ok(SourceAsset::MotionPhoto {
                source_kind: product_source_kind(asset).to_owned(),
                still: photo.still().extents[0].into(),
                video: photo.motion_video().extents[0].into(),
                presentation_timestamp_us: photo
                    .presentation_time()
                    .and_then(|t| t.rescale_exact(1_000_000))
                    .map(|t| t.value),
                presentation_source: asset.presentation.map(|p| {
                    match p.source {
                        TimeSource::Xmp if asset.format == AssetFormat::LegacyMicroVideo => {
                            "legacyMicroVideoXMP"
                        }
                        TimeSource::Xmp => "androidXMP",
                        TimeSource::VendorMetadata => "oppoCoverFrame",
                        TimeSource::AppleTimedMetadata => "appleTimedMetadata",
                        TimeSource::Caller => "caller",
                    }
                    .to_owned()
                }),
                stream_count: asset
                    .containers
                    .iter()
                    .filter(|c| c.kind == ContainerKind::IsoBmff)
                    .count(),
            });
        }
        Err(
            MotionPhotoReadError::Unrecognized
            | MotionPhotoReadError::InvalidStill
            | MotionPhotoReadError::Parse(MotionPhotoError::FileTooSmall),
        ) => {}
        Err(error) => return Err(SourceProbeError::MotionPhoto(error)),
    }

    let extracted = extract_proxdr(data).map_err(SourceProbeError::Unsupported)?;
    Ok(SourceAsset::ProXdr {
        hdr_mode: extracted.mode.as_str().to_owned(),
        metadata_float_count: extracted.meta_floats.len(),
        gain_map_bytes: extracted.mask_jpeg_data.len(),
        manifest_entry_count: extracted.manifest_info.entries.len(),
        has_local_hdr_info: extracted.local_hdr_info.is_some(),
    })
}

pub fn inspect_path(path: impl AsRef<Path>) -> Result<SourceInspection> {
    let path = path.as_ref();
    let data = fs::read(path).map_err(|source| SourceProbeError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let asset = probe_bytes(&data)?;
    Ok(SourceInspection {
        schema_version: INSPECTION_SCHEMA_VERSION,
        input: path.to_path_buf(),
        asset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_box(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let size = u32::try_from(payload.len() + 8).unwrap();
        let mut output = size.to_be_bytes().to_vec();
        output.extend_from_slice(kind);
        output.extend_from_slice(payload);
        output
    }

    fn fake_mp4() -> Vec<u8> {
        let mut mvhd = vec![0; 12];
        mvhd.extend_from_slice(&1000_u32.to_be_bytes());
        mvhd.extend_from_slice(&2000_u32.to_be_bytes());
        mvhd.extend_from_slice(&[0; 80]);
        let mut tkhd = vec![0; 12];
        tkhd.extend_from_slice(&1_u32.to_be_bytes());
        tkhd.extend_from_slice(&[0; 68]);
        let mut mdhd = vec![0; 12];
        mdhd.extend_from_slice(&1000_u32.to_be_bytes());
        mdhd.extend_from_slice(&2000_u32.to_be_bytes());
        mdhd.extend_from_slice(&[0; 4]);
        let mut hdlr = vec![0; 8];
        hdlr.extend_from_slice(b"vide");
        hdlr.extend_from_slice(&[0; 12]);
        let mdia = make_box(
            b"mdia",
            &[make_box(b"mdhd", &mdhd), make_box(b"hdlr", &hdlr)].concat(),
        );
        let trak = make_box(b"trak", &[make_box(b"tkhd", &tkhd), mdia].concat());
        [
            make_box(b"ftyp", b"isom\0\0\x02\0"),
            make_box(b"moov", &[make_box(b"mvhd", &mvhd), trak].concat()),
            make_box(b"mdat", b"test"),
        ]
        .concat()
    }
    fn jpeg_with_xmp(xmp: &str) -> Vec<u8> {
        let packet = [b"http://ns.adobe.com/xap/1.0/\0".as_slice(), xmp.as_bytes()].concat();
        [
            vec![0xff, 0xd8, 0xff, 0xe1],
            ((packet.len() + 2) as u16).to_be_bytes().to_vec(),
            packet,
            vec![0xff, 0xd9],
        ]
        .concat()
    }

    fn android_motion_photo() -> Vec<u8> {
        let video = fake_mp4();
        let xmp = format!(
            r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/" xmlns:Container="http://ns.google.com/photos/1.0/container/" xmlns:Item="http://ns.google.com/photos/1.0/container/item/" Camera:MotionPhoto="1" Camera:MotionPhotoVersion="1" Camera:MotionPhotoPresentationTimestampUs="1417000"><Container:Directory><rdf:Seq><rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="image/jpeg" Item:Semantic="Primary" Item:Length="0" Item:Padding="0"/></rdf:li><rdf:li rdf:parseType="Resource"><Container:Item Item:Mime="video/mp4" Item:Semantic="MotionPhoto" Item:Length="{}" Item:Padding="0"/></rdf:li></rdf:Seq></Container:Directory></rdf:Description></rdf:RDF></x:xmpmeta>"#,
            video.len()
        );
        let mut data = jpeg_with_xmp(&xmp);
        data.extend_from_slice(&video);
        data
    }

    #[test]
    fn routes_android_motion_photo_before_hdr_probe() {
        let data = android_motion_photo();
        let asset = probe_bytes(&data).unwrap();
        let SourceAsset::MotionPhoto {
            source_kind,
            video,
            presentation_timestamp_us,
            stream_count,
            ..
        } = asset
        else {
            panic!("expected Motion Photo routing");
        };
        assert_eq!(source_kind, "androidMotionPhotoV1");
        assert!(video.length > 0);
        assert_eq!(presentation_timestamp_us, Some(1_417_000));
        assert_eq!(stream_count, 1);
    }

    #[test]
    fn unsupported_bytes_do_not_claim_a_product_route() {
        let error = probe_bytes(&[0_u8; 32]).unwrap_err();
        assert!(matches!(error, SourceProbeError::Unsupported(_)));
    }

    #[test]
    fn malformed_motion_metadata_is_not_silently_treated_as_hdr() {
        let data = jpeg_with_xmp("<x:xmpmeta><broken>");
        let error = probe_bytes(&data).unwrap_err();
        assert!(matches!(error, SourceProbeError::MotionPhoto(_)));
    }
}
