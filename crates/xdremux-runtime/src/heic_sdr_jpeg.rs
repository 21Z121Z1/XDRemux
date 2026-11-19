//! Turn an HEIC primary into an 8-bit sRGB JPEG for galleries that do not
//! play HEIC Motion Photos. The gain map is not applied.

use jpeg_encoder::ColorType;

use xdremux_codec::LibHeifProvider;

use crate::{Result, RuntimeError};

pub(crate) const JPEG_QUALITY: u8 = 90;

pub(crate) fn encode_sdr_jpeg(heif: &[u8]) -> Result<Vec<u8>> {
    let raster = LibHeifProvider::new()
        .decode_sdr_display_rgb8(heif)
        .map_err(|error| RuntimeError::external("HEIC SDR JPEG", error))?;
    let width = u16::try_from(raster.width).map_err(|_| {
        RuntimeError::new(
            "HEIC SDR JPEG",
            "decoded still is wider than a JPEG dimension",
        )
    })?;
    let height = u16::try_from(raster.height).map_err(|_| {
        RuntimeError::new(
            "HEIC SDR JPEG",
            "decoded still is taller than a JPEG dimension",
        )
    })?;
    let packed = if raster.bytes_per_row == usize::from(width) * 3 {
        raster.data
    } else {
        let mut tight = Vec::with_capacity(usize::from(width) * usize::from(height) * 3);
        for row in 0..usize::from(height) {
            let start = row * raster.bytes_per_row;
            tight.extend_from_slice(&raster.data[start..start + usize::from(width) * 3]);
        }
        tight
    };
    let mut jpeg = Vec::new();
    jpeg_encoder::Encoder::new(&mut jpeg, JPEG_QUALITY)
        .encode(&packed, width, height, ColorType::Rgb)
        .map_err(|error| RuntimeError::external("HEIC SDR JPEG encode", error))?;
    if !jpeg.starts_with(&[0xff, 0xd8]) {
        return Err(RuntimeError::new(
            "HEIC SDR JPEG",
            "encoder did not write a JPEG SOI",
        ));
    }
    if let Some(payload) = LibHeifProvider::new()
        .primary_exif_payload(heif)
        .map_err(|error| RuntimeError::external("HEIC SDR JPEG EXIF", error))?
    {
        let app1 = exif_app1(&payload)?;
        jpeg.splice(2..2, app1);
    }
    Ok(jpeg)
}

/// Keep Make, Model, and the date tags. Orientation is 1 because `irot` was
/// already applied to the pixels. Other EXIF fields are not copied.
fn exif_app1(payload: &[u8]) -> Result<Vec<u8>> {
    let tiff = heif_exif_tiff(payload)?;
    let kept = read_identity_tags(tiff)?;
    let mut body = Vec::new();
    body.extend_from_slice(b"Exif\0\0");
    body.extend(minimal_tiff(&kept)?);
    if body.len() + 2 > usize::from(u16::MAX) {
        return Err(RuntimeError::new(
            "HEIC SDR JPEG",
            "EXIF identity tags do not fit in one APP1 segment",
        ));
    }
    let mut app1 = Vec::with_capacity(body.len() + 4);
    app1.extend_from_slice(&[0xff, 0xe1]);
    app1.extend(((body.len() + 2) as u16).to_be_bytes());
    app1.extend(body);
    Ok(app1)
}

struct IdentityTags {
    make: Option<Vec<u8>>,
    model: Option<Vec<u8>>,
    date_time: Option<Vec<u8>>,
    original: Option<Vec<u8>>,
    digitized: Option<Vec<u8>>,
}

fn heif_exif_tiff(payload: &[u8]) -> Result<&[u8]> {
    let payload = if payload.starts_with(b"Exif\0\0") {
        &payload[6..]
    } else {
        payload
    };
    if payload.len() < 8 {
        return Err(RuntimeError::new(
            "HEIC SDR JPEG",
            "EXIF payload does not contain a TIFF header",
        ));
    }
    let offset = u32::from_be_bytes(payload[..4].try_into().expect("four bytes")) as usize;
    let start = 4usize
        .checked_add(offset)
        .ok_or_else(|| RuntimeError::new("HEIC SDR JPEG", "EXIF TIFF offset overflows"))?;
    payload.get(start..).ok_or_else(|| {
        RuntimeError::new("HEIC SDR JPEG", "EXIF TIFF offset is outside the payload")
    })
}

