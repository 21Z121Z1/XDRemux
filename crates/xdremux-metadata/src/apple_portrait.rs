use crate::{MetadataError, Result};

const CONTEXT: &str = "Apple Portrait Focus XMP";

fn validate_normalized(name: &str, value: f64) -> Result<()> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(MetadataError::invalid(
            CONTEXT,
            format!("{name} must be finite and within 0 through 1"),
        ));
    }
    Ok(())
}

/// Build the primary-image XMP region that identifies the producer focus.
///
/// The region coordinates are deliberately supplied by Rust product code. The
/// Apple adapter receives the resulting XMP as an opaque metadata primitive and
/// does not choose, transform, or synthesize the focus policy.
pub fn make_apple_portrait_focus_xmp(
    width: u32,
    height: u32,
    x: f64,
    y: f64,
    region_width: f64,
    region_height: f64,
) -> Result<Vec<u8>> {
    if width == 0 || height == 0 {
        return Err(MetadataError::invalid(
            CONTEXT,
            "applied-to dimensions must be non-zero",
        ));
    }
    for (name, value) in [
        ("x", x),
        ("y", y),
        ("region_width", region_width),
        ("region_height", region_height),
    ] {
        validate_normalized(name, value)?;
    }
    if region_width == 0.0 || region_height == 0.0 {
        return Err(MetadataError::invalid(
            CONTEXT,
            "focus region dimensions must be positive",
        ));
    }

    let xml = format!(
        concat!(
            "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"XDRemux\">",
            "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">",
            "<rdf:Description rdf:about=\"\" ",
            "xmlns:mwg-rs=\"http://www.metadataworkinggroup.com/schemas/regions/\" ",
            "xmlns:stArea=\"http://ns.adobe.com/xmp/sType/Area#\" ",
            "xmlns:stDim=\"http://ns.adobe.com/xap/1.0/sType/Dimensions#\">",
            "<mwg-rs:Regions rdf:parseType=\"Resource\">",
            "<mwg-rs:AppliedToDimensions rdf:parseType=\"Resource\">",
            "<stDim:h>{height}</stDim:h><stDim:unit>pixel</stDim:unit>",
            "<stDim:w>{width}</stDim:w></mwg-rs:AppliedToDimensions>",
            "<mwg-rs:RegionList><rdf:Bag><rdf:li rdf:parseType=\"Resource\">",
            "<mwg-rs:Area rdf:parseType=\"Resource\">",
            "<stArea:h>{region_height:.9}</stArea:h><stArea:unit>normalized</stArea:unit>",
            "<stArea:w>{region_width:.9}</stArea:w>",
            "<stArea:x>{x:.9}</stArea:x><stArea:y>{y:.9}</stArea:y>",
            "</mwg-rs:Area><mwg-rs:Type>Focus</mwg-rs:Type>",
            "</rdf:li></rdf:Bag></mwg-rs:RegionList>",
            "</mwg-rs:Regions></rdf:Description></rdf:RDF></x:xmpmeta>"
        ),
        width = width,
        height = height,
        x = x,
        y = y,
        region_width = region_width,
        region_height = region_height,
    );
    Ok(xml.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_xmp_contains_consumer_region_and_type() {
        let xmp = String::from_utf8(
            make_apple_portrait_focus_xmp(4000, 3000, 0.25, 0.4, 0.12, 0.18).unwrap(),
        )
        .unwrap();
        assert!(xmp.contains("<mwg-rs:Type>Focus</mwg-rs:Type>"));
        assert!(xmp.contains("<stDim:w>4000</stDim:w>"));
        assert!(xmp.contains("<stArea:x>0.250000000</stArea:x>"));
        assert!(xmp.contains("<stArea:h>0.180000000</stArea:h>"));
    }

    #[test]
    fn focus_xmp_rejects_invalid_geometry_and_coordinates() {
        assert!(make_apple_portrait_focus_xmp(0, 3000, 0.5, 0.5, 0.1, 0.1).is_err());
        assert!(make_apple_portrait_focus_xmp(4000, 3000, -0.1, 0.5, 0.1, 0.1).is_err());
        assert!(make_apple_portrait_focus_xmp(4000, 3000, 0.5, 0.5, 0.0, 0.1).is_err());
    }
}
