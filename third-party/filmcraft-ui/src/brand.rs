//! Host-provided Purpose branding; upstream ArtCraft artwork is not embedded.
pub fn wordmark(ctx: &egui::Context, _dark: bool) -> Option<egui::TextureHandle> {
    ctx.data(|data| data.get_temp(egui::Id::unique("purpose-logo")))
}
pub fn paint_wordmark(ui: &egui::Ui, left_center: egui::Pos2, height: f32, dark: bool) -> Option<egui::Rect> {
    let texture = wordmark(ui.ctx(), dark)?;
    let size = texture.size_vec2();
    let rect =
        egui::Rect::from_min_size(egui::pos2(left_center.x, left_center.y - height / 2.0), egui::vec2(height * size.x / size.y.max(1.0), height));
    ui.painter().image(texture.id(), rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
    Some(rect)
}
