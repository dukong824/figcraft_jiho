//! Original, reversible torn-paper edge treatment. Lengths are in local document units.
use figcraft_color::Color;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TornPaper {
    pub depth: f64,
    pub size: f64,
    pub roughness: f64,
    pub fibers: f64,
    #[serde(alias = "edgeWidth")]
    pub edge_width: f64,
    pub seed: u32,
    pub color: Color,
}

impl Default for TornPaper {
    fn default() -> Self {
        Self { depth: 12.0, size: 24.0, roughness: 0.7, fibers: 0.5, edge_width: 2.0, seed: 1, color: Color::new(0.97, 0.95, 0.90, 1.0) }
    }
}

impl TornPaper {
    pub fn sanitized(&self) -> Self {
        let d = Self::default();
        let n = |v: f64, fallback: f64, max: f64| if v.is_finite() { v.clamp(0.0, max) } else { fallback };
        Self {
            depth: n(self.depth, d.depth, 128.0),
            size: n(self.size, d.size, 512.0).max(1.0),
            roughness: n(self.roughness, d.roughness, 1.0),
            fibers: n(self.fibers, d.fibers, 1.0),
            edge_width: n(self.edge_width, d.edge_width, 16.0),
            seed: self.seed.min(10000),
            color: self.color.sanitized(),
        }
    }
}
