//! Base-guided suppression of unsupported gain structure on flat surfaces.
//! The gain is regularized; full-resolution primary RGB is never blurred.
use crate::{HdrError, Result};

#[derive(Debug, Clone, Copy)]
pub struct SceneGuidancePolicy {
    pub analysis_edge: usize,
    pub flat_ev: f32,
    pub edge_ev: f32,
    pub primary_span_ev: f32,
    pub limit_ev: f32,
    pub minimum_region_pixels: usize,
    pub feather_radius: usize,
}

#[derive(Debug)]
pub struct SceneGuidanceMetrics {
    pub width: usize,
    pub height: usize,
    pub regions: usize,
    pub region_fraction: f32,
    pub adjustment_rms_ev: f32,
    pub maximum_adjustment_ev: f32,
}

pub(super) struct GuidanceAdjustment {
    pub values: Vec<Option<f32>>,
    pub strength: Vec<f32>,
    pub guide: Vec<f32>,
    pub metrics: SceneGuidanceMetrics,
}

fn invalid(message: &str) -> HdrError {
    HdrError::invalid("scene guidance", message)
}

pub(super) fn validate(policy: SceneGuidancePolicy) -> Result<()> {
    if !(32..=512).contains(&policy.analysis_edge)
        || !policy.flat_ev.is_finite()
        || !(0.01..=0.2).contains(&policy.flat_ev)
        || !policy.edge_ev.is_finite()
        || !(0.01..=0.2).contains(&policy.edge_ev)
        || !policy.primary_span_ev.is_finite()
        || !(0.01..=0.5).contains(&policy.primary_span_ev)
        || !policy.limit_ev.is_finite()
        || !(0.01..=0.5).contains(&policy.limit_ev)
        || !(4..=4096).contains(&policy.minimum_region_pixels)
        || !(1..=8).contains(&policy.feather_radius)
    {
        return Err(invalid("unsupported stabilization policy"));
    }
    Ok(())
}

// Fractional area integration in linear luminance, before taking logarithms.
fn area(input: &[f32], w: usize, h: usize, aw: usize, ah: usize) -> Vec<f32> {
    let mut output = vec![0.; aw * ah];
    for y in 0..ah {
        let y0 = y as f64 * h as f64 / ah as f64;
        let y1 = (y + 1) as f64 * h as f64 / ah as f64;
        for x in 0..aw {
            let x0 = x as f64 * w as f64 / aw as f64;
            let x1 = (x + 1) as f64 * w as f64 / aw as f64;
            let mut sum = 0.;
            for j in y0.floor() as usize..(y1.ceil() as usize).min(h) {
                let wy = (y1.min((j + 1) as f64) - y0.max(j as f64)).max(0.);
                for i in x0.floor() as usize..(x1.ceil() as usize).min(w) {
                    let wx = (x1.min((i + 1) as f64) - x0.max(i as f64)).max(0.);
                    sum += input[j * w + i] as f64 * wx * wy;
                }
            }
            output[y * aw + x] = (sum / ((x1 - x0) * (y1 - y0))) as f32;
        }
    }
    output
}

struct Region {
    gain: f32,
    lo: f32,
    hi: f32,
}

