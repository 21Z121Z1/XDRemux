//! Bounded stabilization of thumbnail guidance in weak primary structure.
//! Only a coarse log-gain adjustment is filtered; primary RGB is never blurred.
use crate::{HdrError, Result};

#[derive(Debug, Clone, Copy)]
pub struct SceneGuidancePolicy {
    pub analysis_edge: usize,
    pub radius: f32,
    pub edge_ev: f32,
    pub flat_ev: f32,
    pub limit_ev: f32,
}

#[derive(Debug)]
pub struct SceneGuidanceMetrics {
    pub width: usize,
    pub height: usize,
    pub iterations: usize,
    pub relative_residual: f64,
    pub adjustment_rms_ev: f32,
    pub maximum_adjustment_ev: f32,
}

pub(super) struct GuidanceAdjustment {
    pub values: Vec<Option<f32>>,
    pub guide: Vec<f32>,
    pub metrics: SceneGuidanceMetrics,
}

fn invalid(message: &str) -> HdrError {
    HdrError::invalid("scene guidance", message)
}

pub(super) fn validate(policy: SceneGuidancePolicy) -> Result<()> {
    if !(32..=512).contains(&policy.analysis_edge)
        || !policy.radius.is_finite()
        || !(1.0..=64.0).contains(&policy.radius)
        || !policy.edge_ev.is_finite()
        || !(0.01..=0.5).contains(&policy.edge_ev)
        || !policy.flat_ev.is_finite()
        || !(0.01..=0.5).contains(&policy.flat_ev)
        || !policy.limit_ev.is_finite()
        || !(0.01..=0.5).contains(&policy.limit_ev)
    {
        return Err(invalid("unsupported stabilization policy"));
    }
    Ok(())
}

// Fractional area integration, in linear luminance, before taking logarithms.
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

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}

struct System {
    width: usize,
    height: usize,
    right: Vec<f64>,
    down: Vec<f64>,
    diagonal: Vec<f64>,
}

impl System {
    fn multiply(&self, input: &[f64], output: &mut [f64]) {
        output.copy_from_slice(input);
        for y in 0..self.height {
            for x in 0..self.width {
                let i = y * self.width + x;
                for (j, weight) in [
                    (i + usize::from(x + 1 < self.width), self.right[i]),
                    (
                        i + if y + 1 < self.height { self.width } else { 0 },
                        self.down[i],
                    ),
                ] {
                    let delta = weight * (input[i] - input[j]);
                    output[i] += delta;
                    output[j] -= delta;
                }
            }
        }
    }

    // SPD screened diffusion, with a Jacobi-preconditioned conjugate gradient
    // solve. A measured residual, not an iteration count, decides convergence.
    fn solve(&self, data: &[f64]) -> Result<(Vec<f64>, usize, f64)> {
        let mut x = data.to_vec();
        let mut product = vec![0.; data.len()];
        self.multiply(&x, &mut product);
        let mut residual = data
            .iter()
            .zip(&product)
            .map(|(b, a)| b - a)
            .collect::<Vec<_>>();
        let mut z = residual
            .iter()
            .zip(&self.diagonal)
            .map(|(r, d)| r / d)
            .collect::<Vec<_>>();
        let mut direction = z.clone();
        let mut rz = dot(&residual, &z);
        let norm = dot(data, data).sqrt().max(1e-12);
        for iteration in 0..=1024 {
            let relative = dot(&residual, &residual).sqrt() / norm;
            if relative <= 1e-5 {
                return Ok((x, iteration, relative));
            }
            if iteration == 1024 {
                break;
            }
            self.multiply(&direction, &mut product);
            let alpha = rz / dot(&direction, &product);
            for i in 0..x.len() {
                x[i] += alpha * direction[i];
                residual[i] -= alpha * product[i];
                z[i] = residual[i] / self.diagonal[i];
            }
            let next = dot(&residual, &z);
            let beta = next / rz;
            for i in 0..x.len() {
                direction[i] = z[i] + beta * direction[i];
            }
            rz = next;
        }
        Err(invalid("stabilization did not converge"))
    }
}

