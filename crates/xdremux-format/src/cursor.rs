use std::ops::Range;

use crate::error::{FormatError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endian {
    Big,
    Little,
}

#[derive(Debug, Clone)]
pub struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
    end: usize,
    endian: Endian,
    context: &'static str,
}

impl<'a> Cursor<'a> {
    pub fn new(data: &'a [u8], endian: Endian, context: &'static str) -> Self {
        Self {
            data,
            pos: 0,
            end: data.len(),
            endian,
            context,
        }
    }

    pub fn bounded(
        data: &'a [u8],
        range: Range<usize>,
        endian: Endian,
        context: &'static str,
    ) -> Result<Self> {
        if range.start > range.end || range.end > data.len() {
            return Err(FormatError::invalid(
                context,
                format!(
                    "bounded cursor range {}..{} exceeds input length {}",
                    range.start,
                    range.end,
                    data.len()
                ),
            ));
        }
        Ok(Self {
            data,
            pos: range.start,
            end: range.end,
            endian,
            context,
        })
    }

    pub const fn position(&self) -> usize {
        self.pos
    }

    pub const fn end(&self) -> usize {
        self.end
    }

    pub const fn remaining(&self) -> usize {
        self.end - self.pos
    }

    pub const fn is_empty(&self) -> bool {
        self.pos == self.end
    }

    pub fn set_position(&mut self, position: usize) -> Result<()> {
        if position > self.end {
            return Err(FormatError::UnexpectedEof {
                context: self.context,
                offset: position,
                needed: 0,
                end: self.end,
            });
        }
        self.pos = position;
        Ok(())
    }

    pub fn skip(&mut self, count: usize) -> Result<()> {
        let next = self
            .pos
            .checked_add(count)
            .ok_or_else(|| FormatError::overflow(self.context))?;
        if next > self.end {
            return Err(FormatError::UnexpectedEof {
                context: self.context,
                offset: self.pos,
                needed: count,
                end: self.end,
            });
        }
        self.pos = next;
        Ok(())
    }

    pub fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let start = self.pos;
        let next = start
            .checked_add(count)
            .ok_or_else(|| FormatError::overflow(self.context))?;
        if next > self.end {
            return Err(FormatError::UnexpectedEof {
                context: self.context,
                offset: start,
                needed: count,
                end: self.end,
            });
        }
        let bytes = self.data.get(start..next).ok_or_else(|| {
            FormatError::UnexpectedEof {
                context: self.context,
                offset: start,
                needed: count,
                end: self.end,
            }
        })?;
        self.pos = next;
        Ok(bytes)
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        let bytes = self.take(2)?;
        Ok(match self.endian {
            Endian::Big => u16::from_be_bytes([bytes[0], bytes[1]]),
            Endian::Little => u16::from_le_bytes([bytes[0], bytes[1]]),
        })
    }

    pub fn read_u24(&mut self) -> Result<u32> {
        let bytes = self.take(3)?;
        Ok(match self.endian {
            Endian::Big => {
                (u32::from(bytes[0]) << 16) | (u32::from(bytes[1]) << 8) | u32::from(bytes[2])
            }
            Endian::Little => {
                u32::from(bytes[0]) | (u32::from(bytes[1]) << 8) | (u32::from(bytes[2]) << 16)
            }
        })
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        let bytes = self.take(4)?;
        Ok(match self.endian {
            Endian::Big => u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            Endian::Little => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        })
    }

    pub fn read_u64(&mut self) -> Result<u64> {
        let bytes = self.take(8)?;
        Ok(match self.endian {
            Endian::Big => u64::from_be_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]),
            Endian::Little => u64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]),
        })
    }

    pub fn read_uint(&mut self, width: usize) -> Result<u64> {
        if width > 8 {
            return Err(FormatError::Unsupported {
                context: self.context,
                value: width as u64,
            });
        }
        let bytes = self.take(width)?;
        let mut value = 0u64;
        match self.endian {
            Endian::Big => {
                for byte in bytes {
                    value = (value << 8) | u64::from(*byte);
                }
            }
            Endian::Little => {
                for (shift, byte) in bytes.iter().enumerate() {
                    value |= u64::from(*byte) << (shift * 8);
                }
            }
        }
        Ok(value)
    }

    pub fn read_c_string(&mut self) -> Result<&'a [u8]> {
        let remaining = self.data.get(self.pos..self.end).ok_or_else(|| {
            FormatError::UnexpectedEof {
                context: self.context,
                offset: self.pos,
                needed: 1,
                end: self.end,
            }
        })?;
        let relative_end = remaining
            .iter()
            .position(|byte| *byte == 0)
            .ok_or_else(|| FormatError::invalid(self.context, "unterminated string"))?;
        let start = self.pos;
        self.pos = self
            .pos
            .checked_add(relative_end + 1)
            .ok_or_else(|| FormatError::overflow(self.context))?;
        self.data
            .get(start..start + relative_end)
            .ok_or_else(|| FormatError::invalid(self.context, "string range is outside input"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endian_reads_are_explicit_and_bounded() {
        let data = [0x01, 0x02, 0x03, 0x04];
        let mut be = Cursor::new(&data, Endian::Big, "test");
        assert_eq!(be.read_u16().unwrap(), 0x0102);
        assert_eq!(be.read_u16().unwrap(), 0x0304);
        assert!(be.read_u8().is_err());

        let mut le = Cursor::new(&data, Endian::Little, "test");
        assert_eq!(le.read_u32().unwrap(), 0x0403_0201);
    }

    #[test]
    fn variable_width_integer_rejects_width_over_eight() {
        let data = [0xff; 8];
        let mut cursor = Cursor::new(&data, Endian::Big, "test");
        assert_eq!(cursor.read_uint(0).unwrap(), 0);
        assert_eq!(cursor.read_uint(8).unwrap(), u64::MAX);
        assert!(cursor.read_uint(9).is_err());
    }

    #[test]
    fn bounded_cursor_cannot_seek_or_read_past_end() {
        let data = [0u8; 8];
        let mut cursor = Cursor::bounded(&data, 2..6, Endian::Big, "test").unwrap();
        assert_eq!(cursor.remaining(), 4);
        cursor.set_position(6).unwrap();
        assert!(cursor.read_u8().is_err());
        assert!(cursor.set_position(7).is_err());
    }
}
