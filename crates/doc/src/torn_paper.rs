//! Papierwerk border: paper is placed behind the source, never cut into it.
use figcraft_color::Color;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TornPaper {
    pub width: f64,
    pub roughness: f64,
    pub seed: u32,
    pub color: Color,
}
impl Default for TornPaper {
    fn default() -> Self {
        Self { width: 22.0, roughness: 0.55, seed: 1, color: Color::new(246.0 / 255.0, 244.0 / 255.0, 238.0 / 255.0, 1.0) }
    }
}
impl TornPaper {
    pub fn sanitized(&self) -> Self {
        let d = Self::default();
        Self {
            width: if self.width.is_finite() { self.width.clamp(0.0, 128.0) } else { d.width },
            roughness: if self.roughness.is_finite() { self.roughness.clamp(0.0, 1.0) } else { d.roughness },
            seed: self.seed.min(10000),
            color: self.color.sanitized(),
        }
    }
    pub fn reach(&self) -> f64 {
        let p = self.sanitized();
        if p.width <= 0.0 || p.color.a <= 0.0 { 0.0 } else { (p.width * 2.0 + 2.0).ceil() }
    }
}
