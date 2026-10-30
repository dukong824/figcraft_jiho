use super::model::AssetKind;
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub fn tool(name: &str) -> PathBuf {
    if name == "ffmpeg"
        && let Some(path) = std::env::var_os("PURPOSE_FFMPEG")
    {
        return path.into();
    }
    let filename = if cfg!(windows) { format!("{name}.exe") } else { name.to_owned() };
    if let Ok(exe) = std::env::current_exe()
        && let Some(directory) = exe.parent()
    {
        let bundled = directory.join("ffmpeg").join("bin").join(&filename);
        if bundled.is_file() {
            return bundled;
        }
    }
    filename.into()
}

#[cfg(test)]
pub fn probe(path: &Path) -> Result<(AssetKind, f64, bool), String> {
    probe_cancellable(path, &AtomicBool::new(false))
}
fn command(name: &str) -> Command {
    let mut command = Command::new(tool(name));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command.stdin(Stdio::null());
    command
}
pub fn probe_cancellable(path: &Path, cancel: &AtomicBool) -> Result<(AssetKind, f64, bool), String> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    if extension == "figcraft" {
        return Ok((AssetKind::Design, 0.0, false));
    }
    let mut child = command("ffprobe")
        .args(["-v", "error", "-show_entries", "format=duration:stream=codec_type", "-of", "json"])
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("ffprobe를 실행할 수 없습니다: {e}"))?;
    let stdout = child.stdout.take().map(|pipe| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe.take(2 * 1024 * 1024).read_to_end(&mut bytes);
            bytes
        })
    });
    let stderr = child.stderr.take().map(|pipe| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe.take(64 * 1024).read_to_end(&mut bytes);
            bytes
        })
    });
    let deadline = Instant::now() + Duration::from_secs(60);
    let result = loop {
        if cancel.load(Ordering::Relaxed) || Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            break Err(if cancel.load(Ordering::Relaxed) {
                "가져오기를 취소했습니다".into()
            } else {
                "파일 분석 시간이 초과되었습니다".into()
            });
        }
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(error.to_string());
            }
        }
    };
    let stdout = stdout.and_then(|worker| worker.join().ok()).unwrap_or_default();
    let stderr = stderr.and_then(|worker| worker.join().ok()).unwrap_or_default();
    if !result?.success() {
        return Err(format!("미디어를 읽을 수 없습니다: {}", String::from_utf8_lossy(&stderr)));
    }
    let info: Value = serde_json::from_slice(&stdout).map_err(|e| e.to_string())?;
    let streams = info["streams"].as_array().ok_or("미디어 스트림이 없습니다")?;
    let video = streams.iter().any(|s| s["codec_type"] == "video");
    let audio = streams.iter().any(|s| s["codec_type"] == "audio");
    let kind = if video {
        if matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tif" | "tiff") { AssetKind::Image } else { AssetKind::Video }
    } else if audio {
        AssetKind::Audio
    } else {
        return Err("지원하는 영상·오디오·이미지 스트림이 없습니다".into());
    };
    let duration = info["format"]["duration"].as_str().and_then(|s| s.parse::<f64>().ok()).filter(|n| n.is_finite() && *n > 0.0).unwrap_or(5.0);
    Ok((kind, duration, audio))
}

pub fn preview_png(path: &Path, seconds: f64) -> Result<Vec<u8>, String> {
    let output = command("ffmpeg")
        .args(["-v", "error", "-nostdin", "-ss"])
        .arg(format!("{:.3}", seconds.max(0.0)))
        .arg("-i")
        .arg(path)
        .args(["-frames:v", "1", "-vf", "scale=960:-2", "-f", "image2pipe", "-vcodec", "png", "-"])
        .output()
        .map_err(|e| format!("ffmpeg를 실행할 수 없습니다: {e}"))?;
    if !output.status.success() || output.stdout.is_empty() {
        return Err(format!("미리보기를 만들 수 없습니다: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(output.stdout)
}
