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
    pub merge_radius: usize,
    pub detail_radius: usize,
    pub detail_threshold_ev: f32,
    pub detail_limit_ev: f32,
    pub minimum_gain_support: f32,
    pub contrast_limit: f32,
    pub minimum_detail_correlation: f32,
    pub maximum_detail_fit_error_ev: f32,
}

#[derive(Debug)]
pub struct SceneGuidanceMetrics {
    pub width: usize,
    pub height: usize,
    pub regions: usize,
    pub region_fraction: f32,
    pub detail_fraction: f32,
    pub detail_seed_candidates: usize,
    pub detail_seed_accepted: usize,
    pub adjustment_rms_ev: f32,
    pub maximum_adjustment_ev: f32,
}

pub(super) struct GuidanceAdjustment {
    pub values: Vec<Option<f32>>,
    pub strength: Vec<f32>,
    pub detail: Vec<f32>,
    pub surface_hdr: Vec<f32>,
    pub contrast: Vec<f32>,
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
        || !(2..=24).contains(&policy.merge_radius)
        || !(1..=4).contains(&policy.detail_radius)
        || !policy.detail_threshold_ev.is_finite()
        || !(0.05..=0.5).contains(&policy.detail_threshold_ev)
        || !policy.detail_limit_ev.is_finite()
        || !(0.25..=2.).contains(&policy.detail_limit_ev)
        || !policy.minimum_gain_support.is_finite()
        || !(0.75..=1.).contains(&policy.minimum_gain_support)
        || !policy.contrast_limit.is_finite()
        || !(1.0..=8.0).contains(&policy.contrast_limit)
        || !policy.minimum_detail_correlation.is_finite()
        || !(0.25..=0.95).contains(&policy.minimum_detail_correlation)
        || !policy.maximum_detail_fit_error_ev.is_finite()
        || !(0.01..=0.2).contains(&policy.maximum_detail_fit_error_ev)
        || policy.detail_limit_ev < policy.limit_ev
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

struct DetailModel {
    slope: f32,
    intercept: f32,
    error_ev: f32,
}

// Account for a small unknown thumbnail blur before estimating contrast.
// Fitting alternate pixels and checking the rest rejects unrelated structure;
// no shifted model is searched, so registration errors do not become a fit.
fn detail_model(
    primary: &[Vec<f32>],
    scene: &[f32],
    valid: &[bool],
    geometry: (usize, usize),
    point: (usize, usize),
    radius: usize,
    policy: SceneGuidancePolicy,
) -> Option<DetailModel> {
    let (w, h) = geometry;
    let (x, y) = point;
    let mut best: Option<DetailModel> = None;
    for plane in primary {
        let (mut n, mut sx, mut sy, mut sxx, mut syy, mut sxy) = (0f64, 0., 0., 0., 0., 0.);
        let mut total = 0;
        let mut supported = 0;
        for j in y.saturating_sub(radius)..=(y + radius).min(h - 1) {
            for k in x.saturating_sub(radius)..=(x + radius).min(w - 1) {
                total += 1;
                let i = j * w + k;
                if !valid[i] {
                    continue;
                }
                supported += 1;
                if (j + k) % 2 != 0 {
                    continue;
                }
                let a = plane[i] as f64;
                let b = scene[i] as f64;
                n += 1.;
                sx += a;
                sy += b;
                sxx += a * a;
                syy += b * b;
                sxy += a * b;
            }
        }
        if n < 8. || supported as f32 / (total as f32) < policy.minimum_gain_support {
            continue;
        }
        let vx = (sxx / n - (sx / n).powi(2)).max(0.);
        let vy = (syy / n - (sy / n).powi(2)).max(0.);
        let covariance = sxy / n - sx * sy / (n * n);
        if vx <= (sx / n * 0.02).powi(2)
            || vy <= 0.
            || covariance <= 0.
            || covariance * covariance
                < vx * vy * (policy.minimum_detail_correlation as f64).powi(2)
        {
            continue;
        }
        let slope = covariance / vx;
        let intercept = (sy - slope * sx) / n;
        let (mut count, mut error) = (0, 0.);
        for j in y.saturating_sub(radius)..=(y + radius).min(h - 1) {
            for k in x.saturating_sub(radius)..=(x + radius).min(w - 1) {
                let i = j * w + k;
                if !valid[i] || (j + k) % 2 == 0 {
                    continue;
                }
                let predicted = slope * plane[i] as f64 + intercept;
                if predicted <= 0. {
                    error = f64::INFINITY;
                } else {
                    error += (predicted / scene[i] as f64).log2().powi(2);
                }
                count += 1;
            }
        }
        let error_ev = (error / count.max(1) as f64).sqrt() as f32;
        if count >= 8
            && error_ev <= policy.maximum_detail_fit_error_ev
            && best.as_ref().is_none_or(|m| error_ev < m.error_ev)
        {
            best = Some(DetailModel {
                slope: slope as f32,
                intercept: intercept as f32,
                error_ev,
            });
        }
    }
    best
}

fn blur_primary(input: &[f32], valid: &[bool], w: usize, h: usize, radius: usize) -> Vec<f32> {
    (0..input.len())
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let (mut sum, mut n) = (0f64, 0);
            for j in y.saturating_sub(radius)..=(y + radius).min(h - 1) {
                for k in x.saturating_sub(radius)..=(x + radius).min(w - 1) {
                    if valid[j * w + k] {
                        sum += input[j * w + k] as f64;
                        n += 1;
                    }
                }
            }
            (sum / n.max(1) as f64) as f32
        })
        .collect()
}

