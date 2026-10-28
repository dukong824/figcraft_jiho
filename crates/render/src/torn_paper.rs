//! Rust port of gentrax/Papierwerk's addTornBorder, exact EDT and smoothNoise.
//! Copyright (c) 2026 gentrax, MIT. See third-party/papierwerk for the pinned source.
//! Adaptations: premultiplied pixels, layer-local noise, bounded cached texture resolution,
//! color alpha, and fallback alpha seeds for uniformly translucent artwork.
use figcraft_doc::{EffectKind, Node, TornPaper};
use figcraft_geom::Affine;
use std::{cell::RefCell, sync::Arc};
use tiny_skia::Pixmap;

pub(crate) fn active(n: &Node) -> bool {
    n.effects.iter().any(|e| e.visible && matches!(&e.kind, EffectKind::TornPaper(p) if p.reach() > 0.0))
}

fn filled<T: Clone>(len: usize, value: T) -> Option<Vec<T>> {
    let mut out = Vec::new();
    out.try_reserve_exact(len).ok()?;
    out.resize(len, value);
    Some(out)
}

struct Random(u32);
impl Random {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6D2B79F5);
        let mut t = (self.0 ^ (self.0 >> 15)).wrapping_mul(1 | self.0);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t));
        f64::from(t ^ (t >> 14)) / 4294967296.0
    }
}

fn smooth_noise(w: usize, h: usize, cell: f64, seed: u32) -> Option<Vec<f32>> {
    let mut rnd = Random(seed);
    let mut total = filled(w.checked_mul(h)?, 0.0_f32)?;
    let (mut amp, mut amp_sum, mut cell) = (1.0, 0.0, cell);
    for _ in 0..3 {
        let c = cell.max(2.0);
        let gw = ((w as f64 / c).round() as usize + 1).max(2);
        let gh = ((h as f64 / c).round() as usize + 1).max(2);
        let mut grid = filled(gw.checked_mul(gh)?, 0.0_f32)?;
        for g in &mut grid {
            *g = rnd.next() as f32;
        }
        let (sx, sy) = ((gw - 1) as f64 / w as f64, (gh - 1) as f64 / h as f64);
        let ease = |t: f64| t * t * (3.0 - 2.0 * t);
        for y in 0..h {
            let fy = y as f64 * sy;
            let y0 = (fy.floor() as usize).min(gh - 2);
            let ty = ease(fy - y0 as f64);
            for x in 0..w {
                let fx = x as f64 * sx;
                let x0 = (fx.floor() as usize).min(gw - 2);
                let tx = ease(fx - x0 as f64);
                let at = |x, y| f64::from(grid.get(y * gw + x).copied().unwrap_or(0.0));
                let (a, b, c, d) = (at(x0, y0), at(x0 + 1, y0), at(x0, y0 + 1), at(x0 + 1, y0 + 1));
                // JS stores each octave in Float32Array before accumulating it.
                let v = ((a + (b - a) * tx) * (1.0 - ty) + (c + (d - c) * tx) * ty) as f32;
                if let Some(t) = total.get_mut(y * w + x) {
                    *t = (f64::from(*t) + f64::from(v) * amp) as f32;
                }
            }
        }
        amp_sum += amp;
        amp *= 0.5;
        cell /= 2.3;
    }
    let mut mean = 0.0;
    for t in &mut total {
        *t = (f64::from(*t) / amp_sum) as f32;
        mean += f64::from(*t);
    }
    mean /= total.len() as f64;
    let mut max = 1e-6_f64;
    for t in &mut total {
        *t = (f64::from(*t) - mean) as f32;
        max = max.max(f64::from(*t).abs());
    }
    for t in &mut total {
        *t = (f64::from(*t) / max) as f32;
    }
    Some(total)
}

struct Noise {
    w: usize,
    h: usize,
    ratio: f64,
    pad: f64,
    coarse: Vec<f32>,
    fine: Vec<f32>,
}
type NoiseKey = (u64, u64, u64, u32);
thread_local! { static NOISE: RefCell<Option<(NoiseKey, Arc<Noise>)>> = const { RefCell::new(None) }; }
fn noise(size: (f64, f64), p: &TornPaper) -> Option<Arc<Noise>> {
    let key = (size.0.to_bits(), size.1.to_bits(), p.width.to_bits(), p.seed);
    if let Some(cached) = NOISE.with(|cache| cache.borrow().as_ref().filter(|(k, _)| *k == key).map(|(_, n)| n.clone())) {
        return Some(cached);
    }
    let pad = (p.width * 2.2).round() + 6.0;
    let (sw, sh) = (size.0.ceil().max(1.0) + pad * 2.0, size.1.ceil().max(1.0) + pad * 2.0);
    if !sw.is_finite() || !sh.is_finite() {
        return None;
    }
    // Only border noise is sampled at this working resolution. Source RGB is never resampled.
    let ratio = (2048.0 / sw.max(sh)).min(1.0);
    let (w, h) = ((sw * ratio).ceil().max(2.0) as usize, (sh * ratio).ceil().max(2.0) as usize);
    let n = Arc::new(Noise {
        w,
        h,
        ratio,
        pad,
        coarse: smooth_noise(w, h, (18.0_f64.max(p.width * 2.2)) * ratio, p.seed)?,
        fine: smooth_noise(w, h, (6.0_f64.max(p.width * 0.6)) * ratio, p.seed.wrapping_add(7))?,
    });
    NOISE.with(|cache| *cache.borrow_mut() = Some((key, n.clone())));
    Some(n)
}
impl Noise {
    fn at(&self, data: &[f32], x: f64, y: f64) -> f64 {
        let x = (x * self.ratio).floor().clamp(0.0, (self.w - 1) as f64) as usize;
        let y = (y * self.ratio).floor().clamp(0.0, (self.h - 1) as f64) as usize;
        f64::from(data.get(y * self.w + x).copied().unwrap_or(0.0))
    }
}

