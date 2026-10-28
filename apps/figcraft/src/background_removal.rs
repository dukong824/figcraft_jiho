//! Pinned, local BiRefNet inference. Network requests fetch public model/runtime bytes only.
use figcraft_ui_egui::{BackgroundEvent, BackgroundJob, BackgroundResult};
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::AtomicBool, mpsc};

pub fn start(bytes: Arc<Vec<u8>>, dir: PathBuf, ctx: Option<egui::Context>) -> Result<BackgroundJob, String> {
    let (tx, events) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    std::thread::Builder::new()
        .name("birefnet".into())
        .spawn(move || {
            let send = |event| {
                let _ = tx.send(event);
                if let Some(ctx) = &ctx {
                    ctx.request_repaint();
                }
            };
            let progress = |s| send(BackgroundEvent::Progress(s));
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| process(&bytes, &dir, &flag, &progress)))
                .unwrap_or_else(|_| Err("BiRefNet runtime failed; the original image was preserved.".into()));
            send(BackgroundEvent::Finished(result));
            #[cfg(windows)]
            release_idle_model();
        })
        .map_err(|e| e.to_string())?;
    Ok(BackgroundJob { events, cancel })
}

#[cfg(windows)]
fn check(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(std::sync::atomic::Ordering::Relaxed) { Err("Cancelled".into()) } else { Ok(()) }
}

#[cfg(not(windows))]
fn process(_: &[u8], _: &Path, _: &AtomicBool, _: &dyn Fn(String)) -> Result<BackgroundResult, String> {
    Err("BiRefNet is currently available in the Windows desktop build.".into())
}

