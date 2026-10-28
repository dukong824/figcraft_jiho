use super::model::{Asset, AssetKind, Clip, Video};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn ffmpeg() -> String {
    std::env::var("PURPOSE_FFMPEG").unwrap_or_else(|_| "ffmpeg".into())
}

pub fn probe(path: &Path) -> Result<(AssetKind, f64, bool), String> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    if extension == "figcraft" {
        return Ok((AssetKind::Design, 0.0, false));
    }
    let output = Command::new("ffprobe")
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
    let output = Command::new(ffmpeg())
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

fn seconds(n: f64) -> String {
    format!("{:.3}", n.max(0.0))
}

pub fn srt_time(n: f64) -> String {
    let ms = (n.max(0.0) * 1000.0).round() as u64;
    format!("{:02}:{:02}:{:02},{:03}", ms / 3_600_000, ms / 60_000 % 60, ms / 1000 % 60, ms % 1000)
}

pub fn export(video: &Video, video_dir: &Path) -> Result<PathBuf, String> {
    let visual: Vec<&Clip> = video.clips.iter().filter(|c| !c.audio_track).collect();
    if visual.is_empty() {
        return Err("영상 트랙에 클립을 추가하세요".into());
    }
    let duration = video.duration();
    if !duration.is_finite() || duration <= 0.0 || duration > 24.0 * 60.0 * 60.0 {
        return Err("영상 길이가 올바르지 않습니다".into());
    }
    let exports = video_dir.join("exports");
    fs::create_dir_all(&exports).map_err(|e| e.to_string())?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs();
    let safe: String = video.name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).take(60).collect();
    let name = format!("{}-{}", if safe.is_empty() { "video" } else { &safe }, stamp);
    let output_path = exports.join(format!("{name}.mp4"));
    let partial_path = exports.join(format!("{name}.partial.mp4"));
    let mut command = Command::new(ffmpeg());
    command.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"]);
    let mut filters = Vec::new();
    let mut video_labels = Vec::new();
    let mut audio_labels = Vec::new();
    let mut input_index = 0;
    let mut timeline = 0.0;
    for clip in &visual {
        let asset = video.asset(clip.asset_id).ok_or("타임라인의 에셋을 찾을 수 없습니다")?;
        let path = checked_asset_path(video_dir, asset)?;
        let length = clip.duration();
        if !length.is_finite() || length < 0.04 {
            return Err("클립 길이는 0.04초 이상이어야 합니다".into());
        }
        if asset.kind == AssetKind::Image {
            command.args(["-loop", "1", "-t"]).arg(seconds(length)).arg("-i").arg(&path);
        } else if asset.kind == AssetKind::Video {
            command.arg("-i").arg(&path);
        } else {
            return Err("영상 트랙에는 이미지나 영상을 넣어야 합니다".into());
        }
        let v = format!("v{}", video_labels.len());
        let trim_start = if asset.kind == AssetKind::Image { 0.0 } else { clip.trim_start };
        filters.push(format!("[{input_index}:v]trim=start={}:duration={},setpts=PTS-STARTPTS,fps=30,scale=1280:720:force_original_aspect_ratio=decrease,pad=1280:720:(ow-iw)/2:(oh-ih)/2,format=yuv420p[{v}]", seconds(trim_start), seconds(length)));
        video_labels.push(format!("[{v}]"));
        if asset.has_audio {
            let a = format!("a{}", audio_labels.len());
            filters.push(audio_filter(input_index, clip, timeline, &a));
            audio_labels.push(format!("[{a}]"));
        }
        timeline += length;
        input_index += 1;
    }
    for clip in video.clips.iter().filter(|c| c.audio_track) {
        let asset = video.asset(clip.asset_id).ok_or("오디오 에셋을 찾을 수 없습니다")?;
        if !asset.has_audio || clip.duration() < 0.04 {
            return Err("오디오 클립이 올바르지 않습니다".into());
        }
        command.arg("-i").arg(checked_asset_path(video_dir, asset)?);
        let a = format!("a{}", audio_labels.len());
        filters.push(audio_filter(input_index, clip, clip.start, &a));
        audio_labels.push(format!("[{a}]"));
        input_index += 1;
    }
    filters.push(format!("{}concat=n={}:v=1:a=0[vout]", video_labels.join(""), video_labels.len()));
    if !audio_labels.is_empty() {
        let mixed = if audio_labels.len() == 1 {
            audio_labels[0].clone()
        } else {
            filters.push(format!("{}amix=inputs={}:duration=longest:normalize=0[amix]", audio_labels.join(""), audio_labels.len()));
            "[amix]".into()
        };
        filters.push(format!("{mixed}atrim=duration={}[aout]", seconds(duration)));
    }
    let captions: Vec<_> = video.captions.iter().filter(|c| !c.text.trim().is_empty() && c.end > c.start).collect();
    if !captions.is_empty() {
        let mut sorted = captions;
        sorted.sort_by(|a, b| a.start.total_cmp(&b.start));
        let body = sorted
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{}\n{} --> {}\n{}\n\n", i + 1, srt_time(c.start), srt_time(c.end), c.text.trim().replace("\n\n", "\n")))
            .collect::<String>();
        let srt = exports.join(format!("{name}.srt"));
        fs::write(&srt, body).map_err(|e| e.to_string())?;
        command.arg("-i").arg(&srt);
    }
    command.arg("-filter_complex").arg(filters.join(";"));
    command.args(["-map", "[vout]"]);
    if !audio_labels.is_empty() {
        command.args(["-map", "[aout]"]);
    }
    if !video.captions.is_empty() && video.captions.iter().any(|c| !c.text.trim().is_empty() && c.end > c.start) {
        command.args(["-map", &format!("{input_index}:s:0"), "-c:s", "mov_text"]);
    }
    command.args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-r", "30", "-c:a", "aac", "-movflags", "+faststart", "-t"]);
    command.arg(seconds(duration)).arg(&partial_path).stdout(Stdio::null());
    let result = command.output().map_err(|e| format!("ffmpeg를 실행할 수 없습니다: {e}"))?;
    if !result.status.success() {
        let _ = fs::remove_file(&partial_path);
        return Err(format!("영상 내보내기 실패: {}", String::from_utf8_lossy(&result.stderr)));
    }
    fs::rename(&partial_path, &output_path).map_err(|e| e.to_string())?;
    Ok(output_path)
}

