//! Bounded, cancellable downloads. Official Unlicense wheels run in the bundled PSF runtime;
//! no shell command, browser credentials or optional GPL modules are used.
use super::{VideoMaker, media, model::Section, navigation::Route, style};
use egui::{RichText, vec2};
use std::{
    collections::VecDeque,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Format {
    #[default]
    Video,
    Mp3,
}
#[derive(Default)]
pub struct Controller {
    url: String,
    pub target: Option<u64>,
    format: Format,
    job: Option<Job>,
    imported: Option<(u64, u64, PathBuf)>,
    retry: Option<(u64, PathBuf, String, PathBuf)>,
    completed: Option<u64>,
    error: String,
    status: String,
    progress: f32,
}
struct Job {
    events: mpsc::Receiver<Event>,
    cancel: Arc<AtomicBool>,
    video: u64,
    directory: PathBuf,
    started: Instant,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
enum Event {
    Progress(f32, String),
    Done(Result<(PathBuf, String), String>),
}

fn validate_url(text: &str) -> Result<String, String> {
    let url = url::Url::parse(text.trim()).map_err(|_| "올바른 영상 링크를 입력하세요")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || text.len() > 8192
    {
        return Err("http 또는 https 영상 링크를 입력하세요".into());
    }
    Ok(url.to_string())
}
fn command(root: &Path) -> Result<Command, String> {
    let runtime = root.join("downloader/python/python.exe");
    let mut cmd = if runtime.is_file() {
        let mut cmd = super::codex::command(&runtime);
        cmd.args(["-I", "-X", "utf8", "-m", "yt_dlp"]);
        cmd.arg("--js-runtimes").arg(format!("deno:{}", root.join("downloader/deno.exe").display()));
        cmd
    } else if cfg!(windows) {
        return Err("다운로더 실행 파일이 없습니다. 최신 설치 파일로 다시 설치하세요".into());
    } else {
        super::codex::command(Path::new("yt-dlp"))
    };
    cmd.args([
        "--ignore-config",
        "--no-plugin-dirs",
        "--no-update",
        "--no-playlist",
        "--playlist-items",
        "1",
        "--newline",
        "--progress",
        "--no-simulate",
        "--windows-filenames",
        "--socket-timeout",
        "30",
        "--retries",
        "3",
    ]);
    let ffmpeg = media::tool("ffmpeg");
    // Bare executable names are resolved by yt-dlp through PATH. Its location flag
    // only accepts an existing file/directory (the Windows bundle is absolute).
    if ffmpeg.is_file() {
        cmd.arg("--ffmpeg-location").arg(ffmpeg);
    }
    cmd.args([
        "--progress-template",
        "download:purpose-progress:%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s",
        "--print",
        "after_move:purpose-result:{\"path\":%(filepath)j,\"title\":%(title)j}",
    ]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    Ok(cmd)
}
fn configure(cmd: &mut Command, directory: &Path, format: Format, url: &str) {
    cmd.arg("-o").arg(directory.join("%(title).160B [%(id)s].%(ext)s"));
    match format {
        Format::Video => {
            cmd.args(["-f", "bv*[ext=mp4]+ba[ext=m4a]/b[ext=mp4]/bv*+ba/b", "--merge-output-format", "mp4", "--remux-video", "mp4"]);
        }
        Format::Mp3 => {
            cmd.args(["-f", "bestaudio/best", "-x", "--audio-format", "mp3", "--audio-quality", "0"]);
        }
    }
    cmd.arg("--").arg(url);
}
fn result_file(line: &str, directory: &Path, format: Format) -> Result<(PathBuf, String), String> {
    let data: serde_json::Value = serde_json::from_str(line).map_err(|e| e.to_string())?;
    let file = PathBuf::from(data["path"].as_str().ok_or("다운로드 파일 경로가 없습니다")?);
    let file = fs::canonicalize(file).map_err(|e| e.to_string())?;
    let base = fs::canonicalize(directory).map_err(|e| e.to_string())?;
    if !file.starts_with(&base) || !file.is_file() || fs::metadata(&file).map_err(|e| e.to_string())?.len() == 0 {
        return Err("다운로드 결과 파일을 확인할 수 없습니다".into());
    }
    let extension = if format == Format::Mp3 { "mp3" } else { "mp4" };
    if file.extension().and_then(|s| s.to_str()) != Some(extension) {
        return Err("요청한 형식으로 변환하지 못했습니다".into());
    }
    let title = data["title"].as_str().filter(|s| !s.trim().is_empty()).unwrap_or("다운로드 영상");
    Ok((file, format!("{}.{}", title.chars().take(240).collect::<String>(), extension)))
}
fn stop(child: &mut std::process::Child) {
    #[cfg(windows)]
    {
        let _ = super::codex::command(Path::new("taskkill.exe")).args(["/PID", &child.id().to_string(), "/T", "/F"]).status();
    }
    let _ = child.kill();
    let _ = child.wait();
}
fn run(
    mut cmd: Command,
    directory: &Path,
    format: Format,
    cancel: &AtomicBool,
    tx: &mpsc::SyncSender<Event>,
    ctx: &egui::Context,
) -> Result<(PathBuf, String), String> {
    let mut child = cmd.spawn().map_err(|e| format!("다운로더 실행 실패: {e}"))?;
    let (lines, rx) = mpsc::sync_channel(128);
    for pipe in [
        child.stdout.take().map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
        child.stderr.take().map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let sender = lines.clone();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(pipe);
            let mut bytes = Vec::new();
            loop {
                bytes.clear();
                // The child can emit arbitrary website errors. Bound each line before allocation.
                use std::io::Read;
                if reader.by_ref().take(65536).read_until(b'\n', &mut bytes).ok().filter(|n| *n > 0).is_none() {
                    break;
                }
                if sender.send(String::from_utf8_lossy(&bytes).trim().to_owned()).is_err() {
                    break;
                }
            }
        });
    }
    drop(lines);
    let mut logs = VecDeque::new();
    let mut result = None;
    let started = Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed) {
            stop(&mut child);
            return Err("다운로드를 취소했습니다".into());
        }
        if started.elapsed() > Duration::from_secs(12 * 3600) {
            stop(&mut child);
            return Err("다운로드 제한 시간을 초과했습니다".into());
        }
        for line in rx.try_iter().take(256) {
            if let Some(data) = line.strip_prefix("purpose-result:") {
                result = Some(result_file(data, directory, format));
            } else if let Some(progress) = line.strip_prefix("purpose-progress:") {
                let mut parts = progress.split('|');
                let fraction = parts.next().and_then(|s| s.trim().trim_end_matches('%').parse::<f32>().ok()).unwrap_or(0.0) / 100.0;
                let message = format!("{} · 남은 시간 {}", parts.next().unwrap_or("변환 중"), parts.next().unwrap_or("계산 중"));
                let _ = tx.try_send(Event::Progress(fraction.clamp(0.0, 1.0), message));
                ctx.request_repaint();
            } else if !line.is_empty() {
                logs.push_back(line);
                if logs.len() > 10 {
                    logs.pop_front();
                }
            }
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                // Drain buffered machine output after the process has closed its pipes.
                let drain = Instant::now();
                while drain.elapsed() < Duration::from_secs(2) {
                    if cancel.load(Ordering::Relaxed) {
                        stop(&mut child);
                        return Err("다운로드를 취소했습니다".into());
                    }
                    let line = match rx.recv_timeout(Duration::from_millis(30)) {
                        Ok(line) => line,
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    };
                    if let Some(data) = line.strip_prefix("purpose-result:") {
                        result = Some(result_file(data, directory, format));
                    } else {
                        logs.push_back(line);
                        if logs.len() > 10 {
                            logs.pop_front();
                        }
                    }
                }
                if status.success() {
                    return result.unwrap_or_else(|| Err("다운로드 결과가 없습니다".into()));
                }
                return Err(logs.into_iter().collect::<Vec<_>>().join("\n"));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(80)),
            Err(e) => {
                stop(&mut child);
                return Err(e.to_string());
            }
        }
    }
}
impl VideoMaker {
    fn start_download(&mut self, ctx: &egui::Context) -> Result<(), String> {
        let url = validate_url(&self.downloader.url)?;
        let video = self
            .downloader
            .target
            .filter(|id| self.store.data.channels.iter().flat_map(|c| &c.videos).any(|v| v.id == *id))
            .ok_or("저장할 영상을 선택하세요")?;
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|e| e.to_string())?;
        let directory = self.store.root.join("downloads").join(format!("{:032x}", u128::from_le_bytes(random)));
        fs::create_dir_all(directory.parent().ok_or("다운로드 폴더가 없습니다")?).map_err(|e| e.to_string())?;
        fs::create_dir(&directory).map_err(|e| e.to_string())?;
        let root = std::env::current_exe().map_err(|e| e.to_string())?.parent().ok_or("설치 폴더가 없습니다")?.to_owned();
        let mut cmd = match command(&root) {
            Ok(cmd) => cmd,
            Err(e) => {
                let _ = fs::remove_dir(&directory);
                return Err(e);
            }
        };
        configure(&mut cmd, &directory, self.downloader.format, &url);
        let (tx, events) = mpsc::sync_channel(64);
        let cancel = Arc::new(AtomicBool::new(false));
        let stop = cancel.clone();
        let dir = directory.clone();
        let format = self.downloader.format;
        let ctx = ctx.clone();
        let worker = std::thread::spawn(move || {
            let result = run(cmd, &dir, format, &stop, &tx, &ctx);
            if result.is_err() {
                let _ = fs::remove_dir_all(&dir);
            }
            let _ = tx.send(Event::Done(result));
            ctx.request_repaint();
        });
        self.downloader.job = Some(Job { events, cancel, video, directory, started: Instant::now(), worker: Some(worker) });
        self.downloader.completed = None;
        self.downloader.error.clear();
        self.downloader.status = "영상 정보를 확인하는 중…".into();
        self.downloader.progress = 0.0;
        Ok(())
    }
    pub(super) fn poll_downloader(&mut self, ctx: &egui::Context) {
        let mut done = None;
        if let Some(job) = &self.downloader.job {
            loop {
                match job.events.try_recv() {
                    Ok(Event::Progress(value, status)) => {
                        self.downloader.progress = value;
                        self.downloader.status = status;
                    }
                    Ok(Event::Done(result)) => {
                        done = Some((job.video, job.directory.clone(), result));
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        done = Some((job.video, job.directory.clone(), Err("다운로드 작업이 종료되었습니다".into())));
                        break;
                    }
                }
            }
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if let Some((video, dir, result)) = done {
            self.downloader.job = None;
            match result {
                Ok((path, title)) => {
                    self.downloader.retry = Some((video, path, title, dir));
                    self.retry_download_import();
                }
                Err(e) => {
                    self.downloader.error = if e.is_empty() { "이 링크에서 영상을 다운로드하지 못했습니다".into() } else { e };
                }
            }
        }
        if let Some((video, asset, directory)) = &self.downloader.imported {
            let registered = self
                .store
                .data
                .channels
                .iter()
                .flat_map(|c| &c.videos)
                .chain(self.store.data.trash.iter().map(|t| &t.video))
                .chain(self.store.data.trashed_channels.iter().flat_map(|t| &t.channel.videos))
                .any(|v| v.id == *video && v.assets.iter().any(|a| a.id == *asset));
            if registered && self.store.save().is_ok() {
                let directory = directory.clone();
                self.downloader.completed = Some(*video);
                self.downloader.imported = None;
                self.downloader.retry = None;
                self.downloader.status = "라이브러리에 추가했습니다".into();
                std::thread::spawn(move || {
                    let _ = fs::remove_dir_all(directory);
                });
            } else if !self.imports.busy() {
                self.downloader.imported = None;
                self.downloader.error = "라이브러리에 추가하지 못했습니다. 다시 시도할 수 있습니다".into();
            }
        }
    }
    fn retry_download_import(&mut self) {
        if let Some((video, path, title, dir)) = self.downloader.retry.clone() {
            match self.queue_import(video, path, Some(title)) {
                Ok(id) => {
                    self.downloader.imported = Some((video, id, dir));
                    self.downloader.error.clear();
                    self.downloader.status = "라이브러리에 추가 중…".into();
                }
                Err(e) => self.downloader.error = e,
            }
        }
    }
    pub(super) fn download_busy(&self) -> bool {
        self.downloader.job.is_some() || self.downloader.imported.is_some()
    }
    pub(super) fn downloader_page(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().id_salt("downloader-page").auto_shrink([false, false]).show(ui, |ui| {
            let (width, left) = style::page_width(ui, false);
            ui.add_space(style::PAGE_TOP);
            ui.horizontal(|ui| {
                ui.add_space(left);
                ui.allocate_ui_with_layout(vec2(width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(width);
                    ui.label(style::heading("Video Downloader", style::PAGE_TITLE));
                    ui.add_space(style::TITLE_GAP);
                    let busy = self.download_busy();
                    ui.add_enabled_ui(!busy && self.downloader.retry.is_none(), |ui| {
                        ui.label(RichText::new("영상 링크").strong());
                        egui::Frame::NONE.fill(style::SIDEBAR).corner_radius(7).inner_margin(12.0).show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.downloader.url)
                                    .desired_width(f32::INFINITY)
                                    .hint_text("https://www.youtube.com/watch?v=…")
                                    .frame(egui::Frame::NONE),
                            );
                        });
                        ui.add_space(14.0);
                        ui.horizontal(|ui| {
                            ui.label("다운로드 형식");
                            if style::filter_tab(ui, "영상 · MP4", self.downloader.format == Format::Video).clicked() {
                                self.downloader.format = Format::Video;
                            }
                            if style::filter_tab(ui, "오디오 · MP3", self.downloader.format == Format::Mp3).clicked() {
                                self.downloader.format = Format::Mp3;
                            }
                        });
                        ui.add_space(10.0);
                        ui.label(RichText::new("저장할 라이브러리").strong());
                        let selected = self
                            .store
                            .data
                            .channels
                            .iter()
                            .flat_map(|c| c.videos.iter().map(move |v| (v.id, format!("{} / {}", c.name, v.name))))
                            .find(|(id, _)| Some(*id) == self.downloader.target)
                            .map_or("영상 선택".into(), |(_, name)| name);
                        let destination = style::dropdown(ui, &selected, width.min(460.0) - 28.0);
                        egui::Popup::menu(&destination).show(|ui| {
                            for channel in &self.store.data.channels {
                                for video in &channel.videos {
                                    if ui
                                        .selectable_value(&mut self.downloader.target, Some(video.id), format!("{} / {}", channel.name, video.name))
                                        .clicked()
                                    {
                                        ui.close();
                                    }
                                }
                            }
                        });
                    });
                    ui.add_space(22.0);
                    if busy {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(&self.downloader.status);
                            if let Some(job) = &self.downloader.job {
                                ui.label(
                                    RichText::new(format!("{}:{:02}", job.started.elapsed().as_secs() / 60, job.started.elapsed().as_secs() % 60))
                                        .color(style::MUTED),
                                );
                                if style::action_button(ui, "취소", false).clicked() {
                                    job.cancel.store(true, Ordering::Relaxed);
                                }
                            }
                        });
                        ui.add(egui::ProgressBar::new(self.downloader.progress).desired_width(width).desired_height(4.0));
                        self.import_progress(ui);
                    } else if self.downloader.retry.is_some() {
                        ui.horizontal(|ui| {
                            if style::action_button(ui, "라이브러리에 다시 추가", true).clicked() {
                                self.retry_download_import();
                            }
                            if style::action_button(ui, "다운로드 파일 지우기", false).clicked()
                                && let Some((_, _, _, dir)) = self.downloader.retry.take()
                            {
                                std::thread::spawn(move || {
                                    let _ = fs::remove_dir_all(dir);
                                });
                            }
                        });
                    } else if style::action_button(ui, "다운로드", true).clicked()
                        && let Err(e) = self.start_download(ui.ctx())
                    {
                        self.downloader.error = e;
                    }
                    if !self.downloader.error.is_empty() {
                        ui.add_space(14.0);
                        egui::Frame::NONE.fill(style::SIDEBAR).corner_radius(7).inner_margin(14.0).show(ui, |ui| {
                            ui.label(RichText::new(&self.downloader.error).color(style::TEXT));
                        });
                    }
                    if let Some(video) = self.downloader.completed {
                        ui.add_space(14.0);
                        ui.horizontal(|ui| {
                            ui.label("라이브러리에 추가했습니다");
                            if style::action_button(ui, "라이브러리 열기", false).clicked() {
                                self.navigate(Route::Video { video_id: video, section: Section::Library });
                            }
                        });
                    }
                });
            });
        });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn links_are_http_only_and_never_command_arguments() {
        assert!(validate_url("file:///etc/passwd").is_err());
        assert!(validate_url("--exec hi").is_err());
        assert!(validate_url("https://user:pass@example.com/a").is_err());
        assert_eq!(validate_url(" https://example.com/watch?v=1 ").unwrap(), "https://example.com/watch?v=1");
    }
    #[test]
    fn result_must_be_a_nonempty_requested_file_inside_job_directory() {
        let root = std::env::temp_dir().join(format!("purpose-download-result-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let file = root.join("sample.mp3");
        fs::write(&file, b"fixture").unwrap();
        let line = serde_json::json!({"path":file,"title":"원래 제목"}).to_string();
        assert_eq!(result_file(&line, &root, Format::Mp3).unwrap().1, "원래 제목.mp3");
        assert!(result_file(&line, &root, Format::Video).is_err());
        assert!(result_file(&line, &root.join("elsewhere"), Format::Mp3).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
