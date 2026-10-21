//! The oracle is an exact, frozen non-test prefix from e107, not a second
//! call into the candidate kernel. Only unchanged public error/scale types
//! are shared. See oracles/provenance.json; do not regenerate on kernel edits.
use xdremux_hdr::{
    apply_iso_gain_sample, reconstruct_scene_linear_hdr, GainMapCodes, HdrError,
    IsoGainApplication, IsoGainChannel, ResolvedScale, Result, SceneLinearRgb,
};

#[allow(dead_code)]
#[rustfmt::skip]
#[path = "oracles/scene_linear_e1072a9.rs"]
mod e107;

fn old_channel(c: IsoGainChannel) -> e107::IsoGainChannel {
    e107::IsoGainChannel {
        minimum_log2: c.minimum_log2,
        maximum_log2: c.maximum_log2,
        gamma: c.gamma,
        base_offset: c.base_offset,
        alternate_offset: c.alternate_offset,
    }
}

fn channel() -> IsoGainChannel {
    IsoGainChannel {
        minimum_log2: -1.0,
        maximum_log2: 3.0,
        gamma: 1.0,
        base_offset: 0.001,
        alternate_offset: 0.002,
    }
}

fn assert_scalar(sdr: f64, code: f64, c: IsoGainChannel, weight: f64) -> Result<f64> {
    let expected = e107::apply_iso_gain_sample(sdr, code, &old_channel(c), weight);
    let actual = apply_iso_gain_sample(sdr, code, &c, weight);
    match (&actual, &expected) {
        (Ok(a), Ok(b)) => assert_eq!(a.to_bits(), b.to_bits(), "scalar bit mismatch"),
        (Err(a), Err(b)) => assert_eq!(a, b, "scalar error mismatch"),
        _ => panic!("scalar result mismatch: actual={actual:?}, expected={expected:?}"),
    }
    actual
}

fn assert_image(
    sdr: &SceneLinearRgb,
    gain: GainMapCodes<'_>,
    application: &IsoGainApplication,
) -> Result<SceneLinearRgb> {
    let expected = e107::reconstruct_scene_linear_hdr(
        &e107::SceneLinearRgb {
            width: sdr.width,
            height: sdr.height,
            samples: sdr.samples.clone(),
        },
        e107::GainMapCodes {
            width: gain.width,
            height: gain.height,
            bytes_per_row: gain.bytes_per_row,
            channels: gain.channels,
            data: gain.data,
        },
        &e107::IsoGainApplication {
            channels: application.channels.map(old_channel),
            weight: application.weight,
        },
    );
    let actual = reconstruct_scene_linear_hdr(sdr, gain, application);
    match (&actual, &expected) {
        (Ok(a), Ok(b)) => {
            assert_eq!((a.width, a.height), (b.width, b.height));
            assert_eq!(a.samples.len(), b.samples.len());
            for (index, (a, b)) in a.samples.iter().zip(&b.samples).enumerate() {
                assert_eq!(
                    a.to_bits(),
                    b.to_bits(),
                    "RGB component {index} bit mismatch"
                );
            }
        }
        (Err(a), Err(b)) => assert_eq!(a, b, "image error mismatch"),
        _ => panic!("image result mismatch: actual={actual:?}, expected={expected:?}"),
    }
    actual
}

#[test]
fn scalar_boundary_values_match_e107_bits_or_errors() {
    for gamma in [f64::MIN_POSITIVE, 0.5, 1.0, 2.2, f64::MAX] {
        for weight in [-0.0, 0.0, 0.125, 0.5, 1.0] {
            for sdr in [
                -f64::MAX,
                -1.0,
                -0.0,
                0.0,
                f64::from_bits(1),
                0.18,
                1.0,
                f64::MAX,
            ] {
                for code in [
                    -f64::MAX,
                    -0.0,
                    0.0,
                    f64::from_bits(1),
                    0.25,
                    0.5,
                    1.0,
                    f64::MAX,
                ] {
                    let _ = assert_scalar(sdr, code, IsoGainChannel { gamma, ..channel() }, weight);
                }
            }
        }
    }
}