pub(super) fn adjustment(
    primary: &[f32],
    thumbnail: &[f32],
    geometry: (usize, usize),
    exposure_ev: f32,
    policy: SceneGuidancePolicy,
) -> Result<GuidanceAdjustment> {
    validate(policy)?;
    let (w, h) = geometry;
    let scale = (policy.analysis_edge as f64 / w.max(h) as f64).min(1.);
    let aw = ((w as f64 * scale).round() as usize).max(1);
    let ah = ((h as f64 * scale).round() as usize).max(1);
    let guide = area(primary, w, h, aw, ah);
    let scene = area(thumbnail, w, h, aw, ah);
    let valid = guide
        .iter()
        .zip(&scene)
        .map(|(a, b)| *a > 1e-5 && *b > 1e-5)
        .collect::<Vec<_>>();
    let log = guide
        .iter()
        .map(|v| v.max(1e-8).log2() as f64)
        .collect::<Vec<_>>();
    let data = guide
        .iter()
        .zip(&scene)
        .zip(&valid)
        .map(|((a, b), valid)| {
            if *valid {
                ((*b / *a).log2() + exposure_ev).clamp(-6., 6.) as f64
            } else {
                0.
            }
        })
        .collect::<Vec<_>>();
    let radius = policy.radius as f64 * aw.max(ah) as f64 / policy.analysis_edge as f64;
    let lambda = radius * radius;
    let mut system = System {
        width: aw,
        height: ah,
        right: vec![0.; data.len()],
        down: vec![0.; data.len()],
        diagonal: vec![1.; data.len()],
    };
    for y in 0..ah {
        for x in 0..aw {
            let i = y * aw + x;
            for (j, horizontal) in [
                (i + usize::from(x + 1 < aw), true),
                (i + if y + 1 < ah { aw } else { 0 }, false),
            ] {
                if j == i || !valid[i] || !valid[j] {
                    continue;
                }
                let d = (log[i] - log[j]) / policy.edge_ev as f64;
                let weight = lambda * (-0.5 * d * d).exp();
                if horizontal {
                    system.right[i] = weight;
                } else {
                    system.down[i] = weight;
                }
                system.diagonal[i] += weight;
                system.diagonal[j] += weight;
            }
        }
    }
    let (smooth, iterations, relative_residual) = system.solve(&data)?;
    // Local primary variance supplies confidence. Strong structure and texture
    // keep their existing recovery. Invalid guidance never participates.
    let neighborhood =
        ((4. * aw.max(ah) as f64 / policy.analysis_edge as f64).round() as usize).max(1);
    let mut values = Vec::with_capacity(data.len());
    let mut square = 0.;
    let mut maximum = 0f32;
    for y in 0..ah {
        for x in 0..aw {
            let i = y * aw + x;
            if !valid[i] {
                values.push(None);
                continue;
            }
            let mut total = 0.;
            let mut sum = 0.;
            let mut sum2 = 0.;
            for j in y.saturating_sub(neighborhood)..=(y + neighborhood).min(ah - 1) {
                for k in x.saturating_sub(neighborhood)..=(x + neighborhood).min(aw - 1) {
                    let n = j * aw + k;
                    if valid[n] {
                        total += 1.;
                        sum += log[n];
                        sum2 += log[n] * log[n];
                    }
                }
            }
            let variance = (sum2 / total - (sum / total).powi(2)).max(0.);
            let flat = (policy.flat_ev as f64).powi(2);
            let confidence = (flat / (flat + variance)).powi(2);
            let limit = policy.limit_ev as f64;
            let delta = (confidence * limit * ((smooth[i] - data[i]) / limit).tanh()) as f32;
            maximum = maximum.max(delta.abs());
            square += (delta as f64).powi(2);
            values.push(Some(delta));
        }
    }
    Ok(GuidanceAdjustment {
        values,
        guide,
        metrics: SceneGuidanceMetrics {
            width: aw,
            height: ah,
            iterations,
            relative_residual,
            adjustment_rms_ev: (square / data.len() as f64).sqrt() as f32,
            maximum_adjustment_ev: maximum,
        },
    })
}