fn audio_filter(input: usize, clip: &Clip, start: f64, label: &str) -> String {
    let delay = (start.max(0.0) * 1000.0).round() as u64;
    format!(
        "[{input}:a]atrim=start={}:duration={},asetpts=PTS-STARTPTS,volume={:.3},adelay={delay}|{delay}[{label}]",
        seconds(clip.trim_start),
        seconds(clip.duration()),
        clip.volume.clamp(0.0, 4.0)
    )
}

fn checked_asset_path(video_dir: &Path, asset: &Asset) -> Result<PathBuf, String> {
    let file = Path::new(&asset.file);
    if file.components().count() != 1 || file.file_name().and_then(|s| s.to_str()) != Some(asset.file.as_str()) {
        return Err("에셋 경로가 올바르지 않습니다".into());
    }
    let path = video_dir.join("assets").join(file);
    if !path.is_file() {
        return Err(format!("에셋 파일이 없습니다: {}", asset.name));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::{export, srt_time};
    use crate::video_maker::model::{Asset, AssetKind, Caption, Clip, Video};
    use std::fs;
    use std::process::Command;

    #[test]
    fn srt_timestamp_uses_milliseconds() {
        assert_eq!(srt_time(3661.234), "01:01:01,234");
    }

    #[test]
    fn exports_image_audio_and_caption_as_playable_mp4_when_ffmpeg_is_available() {
        if Command::new("ffmpeg").arg("-version").output().is_err() {
            return;
        }
        let root = std::env::temp_dir().join(format!("purpose-export-test-{}", std::process::id()));
        let assets = root.join("assets");
        fs::create_dir_all(&assets).unwrap();
        image::RgbaImage::from_pixel(64, 64, image::Rgba([12, 180, 90, 255])).save(assets.join("1.png")).unwrap();
        let tone = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-y", "-f", "lavfi", "-i", "sine=frequency=440:duration=1", "-c:a", "pcm_s16le"])
            .arg(assets.join("2.wav"))
            .status()
            .unwrap();
        assert!(tone.success());
        let video = Video {
            id: 1,
            name: "test".into(),
            assets: vec![
                Asset { id: 1, name: "frame.png".into(), file: "1.png".into(), kind: AssetKind::Image, duration: 1.0, has_audio: false },
                Asset { id: 2, name: "tone.wav".into(), file: "2.wav".into(), kind: AssetKind::Audio, duration: 1.0, has_audio: true },
            ],
            script: Vec::new(),
            clips: vec![
                Clip { id: 3, asset_id: 1, trim_start: 0.0, trim_end: 1.0, start: 0.0, volume: 1.0, audio_track: false },
                Clip { id: 4, asset_id: 2, trim_start: 0.0, trim_end: 1.0, start: 0.0, volume: 0.5, audio_track: true },
            ],
            captions: vec![Caption { id: 5, start: 0.0, end: 0.9, text: "hello".into() }],
        };
        let output = export(&video, &root).unwrap();
        assert!(output.is_file());
        let metadata =
            Command::new("ffprobe").args(["-v", "error", "-show_entries", "stream=codec_type", "-of", "csv=p=0"]).arg(&output).output().unwrap();
        assert!(metadata.status.success());
        let streams = String::from_utf8_lossy(&metadata.stdout);
        assert!(streams.contains("video"));
        assert!(streams.contains("audio"));
        assert!(streams.contains("subtitle"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn trims_and_joins_video_clips_with_source_audio_when_ffmpeg_is_available() {
        if Command::new("ffmpeg").arg("-version").output().is_err() {
            return;
        }
        let root = std::env::temp_dir().join(format!("purpose-video-test-{}", std::process::id()));
        let assets = root.join("assets");
        fs::create_dir_all(&assets).unwrap();
        for (id, color) in [(1, "red"), (2, "blue")] {
            let output = Command::new("ffmpeg")
                .args(["-hide_banner", "-loglevel", "error", "-y", "-f", "lavfi", "-i"])
                .arg(format!("color=c={color}:s=64x64:r=30:d=1"))
                .args(["-f", "lavfi", "-i", "sine=frequency=440:duration=1", "-shortest", "-c:v", "libx264", "-c:a", "aac"])
                .arg(assets.join(format!("{id}.mp4")))
                .status()
                .unwrap();
            assert!(output.success());
        }
        let video = Video {
            id: 1,
            name: "join".into(),
            assets: [1, 2]
                .into_iter()
                .map(|id| Asset { id, name: format!("{id}.mp4"), file: format!("{id}.mp4"), kind: AssetKind::Video, duration: 1.0, has_audio: true })
                .collect(),
            script: Vec::new(),
            clips: [1, 2]
                .into_iter()
                .map(|id| Clip { id: id + 2, asset_id: id, trim_start: 0.2, trim_end: 0.8, start: 0.0, volume: 1.0, audio_track: false })
                .collect(),
            captions: Vec::new(),
        };
        let path = export(&video, &root).unwrap();
        let metadata = Command::new("ffprobe")
            .args(["-v", "error", "-show_entries", "format=duration", "-of", "default=nw=1:nk=1"])
            .arg(&path)
            .output()
            .unwrap();
        let duration: f64 = String::from_utf8_lossy(&metadata.stdout).trim().parse().unwrap();
        assert!((duration - 1.2).abs() < 0.2, "duration: {duration}");
        let _ = fs::remove_dir_all(root);
    }
}
