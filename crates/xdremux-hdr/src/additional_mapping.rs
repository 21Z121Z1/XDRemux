//! Additional matrix and three-dimensional lookup table execution.
//!
//! The contract is explicit because a mapping that is applied with the wrong
//! order or axis convention still produces plausible numbers:
//!
//! 1. The three-by-three matrix is applied first, in `f64`, without clipping.
//! 2. Each mapped component is then normalized by the LUT domain:
//!    `t = (value - domain_min) / (domain_max - domain_min) * (edge - 1)`.
//! 3. Nodes are ordered with the first component fastest:
//!    `index = r + edge * (g + edge * b)`.
//! 4. Trilinear interpolation uses the eight nodes of the containing cell.
//!    A coordinate on the last node degenerates to the previous cell.
//! 5. A value outside the domain follows the caller's policy: reject or clamp.
//!
//! Parsing a mapping payload is a separate concern. This module never decides
//! how an external payload maps onto these fields.
use crate::error::{HdrError, Result};

/// Largest accepted LUT edge. It bounds nodes at 274,625 per mapping.
pub const MAX_LUT_EDGE: usize = 65;

fn invalid(message: impl Into<String>) -> HdrError {
    HdrError::invalid("additional mapping", message)
}

/// What to do when a mapped component falls outside the LUT domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DomainPolicy {
    /// Refuse the sample. This is the default for a file-declared mapping.
    #[default]
    Reject,
    /// Clamp the component to the domain edge.
    Clamp,
}

/// A three-dimensional LUT with an explicit domain and node order.
#[derive(Debug, Clone, PartialEq)]
pub struct RgbLut3d {
    edge: usize,
    domain_min: [f64; 3],
    domain_max: [f64; 3],
    nodes: Vec<[f32; 3]>,
}

impl RgbLut3d {
    pub fn new(
        edge: usize,
        domain_min: [f64; 3],
        domain_max: [f64; 3],
        nodes: Vec<[f32; 3]>,
    ) -> Result<Self> {
        if !(2..=MAX_LUT_EDGE).contains(&edge) {
            return Err(invalid(format!(
                "LUT edge {edge} is outside 2 through {MAX_LUT_EDGE}"
            )));
        }
        let expected = edge
            .checked_mul(edge)
            .and_then(|value| value.checked_mul(edge))
            .ok_or_else(|| invalid("LUT node count overflows"))?;
        if nodes.len() != expected {
            return Err(invalid(format!(
                "LUT edge {edge} requires {expected} nodes, found {}",
                nodes.len()
            )));
        }
        for axis in 0..3 {
            let minimum = domain_min[axis];
            let maximum = domain_max[axis];
            if !minimum.is_finite() || !maximum.is_finite() || minimum >= maximum {
                return Err(invalid("LUT domain must be finite and increasing"));
            }
        }
        if nodes.iter().flatten().any(|value| !value.is_finite()) {
            return Err(invalid("LUT nodes must be finite"));
        }
        Ok(Self {
            edge,
            domain_min,
            domain_max,
            nodes,
        })
    }

    pub const fn edge(&self) -> usize {
        self.edge
    }

    pub const fn domain(&self) -> ([f64; 3], [f64; 3]) {
        (self.domain_min, self.domain_max)
    }

    pub fn nodes(&self) -> &[[f32; 3]] {
        &self.nodes
    }

    fn node(&self, r: usize, g: usize, b: usize) -> [f64; 3] {
        let index = r + self.edge * (g + self.edge * b);
        let node = self.nodes[index];
        [f64::from(node[0]), f64::from(node[1]), f64::from(node[2])]
    }

