//! The seven Apache-2.0 Paper Image Filters. Native GLSL, serialized parameters and animation.
use figcraft_doc::{NodeId, PaperImageFilter};
use figcraft_geom::Affine;
use tiny_skia::Pixmap;
#[cfg(not(target_arch = "wasm32"))]
mod gpu;

#[cfg(not(target_arch = "wasm32"))]
pub fn render_image(source: &Pixmap, params: &PaperImageFilter, time: f64) -> Result<Pixmap, String> {
    gpu::render(source, params, time)
}

pub fn apply(ctx: &crate::draw::Ctx<'_, '_>, id: NodeId, layer: &mut Pixmap, local_to_layer: Affine, params: &PaperImageFilter, effect_index: usize) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let max_side = if ctx.magnified_image_pixels { 512.0 } else { 1024.0 };
        if let Some((source, bounds, scale)) = crate::paper_metal::source(ctx.doc, id, effect_index, max_side, ctx.time) {
            match render_image(&source, params, ctx.time) {
                Ok(output) => {
                    layer.data_mut().fill(0);
                    let xf = local_to_layer * Affine::translate((bounds.x0, bounds.y0)) * Affine::scale(1.0 / scale);
                    layer.draw_pixmap(0, 0, output.as_ref(), &tiny_skia::PixmapPaint::default(), crate::draw::ts_transform(xf), None);
                }
                Err(error) => log::error!("Paper {}: {error}", params.filter.label()),
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (ctx, id, layer, local_to_layer, params, effect_index);
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use figcraft_doc::PaperFilterKind;

    #[allow(clippy::unwrap_used)]
    fn sample() -> Pixmap {
        let mut source = Pixmap::new(48, 40).unwrap();
        for (i, pixel) in source.data_mut().chunks_exact_mut(4).enumerate() {
            let x = i % 48;
            let y = i / 48;
            pixel.copy_from_slice(&[(x * 4) as u8, (y * 5) as u8, 70, 255]);
        }
        source
    }

    #[test]
    fn all_seven_original_image_filters_compile_and_transform_rgb() {
        let source = sample();
        for kind in PaperFilterKind::ALL {
            let params = PaperImageFilter::new(kind);
            let output = render_image(&source, &params, 0.0).unwrap_or_else(|error| panic!("{kind:?}: {error}"));
            assert_eq!((output.width(), output.height()), (48, 40));
            assert_ne!(output.data(), source.data(), "{kind:?} must change the source");
            assert!(output.data().chunks_exact(4).any(|pixel| pixel[3] > 0), "{kind:?} is visible");
        }
    }

    #[test]
    fn water_animates_but_speed_zero_and_still_filters_are_deterministic() {
        let source = sample();
        let mut params = PaperImageFilter::new(PaperFilterKind::Water);
        let first = render_image(&source, &params, 0.0).unwrap();
        let later = render_image(&source, &params, 1.0).unwrap();
        assert_ne!(first.data(), later.data());
        params.speed = 0.0;
        assert_eq!(render_image(&source, &params, 0.0).unwrap().data(), render_image(&source, &params, 4.0).unwrap().data());
        let still = PaperImageFilter::new(PaperFilterKind::PaperTexture);
        assert_eq!(render_image(&source, &still, 0.0).unwrap().data(), render_image(&source, &still, 4.0).unwrap().data());
    }

    #[test]
    fn neutral_filter_keeps_orientation_alpha_and_refreshes_when_rgb_changes() {
        let mut source = sample();
        for pixel in source.data_mut().chunks_exact_mut(4) {
            for channel in pixel.iter_mut().take(3) {
                *channel /= 2;
            }
            pixel[3] = 128;
        }
        let mut params = PaperImageFilter::new(PaperFilterKind::FlutedGlass);
        for name in ["distortion", "shadows", "highlights", "edges", "blur", "stretch"] {
            params.set_value(name, 0.0);
        }
        let first = render_image(&source, &params, 0.0).unwrap();
        // Paper intentionally antialiases the image frame, even with edge distortion off.
        for y in 4..36 {
            for x in 4..44 {
                let offset = (y * 48 + x) * 4;
                for (a, b) in first.data()[offset..offset + 4].iter().zip(&source.data()[offset..offset + 4]) {
                    assert!(a.abs_diff(*b) <= 2, "neutral interior differs at {x},{y}: {a} vs {b}");
                }
            }
        }
        for pixel in source.data_mut().chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
        let changed = render_image(&source, &params, 0.0).unwrap();
        assert_ne!(first.data(), changed.data(), "RGB changes invalidate the upload even when alpha is unchanged");
        assert!(render_image(&Pixmap::new(2049, 1).unwrap(), &params, 0.0).is_err());
    }
}
