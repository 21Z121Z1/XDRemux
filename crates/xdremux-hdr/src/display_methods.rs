//! Independent tonal and opponent-chroma leaves in an explicit HDR domain.
//! These are not Apple's editor coordinates or a complete native renderer.
use crate::{display_p3_luminance as luminance, HdrError, LinearPixel, Result};

#[derive(Clone, Copy, Debug)]
pub struct DisplayMethods {
    pub mids_background: f32,
    pub mids_foreground: f32,
    pub anchor_background: f32,
    pub anchor_skin: f32,
    pub skin_brightness: f32,
    pub skin_highlight: f32,
    pub chroma_gain: f32,
    pub foreground_chroma_protection: f32,
}

pub struct DisplayTarget {
    pub pixels: Vec<LinearPixel>,
    pub exposure_ev: f32,
    pub maximum_rgb: f32,
}

fn invalid(message: &str) -> HdrError {
    HdrError::invalid("display methods", message)
}
fn signed_power(x: f32, e: f32) -> f32 {
    x.signum() * x.abs().powf(e)
}
fn dot(x: [f32; 3], y: [f32; 3]) -> f32 {
    x[0] * y[0] + x[1] * y[1] + x[2] * y[2]
}
fn y(x: [f32; 3]) -> f32 {
    luminance(&[x[0], x[1], x[2], 1.])
}
fn columns(x: [f32; 3], m: [[f32; 3]; 3]) -> [f32; 3] {
    std::array::from_fn(|c| (x[0] * m[0][c] + x[1] * m[1][c]) + x[2] * m[2][c])
}
fn opponent(x: [f32; 3]) -> [f32; 3] {
    let lms = columns(
        x,
        [
            [0.3139902, 0.15537241, 0.017752387],
            [0.63951296, 0.75789446, 0.10944209],
            [0.046497546, 0.086701415, 0.8725692],
        ],
    );
    columns(
        lms.map(|v| signed_power(v, 0.43)),
        [
            [0.4, 4.455, 0.8056],
            [0.4, -4.851, 0.3572],
            [0.2, 0.396, -1.1628],
        ],
    )
}
fn inverse_opponent(p: [f32; 3]) -> [f32; 3] {
    let a = [0.09756893, -0.11387649, 0.03261511];
    let b = [0.20522644, 0.13321716, -0.67688715];
    let q = std::array::from_fn(|c| signed_power((p[1] * a[c] + p[0]) + p[2] * b[c], 2.3255813));
    columns(
        q,
        [
            [5.472212, -1.1252419, 0.029801652],
            [-4.64196, 2.293171, -0.19318072],
            [0.16963708, -0.1678952, 1.1636479],
        ],
    )
}
fn fit_gamut(mut x: [f32; 3], target: f32, cap: f32) -> [f32; 3] {
    let actual = y(x);
    x = if actual > 1e-8 {
        x.map(|v| v * target / actual)
    } else {
        [target; 3]
    };
    let mut amount = 1f32;
    for v in x {
        if v < 0. {
            amount = amount.min(target / (target - v).max(1e-8));
        }
        if v > cap {
            amount = amount.min((cap - target) / (v - target).max(1e-8));
        }
    }
    amount = amount.clamp(0., 1.);
    x.map(|v| (target + amount * (v - target)).clamp(0., cap))
}
fn mids(x: [f32; 3], amount: f32) -> [f32; 3] {
    let c = x.map(|v| v.clamp(0., 1.));
    let gain = 1. + (1. - c.into_iter().fold(0f32, f32::max).sqrt()) * amount;
    std::array::from_fn(|i| x[i].min(0.) + x[i].max(1.) - 1. + (c[i] * gain).clamp(0., 1.))
}
fn skin_method(x: [f32; 3], skin: f32, guide: f32, high: f32, brightness: f32) -> [f32; 3] {
    if skin <= 0. || x.iter().sum::<f32>() <= 1e-6 {
        return x;
    }
    let mut out = x;
    if brightness.abs() >= 1e-5 {
        let c = x.map(|v| v.clamp(0., 1.));
        let root = c.map(f32::sqrt);
        let light = dot(c, [0.333252; 3]).sqrt();
        let a = light * (1. - light) * brightness;
        let rg = guide.clamp(0., 1.).sqrt();
        let offset = 2. * a * (dot(root, [0.299, 0.587, 0.114]) - rg);
        out = std::array::from_fn(|i| {
            let b0 = root[i] + (rg - root[i]) * (-2. * a);
            let r = b0 + ((root[i] + offset) - b0) * 0.799805;
            x[i].max(x[i].min(0.) + x[i].max(1.) - 1. + r.max(0.).powi(2))
        });
    }
    if high.abs() >= 1e-5 {
        let sum = out.iter().sum::<f32>().max(1e-6);
        let norm = out.map(|v| v / sum);
        let light = dot(out, [0.5, 0.300049, 0.199951]);
        let a = ((light - 0.399902) * 1.66602).clamp(0., 1.);
        let b = ((norm[0].max(norm[1]) + norm[1].max(norm[2]) - 0.333252) * 1.49902).clamp(0., 1.);
        let smooth = |v: f32| v * v * (3. - 2. * v);
        let weight = (2. * high * smooth(a) * (1. - smooth(b))).min(1.);
        let warm = [1f32, 0.700195, 0.549805].map(|v| {
            if v > 0.0404358 {
                ((v + 0.0549927) * 0.948242).powf(2.40039)
            } else {
                v * 0.0773925
            }
        });
        out = std::array::from_fn(|i| out[i] + (out[i] * warm[i] - out[i]) * weight);
    }
    std::array::from_fn(|i| x[i] + (out[i] - x[i]) * skin.clamp(0., 1.))
}

