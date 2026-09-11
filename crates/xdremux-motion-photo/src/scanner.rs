use crate::error::{MotionPhotoError, Result};
use crate::model::ByteRange;

fn checked_usize(value: u64) -> Result<usize> {
    usize::try_from(value).map_err(|_| MotionPhotoError::ArithmeticOverflow)
}

pub fn is_ftyp_box_start(data: &[u8], offset: u64, upper_bound: u64) -> Result<bool> {
    if upper_bound <= offset {
        return Ok(false);
    }
    let offset = checked_usize(offset)?;
    let upper_bound = checked_usize(upper_bound)?;
    if upper_bound > data.len() || upper_bound.saturating_sub(offset) < 16 {
        return Ok(false);
    }
    let header_end = offset
        .checked_add(8)
        .ok_or(MotionPhotoError::ArithmeticOverflow)?;
    if header_end > upper_bound || &data[offset + 4..header_end] != b"ftyp" {
        return Ok(false);
    }
    let size32 = u32::from_be_bytes(
        data[offset..offset + 4]
            .try_into()
            .map_err(|_| MotionPhotoError::InvalidVideoPayload)?,
    );
    let available = upper_bound - offset;
    let brand_start = if size32 == 1 {
        if available < 24 || offset + 24 > data.len() {
            return Ok(false);
        }
        let large_size = u64::from_be_bytes(
            data[offset + 8..offset + 16]
                .try_into()
                .map_err(|_| MotionPhotoError::InvalidVideoPayload)?,
        );
        if large_size < 24
            || large_size
                > u64::try_from(available).map_err(|_| MotionPhotoError::ArithmeticOverflow)?
        {
            return Ok(false);
        }
        offset + 16
    } else {
        if size32 < 16
            || u64::from(size32)
                > u64::try_from(available).map_err(|_| MotionPhotoError::ArithmeticOverflow)?
        {
            return Ok(false);
        }
        offset + 8
    };
    let brand_end = brand_start
        .checked_add(4)
        .ok_or(MotionPhotoError::ArithmeticOverflow)?;
    if brand_end > upper_bound {
        return Ok(false);
    }
    Ok(data[brand_start..brand_end]
        .iter()
        .all(|byte| (0x20..=0x7e).contains(byte)))
}

pub fn ftyp_box_offsets(data: &[u8], range: ByteRange, buffer_size: usize) -> Result<Vec<u64>> {
    if buffer_size < 64 {
        return Err(MotionPhotoError::InvalidByteRange);
    }
    let start = checked_usize(range.lower_bound)?;
    let end = checked_usize(range.upper_bound)?;
    if end > data.len() || start > end {
        return Err(MotionPhotoError::InvalidByteRange);
    }

    let mut rough = Vec::new();
    if end.saturating_sub(start) >= 8 {
        let haystack = &data[start..end];
        for index in 4..haystack.len().saturating_sub(3) {
            if &haystack[index..index + 4] == b"ftyp" {
                let candidate = start + index - 4;
                let candidate_u64 =
                    u64::try_from(candidate).map_err(|_| MotionPhotoError::ArithmeticOverflow)?;
                if is_ftyp_box_start(data, candidate_u64, range.upper_bound)? {
                    rough.push(candidate_u64);
                }
            }
        }
    }
    rough.sort_unstable();
    rough.dedup();
    Ok(rough)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ftyp(brand: &[u8; 4]) -> Vec<u8> {
        let mut output = vec![0, 0, 0, 16];
        output.extend_from_slice(b"ftyp");
        output.extend_from_slice(brand);
        output.extend_from_slice(&[0, 0, 0, 0]);
        output
    }

    #[test]
    fn validates_printable_brand_and_rejects_embedded_needle() {
        let mut data = b"noise ftyp noise".to_vec();
        let real = data.len() as u64;
        data.extend_from_slice(&ftyp(b"isom"));
        let range = ByteRange::new(0, data.len() as u64).unwrap();
        assert_eq!(ftyp_box_offsets(&data, range, 64).unwrap(), vec![real]);
    }

    #[test]
    fn truncated_largesize_fails_closed() {
        let data = [0, 0, 0, 1, b'f', b't', b'y', b'p', 0, 0, 0, 0];
        assert!(!is_ftyp_box_start(&data, 0, data.len() as u64).unwrap());
    }
}
