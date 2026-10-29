use super::model::AssetKind;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

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

pub fn probe(path: &Path) -> Result<(AssetKind, f64, bool), String> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    if extension == "figcraft" {
        return Ok((AssetKind::Design, 0.0, false));
    }
    let output = Command::new(tool("ffprobe"))
        .args(["-v", "error", "-show_entries", "format=duration:stream=codec_type", "-of", "json"])
        .arg(path)
        .output()
        .map_err(|e| format!("ffprobe를 실행할 수 없습니다: {e}"))?;
    if !output.status.success() {
        return Err(format!("미디어를 읽을 수 없습니다: {}", String::from_utf8_lossy(&output.stderr)));
    }
    let info: Value = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
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
    let output = Command::new(tool("ffmpeg"))
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
