//! Paper Liquid Metal: original GLSL plus a Rust port of its image preprocessing.
//! Upstream: paper-design/shaders, commit 1923e9f536ea079e2a95d23291cbfacfd69d37f6.
//! Copyright Lost Coast Labs, Inc.; Apache-2.0. See ../paper-shaders/{LICENSE,NOTICE}.
use figcraft_doc::{Document, EffectKind, NodeId, PaperLiquidMetal};
use figcraft_geom::Affine;
#[cfg(not(target_arch = "wasm32"))]
use figcraft_geom::Rect;
use image::{
    RgbaImage,
    imageops::{FilterType, resize},
};
use tiny_skia::Pixmap;
#[cfg(not(target_arch = "wasm32"))]
use tiny_skia::PixmapPaint;
#[cfg(not(target_arch = "wasm32"))]
mod gpu;

/// Render an input image with Paper's original shader at `time` seconds. The image's alpha
/// defines its silhouette. This native API also supports deterministic animation exports.
#[cfg(not(target_arch = "wasm32"))]
pub fn render_image(source: &Pixmap, params: &PaperLiquidMetal, time: f64) -> Result<Pixmap, String> {
    gpu::render(source, params, time)
}

pub(crate) fn effect_animated(e: &figcraft_doc::Effect) -> bool {
    match &e.kind {
        EffectKind::PaperLiquidMetal(p) => p.speed.is_finite() && p.speed != 0.0,
        EffectKind::PaperImageFilter(p) => p.animated(),
        _ => false,
    }
}

pub fn animated(doc: &Document) -> bool {
    doc.nodes().any(|n| n.visible && n.effects.iter().any(|e| e.visible && effect_animated(e)))
}

