use crate::{iso21496, Result};

/// Serialize the canonical product-facing ISO 21496-1 tmap payload.
///
/// ImageIO's native representation omits three reserved GainMapMetadata bytes.
/// The product boundary restores those bytes so the portable Rust output carries
/// the strict ISO payload. The 62-byte ImageIO oracle remains available as
/// `iso21496::make_apple_tmap_payload` for conformance and compatibility work.
pub fn make_apple_tmap_payload(info: &[f64]) -> Result<Vec<u8>> {
    let imageio_payload = iso21496::make_apple_tmap_payload(info)?;
    iso21496::make_strict_tmap_payload(&imageio_payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_payload_restores_iso_reserved_bytes() {
        let ratio = 4.926108360290527;
        let info = [
            1.0, 1.0, 1.0, 1.0, ratio, ratio, ratio, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            1.0, ratio, ratio, 0.0,
        ];
        let native = iso21496::make_apple_tmap_payload(&info).unwrap();
        let product = make_apple_tmap_payload(&info).unwrap();
        assert_eq!(native.len(), 62);
        assert_eq!(product.len(), 65);
        assert_eq!(&product[..6], &native[..6]);
        assert_eq!(&product[6..9], &[0, 0, 0]);
        assert_eq!(&product[9..], &native[6..]);
    }
}