fn read_identity_tags(tiff: &[u8]) -> Result<IdentityTags> {
    let (endian_le, ifd) = tiff_ifd_offset(tiff)?;
    let mut kept = IdentityTags {
        make: None,
        model: None,
        date_time: None,
        original: None,
        digitized: None,
    };
    let exif_ifd = read_ascii_tags(tiff, ifd, endian_le, true, &mut kept)?;
    if let Some(exif_ifd) = exif_ifd {
        read_ascii_tags(tiff, exif_ifd, endian_le, false, &mut kept)?;
    }
    Ok(kept)
}

fn tiff_ifd_offset(tiff: &[u8]) -> Result<(bool, usize)> {
    let endian_le = match tiff.get(..2) {
        Some(b"II") => true,
        Some(b"MM") => false,
        _ => {
            return Err(RuntimeError::new(
                "HEIC SDR JPEG",
                "EXIF TIFF byte order is missing",
            ))
        }
    };
    let read_u16 = |offset: usize| -> Result<u16> {
        let bytes = tiff
            .get(offset..offset + 2)
            .ok_or_else(|| RuntimeError::new("HEIC SDR JPEG", "EXIF TIFF header is truncated"))?;
        Ok(if endian_le {
            u16::from_le_bytes(bytes.try_into().expect("two bytes"))
        } else {
            u16::from_be_bytes(bytes.try_into().expect("two bytes"))
        })
    };
    let read_u32 = |offset: usize| -> Result<u32> {
        let bytes = tiff
            .get(offset..offset + 4)
            .ok_or_else(|| RuntimeError::new("HEIC SDR JPEG", "EXIF TIFF header is truncated"))?;
        Ok(if endian_le {
            u32::from_le_bytes(bytes.try_into().expect("four bytes"))
        } else {
            u32::from_be_bytes(bytes.try_into().expect("four bytes"))
        })
    };
    if read_u16(2)? != 42 {
        return Err(RuntimeError::new(
            "HEIC SDR JPEG",
            "EXIF TIFF magic is invalid",
        ));
    }
    Ok((endian_le, read_u32(4)? as usize))
}

fn read_ascii_tags(
    tiff: &[u8],
    ifd: usize,
    endian_le: bool,
    ifd0: bool,
    kept: &mut IdentityTags,
) -> Result<Option<usize>> {
    let count = read_u16(tiff, ifd, endian_le)? as usize;
    let mut exif_ifd = None;
    for index in 0..count {
        let entry = ifd
            .checked_add(2)
            .and_then(|start| start.checked_add(index * 12))
            .ok_or_else(|| RuntimeError::new("HEIC SDR JPEG", "EXIF IFD entry offset overflows"))?;
        let tag = read_u16(tiff, entry, endian_le)?;
        let kind = read_u16(tiff, entry + 2, endian_le)?;
        let count = read_u32(tiff, entry + 4, endian_le)? as usize;
        let inline = tiff
            .get(entry + 8..entry + 12)
            .ok_or_else(|| RuntimeError::new("HEIC SDR JPEG", "EXIF IFD entry is truncated"))?;
        if ifd0 && tag == 0x8769 && kind == 4 && count == 1 {
            exif_ifd = Some(read_u32(tiff, entry + 8, endian_le)? as usize);
            continue;
        }
        let wanted = if ifd0 {
            matches!(tag, 0x010f | 0x0110 | 0x0132)
        } else {
            matches!(tag, 0x9003 | 0x9004)
        };
        if !wanted || kind != 2 {
            continue;
        }
        let bytes = ascii_value(tiff, inline, count, endian_le)?;
        let slot = match tag {
            0x010f => &mut kept.make,
            0x0110 => &mut kept.model,
            0x0132 => &mut kept.date_time,
            0x9003 => &mut kept.original,
            0x9004 => &mut kept.digitized,
            _ => continue,
        };
        *slot = Some(bytes);
    }
    Ok(exif_ifd)
}

fn ascii_value(tiff: &[u8], inline: &[u8], count: usize, endian_le: bool) -> Result<Vec<u8>> {
    if !(2..=64).contains(&count) {
        return Err(RuntimeError::new(
            "HEIC SDR JPEG",
            "EXIF identity tag length is not usable",
        ));
    }
    let bytes = if count <= 4 {
        inline[..count].to_vec()
    } else {
        let offset = if endian_le {
            u32::from_le_bytes(inline.try_into().expect("four bytes"))
        } else {
            u32::from_be_bytes(inline.try_into().expect("four bytes"))
        } as usize;
        tiff.get(offset..offset + count)
            .ok_or_else(|| {
                RuntimeError::new("HEIC SDR JPEG", "EXIF ASCII tag is outside the TIFF")
            })?
            .to_vec()
    };
    if bytes.last() != Some(&0)
        || bytes[..bytes.len() - 1]
            .iter()
            .any(|byte| !byte.is_ascii_graphic() && *byte != b' ')
    {
        return Err(RuntimeError::new(
            "HEIC SDR JPEG",
            "EXIF identity tag is not printable ASCII",
        ));
    }
    Ok(bytes)
}

