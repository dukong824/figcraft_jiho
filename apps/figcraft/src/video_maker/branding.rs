const PNG: &[u8] = include_bytes!("../../../../assets/purpose/purpose.png");

pub fn window_icon() -> Option<egui::IconData> {
    // Native Windows HICONs use a standard size; the supplied PNG remains unchanged.
    let icon = image::load_from_memory(PNG).ok()?.resize_exact(256, 256, image::imageops::FilterType::Lanczos3).to_rgba8();
    Some(egui::IconData { width: 256, height: 256, rgba: icon.into_raw() })
}

pub(super) fn texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let id = egui::Id::unique("purpose-logo");
    if let Some(texture) = ctx.data(|data| data.get_temp::<egui::TextureHandle>(id)) {
        return Some(texture);
    }
    let image = image::load_from_memory(PNG).ok()?.to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let texture = ctx.load_texture("purpose-logo", egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()), egui::TextureOptions::LINEAR);
    ctx.data_mut(|data| data.insert_temp(id, texture.clone()));
    Some(texture)
}