/// Apply to a stable, uncropped local layer, then transform into the render tile. Panning,
/// rotation and band boundaries must not change the image used by the Poisson solver.
pub fn apply(ctx: &crate::draw::Ctx<'_, '_>, id: NodeId, layer: &mut Pixmap, local_to_layer: Affine, params: &PaperLiquidMetal, effect_index: usize) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // The canvas sets magnified_image_pixels for its live previews. Bound the shader
        // there; still exports and prototype renders keep the higher working resolution.
        let max_side = if ctx.magnified_image_pixels { 512.0 } else { 1024.0 };
        if let Some((source, bounds, scale)) = source(ctx.doc, id, effect_index, max_side, ctx.time) {
            match gpu::render(&source, params, ctx.time) {
                Ok(output) => {
                    layer.data_mut().fill(0);
                    let xf = local_to_layer * Affine::translate((bounds.x0, bounds.y0)) * Affine::scale(1.0 / scale);
                    layer.draw_pixmap(0, 0, output.as_ref(), &PixmapPaint::default(), crate::draw::ts_transform(xf), None);
                }
                Err(e) => log::error!("Paper Liquid Metal: {e}"),
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (ctx, id, layer, local_to_layer, params, effect_index);
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn source(doc: &Document, id: NodeId, effect_index: usize, max_side: f64, time: f64) -> Option<(Pixmap, Rect, f64)> {
    let mut plain = doc.clone();
    let n = plain.node_mut(id)?;
    n.effects.truncate(effect_index);
    n.is_mask = false;
    n.opacity = 1.0;
    n.blend = figcraft_color::BlendMode::Normal;
    n.parent = None;
    n.transform = Affine::IDENTITY;
    let bounds = crate::draw::paint_bounds(&plain, id)?;
    let largest = bounds.width().max(bounds.height());
    if !(largest.is_finite() && largest > 0.0) {
        return None;
    }
    let scale = (max_side / largest).min(2.0);
    crate::render_node_at(&plain, id, scale, time).map(|pix| (pix, bounds, scale))
}

/// The original working resolution is 512 on the shorter side. Keep a bounded allocation
/// for unusually long inputs; ordinary photos and logos use the unchanged upstream size.
pub fn preprocess(src: &Pixmap) -> Option<RgbaImage> {
    let (sw, sh) = (src.width(), src.height());
    let input = RgbaImage::from_raw(sw, sh, src.data().to_vec())?;
    let factor = (512.0 / f64::from(sw.min(sh))).min((4_194_304.0 / (f64::from(sw) * f64::from(sh))).sqrt());
    let (w, h) = ((f64::from(sw) * factor).round().max(1.0) as u32, (f64::from(sh) * factor).round().max(1.0) as u32);
    let small = resize(&input, w, h, FilterType::Triangle);
    let (wi, hi) = (w as usize, h as usize);
    let count = wi.checked_mul(hi)?;
    let shape: Vec<bool> = small.pixels().map(|p| p.0[3] != 0).collect();
    let mut red = Vec::new();
    let mut black = Vec::new();
    if wi > 2 && hi > 2 {
        for y in 1..hi - 1 {
            for x in 1..wi - 1 {
                let i = y * wi + x;
                if shape.get(i).copied().unwrap_or(false)
                    && [i - 1, i + 1, i - wi, i + wi, i - wi - 1, i - wi + 1, i + wi - 1, i + wi + 1]
                        .iter()
                        .all(|j| shape.get(*j).copied().unwrap_or(false))
                {
                    if (x + y) % 2 == 0 {
                        red.push(i);
                    } else {
                        black.push(i);
                    }
                }
            }
        }
    }
    // Float32 storage, f64 intermediates, red then black, omega=1.9, C=0.01, 40 iterations:
    // the same order and numeric storage as solvePoissonSparse in Paper's original code.
    let mut u = vec![0.0_f32; count];
    for _ in 0..40 {
        for pass in [&red, &black] {
            for &i in pass {
                let sum = [i + 1, i - 1, i - wi, i + wi].iter().filter_map(|j| u.get(*j)).map(|v| f64::from(*v)).sum::<f64>();
                let old = u.get(i).copied().unwrap_or(0.0);
                if let Some(value) = u.get_mut(i) {
                    *value = (1.9 * ((0.01 + sum) / 4.0) - 0.9 * f64::from(old)) as f32;
                }
            }
        }
    }
    let max = red.iter().chain(&black).filter_map(|i| u.get(*i)).copied().fold(0.0_f32, f32::max);
    let mut gradient = RgbaImage::new(w, h);
    for (i, p) in gradient.pixels_mut().enumerate() {
        if shape.get(i).copied().unwrap_or(false) {
            let g = if max > 0.0 {
                (255.0 * (1.0 - f64::from(u.get(i).copied().unwrap_or(0.0)) / f64::from(max))).round().clamp(0.0, 255.0) as u8
            } else {
                0
            };
            p.0 = [g, g, g, 255];
        } else {
            p.0 = [255, 255, 255, 0];
        }
    }
    // Canvas interpolation runs on premultiplied colours. Match that before recovering
    // the straight red channel; transparent white must not brighten the silhouette edge.
    for p in gradient.pixels_mut() {
        if p.0[3] == 0 {
            p.0[0] = 0;
            p.0[1] = 0;
            p.0[2] = 0;
        }
    }
    let mut out = resize(&gradient, sw, sh, FilterType::CatmullRom);
    for (p, original) in out.pixels_mut().zip(input.pixels()) {
        let alpha = original.0[3];
        let gradient = if alpha == 0 {
            255
        } else if p.0[3] == 0 {
            0
        } else {
            (f64::from(p.0[0]) * 255.0 / f64::from(p.0[3])).round().clamp(0.0, 255.0) as u8
        };
        p.0 = [gradient, alpha, 255, 255];
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_effect_animates_and_panning_keeps_its_coordinates() {
        use figcraft_color::{Color, Paint};
        use figcraft_doc::{Effect, NodeKind};
        let mut d = Document::new("Paper test");
        let page = d.first_page().unwrap();
        let id = d.create(page, NodeKind::Rectangle, "Metal", 20., 30., 96., 96.).unwrap();
        let n = d.get_mut(id).unwrap();
        n.fills = vec![Paint::solid(Color::WHITE)];
        n.effects.push(Effect { kind: EffectKind::PaperLiquidMetal(PaperLiquidMetal::default()), visible: true });
        let view = crate::View { origin: (0., 0.), scale: 1., width: 160, height: 160 };
        let first = crate::render_view(&d, page, &view, &crate::Options::default()).unwrap();
        let later = crate::render_view(&d, page, &view, &crate::Options { time: 1., ..Default::default() }).unwrap();
        assert_ne!(first.data(), later.data());
        let pan = crate::View { origin: (10., 10.), ..view };
        let panned = crate::render_view(&d, page, &pan, &crate::Options::default()).unwrap();
        for y in 40..100 {
            for x in 30..100 {
                let a = ((y * 160 + x) * 4) as usize;
                let b = (((y - 10) * 160 + x - 10) * 4) as usize;
                assert_eq!(&first.data()[a..a + 4], &panned.data()[b..b + 4]);
            }
        }
    }
    #[test]
    fn preprocessing_preserves_input_alpha_and_empty_inputs_are_finite() {
        let mut p = Pixmap::new(8, 8).unwrap();
        for px in p.data_mut().chunks_exact_mut(4) {
            px.copy_from_slice(&[0, 0, 0, 0]);
        }
        let empty = preprocess(&p).unwrap();
        assert!(empty.pixels().all(|p| p.0 == [255, 0, 255, 255]));
        for y in 2..6 {
            for x in 2..6 {
                p.data_mut()[(y * 8 + x) * 4 + 3] = 128;
            }
        }
        let out = preprocess(&p).unwrap();
        for (px, src) in out.pixels().zip(p.data().chunks_exact(4)) {
            assert_eq!(px.0[1], src[3]);
            assert_eq!(px.0[3], 255);
        }
        let mut solid = Pixmap::new(512, 512).unwrap();
        solid.fill(tiny_skia::Color::WHITE);
        let solid = preprocess(&solid).unwrap();
        assert!(solid.get_pixel(256, 256).0[0] < solid.get_pixel(0, 0).0[0]);
    }
}
