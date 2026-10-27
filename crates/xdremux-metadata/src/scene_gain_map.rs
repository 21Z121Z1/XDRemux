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
    let values = [0., capacity, min_ev, max_ev, 1., offset, offset];
    let mut tmap = vec![0, 0, 0, 0, 0, 0];
    for value in values {
        tmap.extend(((value * 1_000_000.).round() as i32).to_be_bytes());
        tmap.extend(1_000_000u32.to_be_bytes());
    }
    let xmp = format!(
        r#"<?xpacket begin=""?><x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:hdrgm="http://ns.adobe.com/hdr-gain-map/1.0/" hdrgm:Version="1.0" hdrgm:BaseRenditionIsHDR="False" hdrgm:GainMapMin="{min_ev:.8}" hdrgm:GainMapMax="{max_ev:.8}" hdrgm:Gamma="1" hdrgm:OffsetSDR="{offset:.8}" hdrgm:OffsetHDR="{offset:.8}" hdrgm:HDRCapacityMin="0" hdrgm:HDRCapacityMax="{capacity:.8}"/></rdf:RDF></x:xmpmeta><?xpacket end="w"?>"#
    );
    Ok((tmap, xmp.into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scene_gain_uses_the_alternate_working_primaries() {
        let (bytes, _) = make_scene_gain_map_metadata(-3., 6., 1. / 64.).unwrap();
        assert_eq!(
            bytes[5] & 0x40,
            0,
            "P3 gains must not use the preserved base primaries"
        );
    }

    #[test]
    fn serializes_negative_gain_minimum_without_clamping_shadows() {
        let (bytes, xmp) = make_scene_gain_map_metadata(-3., 6., 1. / 64.).unwrap();
        assert_eq!(bytes.len(), 62);
        assert_eq!(
            i32::from_be_bytes(bytes[22..26].try_into().unwrap()),
            -3_000_000
        );
        assert!(String::from_utf8(xmp)
            .unwrap()
            .contains("GainMapMin=\"-3.00000000\""));
        assert!(make_scene_gain_map_metadata(1., 6., 0.).is_err());
    }
    #[test]
    fn display_capacity_is_not_the_largest_local_gain() {
        let (bytes, xmp) =
            make_scene_gain_map_metadata_with_capacity(-4., 7., 1. / 64., 3.).unwrap();
        assert_eq!(
            i32::from_be_bytes(bytes[14..18].try_into().unwrap()),
            3_000_000
        );
        assert!(String::from_utf8(xmp)
            .unwrap()
            .contains("HDRCapacityMax=\"3.00000000\""));
        assert!(make_scene_gain_map_metadata_with_capacity(-4., 7., 1. / 64., f64::NAN).is_err());
    }
}