#[test]
fn public_scalar_preserves_validation_order_and_exact_errors() {
    let sample_error = HdrError::invalid(
        "gain map application",
        "SDR sample and gain code must be finite",
    );
    let channel_error = HdrError::invalid(
        "gain map application",
        "channel minimum, maximum, positive gamma, and offsets must be finite",
    );
    let weight_error = HdrError::invalid(
        "gain map application",
        "application weight must be finite and within 0 through 1",
    );
    let bad_channel = IsoGainChannel {
        gamma: 0.0,
        ..channel()
    };
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            assert_scalar(value, value, bad_channel, f64::NAN),
            Err(sample_error.clone())
        );
        assert_eq!(
            assert_scalar(0.25, value, bad_channel, f64::NAN),
            Err(sample_error.clone())
        );
        for c in [
            IsoGainChannel {
                minimum_log2: value,
                ..channel()
            },
            IsoGainChannel {
                maximum_log2: value,
                ..channel()
            },
            IsoGainChannel {
                gamma: value,
                ..channel()
            },
            IsoGainChannel {
                base_offset: value,
                ..channel()
            },
            IsoGainChannel {
                alternate_offset: value,
                ..channel()
            },
        ] {
            assert_eq!(
                assert_scalar(0.25, 0.5, c, f64::NAN),
                Err(channel_error.clone())
            );
        }
        assert_eq!(
            assert_scalar(0.25, 0.5, channel(), value),
            Err(weight_error.clone())
        );
    }
    for gamma in [-0.0, 0.0, -1.0] {
        assert_eq!(
            assert_scalar(0.25, 0.5, IsoGainChannel { gamma, ..channel() }, f64::NAN),
            Err(channel_error.clone())
        );
    }
    for weight in [-f64::MIN_POSITIVE, 1.0 + f64::EPSILON] {
        assert_eq!(
            assert_scalar(0.25, 0.5, channel(), weight),
            Err(weight_error.clone())
        );
    }
}

