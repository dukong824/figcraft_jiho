//! Lucide 1.64.0 (ISC / Feather MIT), embedded offline from its pinned upstream revision.
//! Original SVG geometry is rasterised at the current display scale, then tinted by egui.
use egui::{Color32, Context, Id, Painter, Rect, TextureHandle, TextureOptions, pos2, vec2};
use std::collections::VecDeque;
mod assets;
pub use assets::ALL;

const CACHE_LIMIT: usize = 192;

#[derive(Clone)]
struct Entry {
    name: &'static str,
    pixels: u32,
    solid: bool,
    texture: TextureHandle,
}
#[derive(Clone, Default)]
struct Cache {
    entries: VecDeque<Entry>,
}

pub fn set_enabled(ctx: &Context, enabled: bool) {
    ctx.data_mut(|data| data.insert_temp(Id::unique("purpose-lucide-enabled"), enabled));
}
pub fn enabled(ctx: &Context) -> bool {
    ctx.data(|data| data.get_temp::<bool>(Id::unique("purpose-lucide-enabled")).unwrap_or(false))
}

/// Paint an embedded glyph once, centred and clipped to the supplied box. Returns false only
/// for unknown assets or a decoding error, allowing the host to retain an existing fallback.
pub fn paint(painter: &Painter, rect: Rect, name: &str, color: Color32) -> bool {
    paint_variant(painter, rect, name, color, false)
}
/// Fill the original closed SVG geometry for an active toggle, without changing its source.
pub fn paint_solid(painter: &Painter, rect: Rect, name: &str, color: Color32) -> bool {
    paint_variant(painter, rect, name, color, true)
}
fn paint_variant(painter: &Painter, rect: Rect, name: &str, color: Color32, solid: bool) -> bool {
    let Some((name, source)) = assets::get(name) else { return false };
    if !rect.is_finite() || !rect.is_positive() || !painter.clip_rect().intersects(rect) {
        return true;
    }
    let side = rect.width().min(rect.height());
    let rect = Rect::from_center_size(rect.center(), vec2(side, side));
    let pixels = (side * painter.ctx().pixels_per_point()).ceil().clamp(8.0, 128.0) as u32;
    let key = Id::unique("purpose-lucide-cache");
    let cached = painter.ctx().data_mut(|data| {
        let cache = data.get_temp_mut_or_default::<Cache>(key);
        let index = cache.entries.iter().position(|entry| entry.name == name && entry.pixels == pixels && entry.solid == solid)?;
        let entry = cache.entries.remove(index)?;
        let texture = entry.texture.clone();
        cache.entries.push_back(entry);
        Some(texture)
    });
    let texture = if let Some(texture) = cached {
        texture
    } else {
        let Some(image) = rasterize_variant(source, pixels, solid) else { return false };
        let texture = painter.ctx().load_texture(format!("lucide-{name}-{pixels}-{solid}"), image, TextureOptions::LINEAR);
        painter.ctx().data_mut(|data| {
            let cache = data.get_temp_mut_or_default::<Cache>(key);
            cache.entries.push_back(Entry { name, pixels, solid, texture: texture.clone() });
            while cache.entries.len() > CACHE_LIMIT {
                cache.entries.pop_front();
            }
        });
        texture
    };
    painter.with_clip_rect(painter.clip_rect().intersect(rect)).image(texture.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), color);
    true
}