#[cfg(windows)]
fn process(bytes: &[u8], dir: &Path, cancel: &AtomicBool, progress: &dyn Fn(String)) -> Result<BackgroundResult, String> {
    use image::{ImageDecoder, ImageEncoder};
    use ort::session::Session;
    const SIDE: u32 = 1024;
    // This waits on the worker, never the UI. A job started during idle GPU cleanup queues safely.
    let mut cache = MODEL.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    check(cancel)?;
    let err = |e: &dyn std::fmt::Display| e.to_string();
    // Reject animated input rather than silently discarding its frames.
    if animated(bytes) {
        return Err("BiRefNet supports still images. Convert the animation to a still image first.".into());
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().map_err(|e| err(&e))?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(256 << 20);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(|e| err(&e))?;
    if decoder.total_bytes() > 256 << 20 {
        return Err("Image is too large (maximum 256 MB decoded).".into());
    }
    if decoder.color_type().bits_per_pixel() > u16::from(decoder.color_type().channel_count()) * 8 {
        return Err("BiRefNet currently supports 8-bit images. The original high-bit-depth image was preserved.".into());
    }
    let profile = decoder.icc_profile().map_err(|e| err(&e))?;
    let mut rgba = image::DynamicImage::from_decoder(decoder).map_err(|e| err(&e))?.to_rgba8();
    let (w, h) = rgba.dimensions();
    check(cancel)?;
    let cached = cache.take().filter(|loaded| loaded.dir == dir);
    let (mut session, mut backend) = if let Some(loaded) = cached {
        progress(format!("BiRefNet ready · {}…", loaded.backend));
        (loaded.session, loaded.backend)
    } else {
        std::fs::create_dir_all(dir).map_err(|e| err(&e))?;
        prepare_runtime(dir, cancel, progress)?;
        let model = fetch(
            dir,
            "BiRefNet-general-fp16.onnx",
            "https://huggingface.co/onnx-community/BiRefNet-ONNX/resolve/534d3c82d3bb8b2f0867db6dfbc3a525b8e42f67/onnx/model_fp16.onnx",
            489_666_272,
            "3654c741eb80bd926ada8fed1713b506ccf8d30eb1f6487e87eb9f234f33df09",
            cancel,
            progress,
        )?;
        check(cancel)?;
        progress("Loading BiRefNet · GPU…".into());
        ort::init_from(dir.join("onnxruntime.dll").to_string_lossy())
            .with_name("figcraft-birefnet")
            .with_telemetry(false)
            .commit()
            .map_err(|e| err(&e))?;
        let builder = || -> ort::Result<_> {
            Session::builder()?
                .with_intra_threads(4)?
                .with_parallel_execution(false)?
                .with_memory_pattern(false)?
                // Large fused BiRefNet partitions can exceed VRAM or Windows' GPU timeout.
                // Dispatch individual operators without changing model resolution or weights.
                .with_config_entry("ep.dml.disable_graph_fusion", "1")
        };
        let gpu = builder()
            .and_then(|b| b.with_execution_providers([ort::execution_providers::DirectMLExecutionProvider::default().build().error_on_failure()]))
            .and_then(|b| b.commit_from_file(&model));
        match gpu {
            Ok(s) => (s, "DirectML GPU"),
            Err(e) => {
                log::warn!("BiRefNet DirectML unavailable: {e}");
                progress("GPU unavailable · Loading BiRefNet on CPU…".into());
                (cpu_session(dir, cancel, progress)?, "CPU")
            }
        }
    };
    check(cancel)?;
    let rgb = image::DynamicImage::ImageRgba8(rgba.clone()).to_rgb8();
    let resized = image::imageops::resize(&rgb, SIDE, SIDE, image::imageops::FilterType::Triangle);
    let plane = (SIDE * SIDE) as usize;
    let mut input = vec![0_f32; 3 * plane];
    for (i, p) in resized.pixels().enumerate() {
        for (channel, (mean, std)) in [(0.485, 0.229), (0.456, 0.224), (0.406, 0.225)].into_iter().enumerate() {
            if let Some(v) = input.get_mut(channel * plane + i) {
                *v = (f32::from(p.0.get(channel).copied().unwrap_or(0)) / 255.0 - mean) / std;
            }
        }
    }
    progress(format!("Removing background · {backend}…"));
    let values = match infer(&mut session, &input) {
        Ok(values) => values,
        Err(e) if backend == "DirectML GPU" => {
            log::warn!("BiRefNet GPU inference failed: {e}");
            check(cancel)?;
            drop(session);
            progress("GPU inference failed · Retrying on CPU…".into());
            session = cpu_session(dir, cancel, progress)?;
            backend = "CPU";
            infer(&mut session, &input)?
        }
        Err(e) => return Err(e),
    };
    *cache = Some(Loaded { session, backend, dir: dir.to_path_buf(), used: std::time::Instant::now() });
    check(cancel)?;
    let alpha: Vec<u8> = values.iter().map(|v| ((1.0 / (1.0 + (-v).exp())) * 255.0).round().clamp(0.0, 255.0) as u8).collect();
    let mask = image::GrayImage::from_raw(SIDE, SIDE, alpha).ok_or("Invalid BiRefNet mask")?;
    progress("Saving original-size soft mask…".into());
    let mask = image::imageops::resize(&mask, w, h, image::imageops::FilterType::Triangle);
    // Keep every original RGB byte, including transparent pixels. Only multiply the alpha.
    for (pixel, alpha) in rgba.pixels_mut().zip(mask.pixels()) {
        if let Some(a) = pixel.0.get_mut(3) {
            *a = ((u16::from(*a) * u16::from(alpha.0[0]) + 127) / 255) as u8;
        }
    }
    let mut png = Vec::new();
    let mut encoder = image::codecs::png::PngEncoder::new(&mut png);
    if let Some(profile) = profile {
        encoder.set_icc_profile(profile).map_err(|e| err(&e))?;
    }
    encoder.write_image(rgba.as_raw(), w, h, image::ExtendedColorType::Rgba8).map_err(|e| err(&e))?;
    let mut mask_png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut mask_png).write_image(mask.as_raw(), w, h, image::ExtendedColorType::L8).map_err(|e| err(&e))?;
    check(cancel)?;
    Ok(BackgroundResult { png, mask: mask_png, backend: backend.into() })
}