#[test]
fn whole_images_match_e107_bits_for_rgb_mono_padding_and_resampling() {
    let mut cases = 0;
    for (width, height) in [(1, 1), (2, 3), (7, 5), (12, 9)] {
        let sdr = SceneLinearRgb {
            width,
            height,
            samples: (0..width * height * 3)
                .map(|i| (i % 257) as f32 / 128.0 - 0.125)
                .collect(),
        };
        for (gain_width, gain_height) in [(1, 1), (2, 2), (3, 1), (2, 4), (7, 5)] {
            for channels in [1, 3] {
                for padding in [0, 7] {
                    let stride = gain_width * channels + padding;
                    let mut data = vec![0xa5; stride * gain_height];
                    for y in 0..gain_height {
                        for x in 0..gain_width * channels {
                            data[y * stride + x] = [0, 1, 127, 128, 254, 255][(x + y * 7) % 6];
                        }
                    }
                    let gain = GainMapCodes {
                        width: gain_width,
                        height: gain_height,
                        bytes_per_row: stride,
                        channels,
                        data: &data,
                    };
                    for gamma in [0.5, 1.0, 2.2] {
                        for weight in [0.0, 0.5, 1.0] {
                            let mut per_channel = [IsoGainChannel { gamma, ..channel() }; 3];
                            per_channel[1].maximum_log2 = 1.25;
                            per_channel[2].base_offset = 0.02;
                            let application = IsoGainApplication {
                                channels: per_channel,
                                weight,
                            };
                            assert_image(&sdr, gain, &application).unwrap();
                            cases += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 720);
}

#[test]
fn whole_image_invalid_inputs_keep_first_error_and_never_panic() {
    let sdr = SceneLinearRgb {
        width: 1,
        height: 1,
        samples: vec![0.25, 0.5, 0.75],
    };
    let gain = GainMapCodes {
        width: 1,
        height: 1,
        bytes_per_row: 1,
        channels: 1,
        data: &[128],
    };
    let good = IsoGainApplication {
        channels: [channel(); 3],
        weight: 1.0,
    };
    let mut bad = good;
    bad.channels[2].gamma = 0.0;
    bad.weight = f64::NAN;
    for invalid in [
        SceneLinearRgb {
            width: 0,
            ..sdr.clone()
        },
        SceneLinearRgb {
            height: 0,
            ..sdr.clone()
        },
        SceneLinearRgb {
            samples: vec![],
            ..sdr.clone()
        },
        SceneLinearRgb {
            samples: vec![0.0; 4],
            ..sdr.clone()
        },
        SceneLinearRgb {
            width: usize::MAX,
            height: 2,
            samples: vec![],
        },
        SceneLinearRgb {
            width: usize::MAX / 2,
            height: 1,
            samples: vec![],
        },
        SceneLinearRgb {
            samples: vec![f32::NAN, 0.0, 0.0],
            ..sdr.clone()
        },
        SceneLinearRgb {
            samples: vec![0.0, f32::INFINITY, 0.0],
            ..sdr.clone()
        },
        SceneLinearRgb {
            samples: vec![0.0, 0.0, f32::NEG_INFINITY],
            ..sdr.clone()
        },
    ] {
        assert_image(
            &invalid,
            GainMapCodes {
                channels: 0,
                ..gain
            },
            &bad,
        )
        .unwrap_err();
    }
    for invalid in [
        GainMapCodes { width: 0, ..gain },
        GainMapCodes { height: 0, ..gain },
        GainMapCodes {
            channels: 0,
            ..gain
        },
        GainMapCodes {
            channels: 2,
            ..gain
        },
        GainMapCodes {
            channels: 4,
            ..gain
        },
        GainMapCodes {
            bytes_per_row: 0,
            ..gain
        },
        GainMapCodes { data: &[], ..gain },
        GainMapCodes {
            width: usize::MAX,
            channels: 3,
            ..gain
        },
        GainMapCodes {
            height: 2,
            bytes_per_row: usize::MAX,
            ..gain
        },
    ] {
        assert_image(&sdr, invalid, &bad).unwrap_err();
    }
    // Channel2 still wins over weight: validating weight while constructing
    // channel0 would change the old whole-image error order.
    let error = assert_image(&sdr, gain, &bad).unwrap_err();
    assert!(error
        .to_string()
        .contains("channel minimum, maximum, positive gamma"));
    for index in 0..3 {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            for c in [
                IsoGainChannel {
                    minimum_log2: value,
                    ..channel()
                },
                IsoGainChannel {
                    maximum_log2: value,
                    ..channel()
                },
                IsoGainChannel {
                    base_offset: value,
                    ..channel()
                },
                IsoGainChannel {
                    alternate_offset: value,
                    ..channel()
                },
            ] {
                let mut application = good;
                application.channels[index] = c;
                assert_image(&sdr, gain, &application).unwrap_err();
            }
        }
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -1.0] {
            let mut application = good;
            application.channels[index].gamma = value;
            assert_image(&sdr, gain, &application).unwrap_err();
        }
    }
    for weight in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.01, 1.01] {
        assert_image(&sdr, gain, &IsoGainApplication { weight, ..good }).unwrap_err();
    }
}

#[test]
fn whole_image_float_edges_keep_signed_zero_subnormal_and_pixel_order() {
    let sdr = SceneLinearRgb {
        width: 2,
        height: 1,
        samples: vec![
            -0.0,
            f32::from_bits(1),
            -f32::MIN_POSITIVE,
            f32::MAX,
            0.25,
            -0.5,
        ],
    };
    let gain = GainMapCodes {
        width: 1,
        height: 1,
        bytes_per_row: 3,
        channels: 3,
        data: &[0, 128, 255],
    };
    for exponent in [-1024.0, -128.0, 0.0] {
        let c = IsoGainChannel {
            minimum_log2: exponent,
            maximum_log2: exponent,
            gamma: 1.0,
            base_offset: 0.0,
            alternate_offset: 0.0,
        };
        assert_image(
            &sdr,
            gain,
            &IsoGainApplication {
                channels: [c; 3],
                weight: 1.0,
            },
        )
        .unwrap();
    }
}

#[test]
fn reconstructed_nonfinite_and_f32_overflow_are_still_rejected() {
    let sdr = SceneLinearRgb {
        width: 1,
        height: 1,
        samples: vec![1.0; 3],
    };
    let gain = GainMapCodes {
        width: 1,
        height: 1,
        bytes_per_row: 1,
        channels: 1,
        data: &[255],
    };
    for exponent in [128.0, 1024.0] {
        let c = IsoGainChannel {
            minimum_log2: exponent,
            maximum_log2: exponent,
            gamma: 1.0,
            base_offset: 0.0,
            alternate_offset: 0.0,
        };
        let scalar = assert_scalar(1.0, 1.0, c, 1.0);
        let error = assert_image(
            &sdr,
            gain,
            &IsoGainApplication {
                channels: [c; 3],
                weight: 1.0,
            },
        )
        .unwrap_err();
        if exponent == 128.0 {
            assert!(scalar.unwrap().is_finite());
            assert!(error.to_string().contains("outside the f32 range"));
        } else {
            assert!(scalar.is_err());
            assert!(error.to_string().contains("non-finite"));
        }
    }
    let c = IsoGainChannel {
        minimum_log2: -f64::MAX,
        maximum_log2: f64::MAX,
        ..channel()
    };
    assert_scalar(1.0, 0.0, c, 0.0).unwrap_err();
    assert_image(
        &sdr,
        gain,
        &IsoGainApplication {
            channels: [c; 3],
            weight: 0.0,
        },
    )
    .unwrap_err();
}
