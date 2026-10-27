//! Export the actual Paper material as an animated GIF, from a transparent PNG.
//! cargo run -p figcraft-render --example paper_metal_gif -- input.png output.gif
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use figcraft_doc::PaperLiquidMetal;
    use image::{
        Delay, Frame, RgbaImage,
        codecs::gif::{GifEncoder, Repeat},
    };
    let args: Vec<String> = std::env::args().collect();
    let input = args.get(1).ok_or("input PNG path required")?;
    let output = args.get(2).ok_or("output GIF path required")?;
    let mut reader = image::ImageReader::open(input)?.with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let input = reader.decode()?;
    let input = input.thumbnail(input.width().min(768), input.height().min(768)).to_rgba8();
    let mut source = figcraft_render::Pixmap::new(input.width(), input.height()).ok_or("could not allocate image")?;
    for (dst, p) in source.data_mut().chunks_exact_mut(4).zip(input.pixels()) {
        let a = p.0[3];
        dst.copy_from_slice(&[a, a, a, a]);
    }
    let params = PaperLiquidMetal { color_back: figcraft_color::Color::new(0.04, 0.04, 0.06, 1.0), speed: 0.75, ..Default::default() };
    let mut encoder = GifEncoder::new_with_speed(std::fs::File::create(output)?, 10);
    encoder.set_repeat(Repeat::Infinite)?;
    for frame in 0..72 {
        let pix = figcraft_render::paper_metal::render_image(&source, &params, f64::from(frame) / 24.0)?;
        let mut data = pix.data().to_vec();
        for p in data.chunks_exact_mut(4) {
            let alpha = p.get(3).copied().unwrap_or(0);
            if alpha > 0 {
                for c in p.iter_mut().take(3) {
                    *c = (f64::from(*c) * 255.0 / f64::from(alpha)).round().clamp(0.0, 255.0) as u8;
                }
            }
        }
        let rgba = RgbaImage::from_raw(pix.width(), pix.height(), data).ok_or("invalid pixel size")?;
        // GIF stores 10ms ticks: distribute them to keep exactly 3s / 72 frames.
        let delay = if frame % 6 == 5 { 50 } else { 40 };
        encoder.encode_frame(Frame::from_parts(rgba, 0, 0, Delay::from_numer_denom_ms(delay, 1)))?;
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {
    eprintln!("Paper GIF export requires the native GPU build");
}
