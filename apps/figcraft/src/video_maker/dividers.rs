use super::{VideoMaker, style};
impl VideoMaker {
    pub fn draw_dividers(&self, ctx: &egui::Context) {
        let mut mesh = egui::Mesh::default();
        for &(a, b) in &self.dividers {
            if let Some(rect) = divider(a, b, ctx.pixels_per_point()) {
                mesh.add_colored_rect(rect, style::BORDER);
            }
        }
        ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::unique("purpose-shell-dividers")))
            .with_clip_rect(ctx.viewport_rect())
            .add(egui::Shape::mesh(mesh));
    }
}
fn divider(a: egui::Pos2, b: egui::Pos2, ppp: f32) -> Option<egui::Rect> {
    if !a.is_finite() || !b.is_finite() || !ppp.is_finite() || ppp <= 0.0 {
        return None;
    }
    let snap = |v: f32| (v * ppp).round() / ppp;
    let pixel = 1.0 / ppp;
    if (a.x - b.x).abs() < 0.01 {
        let x = snap(a.x);
        Some(egui::Rect::from_min_max(egui::pos2(x - pixel, snap(a.y.min(b.y))), egui::pos2(x, snap(a.y.max(b.y)))))
    } else {
        let y = snap(a.y);
        Some(egui::Rect::from_min_max(egui::pos2(snap(a.x.min(b.x)), y - pixel), egui::pos2(snap(a.x.max(b.x)), y)))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shell_boundaries_are_solid_physical_pixels_at_fractional_scales() {
        for dpi in [1.0, 1.25, 1.5, 2.0] {
            for zoom in [0.7, 0.9, 1.0, 1.1, 1.25, 1.5, 2.0] {
                let ppp = dpi * zoom;
                let v = divider(egui::pos2(252.3, 47.2), egui::pos2(252.3, 800.0), ppp).unwrap();
                let h = divider(egui::pos2(252.3, 92.1), egui::pos2(1280.0, 92.1), ppp).unwrap();
                assert!((v.width() * ppp - 1.0).abs() < 0.001);
                assert!((h.height() * ppp - 1.0).abs() < 0.001);
                assert!((v.right() * ppp - (v.right() * ppp).round()).abs() < 0.001);
            }
        }
    }
}
