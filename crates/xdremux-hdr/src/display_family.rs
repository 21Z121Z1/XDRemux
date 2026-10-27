//! Scene-derived range mapping with a separately scheduled spatial residual.
//! The SDR residual is a proxy, not an inversion of camera local processing.
use crate::{
    display_p3_luminance as y, render_display_pixel, DisplayMethods, HdrError, LinearPixel, Result,
};

pub const DISPLAY_FAMILY_OFFSET: f32 = 1. / 64.;
pub const DISPLAY_FAMILY_HEADROOM: f32 = 8.;

#[derive(Debug, Clone, Copy)]
pub struct DisplayFamilyPolicy {
    pub analysis_edge: usize,
    pub local_limit_ev: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct DisplaySchedule {
    pub headroom: f32,
    pub master_weight: f32,
    pub retained_local: f32,
    pub unconstrained_retained_local: f32,
}

#[derive(Debug)]
pub struct DisplayFamily {
    pub exposure_ev: f32,
    pub scene_range_ev: f32,
    pub sdr_global_range_ev: f32,
    pub range_gap_ev: f32,
    pub local_rms_ev: f32,
    pub limited_fraction: f32,
    pub curve: Vec<[f32; 2]>,
    width: usize,
    height: usize,
    guide: Vec<f32>,
    local: Vec<f32>,
}

pub struct DisplayFamilyTarget {
    pub pixels: Vec<LinearPixel>,
    pub maximum_rgb: f32,
    pub range_limited_fraction: f32,
    pub negative_sdr_fraction: f32,
}

fn invalid(message: &str) -> HdrError {
    HdrError::invalid("display family", message)
}
fn log_light(v: f32) -> f32 {
    (v.max(0.) + DISPLAY_FAMILY_OFFSET).log2()
}
fn quantile(sorted: &[f32], q: f32) -> f32 {
    let i = (sorted.len() - 1) as f32 * q;
    let lo = i.floor() as usize;
    sorted[lo] + (sorted[i.ceil() as usize] - sorted[lo]) * (i - lo as f32)
}
fn raster(p: &[LinearPixel], w: usize, h: usize) -> Result<()> {
    if w == 0
        || h == 0
        || w.checked_mul(h) != Some(p.len())
        || p.iter().flatten().any(|v| !v.is_finite())
    {
        return Err(invalid("invalid geometry or nonfinite pixels"));
    }
    Ok(())
}

// Robust bin medians followed by weighted isotonic regression. A monotone
// global curve cannot explain spatial differences between equal scene values.
fn monotone_curve(x: &[f32], target: &[f32]) -> Vec<[f32; 2]> {
    let mut sorted = x.to_vec();
    sorted.sort_by(f32::total_cmp);
    let lo = quantile(&sorted, 0.02);
    let hi = quantile(&sorted, 0.98).max(lo + 0.01);
    let mut bins = vec![Vec::new(); 64];
    for (&v, &t) in x.iter().zip(target) {
        let b = (((v - lo) / (hi - lo) * 64.).floor() as i32).clamp(0, 63) as usize;
        bins[b].push((v, t));
    }
    let mut nodes = Vec::new();
    let mut weights = Vec::new();
    for b in &mut bins {
        if b.is_empty() {
            continue;
        }
        let x = b.iter().map(|p| p.0 as f64).sum::<f64>() / b.len() as f64;
        let mut values = b.iter().map(|p| p.1).collect::<Vec<_>>();
        values.sort_by(f32::total_cmp);
        nodes.push([x as f32, quantile(&values, 0.5)]);
        weights.push(b.len() as f64);
    }
    let mut blocks: Vec<(usize, usize, f64, f64)> = Vec::new();
    for (i, node) in nodes.iter().enumerate() {
        blocks.push((i, i + 1, node[1] as f64 * weights[i], weights[i]));
        while blocks.len() > 1 {
            let k = blocks.len();
            let a = blocks[k - 2];
            let b = blocks[k - 1];
            if a.2 / a.3 <= b.2 / b.3 {
                break;
            }
            blocks.truncate(k - 2);
            blocks.push((a.0, b.1, a.2 + b.2, a.3 + b.3));
        }
    }
    for (start, end, sum, weight) in blocks {
        for node in &mut nodes[start..end] {
            node[1] = (sum / weight) as f32;
        }
    }
    nodes
}
fn curve_value(nodes: &[[f32; 2]], x: f32) -> f32 {
    if nodes.len() == 1 {
        return nodes[0][1] + x - nodes[0][0];
    }
    let k = nodes
        .partition_point(|p| p[0] < x)
        .clamp(1, nodes.len() - 1);
    let a = nodes[k - 1];
    let b = nodes[k];
    let mut slope = (b[1] - a[1]) / (b[0] - a[0]).max(1e-6);
    if x < nodes[0][0] || x > nodes[nodes.len() - 1][0] {
        slope = slope.clamp(0.05, 1.);
    }
    a[1] + slope * (x - a[0])
}

// A positive-weight joint bilateral average cannot create overshoot in the
// residual. Both passes follow scene luminance, not the already processed SDR.
fn smooth(local: &[f32], guide: &[f32], w: usize, h: usize) -> Vec<f32> {
    let mut a = local.to_vec();
    let spatial = (-12..=12)
        .map(|i| (-(i * i) as f32 / 32.).exp())
        .collect::<Vec<_>>();
    for horizontal in [true, false] {
        let mut b = vec![0.; a.len()];
        for row in 0..h {
            for col in 0..w {
                let i = row * w + col;
                let mut sum = 0.;
                let mut total = 0.;
                for (k, &s) in spatial.iter().enumerate() {
                    let d = k as isize - 12;
                    let x = (col as isize + if horizontal { d } else { 0 }).clamp(0, w as isize - 1)
                        as usize;
                    let z = (row as isize + if horizontal { 0 } else { d }).clamp(0, h as isize - 1)
                        as usize;
                    let j = z * w + x;
                    let q = s * (-0.5 * ((guide[j] - guide[i]) / 0.5).powi(2)).exp();
                    sum += q * a[j];
                    total += q;
                }
                b[i] = sum / total;
            }
        }
        a = b;
    }
    a
}

impl DisplayFamily {
    pub fn analyze(
        scene: &[LinearPixel],
        sdr: &[LinearPixel],
        w: usize,
        h: usize,
        policy: DisplayFamilyPolicy,
    ) -> Result<Self> {
        raster(scene, w, h)?;
        raster(sdr, w, h)?;
        if !(32..=1024).contains(&policy.analysis_edge)
            || !policy.local_limit_ev.is_finite()
            || !(0.0..=3.0).contains(&policy.local_limit_ev)
        {
            return Err(invalid("unsupported analysis policy"));
        }
        let scale = (policy.analysis_edge as f64 / w.max(h) as f64).min(1.);
        let aw = ((w as f64 * scale).round() as usize).max(1);
        let ah = ((h as f64 * scale).round() as usize).max(1);
        let r = crate::scene_reconstruction::area_luminance(scene, w, h, aw, ah);
        let s = crate::scene_reconstruction::area_luminance(sdr, w, h, aw, ah);
        let mut anchors = r
            .iter()
            .zip(&s)
            .filter(|(r, s)| **r > 1e-5 && **s > 1e-5)
            .map(|(r, s)| (s / r).log2())
            .collect::<Vec<_>>();
        if anchors.is_empty() {
            return Err(invalid("no positive scene/SDR anchors"));
        }
        anchors.sort_by(f32::total_cmp);
        let exposure_ev = quantile(&anchors, 0.5);
        if !exposure_ev.is_finite() {
            return Err(invalid("nonfinite exposure anchor"));
        }
        let guide = r
            .iter()
            .map(|v| log_light(v * exposure_ev.exp2()))
            .collect::<Vec<_>>();
        if guide.iter().any(|v| !v.is_finite()) {
            return Err(invalid("scene/exposure product exceeds the finite domain"));
        }
        let s = s.iter().map(|v| log_light(*v)).collect::<Vec<_>>();
        let curve = monotone_curve(&guide, &s);
        let mut sorted = guide.clone();
        sorted.sort_by(f32::total_cmp);
        let lo = quantile(&sorted, 0.02);
        let hi = quantile(&sorted, 0.98);
        let scene_range_ev = hi - lo;
        let sdr_global_range_ev = (curve_value(&curve, hi) - curve_value(&curve, lo)).max(0.);
        let range_gap_ev = (scene_range_ev - sdr_global_range_ev).max(0.);
        let mut limited = 0;
        let residual = guide
            .iter()
            .zip(&s)
            .map(|(&r, &s)| {
                let c = s - curve_value(&curve, r);
                limited += usize::from(c.abs() > policy.local_limit_ev);
                c.clamp(-policy.local_limit_ev, policy.local_limit_ev)
            })
            .collect::<Vec<_>>();
        let local = smooth(&residual, &guide, aw, ah);
        let local_rms_ev = (local.iter().map(|v| (*v as f64).powi(2)).sum::<f64>()
            / local.len() as f64)
            .sqrt() as f32;
        Ok(Self {
            exposure_ev,
            scene_range_ev,
            sdr_global_range_ev,
            range_gap_ev,
            local_rms_ev,
            limited_fraction: limited as f32 / local.len() as f32,
            curve,
            width: aw,
            height: ah,
            guide,
            local,
        })
    }

