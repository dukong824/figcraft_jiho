//! Parameters of Paper's Apache-2.0 Liquid Metal shader.
use figcraft_color::Color;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PaperLiquidMetal {
    pub repetition: f64,
    pub softness: f64,
    #[serde(alias = "shiftRed")]
    pub shift_red: f64,
    #[serde(alias = "shiftBlue")]
    pub shift_blue: f64,
    pub distortion: f64,
    pub contour: f64,
    pub angle: f64,
    pub speed: f64,
    /// Paper's starting animation frame, in milliseconds.
    pub frame: f64,
    pub scale: f64,
    pub rotation: f64,
    #[serde(alias = "colorBack")]
    pub color_back: Color,
    #[serde(alias = "colorTint")]
    pub color_tint: Color,
    #[serde(alias = "useImage")]
    pub use_image: bool,
    /// Original Paper shape: 0 none, 1 circle, 2 daisy, 3 diamond, 4 metaballs.
    pub shape: u32,
}

impl Default for PaperLiquidMetal {
    fn default() -> Self {
        Self {
            repetition: 2.0,
            softness: 0.1,
            shift_red: 0.3,
            shift_blue: 0.3,
            distortion: 0.07,
            contour: 0.4,
            angle: 70.0,
            speed: 1.0,
            frame: 0.0,
            scale: 1.0,
            rotation: 0.0,
            color_back: Color::TRANSPARENT,
            color_tint: Color::WHITE,
            use_image: true,
            shape: 0,
        }
    }
}

impl PaperLiquidMetal {
    pub fn sanitized(&self) -> Self {
        let d = Self::default();
        let n = |v: f64, fallback: f64, min: f64, max: f64| if v.is_finite() { v.clamp(min, max) } else { fallback };
        Self {
            repetition: n(self.repetition, d.repetition, 1.0, 10.0),
            softness: n(self.softness, d.softness, 0.0, 1.0),
            shift_red: n(self.shift_red, d.shift_red, -1.0, 1.0),
            shift_blue: n(self.shift_blue, d.shift_blue, -1.0, 1.0),
            distortion: n(self.distortion, d.distortion, 0.0, 1.0),
            contour: n(self.contour, d.contour, 0.0, 1.0),
            angle: n(self.angle, d.angle, 0.0, 360.0),
            speed: n(self.speed, d.speed, -3.0, 3.0),
            frame: n(self.frame, 0.0, -3.6e6, 3.6e6),
            scale: n(self.scale, d.scale, 0.1, 4.0),
            rotation: n(self.rotation, 0.0, 0.0, 360.0),
            color_back: self.color_back.sanitized(),
            color_tint: self.color_tint.sanitized(),
            use_image: self.use_image,
            shape: self.shape.min(4),
        }
    }
}