#[cfg(windows)]
struct Loaded {
    session: ort::session::Session,
    backend: &'static str,
    dir: PathBuf,
    used: std::time::Instant,
}

#[cfg(windows)]
static MODEL: std::sync::Mutex<Option<Loaded>> = std::sync::Mutex::new(None);

#[cfg(windows)]
fn release_idle_model() {
    let _ = std::thread::Builder::new().name("birefnet-idle".into()).spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(20));
        if let Ok(mut cache) = MODEL.try_lock()
            && cache.as_ref().is_some_and(|loaded| loaded.used.elapsed().as_secs() >= 20)
        {
            *cache = None;
        }
    });
}

#[cfg(windows)]
fn animated(bytes: &[u8]) -> bool {
    if bytes.starts_with(b"GIF8") {
        return true;
    }
    let png = bytes.starts_with(b"\x89PNG\r\n\x1a\n");
    let webp = bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP");
    if !png && !webp {
        return false;
    }
    let mut offset = if png { 8_usize } else { 12 };
    while let Some(header) = offset.checked_add(8).and_then(|end| bytes.get(offset..end)) {
        let (name, length) = if png {
            (header.get(4..8), u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize)
        } else {
            (header.get(..4), u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize)
        };
        if name == Some(if png { b"acTL" } else { b"ANIM" }) {
            return true;
        }
        let overhead = if png { 12 } else { 8 + length % 2 };
        let Some(next) = offset.checked_add(length).and_then(|n| n.checked_add(overhead)) else { break };
        offset = next;
    }
    false
}

#[cfg(windows)]
fn cpu_session(dir: &Path, cancel: &AtomicBool, progress: &dyn Fn(String)) -> Result<ort::session::Session, String> {
    let model = fetch(
        dir,
        "BiRefNet-general-epoch_244.onnx",
        "https://github.com/ZhengPeng7/BiRefNet/releases/download/v1/BiRefNet-general-epoch_244.onnx",
        972_666_916,
        "58f621f00f5d756097615970a88a791584600dcf7c45b18a0a6267535a1ebd3c",
        cancel,
        progress,
    )?;
    check(cancel)?;
    progress("Loading BiRefNet · CPU…".into());
    ort::session::Session::builder().and_then(|b| b.with_intra_threads(4)).and_then(|b| b.commit_from_file(model)).map_err(|e| e.to_string())
}

#[cfg(windows)]
fn infer(session: &mut ort::session::Session, input: &[f32]) -> Result<Vec<f32>, String> {
    let input = ort::value::Tensor::from_array(([1_usize, 3, 1024, 1024], input.to_vec())).map_err(|e| e.to_string())?;
    let outputs = session.run(ort::inputs![input]).map_err(|e| e.to_string())?;
    let output = outputs.iter().last().ok_or("BiRefNet returned no mask")?.1;
    let (shape, values) = output.try_extract_tensor::<f32>().map_err(|e| e.to_string())?;
    if shape.as_ref() != [1, 1, 1024, 1024] || values.len() != 1024 * 1024 || !values.iter().all(|v| v.is_finite()) {
        return Err("Invalid BiRefNet mask dimensions or values".into());
    }
    Ok(values.to_vec())
}

