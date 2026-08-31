use std::fmt;

use crate::error::{FormatError, Result};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FourCC([u8; 4]);

impl FourCC {
    pub const fn new(bytes: [u8; 4]) -> Self {
        Self(bytes)
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 4 {
            return Err(FormatError::invalid(
                "fourcc",
                format!("expected exactly 4 bytes, got {}", bytes.len()),
            ));
        }
        Ok(Self([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub const fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }
}

impl fmt::Debug for FourCC {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FourCC(\"")?;
        fmt::Display::fmt(self, f)?;
        write!(f, "\")")
    }
}

impl fmt::Display for FourCC {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            if byte.is_ascii_graphic() || byte == b' ' {
                write!(f, "{}", char::from(byte))?;
            } else {
                write!(f, "\\x{byte:02x}")?;
            }
        }
        Ok(())
    }
}

impl From<[u8; 4]> for FourCC {
    fn from(value: [u8; 4]) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_fourcc_round_trips_high_bytes() {
        let raw = [0xa9, b'T', b'O', b'O'];
        let code = FourCC::new(raw);
        assert_eq!(*code.as_bytes(), raw);
        assert_eq!(format!("{code}"), "\\xa9TOO");
    }
}