pub(super) fn adjustment(
    primary_hdr: &[f32],
    thumbnail: &[f32],
    base_sdr: &[f32],
    geometry: (usize, usize),
    exposure_ev: f32,
    policy: SceneGuidancePolicy,
) -> Result<GuidanceAdjustment> {
    validate(policy)?;
    let (w, h) = geometry;
    let scale = (policy.analysis_edge as f64 / w.max(h) as f64).min(1.);
    let aw = ((w as f64 * scale).round() as usize).max(1);
    let ah = ((h as f64 * scale).round() as usize).max(1);
    let primary = area(primary_hdr, w, h, aw, ah);
    let scene = area(thumbnail, w, h, aw, ah);
    let guide = area(base_sdr, w, h, aw, ah);
    let valid = primary
        .iter()
        .zip(&scene)
        .zip(&guide)
        .map(|((a, b), c)| *a > 1e-5 && *b > 1e-5 && *c > 1e-5)
        .collect::<Vec<_>>();
    let log = guide.iter().map(|v| v.max(1e-8).log2()).collect::<Vec<_>>();
    let data = primary
        .iter()
        .zip(&scene)
        .zip(&valid)
        .map(|((a, b), valid)| {
            if *valid {
                Some(((*b / *a).log2() + exposure_ev).clamp(-6., 6.))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let relative = aw.max(ah) as f32 / policy.analysis_edge as f32;
    let neighborhood = (4. * relative).round().max(1.) as usize;
    let minimum = (policy.minimum_region_pixels as f32 * relative * relative)
        .ceil()
        .max(4.) as usize;
    let feather = (policy.feather_radius as f32 * relative).round().max(1.) as usize;
    let mut flat = vec![false; data.len()];
    for y in 0..ah {
        for x in 0..aw {
            let i = y * aw + x;
            if !valid[i] {
                continue;
            }
            let (mut count, mut sum, mut sum2) = (0., 0., 0.);
            for j in y.saturating_sub(neighborhood)..=(y + neighborhood).min(ah - 1) {
                for k in x.saturating_sub(neighborhood)..=(x + neighborhood).min(aw - 1) {
                    let n = j * aw + k;
                    if valid[n] {
                        let v = log[n] as f64;
                        count += 1.;
                        sum += v;
                        sum2 += v * v;
                    }
                }
            }
            flat[i] =
                (sum2 / count - (sum / count).powi(2)).max(0.) < (policy.flat_ev as f64).powi(2);
        }
    }
    let mut owner = vec![None; data.len()];
    let mut seen = vec![false; data.len()];
    let mut regions = Vec::<Region>::new();
    let mut queue = Vec::new();
    for seed in 0..data.len() {
        if seen[seed] || !flat[seed] {
            continue;
        }
        queue.clear();
        queue.push(seed);
        seen[seed] = true;
        let mut cursor = 0;
        while cursor < queue.len() {
            let i = queue[cursor];
            cursor += 1;
            let (x, y) = (i % aw, i / aw);
            let neighbors = [
                x.checked_sub(1).map(|_| i - 1),
                (x + 1 < aw).then_some(i + 1),
                y.checked_sub(1).map(|_| i - aw),
                (y + 1 < ah).then_some(i + aw),
            ];
            for j in neighbors.into_iter().flatten() {
                if !seen[j] && flat[j] && (log[i] - log[j]).abs() <= policy.edge_ev {
                    seen[j] = true;
                    queue.push(j);
                }
            }
        }
        if queue.len() < minimum {
            continue;
        }
        let lo = queue.iter().map(|i| log[*i]).fold(f32::INFINITY, f32::min);
        let hi = queue
            .iter()
            .map(|i| log[*i])
            .fold(f32::NEG_INFINITY, f32::max);
        if hi - lo > policy.primary_span_ev {
            continue;
        }
        let mut gains = queue.iter().map(|i| data[*i].unwrap()).collect::<Vec<_>>();
        gains.sort_unstable_by(f32::total_cmp);
        let center = gains[gains.len() / 2];
        // Large, possibly genuine scene differences must not be flattened.
        if gains.iter().any(|v| (v - center).abs() > policy.limit_ev) {
            continue;
        }
        let id = regions.len();
        regions.push(Region {
            gain: center,
            lo,
            hi,
        });
        for &i in &queue {
            owner[i] = Some(id);
        }
    }
    let region_fraction = owner.iter().filter(|v| v.is_some()).count() as f32 / data.len() as f32;
    let mut values = data.clone();
    let mut strength = vec![0.; data.len()];
    for y in 0..ah {
        for x in 0..aw {
            let i = y * aw + x;
            if !valid[i] {
                continue;
            }
            if let Some(id) = owner[i] {
                values[i] = Some(regions[id].gain);
                strength[i] = 1.;
                continue;
            }
            let mut nearest = None;
            let mut distance = usize::MAX;
            for j in y.saturating_sub(feather)..=(y + feather).min(ah - 1) {
                for k in x.saturating_sub(feather)..=(x + feather).min(aw - 1) {
                    let Some(id) = owner[j * aw + k] else {
                        continue;
                    };
                    let r = &regions[id];
                    if log[i] < r.lo - policy.edge_ev || log[i] > r.hi + policy.edge_ev {
                        continue;
                    }
                    let d = x.abs_diff(k).pow(2) + y.abs_diff(j).pow(2);
                    if d < distance && d < feather * feather {
                        nearest = Some(id);
                        distance = d;
                    }
                }
            }
            if let Some(id) = nearest {
                values[i] = Some(regions[id].gain);
                // Fill a compatible inner boundary band, then feather the rest.
                let a = (2. * (1. - (distance as f32).sqrt() / feather as f32)).clamp(0., 1.);
                strength[i] = a * a * (3. - 2. * a);
            }
        }
    }
    Ok(GuidanceAdjustment {
        values,
        strength,
        guide,
        metrics: SceneGuidanceMetrics {
            width: aw,
            height: ah,
            regions: regions.len(),
            region_fraction,
            adjustment_rms_ev: 0.,
            maximum_adjustment_ev: 0.,
        },
    })
}