#[cfg(windows)]
fn fetch(dir: &Path, name: &str, url: &str, size: u64, sha: &str, cancel: &AtomicBool, progress: &dyn Fn(String)) -> Result<PathBuf, String> {
    use sha2::{Digest, Sha256};
    use std::io::{Read, Write};
    let path = dir.join(name);
    let verified = |p: &Path| -> Result<bool, String> {
        let Ok(file) = std::fs::File::open(p) else { return Ok(false) };
        if file.metadata().map_err(|e| e.to_string())?.len() != size {
            return Ok(false);
        }
        let mut reader = std::io::BufReader::new(file);
        let mut hash = Sha256::new();
        let mut buf = vec![0_u8; 1 << 20];
        loop {
            check(cancel)?;
            let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            hash.update(&buf[..n]);
        }
        Ok(format!("{:x}", hash.finalize()) == sha)
    };
    progress(format!("Checking {name}…"));
    if verified(&path)? {
        return Ok(path);
    }
    let part = path.with_extension("part");
    // The process-wide guard serializes jobs (and their temporary cache files) across windows.
    let config = ureq::Agent::config_builder()
        .tls_config(ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build())
        .timeout_global(Some(std::time::Duration::from_secs(3600)))
        .timeout_connect(Some(std::time::Duration::from_secs(15)))
        .timeout_per_read(Some(std::time::Duration::from_secs(30)))
        .build();
    let mut response = ureq::Agent::new_with_config(config)
        .get(url)
        .call()
        .map_err(|e| format!("Download failed: {e}. Check the internet connection and retry."))?;
    let mut input = response.body_mut().as_reader();
    let mut file = std::fs::File::create(&part).map_err(|e| e.to_string())?;
    let mut total = 0_u64;
    let mut last_mb = u64::MAX;
    let mut hash = Sha256::new();
    let mut buf = vec![0_u8; 1 << 20];
    loop {
        check(cancel)?;
        let n = input.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > size {
            return Err("Download exceeded the pinned file size".into());
        }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        hash.update(&buf[..n]);
        let mb = total / 1_000_000;
        if mb != last_mb {
            progress(format!("Downloading {name} · {mb}/{} MB", size / 1_000_000));
            last_mb = mb;
        }
    }
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    if total != size || format!("{:x}", hash.finalize()) != sha {
        let _ = std::fs::remove_file(&part);
        return Err("Download checksum mismatch; please retry.".into());
    }
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&part, &path).map_err(|e| e.to_string())?;
    Ok(path)
}

#[cfg(windows)]
fn prepare_runtime(dir: &Path, cancel: &AtomicBool, progress: &dyn Fn(String)) -> Result<(), String> {
    for (name, url, size, sha, entries) in [
        (
            "ort-directml-1.22.0.zip",
            "https://api.nuget.org/v3-flatcontainer/microsoft.ml.onnxruntime.directml/1.22.0/microsoft.ml.onnxruntime.directml.1.22.0.nupkg",
            17_898_472,
            "29f9872d786236b79aa83f94482f3a17c14297e4833768d6d0ed4883ee732e60",
            vec![
                ("runtimes/win-x64/native/onnxruntime.dll", "onnxruntime.dll"),
                ("runtimes/win-x64/native/onnxruntime_providers_shared.dll", "onnxruntime_providers_shared.dll"),
                ("LICENSE", "ONNX-Runtime-LICENSE.txt"),
                ("ThirdPartyNotices.txt", "ONNX-Runtime-ThirdPartyNotices.txt"),
            ],
        ),
        (
            "directml-1.15.4.zip",
            "https://api.nuget.org/v3-flatcontainer/microsoft.ai.directml/1.15.4/microsoft.ai.directml.1.15.4.nupkg",
            202_292_617,
            "4e7cb7ddce8cf837a7a75dc029209b520ca0101470fcdf275c1f49736a3615b9",
            vec![("bin/x64-win/DirectML.dll", "DirectML.dll"), ("LICENSE.txt", "DirectML-LICENSE.txt")],
        ),
    ] {
        let path = fetch(dir, name, url, size, sha, cancel, progress)?;
        let mut zip = zip::ZipArchive::new(std::fs::File::open(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        for (entry, out) in entries {
            check(cancel)?;
            let mut input = zip.by_name(entry).map_err(|e| e.to_string())?;
            if input.size() > 150 << 20 {
                return Err("Runtime entry exceeds size limit".into());
            }
            let target = dir.join(out);
            // Recheck and replace only when changed; the runtime DLL may be loaded by a previous job.
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(&mut input, &mut bytes).map_err(|e| e.to_string())?;
            if std::fs::read(&target).ok().as_deref() != Some(&bytes) {
                std::fs::write(target, bytes).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}
