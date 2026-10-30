//! Media probing and cancellable streaming copies never run on the UI thread.
use super::{VideoMaker, media, model::Asset};
use std::{
    collections::VecDeque,
    fs,
    io::{Read, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

pub struct Task {
    pub video: u64,
    pub id: u64,
    pub source: PathBuf,
    pub directory: PathBuf,
    pub name: String,
}
pub enum Event {
    Progress(String, f32),
    Ready(u64, Result<Asset, String>),
}
pub struct Running {
    events: mpsc::Receiver<Event>,
    cancel: Arc<AtomicBool>,
    pub name: String,
    pub progress: f32,
}
impl Drop for Running {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
#[derive(Default)]
pub struct Queue {
    pending: VecDeque<Task>,
    running: Option<Running>,
    pub completed: usize,
    pub total: usize,
    pub errors: Vec<String>,
}
fn transfer(task: &Task, cancel: &AtomicBool, progress: impl Fn(f32)) -> Result<Asset, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("가져오기를 취소했습니다".into());
    }
    let (kind, duration, has_audio) = media::probe_cancellable(&task.source, cancel)?;
    if cancel.load(Ordering::Relaxed) {
        return Err("가져오기를 취소했습니다".into());
    }
    fs::create_dir_all(&task.directory).map_err(|e| e.to_string())?;
    let extension: String =
        task.source.extension().and_then(|e| e.to_str()).unwrap_or("bin").chars().filter(|c| c.is_ascii_alphanumeric()).take(10).collect();
    let file = format!("{}.{}", task.id, if extension.is_empty() { "bin" } else { &extension });
    let temp = task.directory.join(format!(".import-{}.tmp", task.id));
    let result = (|| {
        let mut input = fs::File::open(&task.source).map_err(|e| e.to_string())?;
        let len = input.metadata().map_err(|e| e.to_string())?.len();
        let mut output = fs::File::create(&temp).map_err(|e| e.to_string())?;
        let mut buffer = vec![0; 1024 * 1024];
        let mut copied = 0u64;
        let mut last = std::time::Instant::now();
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("가져오기를 취소했습니다".to_owned());
            }
            let n = input.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            output.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
            copied += n as u64;
            if last.elapsed() >= Duration::from_millis(80) {
                progress(copied as f32 / len.max(1) as f32);
                last = std::time::Instant::now();
            }
        }
        output.sync_all().map_err(|e| e.to_string())?;
        drop(output);
        fs::rename(&temp, task.directory.join(&file)).map_err(|e| e.to_string())?;
        Ok(Asset { id: task.id, name: task.name.clone(), file, kind, duration, has_audio })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}
impl Queue {
    pub fn push(&mut self, task: Task) {
        if !self.busy() {
            self.completed = 0;
            self.total = 0;
            self.errors.clear();
        }
        self.total += 1;
        self.pending.push_back(task);
    }
    pub fn busy(&self) -> bool {
        self.running.is_some() || !self.pending.is_empty()
    }
    pub fn cancel(&mut self) {
        self.pending.clear();
        if let Some(running) = &self.running {
            running.cancel.store(true, Ordering::Relaxed);
        }
    }
    pub fn progress(&self) -> (&str, f32) {
        self.running.as_ref().map_or(("분석 준비 중…", 0.0), |r| (r.name.as_str(), r.progress))
    }
    fn poll(&mut self, ctx: &egui::Context) -> Vec<(u64, Result<Asset, String>)> {
        let mut ready = Vec::new();
        let mut done = false;
        if let Some(running) = &mut self.running {
            loop {
                match running.events.try_recv() {
                    Ok(Event::Progress(name, value)) => {
                        running.name = name;
                        running.progress = value;
                    }
                    Ok(Event::Ready(id, result)) => {
                        ready.push((id, result));
                        done = true;
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.errors.push("파일 가져오기 작업이 종료되었습니다".into());
                        done = true;
                        break;
                    }
                }
            }
        }
        if done {
            self.running = None;
            self.completed += 1;
        }
        if self.running.is_none()
            && let Some(task) = self.pending.pop_front()
        {
            let (tx, events) = mpsc::channel();
            let cancel = Arc::new(AtomicBool::new(false));
            let stop = cancel.clone();
            let name = format!("분석 중 · {}", task.name);
            let wake = ctx.clone();
            std::thread::spawn(move || {
                let result = transfer(&task, &stop, |value| {
                    let _ = tx.send(Event::Progress(task.name.clone(), value));
                    wake.request_repaint();
                });
                let _ = tx.send(Event::Ready(task.video, result));
                wake.request_repaint();
            });
            self.running = Some(Running { events, cancel, name, progress: 0.0 });
        }
        if self.busy() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        ready
    }
}
impl VideoMaker {
    pub(super) fn queue_import(&mut self, video: u64, source: PathBuf, name: Option<String>) -> Result<u64, String> {
        if !self.store.data.channels.iter().flat_map(|c| &c.videos).any(|v| v.id == video) {
            return Err("대상 영상이 없습니다".into());
        }
        let id = self.store.data.id();
        let name = name.unwrap_or_else(|| super::model::file_name(&source));
        self.store.save()?; // Reserve the id before a worker creates its file.
        self.imports.push(Task { video, id, source, directory: self.store.video_dir(video).join("assets"), name });
        Ok(id)
    }
    pub(super) fn poll_imports(&mut self, ctx: &egui::Context) {
        let was_busy = self.imports.busy();
        for (id, result) in self.imports.poll(ctx) {
            match result {
                Ok(asset) => {
                    if let Some(video) = self.store.data.channels.iter_mut().flat_map(|c| &mut c.videos).find(|v| v.id == id) {
                        video.assets.push(asset);
                        self.save();
                    } else {
                        // A video moved to the trash while importing retains its completed source.
                        let video = self
                            .store
                            .data
                            .trash
                            .iter_mut()
                            .map(|t| &mut t.video)
                            .chain(self.store.data.trashed_channels.iter_mut().flat_map(|t| &mut t.channel.videos))
                            .find(|v| v.id == id);
                        if let Some(video) = video {
                            video.assets.push(asset);
                            self.save();
                        }
                    }
                }
                Err(error) => self.imports.errors.push(error),
            }
        }
        if was_busy && !self.imports.busy() {
            self.message = if self.imports.errors.is_empty() {
                format!("파일 {}개를 가져왔습니다", self.imports.completed)
            } else {
                format!("일부 파일 가져오기 실패: {}", self.imports.errors.join(" · "))
            };
        }
    }
    pub(super) fn import_progress(&mut self, ui: &mut egui::Ui) {
        if !self.imports.busy() {
            return;
        }
        let (name, progress) = self.imports.progress();
        let name = name.to_owned();
        ui.add_space(8.0);
        egui::Frame::NONE.fill(super::style::SIDEBAR).corner_radius(7).inner_margin(10.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.add(egui::Label::new(format!("{} / {} · {name}", self.imports.completed + 1, self.imports.total)).truncate());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("취소").clicked() {
                        self.imports.cancel();
                    }
                });
            });
            ui.add(egui::ProgressBar::new(progress).desired_height(3.0));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_transfer_keeps_source_and_does_not_publish_a_partial_asset() {
        let root = std::env::temp_dir().join(format!("purpose-import-cancel-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("original.figcraft");
        fs::write(&source, b"original bytes").unwrap();
        let task = Task { video: 1, id: 2, source: source.clone(), directory: root.join("assets"), name: "original".into() };
        assert!(transfer(&task, &AtomicBool::new(true), |_| {}).is_err());
        assert_eq!(fs::read(source).unwrap(), b"original bytes");
        assert!(!task.directory.exists());
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn worker_preserves_the_display_name_and_streams_into_the_reserved_file() {
        let root = std::env::temp_dir().join(format!("purpose-import-copy-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("internal.figcraft");
        fs::write(&source, b"synthetic design source").unwrap();
        let task = Task { video: 8, id: 19, source, directory: root.join("assets"), name: "디자인 원본.figcraft".into() };
        let asset = transfer(&task, &AtomicBool::new(false), |_| {}).unwrap();
        assert_eq!(asset.name, "디자인 원본.figcraft");
        assert_eq!(asset.kind, super::super::model::AssetKind::Design);
        assert_eq!(fs::read(task.directory.join(asset.file)).unwrap(), b"synthetic design source");
        let _ = fs::remove_dir_all(root);
    }
}
