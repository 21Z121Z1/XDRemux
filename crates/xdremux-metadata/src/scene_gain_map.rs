use crate::{MetadataError, Result};

/// ImageIO-compatible ISO metadata with signed gain bounds, computed in the
/// alternate image's linear RGB primaries. Equal parameters apply independently
/// to every channel of an RGB gain image.
pub fn make_scene_gain_map_metadata(
    min_ev: f64,
    max_ev: f64,
    offset: f64,
) -> Result<(Vec<u8>, Vec<u8>)> {
    make_scene_gain_map_metadata_with_capacity(min_ev, max_ev, offset, max_ev.max(1.))
}

/// Display capacity and local pixel gain are independent quantities.
pub fn make_scene_gain_map_metadata_with_capacity(
    min_ev: f64,
    max_ev: f64,
    offset: f64,
    capacity: f64,
) -> Result<(Vec<u8>, Vec<u8>)> {
    if !min_ev.is_finite()
        || !max_ev.is_finite()
        || !offset.is_finite()
        || min_ev > 0.
        || max_ev <= 0.
        || min_ev >= max_ev
        || min_ev < -16.
        || max_ev > 16.
        || offset < 0.
        || !capacity.is_finite()
        || !(1. ..=16.).contains(&capacity)
    {
        return Err(MetadataError::invalid(
            "scene Gain Map metadata",
            "invalid signed gain range or offset",
        ));
    }
    let tmap = crate::iso21496::make_log_tmap(
        [0., capacity],
        [[min_ev, max_ev, 1., offset, offset]; 3],
        false,
        1,
    )?;
    let xmp = crate::iso21496::write_hdrgm_xmp(
        [min_ev; 3],
        [max_ev; 3],
        [1.; 3],
        [[offset; 3]; 2],
        [0., capacity],
    );
    Ok((tmap, xmp))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rational(bytes: &[u8], offset: usize) -> f64 {
        f64::from(i32::from_be_bytes(
            bytes[offset..offset + 4].try_into().unwrap(),
        )) / f64::from(u32::from_be_bytes(
            bytes[offset + 4..offset + 8].try_into().unwrap(),
        ))
    }
    #[test]
    fn scene_gain_uses_the_alternate_working_primaries() {
        let (bytes, _) = make_scene_gain_map_metadata(-3., 6., 1. / 64.).unwrap();
        assert_eq!(
            bytes[5] & 0x40,
            0,
            "scene gains must use the alternate working primaries"
        );
    }

    #[test]
    fn serializes_negative_gain_minimum_without_clamping_shadows() {
        let (bytes, xmp) = make_scene_gain_map_metadata(-3., 6., 1. / 64.).unwrap();
        assert_eq!(bytes.len(), 62);
        assert_eq!(rational(&bytes, 22), -3.);
        assert_eq!(rational(&bytes, 46), 1. / 64.);
        assert!(String::from_utf8(xmp)
            .unwrap()
            .contains("<hdrgm:GainMapMin>-3 -3 -3</hdrgm:GainMapMin>"));
        assert!(make_scene_gain_map_metadata(1., 6., 0.).is_err());
    }
    #[test]
    fn display_capacity_is_not_the_largest_local_gain() {
        let (bytes, xmp) =
            make_scene_gain_map_metadata_with_capacity(-4., 7., 1. / 64., 3.).unwrap();
        assert_eq!(rational(&bytes, 14), 3.);
        assert!(String::from_utf8(xmp)
            .unwrap()
            .contains("<hdrgm:HDRCapacityMax>3</hdrgm:HDRCapacityMax>"));
        assert!(make_scene_gain_map_metadata_with_capacity(-4., 7., 1. / 64., f64::NAN).is_err());
    }
}
