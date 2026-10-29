use xdremux_engine::{apple_style_histogram_light_map, AppleStyleLightMapRequest};

#[test]
fn histogram_writer_keeps_native_half_floor_and_quantized_median() {
    // Ordered AIR tail, independently retained in research reference 979bc34:
    // lightmap_model_s55.py and the 269-case native s56 tail comparison.
    for (input, expected) in [
        (0.0, 0x2937_u16),
        (0.125, 0x302f),
        (0.5, 0x366b),
        (1.0, 0x3a11),
    ] {
        let values = vec![input; 32 * 32];
        let request = AppleStyleLightMapRequest {
            luma: &values,
            width: 32,
            height: 32,
            storage_orientation: 1,
        };
        let map = apple_style_histogram_light_map(&request, 1.0).unwrap();
        assert!(map
            .as_chunks::<2>()
            .0
            .iter()
            .all(|v| u16::from_le_bytes([v[0], v[1]]) == expected));
    }
}

#[test]
fn isolated_highlights_do_not_replace_the_local_median() {
    let mut values = vec![0.0; 64 * 64];
    // Each 2x2 cell contains one white sample and three black samples.
    for y in (0..64).step_by(2) {
        for x in (0..64).step_by(2) {
            values[y * 64 + x] = 1.0;
        }
    }
    let request = AppleStyleLightMapRequest {
        luma: &values,
        width: 64,
        height: 64,
        storage_orientation: 1,
    };
    let map = apple_style_histogram_light_map(&request, 1.0).unwrap();
    assert!(map
        .as_chunks::<2>()
        .0
        .iter()
        .all(|v| u16::from_le_bytes([v[0], v[1]]) == 0x2937));
    assert!(apple_style_histogram_light_map(&request, 0.0).is_err());
    let extreme = vec![65504.0; 32 * 32];
    assert!(apple_style_histogram_light_map(
        &AppleStyleLightMapRequest {
            luma: &extreme,
            width: 32,
            height: 32,
            storage_orientation: 1,
        },
        65504.0
    )
    .is_err());
}
