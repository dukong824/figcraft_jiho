const PNG: &[u8] = include_bytes!("../../../../assets/purpose/purpose.png");

pub fn window_icon() -> Option<egui::IconData> {
    // Native Windows HICONs use a standard size; the supplied PNG remains unchanged.
    let icon = image::load_from_memory(PNG).ok()?.resize_exact(256, 256, image::imageops::FilterType::Lanczos3).to_rgba8();
    Some(egui::IconData { width: 256, height: 256, rgba: icon.into_raw() })
}

#[cfg(windows)]
pub fn set_windows_icons(window: &winit::window::Window) -> Result<(), winit::window::BadIcon> {
    use winit::platform::windows::{IconExtWindows, WindowExtWindows};
    use winit::{dpi::PhysicalSize, window::Icon};

    let load = |size| {
        if cfg!(feature = "purpose-branding") {
            // build.rs embeds the owner's multi-size ICO as resource 1. Let Windows
            // select its native image instead of constructing a legacy bitmap HICON.
            Icon::from_resource(1, Some(PhysicalSize::new(size, size)))
        } else {
            let pixels = image::load_from_memory(PNG).map(|image| image.resize_exact(size, size, image::imageops::FilterType::Lanczos3).to_rgba8());
            match pixels {
                Ok(pixels) => Icon::from_rgba(pixels.into_raw(), size, size),
                Err(error) => Err(winit::window::BadIcon::OsError(std::io::Error::other(error))),
            }
        }
    };
    let small = load(32)?;
    let taskbar = load(48)?;
    log::info!("Purpose native window icons loaded: small={small:?}, taskbar={taskbar:?}");
    window.set_window_icon(Some(small));
    window.set_taskbar_icon(Some(taskbar));
    Ok(())
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