/// R remains unbounded. Normalize a declared display target by its headroom,
/// apply bounded leaves, then restore the HDR scale. Source SDR may be extended.
#[allow(clippy::too_many_arguments)]
pub fn render_display_pixel(
    master: [f32; 3],
    sdr: [f32; 3],
    skin: f32,
    foreground: f32,
    guide: f32,
    exposure: f32,
    headroom: f32,
    c: DisplayMethods,
) -> [f32; 3] {
    let a = master.map(|v| v.max(0.) * exposure);
    let base = a.map(|v| v * headroom / (headroom + y(a)));
    let mut x = fit_gamut(base, y(base), headroom).map(|v| v / headroom);
    x = mids(
        x,
        c.mids_background + (c.mids_foreground - c.mids_background) * foreground,
    );
    x = skin_method(x, skin, guide, c.skin_highlight, c.skin_brightness);
    let mut target = y(x);
    if target >= 1. {
        x = x.map(|v| v * 0.999999 / target);
        target = 0.999999;
    }
    x = fit_gamut(x, target, 1.);
    let mut current = opponent(x.map(|v| v.clamp(0., 1.)));
    let reference = opponent(sdr);
    if reference[0] > 1e-4 && reference.iter().all(|v| v.is_finite()) && current[0] > 1e-8 {
        let weight = c.anchor_background + (c.anchor_skin - c.anchor_background) * skin;
        for i in 1..3 {
            current[i] += (current[0] * reference[i] / reference[0] - current[i]) * weight;
        }
    }
    let gain = c.chroma_gain + (1. - c.chroma_gain) * foreground * c.foreground_chroma_protection;
    current[1] *= gain;
    current[2] *= gain;
    fit_gamut(inverse_opponent(current), target, 1.).map(|v| v * headroom)
}

fn reflect(mut i: isize, n: usize) -> usize {
    let n = n as isize;
    while i < 0 || i >= n {
        i = if i < 0 { -i - 1 } else { 2 * n - i - 1 };
    }
    i as usize
}
fn guide(
    scene: &[LinearPixel],
    w: usize,
    h: usize,
    exposure: f32,
    headroom: f32,
) -> (Vec<f32>, usize, usize) {
    let gw = w.min(1024);
    let gh = ((h as f64 * gw as f64 / w as f64).round() as usize).max(1);
    let mut a = crate::scene_reconstruction::area_luminance(scene, w, h, gw, gh);
    for v in &mut a {
        let light = (*v * exposure).max(0.);
        *v = (light / (headroom + light)).max(1e-8).log2();
    }
    let mut weights = (-8..=8)
        .map(|i| (-(i * i) as f32 / 8.).exp())
        .collect::<Vec<_>>();
    let total: f32 = weights.iter().sum();
    for q in &mut weights {
        *q /= total;
    }
    let mut b = vec![0.; a.len()];
    for row in 0..gh {
        for col in 0..gw {
            b[row * gw + col] = weights
                .iter()
                .enumerate()
                .map(|(i, q)| q * a[row * gw + reflect(col as isize + i as isize - 8, gw)])
                .sum();
        }
    }
    for row in 0..gh {
        for col in 0..gw {
            a[row * gw + col] = weights
                .iter()
                .enumerate()
                .map(|(i, q)| q * b[reflect(row as isize + i as isize - 8, gh) * gw + col])
                .sum::<f32>()
                .exp2();
        }
    }
    (a, gw, gh)
}
fn sample(a: &[f32], w: usize, h: usize, x: f32, y: f32) -> f32 {
    let x = x.clamp(0., (w - 1) as f32);
    let y = y.clamp(0., (h - 1) as f32);
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let top = a[y0 * w + x0] + (a[y0 * w + x1] - a[y0 * w + x0]) * fx;
    let bottom = a[y1 * w + x0] + (a[y1 * w + x1] - a[y1 * w + x0]) * fx;
    top + (bottom - top) * fy
}