    /// Map one linear RGB triple through the LUT.
    pub fn sample(&self, rgb: [f64; 3], policy: DomainPolicy) -> Result<[f64; 3]> {
        if rgb.iter().any(|value| !value.is_finite()) {
            return Err(invalid("mapping input must be finite"));
        }
        let last = self.edge - 1;
        let mut coordinates = [0.0_f64; 3];
        for axis in 0..3 {
            let span = self.domain_max[axis] - self.domain_min[axis];
            let mut position = (rgb[axis] - self.domain_min[axis]) / span;
            if !(0.0..=1.0).contains(&position) {
                match policy {
                    DomainPolicy::Reject => {
                        return Err(invalid(format!(
                            "component {axis} is outside the LUT domain"
                        )))
                    }
                    DomainPolicy::Clamp => position = position.clamp(0.0, 1.0),
                }
            }
            coordinates[axis] = position * last as f64;
        }
        let base = coordinates.map(|value| {
            let floor = value.floor();
            let index = if floor >= last as f64 {
                last - 1
            } else {
                floor as usize
            };
            (index, value - index as f64)
        });
        let mut result = [0.0_f64; 3];
        for (dr, wr) in [(0usize, 1.0 - base[0].1), (1, base[0].1)] {
            for (dg, wg) in [(0usize, 1.0 - base[1].1), (1, base[1].1)] {
                for (db, wb) in [(0usize, 1.0 - base[2].1), (1, base[2].1)] {
                    let weight = wr * wg * wb;
                    if weight == 0.0 {
                        continue;
                    }
                    let node = self.node(base[0].0 + dr, base[1].0 + dg, base[2].0 + db);
                    for channel in 0..3 {
                        result[channel] += weight * node[channel];
                    }
                }
            }
        }
        Ok(result)
    }
}

/// A matrix followed by an optional LUT, applied in that order.
#[derive(Debug, Clone, PartialEq)]
pub struct AdditionalMapping {
    matrix: [[f64; 3]; 3],
    lut: Option<RgbLut3d>,
}

impl AdditionalMapping {
    pub fn new(matrix: [[f64; 3]; 3], lut: Option<RgbLut3d>) -> Result<Self> {
        if matrix.iter().flatten().any(|value| !value.is_finite()) {
            return Err(invalid("matrix elements must be finite"));
        }
        Ok(Self { matrix, lut })
    }

    pub const fn matrix(&self) -> &[[f64; 3]; 3] {
        &self.matrix
    }

    pub const fn lut(&self) -> Option<&RgbLut3d> {
        self.lut.as_ref()
    }

