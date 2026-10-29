//! Purpose video maker: a local channel/video workspace around the FigCraft editor.
mod branding;
mod codex;
pub use branding::window_icon;
mod film;
mod film_audio;
mod film_audio_in;
mod history;
mod media;
mod model;
mod navigation;
mod panels;
mod style;
mod ui;

use crate::services;
use figcraft_engine::Session;
use figcraft_ui_egui::FigApp;
use model::{Asset, Section, Store, Video};
use navigation::{Route, Tabs};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::mpsc::{self, Receiver};

pub struct VideoMaker {
    store: Store,
    section: Section,
    tabs: Tabs,
    sidebar_open: bool,
    side_view: Option<SideView>,
    revision_due: Option<std::time::Instant>,
    auth_job: Option<Receiver<Result<bool, String>>>,
    authenticated: Option<bool>,
    login: Option<codex::Login>,
    login_output: String,
    auth_error: String,
    search: String,
    history_all: bool,
    history_query: String,
    history_video: Option<u64>,
    library_query: String,
    library_view: LibraryView,
    thumbnails: HashMap<u64, egui::TextureHandle>,
    thumbnail_jobs: HashMap<u64, Receiver<Result<egui::ColorImage, String>>>,
    thumbnail_failed: HashSet<u64>,
    message: String,
    fonts_ready: bool,
    design_loaded: Option<u64>,
    film: Option<film::Editor>,
    film_ime_ready: bool,
    prompt: String,
    ai_response: String,
    ai_job: Option<Receiver<Result<String, String>>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SideView {
    Codex,
    Trash,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LibraryView {
    Tiles,
    List,
}

impl VideoMaker {
    pub fn section_is_design(&self) -> bool {
        self.tabs.route().is_some_and(|route| route.video().is_some_and(|(_, section)| section == Section::Design))
    }
    pub fn new() -> Result<Self, String> {
        // Join the startup scan before the host chooses its shared UI font set.
        figcraft_text::scan_system_fonts();
        let root = std::env::var_os("PURPOSE_PROJECTS_DIR")
            .map(PathBuf::from)
            .or_else(|| services::config_dir().map(|p| p.join("Purpose video maker")))
            .ok_or("프로젝트 폴더를 결정할 수 없습니다")?;
        Self::at(root)
    }

    fn at(root: PathBuf) -> Result<Self, String> {
        let mut store = Store::load(root)?;
        if store.data.channels.is_empty() {
            let channel = store.data.add_channel();
            let _ = store.data.add_video(channel);
            store.save()?;
        }
        store.data.checkpoint("프로젝트 열기");
        let initial = store.data.selected_video.map(|video_id| Route::Video { video_id, section: Section::Script });
        Ok(Self {
            store,
            section: Section::Script,
            tabs: Tabs::new(initial),
            sidebar_open: true,
            side_view: None,
            revision_due: None,
            auth_job: None,
            authenticated: None,
            login: None,
            login_output: String::new(),
            auth_error: String::new(),
            search: String::new(),
            history_all: false,
            history_query: String::new(),
            history_video: None,
            library_query: String::new(),
            library_view: LibraryView::Tiles,
            thumbnails: HashMap::new(),
            thumbnail_jobs: HashMap::new(),
            thumbnail_failed: HashSet::new(),
            message: String::new(),
            fonts_ready: false,
            design_loaded: None,
            film: None,
            film_ime_ready: false,
            prompt: String::new(),
            ai_response: String::new(),
            ai_job: None,
        })
    }

    pub fn title(&self) -> String {
        if self.tabs.route() == Some(Route::History) {
            return "수정 기록 — Purpose video maker".into();
        }
        self.tabs
            .route()
            .and_then(|route| route.video())
            .and_then(|(id, _)| self.store.data.channels.iter().flat_map(|channel| &channel.videos).find(|video| video.id == id))
            .map_or_else(|| "Purpose video maker".into(), |video| format!("{} — Purpose video maker", video.name))
    }

    fn sync_route(&mut self) {
        if self.section == Section::Edit {
            // Keep the unsaved editor alive on an I/O error. Opening another editor and
            // closing the window both retry this save before dropping its session.
            if let Err(error) = self.save_film() {
                self.message = error;
            }
            if let Some(editor) = &mut self.film {
                editor.app.stop();
            }
        }
        if let Some((video_id, section)) = self.tabs.route().and_then(Route::video) {
            if self.store.data.selected_video != Some(video_id) {
                self.checkpoint();
                self.store.data.selected_video = Some(video_id);
                self.save();
            }
            self.section = section;
        } else {
            self.section = Section::Library;
        }
    }

    fn navigate(&mut self, route: Route) {
        self.tabs.navigate(route);
        self.sync_route();
    }

    fn checkpoint(&mut self) {
        self.store.data.checkpoint("자동 저장");
        self.revision_due = None;
        if let Err(e) = self.store.save() {
            self.message = e;
        }
    }

    fn save(&mut self) {
        if let Some(video) = self.store.data.selected_mut() {
            video.script_title = video.name.clone();
        }
        self.revision_due = Some(std::time::Instant::now());
        if let Err(e) = self.store.save() {
            self.message = format!("저장 실패: {e}");
        }
    }

    pub fn prepare_close(&mut self, fig: &mut FigApp) -> Result<(), String> {
        self.save_film()?;
        if self.design_loaded.is_some() {
            fig.session.execute("file.save", &json!({})).map_err(|e| format!("디자인 저장 실패: {e}"))?;
        }
        self.checkpoint();
        self.store.save().map_err(|e| format!("프로젝트 저장 실패: {e}"))
    }

    pub fn show_error(&mut self, error: String) {
        self.message = error;
    }

    fn video(&self) -> Option<&Video> {
        self.store.data.selected().map(|(_, v)| v)
    }

    fn video_id(&self) -> Option<u64> {
        self.store.data.selected_video
    }

    fn import_path(&mut self, source: &Path) -> Result<(), String> {
        let video_id = self.video_id().ok_or("영상을 먼저 선택하세요")?;
        if !source.is_file() {
            return Err("파일이 없습니다".into());
        }
        let (kind, duration, has_audio) = media::probe(source)?;
        let id = self.store.data.id();
        let extension = source.extension().and_then(|e| e.to_str()).unwrap_or("bin");
        let extension: String = extension.chars().filter(|c| c.is_ascii_alphanumeric()).take(10).collect();
        let file = format!("{id}.{}", if extension.is_empty() { "bin" } else { &extension });
        let dir = self.store.video_dir(video_id).join("assets");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        fs::copy(source, dir.join(&file)).map_err(|e| e.to_string())?;
        let asset = Asset { id, name: model::file_name(source), file, kind, duration, has_audio };
        if let Some(video) = self.store.data.selected_mut() {
            video.assets.push(asset);
            self.save();
        }
        Ok(())
    }

    fn add_to_script(&mut self, id: u64) {
        let video_id = self.video_id();
        if let Some(video) = self.store.data.selected_mut()
            && video.asset(id).is_some()
            && !video.script_assets.contains(&id)
        {
            video.script_assets.push(id);
            self.save();
        }
        if let Some(video_id) = video_id {
            self.navigate(Route::Video { video_id, section: Section::Script });
        }
    }

    fn add_to_timeline(&mut self, id: u64) {
        let Some(video_id) = self.video_id() else { return };
        match self.append_film_asset(id) {
            Ok(()) => self.navigate(Route::Video { video_id, section: Section::Edit }),
            Err(error) => self.message = error,
        }
    }

    pub fn ensure_design(&mut self, fig: &mut FigApp) {
        let Some(id) = self.video_id() else { return };
        if fig.session.docs.is_empty() {
            self.design_loaded = None;
        }
        fig.ui.home = false;
        if self.design_loaded == Some(id) {
            return;
        }
        if self.design_loaded.is_some()
            && let Err(e) = fig.session.execute("file.save", &json!({}))
        {
            self.message = format!("이전 디자인 저장 실패: {e}");
            return;
        }
        let path = self.store.design_path(id);
        if !path.exists() {
            if let Err(e) = fs::create_dir_all(self.store.video_dir(id)) {
                self.message = format!("디자인 폴더 생성 실패: {e}");
                return;
            }
            let mut initial = Session::new();
            if let Err(e) = initial.execute("file.saveAs", &json!({"path": path})) {
                self.message = format!("디자인 생성 실패: {e}");
                return;
            }
        }
        if let Err(e) = fig.session.execute("file.open", &json!({"path": path})) {
            self.message = format!("디자인 열기 실패: {e}");
            return;
        }
        fig.ui.home = false;
        self.design_loaded = Some(id);
    }

    pub fn export_design(&mut self, fig: &mut FigApp) {
        let Some(id) = self.video_id() else { return };
        self.ensure_design(fig);
        if self.design_loaded != Some(id) {
            return;
        }
        if let Err(e) = fig.session.execute("file.save", &json!({})) {
            self.message = format!("디자인 저장 실패: {e}");
            return;
        }
        let asset_id = self.store.data.id();
        let file = format!("design-{asset_id}.png");
        let dir = self.store.video_dir(id).join("assets");
        if let Err(e) = fs::create_dir_all(&dir) {
            self.message = e.to_string();
            return;
        }
        let path = dir.join(&file);
        match fig.session.execute("file.export", &json!({"format": "png", "path": path, "scale": 1.0})) {
            Ok(_) => {
                if let Some(video) = self.store.data.selected_mut() {
                    video.assets.push(Asset {
                        id: asset_id,
                        name: format!("{} 디자인", video.name),
                        file,
                        kind: model::AssetKind::Image,
                        duration: 5.0,
                        has_audio: false,
                    });
                    self.save();
                    self.message = "디자인 PNG를 라이브러리에 저장했습니다".into();
                }
            }
            Err(e) => self.message = format!("디자인 내보내기 실패: {e}"),
        }
    }

    fn poll_jobs(&mut self, ctx: &egui::Context) {
        self.poll_auth(ctx);
        if let Some(due) = self.revision_due {
            if due.elapsed() >= std::time::Duration::from_secs(1) {
                self.checkpoint();
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(250));
            }
        }
        let finished: Vec<_> = self
            .thumbnail_jobs
            .iter()
            .filter_map(|(&id, rx)| match rx.try_recv() {
                Ok(result) => Some((id, result)),
                Err(mpsc::TryRecvError::Disconnected) => Some((id, Err("미리보기 작업 종료".into()))),
                Err(mpsc::TryRecvError::Empty) => None,
            })
            .collect();
        for (id, result) in finished {
            self.thumbnail_jobs.remove(&id);
            match result {
                Ok(image) => {
                    self.thumbnails.insert(id, ctx.load_texture(format!("purpose-asset-{id}"), image, egui::TextureOptions::LINEAR));
                }
                Err(_) => {
                    self.thumbnail_failed.insert(id);
                }
            }
            ctx.request_repaint();
        }
        if !self.thumbnail_jobs.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }
        if let Some(rx) = &self.ai_job {
            match rx.try_recv() {
                Ok(Ok(text)) => {
                    self.ai_response = text;
                    self.ai_job = None;
                }
                Ok(Err(e)) => {
                    self.ai_response = format!("Codex 오류: {e}");
                    self.ai_job = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.ai_response = "Codex 작업이 종료되었습니다".into();
                    self.ai_job = None;
                }
                Err(mpsc::TryRecvError::Empty) => ctx.request_repaint_after(std::time::Duration::from_millis(250)),
            }
        }
    }

