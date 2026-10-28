//! Original torn-paper silhouette, anchored to layer coordinates on CPU and WASM.
//! A two-pass chamfer field erodes existing alpha; it never resamples the layer's interior.
use figcraft_doc::{EffectKind, Node, TornPaper};
use figcraft_geom::Affine;
use tiny_skia::Pixmap;

pub(crate) fn active(n: &Node) -> bool {
    n.effects.iter().any(|e| e.visible && matches!(&e.kind, EffectKind::TornPaper(p) if p.sanitized().depth > 0.0))
}

fn hash(x: i64, y: i64, seed: u32) -> f64 {
    let mut v = (x as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ (y as u64).wrapping_mul(0xC2B2AE3D27D4EB4F) ^ u64::from(seed);
    v = (v ^ (v >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    v = (v ^ (v >> 27)).wrapping_mul(0x94D049BB133111EB);
    f64::from(((v ^ (v >> 31)) & 0xffff) as u16) / 65535.0
}

fn noise(x: f64, y: f64, seed: u32) -> f64 {
    let (ix, iy) = (x.floor(), y.floor());
    let ease = |v: f64| v * v * (3.0 - 2.0 * v);
    let (tx, ty) = (ease(x - ix), ease(y - iy));
    let (ix, iy) = (ix as i64, iy as i64);
    let a = hash(ix, iy, seed);
    let b = hash(ix.saturating_add(1), iy, seed);
    let c = hash(ix, iy.saturating_add(1), seed);
    let d = hash(ix.saturating_add(1), iy.saturating_add(1), seed);
    (a + (b - a) * tx) * (1.0 - ty) + (c + (d - c) * tx) * ty
}

/// Cut the painted silhouette inward, with optional exposed paper along the new edge.
pub(crate) fn apply(layer: &mut Pixmap, p: &TornPaper, local_to_pixels: Affine) {
    let p = p.sanitized();
    if p.depth <= 0.0 {
        return;
    }
    let inv = local_to_pixels.inverse().as_coeffs();
    if !inv.iter().all(|v| v.is_finite()) {
        return;
    }
    let (w, h) = (layer.width() as usize, layer.height() as usize);
    let Some(count) = w.checked_mul(h) else { return };
    // Two bytes per pixel, bounded by the renderer's existing 16k-square surface limit.
    if count > 256 * 1024 * 1024 {
        log::warn!("Torn Paper: surface exceeds the renderer's 16k-square budget");
        return;
    }
    let mut field = Vec::<u16>::new();
    if field.try_reserve_exact(count).is_err() {
        log::warn!("Torn Paper: cannot allocate edge field");
        return;
    }
    const Q: f64 = 256.0;
    let q = |v: f64| (v * Q).ceil().clamp(1.0, 65535.0) as u16;
    let sx = inv[0].hypot(inv[1]);
    let sy = inv[2].hypot(inv[3]);
    let (cx, cy) = (q(sx), q(sy));
    let cp = q((inv[0] + inv[2]).hypot(inv[1] + inv[3]));
    let cm = q((inv[0] - inv[2]).hypot(inv[1] - inv[3]));
    let limit = q(p.depth * 1.2 + p.edge_width + 4.0);
    // Opacity is not geometry: a uniformly translucent image still has a full silhouette.
    field.extend(layer.data().chunks_exact(4).map(|pixel| if pixel[3] == 0 { 0 } else { limit }));
    if field.len() != count {
        return;
    }
    let sample = |f: &[u16], i: usize, cost: u16| f.get(i).copied().unwrap_or(limit).saturating_add(cost);
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let mut d = field.get(i).copied().unwrap_or(limit);
            if x > 0 {
                d = d.min(sample(&field, i - 1, cx));
            }
            if y > 0 {
                d = d.min(sample(&field, i - w, cy));
                if x > 0 {
                    d = d.min(sample(&field, i - w - 1, cp));
                }
                if x + 1 < w {
                    d = d.min(sample(&field, i - w + 1, cm));
                }
            }
            if let Some(slot) = field.get_mut(i) {
                *slot = d;
            }
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            let mut d = field.get(i).copied().unwrap_or(limit);
            if x + 1 < w {
                d = d.min(sample(&field, i + 1, cx));
            }
            if y + 1 < h {
                d = d.min(sample(&field, i + w, cy));
                if x > 0 {
                    d = d.min(sample(&field, i + w - 1, cm));
                }
                if x + 1 < w {
                    d = d.min(sample(&field, i + w + 1, cp));
                }
            }
            if let Some(slot) = field.get_mut(i) {
                *slot = d;
            }
        }
    }
    let aa = sx.min(sy).clamp(1.0 / Q, 4.0);
    let color = [f64::from(p.color.r), f64::from(p.color.g), f64::from(p.color.b)];
    for (i, pixel) in layer.data_mut().chunks_exact_mut(4).enumerate() {
        if pixel[3] == 0 {
            continue;
        }
        let distance = f64::from(field.get(i).copied().unwrap_or(limit)) / Q - 0.5 * aa;
        if distance > p.depth + p.edge_width + 2.0 {
            continue;
        }
        let (px, py) = ((i % w) as f64 + 0.5, (i / w) as f64 + 0.5);
        let x = inv[0] * px + inv[2] * py + inv[4];
        let y = inv[1] * px + inv[3] * py + inv[5];
        let (u, v) = (x / p.size, y / p.size);
        let coarse = noise(u, v, p.seed);
        let detail = noise(u * 3.1, v * 3.1, p.seed.wrapping_add(31));
        let fine = noise(u * 9.7, v * 9.7, p.seed.wrapping_add(73));
        let wave = 0.58 * coarse + 0.28 * detail + 0.14 * fine;
        let fibers = noise(x * 1.7, y * 1.7, p.seed.wrapping_add(113));
        let edge = p.depth * (0.32 + 0.65 * (0.5 + (wave - 0.5) * p.roughness)) + (fibers - 0.5) * p.fibers * 2.0;
        let inside = distance - edge;
        let cover = ((inside + aa * 0.5) / aa).clamp(0.0, 1.0);
        let tint = if p.edge_width > 0.0 { (1.0 - inside.max(0.0) / p.edge_width).clamp(0.0, 1.0) * f64::from(p.color.a) } else { 0.0 };
        let alpha = f64::from(pixel[3]) * cover;
        for (c, tint_color) in color.into_iter().enumerate() {
            if let Some(channel) = pixel.get_mut(c) {
                *channel = (f64::from(*channel) * cover * (1.0 - tint) + tint_color * alpha * tint).round().clamp(0.0, alpha) as u8;
            }
        }
        if let Some(a) = pixel.get_mut(3) {
            *a = alpha.round().clamp(0.0, 255.0) as u8;
        }
    }
}