    /// Apply the matrix and then the LUT.
    ///
    /// Negative intermediate values stay negative: the matrix result is passed
    /// to the LUT unchanged, and a LUT that maps a negative result is subject
    /// to the caller's domain policy.
    pub fn apply(&self, rgb: [f64; 3], policy: DomainPolicy) -> Result<[f64; 3]> {
        if rgb.iter().any(|value| !value.is_finite()) {
            return Err(invalid("mapping input must be finite"));
        }
        let mut mapped = [0.0_f64; 3];
        for (row, output) in self.matrix.iter().zip(mapped.iter_mut()) {
            *output = row[0] * rgb[0] + row[1] * rgb[1] + row[2] * rgb[2];
        }
        match &self.lut {
            Some(lut) => lut.sample(mapped, policy),
            None => Ok(mapped),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P3 to BT.2020 D65, as an independent reference matrix.
    const P3_TO_BT2020: [[f64; 3]; 3] = [
        [0.7538330343617218, 0.1985973690526163, 0.04756959658566188],
        [0.04574384896535833, 0.9417772198116935, 0.01247893122294812],
        [
            -0.001210340354518324,
            0.01760171730108989,
            0.9836086230534284,
        ],
    ];

    fn close(left: [f64; 3], right: [f64; 3]) -> bool {
        left.iter()
            .zip(right.iter())
            .all(|(a, b)| (a - b).abs() < 1e-12)
    }

    #[test]
    fn the_matrix_keeps_neutral_white_and_small_negative_values() {
        let mapping = AdditionalMapping::new(P3_TO_BT2020, None).unwrap();
        let white = mapping
            .apply([1.0, 1.0, 1.0], DomainPolicy::Reject)
            .unwrap();
        assert!(close(white, [1.0, 1.0, 1.0]), "{white:?}");

        // Independent reference values for a non-neutral colour.
        let mapped = mapping
            .apply([0.2, 0.5, 0.1], DomainPolicy::Reject)
            .unwrap();
        assert!(
            close(
                mapped,
                [0.25482225105721873, 0.4812852728212132, 0.10691965288498413]
            ),
            "{mapped:?}"
        );
        // The first row has a negative coefficient and the result keeps it.
        let blue = mapping
            .apply([0.0, 0.0, 1.0], DomainPolicy::Reject)
            .unwrap();
        assert!(
            close(
                blue,
                [0.04756959658566188, 0.01247893122294812, 0.9836086230534284]
            ),
            "{blue:?}"
        );
    }

    #[test]
    fn trilinear_interpolation_matches_an_independent_linear_function() {
        // Node (r, g, b) holds [r + 2g, g + 3b, b + 4r]. R is the fastest axis.
        let mut nodes = Vec::new();
        for b in [0.0_f32, 1.0] {
            for g in [0.0_f32, 1.0] {
                for r in [0.0_f32, 1.0] {
                    nodes.push([r + 2.0 * g, g + 3.0 * b, b + 4.0 * r]);
                }
            }
        }
        let lut = RgbLut3d::new(2, [0.0; 3], [1.0; 3], nodes).unwrap();
        let sampled = lut.sample([0.25, 0.5, 0.75], DomainPolicy::Reject).unwrap();
        assert!(close(sampled, [1.25, 2.75, 1.75]), "{sampled:?}");
    }

    #[test]
    fn node_axis_order_is_red_fastest() {
        // Every node keeps its red index and zeroes the other outputs.
        let mut nodes = Vec::new();
        for _b in [0.0_f32, 1.0] {
            for _g in [0.0_f32, 1.0] {
                for r in [0.0_f32, 1.0] {
                    nodes.push([r, 0.0, 0.0]);
                }
            }
        }
        let lut = RgbLut3d::new(2, [0.0; 3], [1.0; 3], nodes).unwrap();
        assert_eq!(
            lut.sample([1.0, 0.0, 0.0], DomainPolicy::Reject).unwrap(),
            [1.0, 0.0, 0.0]
        );
        assert_eq!(
            lut.sample([0.0, 1.0, 0.0], DomainPolicy::Reject).unwrap(),
            [0.0, 0.0, 0.0]
        );
    }

    #[test]
    fn an_edge_three_lut_interpolates_inside_a_cell() {
        // Node (r, g, b) holds [r*r, 0.5g + b, 1 - r*g] on a 3-node edge.
        let steps = [0.0_f32, 0.5, 1.0];
        let mut nodes = Vec::new();
        for b in steps {
            for g in steps {
                for r in steps {
                    nodes.push([r * r, 0.5 * g + b, 1.0 - r * g]);
                }
            }
        }
        let lut = RgbLut3d::new(3, [0.0; 3], [1.0; 3], nodes).unwrap();
        let sampled = lut.sample([0.25, 0.6, 0.1], DomainPolicy::Reject).unwrap();
        assert!(
            close(sampled, [0.125, 0.4, 0.8500000000000001]),
            "{sampled:?}"
        );
    }

    #[test]
    fn the_domain_maps_the_input_range_to_the_edge() {
        let mut nodes = Vec::new();
        for b in [0.0_f32, 1.0] {
            for g in [0.0_f32, 1.0] {
                for r in [0.0_f32, 1.0] {
                    nodes.push([2.0 * r, 2.0 * g, 2.0 * b]);
                }
            }
        }
        let lut = RgbLut3d::new(2, [0.0; 3], [2.0; 3], nodes).unwrap();
        let sampled = lut.sample([0.5, 1.0, 1.5], DomainPolicy::Reject).unwrap();
        assert!(close(sampled, [0.5, 1.0, 1.5]), "{sampled:?}");
        assert!(lut.sample([0.5, 1.0, 2.5], DomainPolicy::Reject).is_err());
        assert!(close(
            lut.sample([0.5, 1.0, 2.5], DomainPolicy::Clamp).unwrap(),
            [0.5, 1.0, 2.0]
        ));
    }

    #[test]
    fn mapping_rejects_malformed_luts_and_non_finite_input() {
        assert!(RgbLut3d::new(1, [0.0; 3], [1.0; 3], vec![[0.0; 3]; 1]).is_err());
        assert!(RgbLut3d::new(2, [0.0; 3], [1.0; 3], vec![[0.0; 3]; 7]).is_err());
        assert!(RgbLut3d::new(2, [1.0; 3], [1.0; 3], vec![[0.0; 3]; 8]).is_err());
        assert!(RgbLut3d::new(2, [0.0; 3], [1.0; 3], vec![[f32::NAN; 3]; 8]).is_err());
        let lut = RgbLut3d::new(2, [0.0; 3], [1.0; 3], vec![[0.0; 3]; 8]).unwrap();
        assert!(lut
            .sample([f64::INFINITY, 0.0, 0.0], DomainPolicy::Clamp)
            .is_err());
        assert!(AdditionalMapping::new([[f64::NAN; 3]; 3], None).is_err());
    }
}