    fn ask_codex(&mut self, request: String) {
        if self.ai_job.is_some() || request.trim().is_empty() {
            return;
        }
        let Some(cli) = codex::executable(self.store.data.codex_path.as_deref()) else {
            self.auth_error = "Codex CLI를 먼저 연결하세요".into();
            return;
        };
        if self.authenticated != Some(true) {
            self.auth_error = "ChatGPT 계정으로 로그인하세요".into();
            return;
        }
        let Some(video) = self.video().cloned() else { return };
        let root = self.store.video_dir(video.id);
        let script = format!("{}\n\n{}", video.script_title, video.script_body);
        let assets = video.assets.iter().map(|a| format!("{} ({}, {:.1}s)", a.name, a.kind.label(), a.duration)).collect::<Vec<_>>().join("\n");
        let clips = video
            .clips
            .iter()
            .map(|c| {
                format!(
                    "{}: {:.1}s–{:.1}s, volume {:.0}%",
                    video.asset(c.asset_id).map_or("missing", |a| a.name.as_str()),
                    c.trim_start,
                    c.trim_end,
                    c.volume * 100.0
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let native_edit = self
            .film
            .as_ref()
            .filter(|f| f.video_id == video.id)
            .map(|f| f.app.session.project.to_json().to_string())
            .unwrap_or_else(|| "편집 파일: edit.fcproj".into());
        let prompt = format!(
            "당신은 이 영상 프로젝트의 제작 보조자입니다. 한국어로 답하세요. 파일을 수정하지 말고 요청에 대한 제안이나 수정된 대본을 답변으로만 제시하세요.\n영상: {}\n대본:\n{}\n라이브러리:\n{}\n이전 타임라인:\n{clips}\nFilmCraft 프로젝트:\n{native_edit}\n요청: {}",
            video.name, script, assets, request
        );
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(run_codex(&cli, &root, &prompt));
        });
        self.ai_response.clear();
        self.ai_job = Some(rx);
        self.prompt.clear();
    }
}

fn run_codex(cli: &Path, root: &Path, prompt: &str) -> Result<String, String> {
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
    let output_path = std::env::temp_dir().join(format!("purpose-codex-{}-{stamp}.txt", std::process::id()));
    let mut child = codex::command(cli)
        .env_remove("OPENAI_API_KEY")
        .env_remove("CODEX_API_KEY")
        .args(["-c", "forced_login_method=\"chatgpt\"", "-c", "model_provider=\"openai\""])
        .args(["exec", "--sandbox", "read-only", "--ephemeral", "--skip-git-repo-check", "--output-last-message"])
        .arg(&output_path)
        .arg("-C")
        .arg(root)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Codex CLI를 실행할 수 없습니다. 설치와 ChatGPT 로그인을 확인하세요: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin.write_all(prompt.as_bytes()).map_err(|e| e.to_string())?;
    }
    let result = child.wait_with_output().map_err(|e| e.to_string())?;
    let answer = fs::read_to_string(&output_path).unwrap_or_default();
    let _ = fs::remove_file(&output_path);
    if !result.status.success() {
        return Err(String::from_utf8_lossy(&result.stderr).chars().take(2000).collect());
    }
    if answer.trim().is_empty() {
        return Err("Codex가 빈 응답을 보냈습니다".into());
    }
    Ok(answer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn editing_the_document_title_persists_the_video_name() {
        let root = std::env::temp_dir().join(format!("purpose-title-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let mut maker = VideoMaker::at(root.clone()).unwrap();
        maker.store.data.selected_mut().unwrap().name = "내 영상 제목".into();
        maker.save();
        let store = Store::load(root.clone()).unwrap();
        let (_, video) = store.data.selected().unwrap();
        assert_eq!(video.name, "내 영상 제목");
        assert_eq!(video.script_title, video.name);
        assert!(maker.title().starts_with("내 영상 제목"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn imported_asset_is_shared_by_script_and_timeline_after_reopen() {
        if Command::new("ffprobe").arg("-version").output().is_err() {
            return;
        }
        let root = std::env::temp_dir().join(format!("purpose-project-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let source = root.join("sample.png");
        image::RgbaImage::from_pixel(24, 24, image::Rgba([40, 80, 160, 255])).save(&source).unwrap();
        let mut maker = VideoMaker::at(root.clone()).unwrap();
        maker.import_path(&source).unwrap();
        let id = maker.video().unwrap().assets[0].id;
        maker.add_to_script(id);
        maker.add_to_timeline(id);
        let store = Store::load(root.clone()).unwrap();
        let (_, video) = store.data.selected().unwrap();
        assert_eq!(video.assets[0].kind, model::AssetKind::Image);
        assert!(store.asset_path(video.id, &video.assets[0]).is_file());
        assert!(video.script_assets.contains(&id));
        assert!(store.video_dir(video.id).join("edit.fcproj").is_file());
        let editor = maker.film.as_ref().unwrap();
        assert_eq!(editor.app.session.active_sequence().unwrap().video_tracks[0].items.len(), 1);
        maker.shutdown_film();
        let _ = fs::remove_dir_all(root);
    }
}