struct Region {
    gain: f32,
    lo: f32,
    hi: f32,
    gain_lo: f32,
    gain_hi: f32,
    sum: f64,
    count: usize,
}

fn root(parent: &[usize], mut id: usize) -> usize {
    while parent[id] != id {
        id = parent[id];
    }
    id
}

// Thin text/rules can split one flat surface into several connected cores.
// Share their plateau only when proximity, base tone and gain bounds agree.
fn merge_regions(
    owner: &mut [Option<usize>],
    regions: &mut [Region],
    data: &[Option<f32>],
    geometry: (usize, usize),
    radius: usize,
    policy: SceneGuidancePolicy,
) -> usize {
    let (w, h) = geometry;
    let mut parent = (0..regions.len()).collect::<Vec<_>>();
    for y in 0..h {
        for x in 0..w {
            let Some(id) = owner[y * w + x] else {
                continue;
            };
            for j in y.saturating_sub(radius)..=(y + radius).min(h - 1) {
                for k in x.saturating_sub(radius)..=(x + radius).min(w - 1) {
                    let Some(other) = owner[j * w + k] else {
                        continue;
                    };
                    if other == id || x.abs_diff(k).pow(2) + y.abs_diff(j).pow(2) > radius * radius
                    {
                        continue;
                    }
                    let a = root(&parent, id);
                    let b = root(&parent, other);
                    if a == b {
                        continue;
                    }
                    let ra = &regions[a];
                    let rb = &regions[b];
                    let lo = ra.lo.min(rb.lo);
                    let hi = ra.hi.max(rb.hi);
                    let gain_lo = ra.gain_lo.min(rb.gain_lo);
                    let gain_hi = ra.gain_hi.max(rb.gain_hi);
                    if hi - lo > policy.primary_span_ev
                        || gain_hi - gain_lo > 2. * policy.limit_ev
                        || (ra.sum / ra.count as f64 - rb.sum / rb.count as f64).abs()
                            > policy.edge_ev as f64
                    {
                        continue;
                    }
                    let sum = ra.sum + rb.sum;
                    let count = ra.count + rb.count;
                    parent[b] = a;
                    regions[a].lo = lo;
                    regions[a].hi = hi;
                    regions[a].gain_lo = gain_lo;
                    regions[a].gain_hi = gain_hi;
                    regions[a].sum = sum;
                    regions[a].count = count;
                }
            }
        }
    }
    let mut gains = vec![Vec::<f32>::new(); regions.len()];
    for (i, id) in owner.iter_mut().enumerate() {
        if let Some(old) = *id {
            let new = root(&parent, old);
            *id = Some(new);
            gains[new].push(data[i].unwrap());
        }
    }
    let mut count = 0;
    for (id, values) in gains.iter_mut().enumerate() {
        if values.is_empty() {
            continue;
        }
        count += 1;
        values.sort_unstable_by(f32::total_cmp);
        let region = &mut regions[id];
        region.gain = values[values.len() / 2].clamp(
            region.gain_hi - policy.limit_ev,
            (region.gain_lo + policy.limit_ev).max(region.gain_hi - policy.limit_ev),
        );
    }
    count
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
    let exposure_gain = exposure_ev.exp2() as f64;
    let relative = aw.max(ah) as f32 / policy.analysis_edge as f32;
    let neighborhood = (4. * relative).round().max(1.) as usize;
    let minimum = (policy.minimum_region_pixels as f32 * relative * relative)
        .ceil()
        .max(4.) as usize;
    let feather = (policy.feather_radius as f32 * relative).round().max(1.) as usize;
    let merge = (policy.merge_radius as f32 * relative).round().max(2.) as usize;
    let detail_radius = (policy.detail_radius as f32 * relative).round().max(1.) as usize;
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
        // Small aliasing outliers must not invalidate an otherwise reliable
        // surface. Broad scene differences still fail the support requirement.
        let supported = queue
            .iter()
            .copied()
            .filter(|i| (data[*i].unwrap() - center).abs() <= policy.limit_ev)
            .collect::<Vec<_>>();
        if supported.len() < minimum
            || (supported.len() as f32 / queue.len() as f32) < policy.minimum_gain_support
        {
            continue;
        }
        let gain_lo = supported
            .iter()
            .map(|i| data[*i].unwrap())
            .fold(f32::INFINITY, f32::min);
        let gain_hi = supported
            .iter()
            .map(|i| data[*i].unwrap())
            .fold(f32::NEG_INFINITY, f32::max);
        let id = regions.len();
        regions.push(Region {
            gain: center,
            lo,
            hi,
            gain_lo,
            gain_hi,
            sum: supported.iter().map(|i| log[*i] as f64).sum(),
            count: supported.len(),
        });
        for &i in &supported {
            owner[i] = Some(id);
        }
    }
    let region_count = merge_regions(&mut owner, &mut regions, &data, (aw, ah), merge, policy);
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
    // A median surface guide removes thin strokes but keeps broad step edges.
    // It detects undersampled structure; it never replaces the primary pixels.
    let mut surface = log.clone();
    let mut surface_hdr = primary.clone();
    let mut small = vec![false; data.len()];
    let mut samples = Vec::new();
    for y in 0..ah {
        for x in 0..aw {
            let i = y * aw + x;
            if !valid[i] {
                continue;
            }
            samples.clear();
            for j in y.saturating_sub(detail_radius)..=(y + detail_radius).min(ah - 1) {
                for k in x.saturating_sub(detail_radius)..=(x + detail_radius).min(aw - 1) {
                    if valid[j * aw + k] {
                        samples.push(log[j * aw + k]);
                    }
                }
            }
            samples.sort_unstable_by(f32::total_cmp);
            surface[i] = samples[samples.len() / 2];
            small[i] = (log[i] - surface[i]).abs() > policy.detail_threshold_ev;
            samples.clear();
            for j in y.saturating_sub(detail_radius)..=(y + detail_radius).min(ah - 1) {
                for k in x.saturating_sub(detail_radius)..=(x + detail_radius).min(aw - 1) {
                    if valid[j * aw + k] {
                        samples.push(primary[j * aw + k]);
                    }
                }
            }
            samples.sort_unstable_by(f32::total_cmp);
            surface_hdr[i] = samples[samples.len() / 2];
        }
    }
    let primary_models = vec![
        primary.clone(),
        blur_primary(&primary, &valid, aw, ah, 1),
        blur_primary(&primary, &valid, aw, ah, 2),
    ];
    let scene_scaled = scene
        .iter()
        .map(|v| (*v as f64 * exposure_gain) as f32)
        .collect::<Vec<_>>();
    let detail_seed_candidates = small.iter().filter(|v| **v).count();
    let mut seeds = vec![None; data.len()];
    for y in 0..ah {
        for x in 0..aw {
            let i = y * aw + x;
            if !small[i] {
                continue;
            }
            let Some(model) = detail_model(
                &primary_models,
                &scene_scaled,
                &valid,
                (aw, ah),
                (x, y),
                detail_radius,
                policy,
            ) else {
                continue;
            };
            let mut nearest = None;
            let mut distance = usize::MAX;
            for j in y.saturating_sub(merge)..=(y + merge).min(ah - 1) {
                for k in x.saturating_sub(merge)..=(x + merge).min(aw - 1) {
                    let Some(id) = owner[j * aw + k] else {
                        continue;
                    };
                    let region = &regions[id];
                    let gain = region.gain.exp2();
                    let contrast = model.slope / gain;
                    let anchor = model.slope * surface_hdr[i] + model.intercept;
                    if surface[i] < region.lo - policy.edge_ev
                        || surface[i] > region.hi + policy.edge_ev
                        || !(1. / policy.contrast_limit..=policy.contrast_limit).contains(&contrast)
                        || anchor <= 0.
                        || (anchor / (surface_hdr[i] * gain)).log2().abs()
                            > policy.maximum_detail_fit_error_ev
                    {
                        continue;
                    }
                    let d = x.abs_diff(k).pow(2) + y.abs_diff(j).pow(2);
                    if d < distance && d <= merge * merge {
                        nearest = Some((id, contrast));
                        distance = d;
                    }
                }
            }
            seeds[i] = nearest;
        }
    }
    let detail_seed_accepted = seeds.iter().filter(|v| v.is_some()).count();
    let mut detail = vec![0.; data.len()];
    let mut contrast = vec![1.; data.len()];
    for y in 0..ah {
        for x in 0..aw {
            let i = y * aw + x;
            if !valid[i] {
                continue;
            }
            let outer = detail_radius * 2;
            let mut nearest = None;
            let mut distance = usize::MAX;
            for j in y.saturating_sub(outer)..=(y + outer).min(ah - 1) {
                for k in x.saturating_sub(outer)..=(x + outer).min(aw - 1) {
                    let Some((id, c)) = seeds[j * aw + k] else {
                        continue;
                    };
                    let region = &regions[id];
                    if surface[i] < region.lo - policy.edge_ev
                        || surface[i] > region.hi + policy.edge_ev
                    {
                        continue;
                    }
                    let d = x.abs_diff(k).pow(2) + y.abs_diff(j).pow(2);
                    if d < distance && d < outer * outer {
                        nearest = Some((id, c));
                        distance = d;
                    }
                }
            }
            if let Some((id, c)) = nearest {
                let a = (2. - (distance as f32).sqrt() / detail_radius as f32).clamp(0., 1.);
                let a = a * a * (3. - 2. * a);
                values[i] = Some(regions[id].gain);
                strength[i] = strength[i].max(a);
                detail[i] = a;
                contrast[i] = c;
            }
        }
    }
    let detail_fraction = detail.iter().filter(|v| **v > 0.).count() as f32 / data.len() as f32;
    Ok(GuidanceAdjustment {
        values,
        strength,
        detail,
        surface_hdr,
        contrast,
        guide,
        metrics: SceneGuidanceMetrics {
            width: aw,
            height: ah,
            regions: region_count,
            region_fraction,
            detail_fraction,
            detail_seed_candidates,
            detail_seed_accepted,
            adjustment_rms_ev: 0.,
            maximum_adjustment_ev: 0.,
        },
    })
}