#[allow(clippy::too_many_arguments)]
pub fn render_styles_display(
    scene: &[LinearPixel],
    sdr: &[LinearPixel],
    skin: &[f32],
    foreground: &[f32],
    width: usize,
    height: usize,
    headroom: u32,
    c: DisplayMethods,
) -> Result<DisplayTarget> {
    let n = width
        .checked_mul(height)
        .ok_or_else(|| invalid("geometry overflow"))?;
    if n == 0
        || ![2, 4, 8].contains(&headroom)
        || [scene.len(), sdr.len(), skin.len(), foreground.len()]
            .iter()
            .any(|v| *v != n)
        || scene.iter().chain(sdr).flatten().any(|v| !v.is_finite())
        || skin
            .iter()
            .chain(foreground)
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
    {
        return Err(invalid(
            "invalid display geometry, headroom, pixels or semantic weights",
        ));
    }
    let mut exposure = Vec::new();
    for row in (0..height).step_by(32) {
        for col in (0..width).step_by(32) {
            let i = row * width + col;
            let a = luminance(&scene[i]);
            let b = luminance(&sdr[i]);
            if a > 0. && b > 0. {
                exposure.push((b / a).log2());
            }
        }
    }
    if exposure.is_empty() {
        return Err(invalid("no positive source exposure anchors"));
    }
    exposure.sort_by(f32::total_cmp);
    let k = exposure.len() / 2;
    let ev = if exposure.len() % 2 == 0 {
        (exposure[k - 1] + exposure[k]) * 0.5
    } else {
        exposure[k]
    };
    let scale = ev.exp2();
    let (guidance, gw, gh) = guide(scene, width, height, scale, headroom as f32);
    let mut pixels = Vec::with_capacity(n);
    let mut maximum = 0f32;
    for row in 0..height {
        for col in 0..width {
            let i = row * width + col;
            let a = scene[i];
            let b = sdr[i];
            let g = sample(
                &guidance,
                gw,
                gh,
                (col as f32 + 0.5) * gw as f32 / width as f32 - 0.5,
                (row as f32 + 0.5) * gh as f32 / height as f32 - 0.5,
            );
            let rgb = render_display_pixel(
                [a[0], a[1], a[2]],
                [b[0], b[1], b[2]],
                skin[i],
                foreground[i],
                g,
                scale,
                headroom as f32,
                c,
            );
            if rgb
                .iter()
                .any(|v| !v.is_finite() || *v < 0. || *v > headroom as f32 + 2e-6)
            {
                return Err(invalid("nonfinite or out-of-range display result"));
            }
            maximum = maximum.max(rgb.into_iter().fold(0f32, f32::max));
            pixels.push([rgb[0], rgb[1], rgb[2], 1.]);
        }
    }
    Ok(DisplayTarget {
        pixels,
        exposure_ev: ev,
        maximum_rgb: maximum,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    include!("display_methods_vectors.rs");
    fn coefficients() -> DisplayMethods {
        DisplayMethods {
            mids_background: 0.08,
            mids_foreground: 0.04,
            anchor_background: 0.75,
            anchor_skin: 0.95,
            skin_brightness: -0.03,
            skin_highlight: 0.,
            chroma_gain: 0.98,
            foreground_chroma_protection: 0.85,
        }
    }
    #[test]
    fn matches_independent_float_and_metal_vectors_including_extended_sdr() {
        for (r, s, skin, fg, g, exposure, h, expected) in VECTORS {
            let actual = render_display_pixel(r, s, skin, fg, g, exposure, h, coefficients());
            for k in 0..3 {
                assert!(
                    (actual[k] - expected[k]).abs() < 0.0001,
                    "{actual:?} vs {expected:?}"
                );
            }
        }
    }
    #[test]
    fn hdr_is_normalized_by_headroom_without_clipping_scene_input() {
        let c = coefficients();
        let a = render_display_pixel([3., 2., 1.], [0.4, 0.3, 0.2], 0., 0., 0.3, 1., 8., c);
        let b = render_display_pixel([1., 1., 1.], [0.4, 0.3, 0.2], 0., 0., 0.3, 1., 8., c);
        assert!(y(a) > y(b) + 0.5);
        assert!(a.iter().all(|v| *v >= 0. && *v <= 8.));
        assert!(render_styles_display(&[[0.; 4]], &[[0.; 4]], &[0.], &[0.], 1, 1, 8, c).is_err());
    }
    #[test]
    fn chroma_anchor_preserves_post_tone_luminance() {
        let c = coefficients();
        let mut off = c;
        off.anchor_background = 0.;
        off.anchor_skin = 0.;
        off.chroma_gain = 1.;
        let a = render_display_pixel([2., 0.8, 0.2], [0.2, 0.3, 0.6], 0.8, 0.9, 0.3, 1., 4., c);
        let b = render_display_pixel([2., 0.8, 0.2], [0.2, 0.3, 0.6], 0.8, 0.9, 0.3, 1., 4., off);
        assert!((y(a) - y(b)).abs() < 1e-5);
        assert!((a[0] - b[0]).abs() > 0.05);
    }
}