fn read_u16(tiff: &[u8], offset: usize, endian_le: bool) -> Result<u16> {
    let bytes = tiff
        .get(offset..offset + 2)
        .ok_or_else(|| RuntimeError::new("HEIC SDR JPEG", "EXIF field is truncated"))?;
    Ok(if endian_le {
        u16::from_le_bytes(bytes.try_into().expect("two bytes"))
    } else {
        u16::from_be_bytes(bytes.try_into().expect("two bytes"))
    })
}

fn read_u32(tiff: &[u8], offset: usize, endian_le: bool) -> Result<u32> {
    let bytes = tiff
        .get(offset..offset + 4)
        .ok_or_else(|| RuntimeError::new("HEIC SDR JPEG", "EXIF field is truncated"))?;
    Ok(if endian_le {
        u32::from_le_bytes(bytes.try_into().expect("four bytes"))
    } else {
        u32::from_be_bytes(bytes.try_into().expect("four bytes"))
    })
}

fn minimal_tiff(tags: &IdentityTags) -> Result<Vec<u8>> {
    let mut strings = Vec::new();
    let mut entries = vec![(0x0112u16, 3u16, 1u32, 1u32)];
    for (tag, value) in [
        (0x010f, tags.make.as_deref()),
        (0x0110, tags.model.as_deref()),
        (0x0132, tags.date_time.as_deref()),
    ] {
        if let Some(value) = value {
            let offset = (strings.len() as u32) | 0x8000_0000;
            strings.extend(value);
            entries.push((tag, 2, value.len() as u32, offset));
        }
    }
    let sub = [tags.original.as_deref(), tags.digitized.as_deref()];
    let has_sub = sub.iter().any(|value| value.is_some());
    if has_sub {
        entries.push((0x8769, 4, 1, 0));
    }
    entries.sort_by_key(|entry| entry.0);
    let ifd0_bytes = 2 + entries.len() * 12 + 4;
    let mut sub_entries = Vec::new();
    if has_sub {
        for (tag, value) in [(0x9003u16, sub[0]), (0x9004, sub[1])] {
            if let Some(value) = value {
                let offset = (strings.len() as u32) | 0x8000_0000;
                strings.extend(value);
                sub_entries.push((tag, 2u16, value.len() as u32, offset));
            }
        }
    }
    let sub_at = ifd0_bytes;
    let strings_at = sub_at
        + if has_sub {
            2 + sub_entries.len() * 12 + 4
        } else {
            0
        };
    if let Some(entry) = entries.iter_mut().find(|entry| entry.0 == 0x8769) {
        entry.3 = sub_at as u32;
    }
    let mut tiff = vec![0u8; strings_at + strings.len()];
    tiff[0..2].copy_from_slice(b"II");
    tiff[2..4].copy_from_slice(&42u16.to_le_bytes());
    tiff[4..8].copy_from_slice(&8u32.to_le_bytes());
    write_ifd(&mut tiff, 8, &entries, strings_at)?;
    if has_sub {
        write_ifd(&mut tiff, sub_at, &sub_entries, strings_at)?;
    }
    tiff[strings_at..].copy_from_slice(&strings);
    Ok(tiff)
}

fn write_ifd(
    tiff: &mut [u8],
    at: usize,
    entries: &[(u16, u16, u32, u32)],
    strings_at: usize,
) -> Result<()> {
    tiff[at..at + 2].copy_from_slice(&(entries.len() as u16).to_le_bytes());
    for (index, (tag, kind, count, value)) in entries.iter().copied().enumerate() {
        let entry = at + 2 + index * 12;
        tiff[entry..entry + 2].copy_from_slice(&tag.to_le_bytes());
        tiff[entry + 2..entry + 4].copy_from_slice(&kind.to_le_bytes());
        tiff[entry + 4..entry + 8].copy_from_slice(&count.to_le_bytes());
        let stored = if value & 0x8000_0000 != 0 {
            strings_at as u32 + (value & 0x7fff_ffff)
        } else {
            value
        };
        tiff[entry + 8..entry + 12].copy_from_slice(&stored.to_le_bytes());
    }
    let next = at + 2 + entries.len() * 12;
    tiff[next..next + 4].copy_from_slice(&0u32.to_le_bytes());
    Ok(())
}
