//! Manual integration probe: cargo run -p figcraft --example birefnet_probe -- INPUT CACHE OUTPUT
#[path = "../src/background_removal.rs"]
mod background_removal;

fn main() -> Result<(), String> {
    env_logger::init();
    let args: Vec<_> = std::env::args().collect();
    let input = args.get(1).ok_or("input image required")?;
    let cache = args.get(2).ok_or("cache folder required")?;
    let output = args.get(3).ok_or("output PNG required")?;
    let bytes = std::fs::read(input).map_err(|e| e.to_string())?;
    let bytes = std::sync::Arc::new(bytes);
    let repeat = args.get(4).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1).min(3);
    for _ in 0..repeat {
        let job = background_removal::start(bytes.clone(), cache.into(), None)?;
        let start = std::time::Instant::now();
        loop {
            match job.events.recv().map_err(|e| e.to_string())? {
                figcraft_ui_egui::BackgroundEvent::Progress(s) => println!("{:.1}s {s}", start.elapsed().as_secs_f32()),
                figcraft_ui_egui::BackgroundEvent::Finished(result) => {
                    let result = result?;
                    std::fs::write(output, result.png).map_err(|e| e.to_string())?;
                    std::fs::write(format!("{output}.mask.png"), result.mask).map_err(|e| e.to_string())?;
                    println!("Complete: {} in {:.1}s", result.backend, start.elapsed().as_secs_f32());
                    break;
                }
            }
        }
    }
    Ok(())
}