#[cfg(test)]
fn rasterize(source: &[u8], pixels: u32) -> Option<egui::ColorImage> {
    rasterize_variant(source, pixels, false)
}
fn rasterize_variant(source: &[u8], pixels: u32, solid: bool) -> Option<egui::ColorImage> {
    let tree = usvg::Tree::from_data(source, &usvg::Options::default()).ok()?;
    let mut pixmap = tiny_skia::Pixmap::new(pixels, pixels)?;
    let scale = tiny_skia::Transform::from_scale(pixels as f32 / tree.size().width(), pixels as f32 / tree.size().height());
    if !draw(tree.root(), &mut pixmap, scale, solid) {
        return None;
    }
    Some(egui::ColorImage::from_rgba_premultiplied([pixels as usize, pixels as usize], pixmap.data()))
}
fn draw(group: &usvg::Group, pixmap: &mut tiny_skia::Pixmap, scale: tiny_skia::Transform, solid: bool) -> bool {
    for node in group.children() {
        match node {
            usvg::Node::Group(group) => {
                if !draw(group, pixmap, scale, solid) {
                    return false;
                }
            }
            usvg::Node::Path(path) => {
                if !path.is_visible() {
                    continue;
                }
                let transform = scale.pre_concat(path.abs_transform());
                let mut paint = tiny_skia::Paint { anti_alias: true, ..Default::default() };
                if path.fill().is_some() || solid {
                    paint.set_color_rgba8(255, 255, 255, path.fill().map_or(255, |fill| (fill.opacity().get() * 255.0).round() as u8));
                    let rule = match path.fill().map_or(usvg::FillRule::NonZero, |fill| fill.rule()) {
                        usvg::FillRule::NonZero => tiny_skia::FillRule::Winding,
                        usvg::FillRule::EvenOdd => tiny_skia::FillRule::EvenOdd,
                    };
                    pixmap.fill_path(path.data(), &paint, rule, transform, None);
                }
                if let Some(stroke) = path.stroke() {
                    paint.set_color_rgba8(255, 255, 255, (stroke.opacity().get() * 255.0).round() as u8);
                    pixmap.stroke_path(path.data(), &paint, &stroke.to_tiny_skia(), transform, None);
                }
            }
            // The vendored icon set contains only paths. Reject image/text content rather than
            // invoking fonts, external resource loaders or a second rendering stack.
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn active_lightning_uses_a_filled_mask_without_changing_the_source() {
        let (_, svg) = assets::get("zap").unwrap();
        let outline = rasterize_variant(svg, 32, false).unwrap();
        let solid = rasterize_variant(svg, 32, true).unwrap();
        assert!(solid.pixels.iter().filter(|p| p.a() > 0).count() > outline.pixels.iter().filter(|p| p.a() > 0).count());
        assert!(solid.pixels.iter().all(|p| p.r() == p.a() && p.g() == p.a() && p.b() == p.a()));
    }
    #[test]
    fn every_downloaded_svg_renders_at_small_and_high_dpi_sizes_without_fonts() {
        for name in ALL {
            let (_, svg) = assets::get(name).unwrap();
            let tree = usvg::Tree::from_data(svg, &usvg::Options::default()).unwrap();
            assert_eq!((tree.size().width(), tree.size().height()), (24.0, 24.0), "{name}");
            for pixels in [12, 18, 32, 64] {
                let image = rasterize(svg, pixels).unwrap_or_else(|| panic!("cannot render {name}"));
                assert_eq!(image.size, [pixels as usize, pixels as usize]);
                assert!(image.pixels.iter().any(|p| p.a() > 0), "empty {name} at {pixels}");
                assert!(image.pixels.iter().all(|p| p.r() == p.a() && p.g() == p.a() && p.b() == p.a()), "tint mask {name}");
            }
        }
    }
    #[test]
    fn texture_cache_is_bounded_across_ui_scales() {
        let ctx = Context::default();
        for scale in 1..=3 {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                for name in ALL {
                    assert!(paint(
                        ui.painter(),
                        Rect::from_min_size(pos2(10.0, 10.0), vec2(16.0 * scale as f32, 16.0 * scale as f32)),
                        name,
                        Color32::RED
                    ));
                }
            });
            output.textures_delta.clear();
            assert!(ctx.data(|data| data.get_temp::<Cache>(Id::unique("purpose-lucide-cache")).unwrap().entries.len()) <= CACHE_LIMIT);
        }
        assert!(!paint(&ctx.debug_painter(), Rect::EVERYTHING, "not-an-icon", Color32::WHITE));
    }
}

// The desktop host scopes text sizing to one application frame. Geometry and authored
// document text stay independent of this preference; standalone editors default to 1.
thread_local! { static FONT_SCALE: std::cell::Cell<f32> = const { std::cell::Cell::new(1.0) }; }
pub fn font_size(size: f32) -> f32 {
    FONT_SCALE.with(|scale| size * scale.get())
}
pub fn with_font_scale<R>(scale: f32, draw: impl FnOnce() -> R) -> R {
    struct Restore(f32);
    impl Drop for Restore {
        fn drop(&mut self) {
            FONT_SCALE.with(|scale| scale.set(self.0));
        }
    }
    let scale = if scale.is_finite() { scale.clamp(0.5, 2.0) } else { 1.0 };
    let _restore = Restore(FONT_SCALE.with(|current| current.replace(scale)));
    draw()
}

#[cfg(test)]
mod font_scale_tests {
    #[test]
    fn text_scale_is_scoped_nested_and_restored_after_panic() {
        assert_eq!(super::font_size(14.0), 14.0);
        super::with_font_scale(20.0 / 14.0, || {
            assert_eq!(super::font_size(14.0), 20.0);
            super::with_font_scale(1.0, || assert_eq!(super::font_size(14.0), 14.0));
            assert_eq!(super::font_size(14.0), 20.0);
        });
        let _ = std::panic::catch_unwind(|| super::with_font_scale(2.0, || panic!("test unwind")));
        assert_eq!(super::font_size(14.0), 14.0);
    }
}