/// Exact separable squared Euclidean distance transform, from Papierwerk's edt1d.
fn edt(f: &[f64], d: &mut [f64], v: &mut [usize], z: &mut [f64]) {
    let n = f.len();
    if n == 0 {
        return;
    }
    let mut k = 0;
    v[0] = 0;
    z[0] = f64::NEG_INFINITY;
    z[1] = f64::INFINITY;
    for q in 1..n {
        let cross = |p: usize| ((f[q] + (q * q) as f64) - (f[p] + (p * p) as f64)) / (2.0 * (q - p) as f64);
        let mut s = cross(v[k]);
        while k > 0 && s <= z[k] {
            k -= 1;
            s = cross(v[k]);
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = f64::INFINITY;
    }
    k = 0;
    for q in 0..n {
        while z[k + 1] < q as f64 {
            k += 1;
        }
        let dq = q as f64 - v[k] as f64;
        d[q] = dq * dq + f[v[k]];
    }
}

fn distance_outside(layer: &Pixmap) -> Option<Vec<u32>> {
    let (w, h) = (layer.width() as usize, layer.height() as usize);
    let max_alpha = layer.data().chunks_exact(4).map(|p| p[3]).max().unwrap_or(0);
    if max_alpha == 0 {
        return None;
    }
    let threshold = if max_alpha > 110 { 110 } else { max_alpha / 2 };
    const INF: u32 = 1_000_000_000;
    let mut sq = filled(w.checked_mul(h)?, INF)?;
    for (s, p) in sq.iter_mut().zip(layer.data().chunks_exact(4)) {
        if p[3] > threshold {
            *s = 0;
        }
    }
    let n = w.max(h);
    let (mut f, mut d, mut v, mut z) = (filled(n, 0.0)?, filled(n, 0.0)?, filled(n, 0_usize)?, filled(n + 1, 0.0)?);
    for x in 0..w {
        for y in 0..h {
            f[y] = f64::from(sq[y * w + x]);
        }
        edt(&f[..h], &mut d[..h], &mut v, &mut z);
        for y in 0..h {
            sq[y * w + x] = d[y] as u32;
        }
    }
    for y in 0..h {
        for x in 0..w {
            f[x] = f64::from(sq[y * w + x]);
        }
        edt(&f[..w], &mut d[..w], &mut v, &mut z);
        for x in 0..w {
            sq[y * w + x] = d[x] as u32;
        }
    }
    Some(sq)
}

pub(crate) fn apply(layer: &mut Pixmap, p: &TornPaper, xf: Affine, size: (f64, f64)) {
    let p = p.sanitized();
    if p.reach() <= 0.0 {
        return;
    }
    let m = xf.inverse().as_coeffs();
    let scale = xf.determinant().abs().sqrt();
    if !m.iter().all(|v| v.is_finite()) || !scale.is_finite() || scale <= 1e-6 {
        return;
    }
    let Some(noise) = noise(size, &p) else { return };
    let Some(dist) = distance_outside(layer) else { return };
    let w = layer.width() as usize;
    for (i, pixel) in layer.data_mut().chunks_exact_mut(4).enumerate() {
        if pixel[3] == 255 {
            continue;
        } // Opaque source pixels are byte-for-byte unchanged.
        let distance = f64::from(dist.get(i).copied().unwrap_or(u32::MAX)).sqrt() / scale;
        if distance > p.width * 2.0 + 1.0 {
            continue;
        }
        let (px, py) = ((i % w) as f64 + 0.5, (i / w) as f64 + 0.5);
        let (x, y) = (m[0] * px + m[2] * py + m[4] + noise.pad, m[1] * px + m[3] * py + m[5] + noise.pad);
        let (qx, qy) = ((x / 3.0).floor() * 3.0, (y / 3.0).floor() * 3.0);
        let nc = noise.at(&noise.coarse, qx, qy);
        let nf = noise.at(&noise.fine, qx, qy);
        let th = (p.width * (1.0 + p.roughness * (0.75 * nc + 0.25 * nf))).clamp(p.width * 0.25, p.width * 2.0);
        let pa = ((th - distance) / 1.3 + 0.5).clamp(0.0, 1.0) * f64::from(p.color.a);
        let sa = f64::from(pixel[3]) / 255.0;
        let pw = pa * (1.0 - sa);
        if pw <= 0.0 {
            continue;
        }
        let tex = 0.94 + 0.06 * noise.at(&noise.fine, x, y);
        let alpha = ((sa + pw) * 255.0).round().clamp(0.0, 255.0);
        for (channel, color) in pixel[..3].iter_mut().zip([p.color.r, p.color.g, p.color.b]) {
            *channel = (f64::from(*channel) + f64::from(color) * tex * pw * 255.0).round().clamp(0.0, alpha) as u8;
        }
        pixel[3] = alpha as u8;
    }
}