    /// An affine schedule is required by the single-map ISO log-gain family.
    /// The standalone budget schedule is reported separately, including its
    /// kink when all estimated missing range has already been recovered.
    pub fn schedule(&self, headroom: f32) -> Result<DisplaySchedule> {
        if !headroom.is_finite() || !(1.0..=DISPLAY_FAMILY_HEADROOM).contains(&headroom) {
            return Err(invalid("headroom must be 1 through 8"));
        }
        let stops = headroom.log2();
        let gap = self.range_gap_ev.max(0.01);
        Ok(DisplaySchedule {
            headroom,
            master_weight: stops / 3.,
            retained_local: 1. - stops / gap.max(3.),
            unconstrained_retained_local: (1. - stops / gap).max(0.),
        })
    }

    fn local_at(&self, col: usize, row: usize, w: usize, h: usize, scene_y: f32) -> f32 {
        let x = ((col as f32 + 0.5) * self.width as f32 / w as f32 - 0.5)
            .clamp(0., (self.width - 1) as f32);
        let z = ((row as f32 + 0.5) * self.height as f32 / h as f32 - 0.5)
            .clamp(0., (self.height - 1) as f32);
        let light = log_light(scene_y * self.exposure_ev.exp2());
        let mut sum = 0.;
        let mut total = 0.;
        for dy in 0..=1 {
            for dx in 0..=1 {
                let ix = (x.floor() as usize + dx).min(self.width - 1);
                let iz = (z.floor() as usize + dy).min(self.height - 1);
                let spatial = if dx == 0 { 1. - x.fract() } else { x.fract() }
                    * if dy == 0 { 1. - z.fract() } else { z.fract() };
                let i = iz * self.width + ix;
                let q = spatial * (-0.5 * ((light - self.guide[i]) / 0.5).powi(2)).exp();
                sum += q * self.local[i];
                total += q;
            }
        }
        if total > 1e-8 {
            sum / total
        } else {
            self.local[z.round() as usize * self.width + x.round() as usize]
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        scene: &[LinearPixel],
        sdr: &[LinearPixel],
        skin: &[f32],
        foreground: &[f32],
        w: usize,
        h: usize,
        headroom: f32,
        methods: DisplayMethods,
    ) -> Result<DisplayFamilyTarget> {
        raster(scene, w, h)?;
        raster(sdr, w, h)?;
        let schedule = self.schedule(headroom)?;
        if [skin, foreground].iter().any(|p| {
            p.len() != scene.len() || p.iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        }) {
            return Err(invalid("invalid semantic weights"));
        }
        // This family schedules range compensation itself. The independently
        // callable tonal leaves are not silently added to its range budget.
        if [
            methods.mids_background,
            methods.mids_foreground,
            methods.skin_brightness,
            methods.skin_highlight,
        ]
        .iter()
        .any(|v| *v != 0.)
        {
            return Err(invalid(
                "range family requires separate chroma-only methods",
            ));
        }
        let mut pixels = Vec::with_capacity(scene.len());
        let mut maximum_rgb = 0f32;
        let mut limited = 0;
        let mut negative = 0;
        for row in 0..h {
            for col in 0..w {
                let i = row * w + col;
                negative += usize::from(sdr[i][..3].iter().any(|v| *v < 0.));
                if headroom == 1. {
                    pixels.push(sdr[i]);
                    maximum_rgb = maximum_rgb.max(sdr[i][..3].iter().copied().fold(0., f32::max));
                    continue;
                }
                let clean = render_display_pixel(
                    [scene[i][0], scene[i][1], scene[i][2]],
                    [sdr[i][0], sdr[i][1], sdr[i][2]],
                    skin[i],
                    foreground[i],
                    0.,
                    self.exposure_ev.exp2(),
                    DISPLAY_FAMILY_HEADROOM,
                    methods,
                );
                let local = self.local_at(col, row, w, h, y(&scene[i]));
                let mut p = [0., 0., 0., 1.];
                let mut clipped = false;
                for c in 0..3 {
                    let base = log_light(sdr[i][c]) - local;
                    let global = base + (log_light(clean[c]) - base) * schedule.master_weight;
                    let v =
                        (global + schedule.retained_local * local).exp2() - DISPLAY_FAMILY_OFFSET;
                    clipped |= !(0.0..=headroom).contains(&v);
                    p[c] = v.clamp(0., headroom);
                    if !p[c].is_finite() {
                        return Err(invalid("nonfinite display result"));
                    }
                    maximum_rgb = maximum_rgb.max(p[c]);
                }
                limited += usize::from(clipped);
                pixels.push(p);
            }
        }
        Ok(DisplayFamilyTarget {
            pixels,
            maximum_rgb,
            range_limited_fraction: limited as f32 / (w * h) as f32,
            negative_sdr_fraction: negative as f32 / (w * h) as f32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn methods() -> DisplayMethods {
        DisplayMethods {
            mids_background: 0.,
            mids_foreground: 0.,
            anchor_background: 0.75,
            anchor_skin: 0.95,
            skin_brightness: 0.,
            skin_highlight: 0.,
            chroma_gain: 0.98,
            foreground_chroma_protection: 0.85,
        }
    }
    fn paired() -> (Vec<LinearPixel>, Vec<LinearPixel>) {
        let mut r = Vec::new();
        let mut s = Vec::new();
        for row in 0..64 {
            for col in 0..128 {
                let light = 2f32.powf(-5. + col as f32 / 127. * 9.);
                // Repeated scene ramp, globally compressed SDR, and a known
                // smooth exposure lift/darkening that depends only on position.
                let local = 0.45 * (row as f32 / 63. * std::f32::consts::TAU).sin();
                let base = (light * 0.2).powf(0.5);
                let sd = (base + DISPLAY_FAMILY_OFFSET) * local.exp2() - DISPLAY_FAMILY_OFFSET;
                r.push([light, light, light, 1.]);
                s.push([sd, sd, sd, 1.]);
            }
        }
        (r, s)
    }
    fn family(r: &[LinearPixel], s: &[LinearPixel], w: usize, h: usize) -> DisplayFamily {
        DisplayFamily::analyze(
            r,
            s,
            w,
            h,
            DisplayFamilyPolicy {
                analysis_edge: 512,
                local_limit_ev: 1.5,
            },
        )
        .unwrap()
    }
    #[test]
    fn scene_gap_controls_local_compensation_instead_of_fixed_editor_coefficients() {
        let (r, s) = paired();
        let f = family(&r, &s, 128, 64);
        assert!(f.local_rms_ev > 0.2);
        assert!(f.range_gap_ev > 1.);
        let mut previous = 1.;
        for h in [1., 1.5, 2., 3., 4., 6., 8.] {
            let q = f.schedule(h).unwrap();
            assert!(q.retained_local <= previous);
            previous = q.retained_local;
        }
        // This pair loses more than three stops globally, so even an 8x
        // display still needs some compensation. It must remove most of it.
        assert!(previous < 0.3, "gap {} retains {previous}", f.range_gap_ev);
        let mut wide_r = r.clone();
        let mut wide_s = s.clone();
        for (i, (r, s)) in wide_r.iter_mut().zip(&mut wide_s).enumerate() {
            let v = 2f32.powf(-10. + 20. * (i % 128) as f32 / 127.);
            *r = [v, v, v, 1.];
            let v = 0.1 * v.powf(0.25);
            *s = [v, v, v, 1.];
        }
        let wide = family(&wide_r, &wide_s, 128, 64);
        assert!(wide.range_gap_ev > 6.);
        assert!(wide.schedule(8.).unwrap().retained_local > 0.5);
        let identity = family(&r, &r, 128, 64);
        assert!(identity.local_rms_ev < 0.002);
    }
    #[test]
    fn independent_targets_form_one_gain_map_family_between_endpoints() {
        let (r, s) = paired();
        let f = family(&r, &s, 128, 64);
        let zero = vec![0.; r.len()];
        let high = f
            .render(&r, &s, &zero, &zero, 128, 64, 8., methods())
            .unwrap();
        let mut max_error = 0f32;
        for h in [1.5f32, 2., 3., 4., 6.] {
            let t = f
                .render(&r, &s, &zero, &zero, 128, 64, h, methods())
                .unwrap();
            for ((s, hi), t) in s.iter().zip(&high.pixels).zip(&t.pixels) {
                for c in 0..3 {
                    let encoded = (s[c] + DISPLAY_FAMILY_OFFSET)
                        * ((hi[c] + DISPLAY_FAMILY_OFFSET) / (s[c] + DISPLAY_FAMILY_OFFSET))
                            .powf(h.log2() / 3.)
                        - DISPLAY_FAMILY_OFFSET;
                    if (0.0..=h).contains(&encoded) {
                        max_error = max_error.max((encoded - t[c]).abs());
                    }
                }
            }
        }
        assert!(max_error < 2e-5, "{max_error}");
        assert_eq!(
            f.render(&r, &s, &zero, &zero, 128, 64, 1., methods())
                .unwrap()
                .pixels,
            s
        );
    }
    #[test]
    fn coarse_spatial_compensation_is_reduced_without_new_strong_edge_halos() {
        let (r, s) = paired();
        let f = family(&r, &s, 128, 64);
        let zero = vec![0.; r.len()];
        let high = f
            .render(&r, &s, &zero, &zero, 128, 64, 8., methods())
            .unwrap();
        let low = f
            .render(&r, &s, &zero, &zero, 128, 64, 2., methods())
            .unwrap();
        let contrast = |p: &[LinearPixel]| {
            (log_light(y(&p[16 * 128 + 60])) - log_light(y(&p[48 * 128 + 60]))).abs()
        };
        assert!(contrast(&s) > 0.7);
        assert!(contrast(&high.pixels) < contrast(&s) * 0.3);
        assert!(contrast(&low.pixels) < contrast(&s));
        let r = (0..64 * 128)
            .map(|i| {
                let v = if i % 128 < 64 { 0.1 } else { 12. };
                [v, v, v, 1.]
            })
            .collect::<Vec<_>>();
        let s = r
            .iter()
            .map(|p| {
                let v = y(p) / (1. + y(p));
                [v, v, v, 1.]
            })
            .collect::<Vec<_>>();
        let f = family(&r, &s, 128, 64);
        let t = f
            .render(&r, &s, &zero, &zero, 128, 64, 8., methods())
            .unwrap();
        for row in t.pixels.chunks(128) {
            for side in [&row[..64], &row[64..]] {
                let first = y(&side[0]);
                assert!(side.iter().all(|p| (y(p) - first).abs() < 1e-5));
            }
        }

        // Nonzero residuals of opposite sign meet at a real scene edge.
        // A spatial Gaussian would spread this field across that edge.
        let s = r
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let sign = if i / 128 < 32 { 1. } else { -1. };
                let c = sign * if i % 128 < 64 { 0.5 } else { -0.5 };
                let global = 0.1 * y(p).powf(0.1);
                let v = (global + DISPLAY_FAMILY_OFFSET) * 2f32.powf(c) - DISPLAY_FAMILY_OFFSET;
                [v, v, v, 1.]
            })
            .collect::<Vec<_>>();
        let f = family(&r, &s, 128, 64);
        assert!(f.schedule(8.).unwrap().retained_local > 0.1);
        let t = f
            .render(&r, &s, &zero, &zero, 128, 64, 8., methods())
            .unwrap();
        let row = &t.pixels[16 * 128..17 * 128];
        assert!((log_light(y(&row[30])) - log_light(y(&row[63]))).abs() < 0.002);
        assert!((log_light(y(&row[64])) - log_light(y(&row[100]))).abs() < 0.002);
    }
    #[test]
    fn chroma_protection_does_not_change_range_mapping_and_invalid_inputs_fail() {
        let (mut r, mut s) = paired();
        for p in &mut r {
            let light = y(p);
            let weights = [1.5, 0.9, 0.4, 1.];
            let norm = y(&weights);
            for c in 0..3 {
                p[c] = light * weights[c] / norm;
            }
        }
        for p in &mut s {
            let light = y(p);
            let weights = [0.6, 1., 2., 1.];
            let norm = y(&weights);
            for c in 0..3 {
                p[c] = light * weights[c] / norm;
            }
        }
        let f = family(&r, &s, 128, 64);
        let zero = vec![0.; r.len()];
        let mut c = methods();
        c.chroma_gain = 0.5;
        let a = f.render(&r, &s, &zero, &zero, 128, 64, 8., c).unwrap();
        let b = f
            .render(&r, &s, &zero, &zero, 128, 64, 8., methods())
            .unwrap();
        assert!(a
            .pixels
            .iter()
            .zip(&b.pixels)
            .all(|(a, b)| (y(a) - y(b)).abs() < 1e-5));
        assert!(a
            .pixels
            .iter()
            .zip(&b.pixels)
            .any(|(a, b)| (a[0] - b[0]).abs() > 0.01));
        for h in [0., 0.9, 8.1, f32::NAN] {
            assert!(f.schedule(h).is_err());
        }
        assert!(DisplayFamily::analyze(
            &r,
            &s,
            0,
            64,
            DisplayFamilyPolicy {
                analysis_edge: 512,
                local_limit_ev: 1.5
            }
        )
        .is_err());
        assert!(DisplayFamily::analyze(
            &[[0.; 4]],
            &[[0.; 4]],
            1,
            1,
            DisplayFamilyPolicy {
                analysis_edge: 512,
                local_limit_ev: 1.5
            }
        )
        .is_err());
        let r = [[3., 2., 0.5, 1.]];
        let s = [[-0.02, 0.3, 1.1, 1.]];
        let f = family(&r, &s, 1, 1);
        assert_eq!(
            f.render(&r, &s, &[0.], &[0.], 1, 1, 1., methods())
                .unwrap()
                .pixels,
            s
        );
        let t = f.render(&r, &s, &[0.], &[0.], 1, 1, 2., methods()).unwrap();
        assert_eq!(t.negative_sdr_fraction, 1.);
        assert!(t.pixels[0][..3]
            .iter()
            .all(|v| v.is_finite() && (0.0..=2.0).contains(v)));
    }

    #[test]
    fn hdr_keeps_scene_microdetail_without_an_sdr_sharpening_or_blur_pass() {
        let (mut r, mut s) = paired();
        for row in 0..64 {
            for col in 0..128 {
                let i = row * 128 + col;
                if (40..80).contains(&col) {
                    let detail = if col % 2 == 0 { 0.06f32 } else { -0.06 };
                    let v = 0.4 * detail.exp2();
                    r[i] = [v, v, v, 1.];
                    s[i] = [0.25, 0.25, 0.25, 1.];
                }
            }
        }
        let f = family(&r, &s, 128, 64);
        let zero = vec![0.; r.len()];
        let t = f
            .render(&r, &s, &zero, &zero, 128, 64, 8., methods())
            .unwrap();
        let a = 32 * 128 + 60;
        let b = a + 1;
        assert!((log_light(y(&t.pixels[a])) - log_light(y(&t.pixels[b]))).abs() > 0.05);
        assert_eq!(s[a], s[b]);
    }
}
