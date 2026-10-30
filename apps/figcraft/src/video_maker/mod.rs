//! Purpose video maker: a local channel/video workspace around the FigCraft editor.
mod backup;
mod branding;
mod chat;
mod codex;
mod codex_options;
mod document;
mod interactions;
mod rich_script;
mod settings;
#[cfg(windows)]
pub use branding::set_windows_icons;
pub use branding::window_icon;
mod agent;
mod agent_transport;
mod appearance;
mod film;
mod film_audio;
mod film_audio_in;
mod guide;
mod history;
mod imports;
mod library;
mod trash;
pub use agent_transport::stdio as agent_stdio;
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
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;
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
    catalog: Vec<codex_options::Model>,
    catalog_job: Option<Receiver<Result<Vec<codex_options::Model>, String>>>,
    catalog_error: String,
    settings_open: bool,
    trash_open: bool,
    trash_query: String,
    settings_section: usize,
    font_query: String,
    font_names: Vec<String>,
    font_applied: Option<String>,
    base_fonts: Option<egui::FontDefinitions>,
    imports: imports::Queue,
    library_selection: Vec<u64>,
    library_focused: bool,
    library_anchor: Option<u64>,
    library_clipboard: Vec<(u64, u64)>,
    library_clipboard_marker: String,
    library_rename: Option<(u64, String)>,
    library_marquee: Option<(egui::Pos2, Vec<u64>)>,
    backup: backup::Controller,
    toast: Option<(String, std::time::Instant)>,
    design_canvas: Option<egui::Rect>,
    design_backup_saved: Option<std::time::Instant>,
    search: String,
    rename_video: Option<(model::Item, String)>,
    history_all: bool,
    history_query: String,
    history_video: Option<u64>,
    library_query: String,
    library_view: LibraryView,
    thumbnails: HashMap<u64, egui::TextureHandle>,
    thumbnail_recent: Vec<u64>,
    thumbnail_jobs: HashMap<u64, Receiver<Result<egui::ColorImage, String>>>,
    thumbnail_failed: HashSet<u64>,
    message: String,
    fonts_ready: bool,
    design_loaded: Option<u64>,
    film: Option<film::Editor>,
    film_ime_ready: bool,
    prompt: String,
    prompt_video: Option<u64>,
    drafts: HashMap<u64, String>,
    chats: HashMap<u64, chat::Conversation>,
    chat_job: Option<chat::Job>,
    agent: Option<agent_transport::Bridge>,
    ui_context: Option<egui::Context>,
    pending_design_asset: Option<u64>,
    zoom_ready: bool,
    zoom_save_due: Option<std::time::Instant>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SideView {
    Codex,
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
        let message = store.recovered.take().unwrap_or_default();
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
            catalog: Vec::new(),
            catalog_job: None,
            catalog_error: String::new(),
            settings_open: false,
            trash_open: false,
            trash_query: String::new(),
            settings_section: 0,
            font_query: String::new(),
            font_names: figcraft_text::family_names(),
            font_applied: None,
            base_fonts: None,
            imports: Default::default(),
            library_selection: Vec::new(),
            library_focused: false,
            library_anchor: None,
            library_clipboard: Vec::new(),
            library_clipboard_marker: String::new(),
            library_rename: None,
            library_marquee: None,
            backup: Default::default(),
            toast: None,
            design_canvas: None,
            design_backup_saved: None,
            search: String::new(),
            rename_video: None,
            history_all: false,
            history_query: String::new(),
            history_video: None,
            library_query: String::new(),
            library_view: LibraryView::Tiles,
            thumbnails: HashMap::new(),
            thumbnail_recent: Vec::new(),
            thumbnail_jobs: HashMap::new(),
            thumbnail_failed: HashSet::new(),
            message,
            fonts_ready: false,
            design_loaded: None,
            film: None,
            film_ime_ready: false,
            prompt: String::new(),
            prompt_video: None,
            drafts: HashMap::new(),
            chats: HashMap::new(),
            chat_job: None,
            agent: None,
            ui_context: None,
            pending_design_asset: None,
            zoom_ready: false,
            zoom_save_due: None,
        })
    }

    pub fn title(&self) -> String {
        if self.tabs.route() == Some(Route::History) {
            return "수정 기록 — Purpose video maker".into();
        }
        if matches!(self.tabs.route(), Some(Route::Guide { .. })) {
            return "Codex 가이드 — Purpose video maker".into();
        }
        self.tabs
            .route()
            .and_then(|route| route.video())
            .and_then(|(id, _)| self.store.data.channels.iter().flat_map(|channel| &channel.videos).find(|video| video.id == id))
            .map_or_else(|| "Purpose video maker".into(), |video| format!("{} — Purpose video maker", video.name))
    }

    fn sync_route(&mut self) {
        if matches!(self.tabs.route(), Some(Route::Guide { .. } | Route::History)) {
            self.side_view = None;
        }
        let next = self.tabs.route().and_then(Route::video).map(|(id, _)| id);
        if self.prompt_video != next {
            if let Some(id) = self.prompt_video {
                self.drafts.insert(id, std::mem::take(&mut self.prompt));
            }
            self.prompt = next.and_then(|id| self.drafts.remove(&id)).unwrap_or_default();
            self.prompt_video = next;
            self.library_selection.clear();
            self.library_anchor = None;
            self.library_marquee = None;
        }
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
        self.store.save().map_err(|e| format!("프로젝트 저장 실패: {e}"))?;
        if let Err(error) = self.backup.finish(&self.store.root, self.store.data.backup_directory.as_deref(), &self.store.data.backup_id) {
            // The primary project is already durable. An unplugged backup disk must not trap the window open.
            log::warn!("마지막 백업 실패 (프로젝트 원본은 저장됨): {error}");
        }
        Ok(())
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

    #[cfg(test)]
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
        {
            if !video.script_assets.contains(&id) {
                video.script_assets.push(id);
            }
            if video.asset(id).is_some_and(|a| a.kind == model::AssetKind::Image) && !video.script_rich.images.iter().any(|i| i.asset_id == id) {
                video.script_rich.insert_image(id, video.script_body.chars().count());
            }
            self.save();
        }
        if let Some(video_id) = video_id {
            self.navigate(Route::Video { video_id, section: Section::Script });
        }
    }

    #[cfg(test)]
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
        let selected = fig.session.ds().map(|ds| ds.selection.clone()).unwrap_or_default();
        let jobs = fig
            .session
            .doc()
            .ok()
            .map(|doc| {
                if selected.is_empty() {
                    let name = fig.session.page().ok().and_then(|id| doc.node(id)).map_or("Page", |n| n.name.as_str());
                    vec![(Vec::new(), name.to_owned(), 1.0)]
                } else {
                    selected
                        .iter()
                        .filter_map(|id| {
                            doc.node(*id).map(|node| {
                                let suffix = node.exports.first().map(|e| e.file_suffix()).unwrap_or_default();
                                let scale = node.exports.first().map_or(1.0, |e| figcraft_engine::cmd::file::export_scale(doc, &[*id], e));
                                (vec![*id], format!("{}{suffix}", node.name), scale)
                            })
                        })
                        .collect()
                }
            })
            .unwrap_or_default();
        let dir = self.store.video_dir(id).join("assets");
        if let Err(error) = fs::create_dir_all(&dir) {
            self.message = error.to_string();
            return;
        }
        let mut exported = 0;
        for (ids, name, scale) in jobs {
            let asset_id = self.store.data.id();
            let file = format!("design-{asset_id}.png");
            let path = dir.join(&file);
            if let Err(error) = fig.session.execute("file.export", &json!({"format":"png","path":path,"ids":ids,"scale":scale})) {
                self.message = format!("디자인 내보내기 실패: {error}");
                return;
            }
            let stem = name.rsplit('/').next().unwrap_or("Untitled").trim();
            let stem = if stem.is_empty() { "Untitled" } else { stem };
            if let Some(video) = self.store.data.selected_mut() {
                let mut name = format!("{stem}.png");
                let mut index = 2;
                while video.assets.iter().any(|a| a.name == name) {
                    name = format!("{stem} ({index}).png");
                    index += 1;
                }
                video.assets.push(Asset { id: asset_id, name, file, kind: model::AssetKind::Image, duration: 5.0, has_audio: false });
                exported += 1;
            }
            self.save();
        }
        self.toast = Some((format!("{exported}개 디자인을 라이브러리에 저장했습니다"), std::time::Instant::now()));
    }

    fn poll_jobs(&mut self, ctx: &egui::Context) {
        self.poll_imports(ctx);
        self.poll_auth(ctx);
        self.poll_catalog(ctx);
        self.backup.poll(&self.store.root, self.store.data.backup_directory.as_deref(), &self.store.data.backup_id, ctx);
        if let Some(due) = self.zoom_save_due {
            if due.elapsed() >= std::time::Duration::from_millis(300) {
                self.zoom_save_due = None;
                if let Err(error) = self.store.save() {
                    self.message = error;
                }
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }
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
                    self.thumbnail_recent.retain(|used| *used != id);
                    self.thumbnail_recent.push(id);
                    while self.thumbnail_recent.len() > 32 {
                        let old = self.thumbnail_recent.remove(0);
                        self.thumbnails.remove(&old);
                    }
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
        self.poll_chat(ctx);
    }

    pub fn design_frame(&mut self, fig: &mut FigApp, ctx: &egui::Context) {
        self.design_canvas = Some(fig.canvas.rect);
        if self.store.data.backup_directory.is_none() || !fig.session.ds().is_ok_and(|ds| ds.is_dirty()) {
            return;
        }
        let interval = std::time::Duration::from_secs(3);
        let wait = self.design_backup_saved.map_or(std::time::Duration::ZERO, |at| interval.saturating_sub(at.elapsed()));
        if wait.is_zero() {
            self.design_backup_saved = Some(std::time::Instant::now());
            if let Err(error) = fig.session.execute("file.save", &json!({})) {
                self.message = error.to_string();
            }
        } else {
            ctx.request_repaint_after(wait);
        }
    }

    fn ask_codex(&mut self, request: String) {
        let Some(video) = self.video().cloned() else { return };
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
            "당신은 이 영상 프로젝트의 제작 보조자입니다. 한국어로 답하세요. 첫 행동으로 purpose MCP의 read_guide를 호출해 공통·채널 가이드를 읽고 준수하세요. 디자인·FilmCraft 편집·라이브러리는 purpose 도구로 직접 수정하고 결과를 확인하세요. 대본은 읽기 전용이며 수정안을 답변으로만 제안하세요. 대본 파일이나 workspace.json을 직접 쓰지 마세요. 현재 탭: {:?}\n영상: {}\n대본:\n{}\n라이브러리:\n{}\n이전 타임라인:\n{clips}\nFilmCraft 프로젝트:\n{native_edit}",
            self.section, video.name, script, assets
        );
        self.start_chat(request, prompt);
    }
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

#[cfg(test)]
mod workspace_regressions {
    use super::*;
    #[test]
    fn selected_design_exports_to_a_real_library_png_and_can_enter_the_native_editor() {
        let root = std::env::temp_dir().join(format!("purpose-design-export-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let mut maker = VideoMaker::at(root.clone()).unwrap();
        let mut fig = FigApp::new(Session::new(), Box::new(figcraft_ui_egui::NoServices));
        maker.ensure_design(&mut fig);
        fig.session.execute("shape.rectangle", &json!({"x":0,"y":0,"width":96,"height":64})).unwrap();
        let selected = fig.session.selection().to_vec();
        fig.session.execute("node.set", &json!({"ids":selected,"props":{"name":"썸네일 원본"}})).unwrap();
        maker.export_design(&mut fig);
        let asset = maker.video().unwrap().assets[0].clone();
        assert_eq!(asset.name, "썸네일 원본.png");
        maker.export_design(&mut fig);
        assert_eq!(maker.video().unwrap().assets[1].name, "썸네일 원본 (2).png");
        let file = maker.store.asset_path(maker.video_id().unwrap(), &asset);
        let pixels = image::open(&file).unwrap();
        assert_eq!(pixels.width(), 96);
        assert_eq!(pixels.height(), 64);
        maker.append_film_asset(asset.id).unwrap();
        assert_eq!(maker.film.as_ref().unwrap().app.session.active_sequence().unwrap().video_tracks[0].items.len(), 1);
        maker.shutdown_film();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn selected_script_formatting_images_and_channel_folding_survive_reopen() {
        let root = std::env::temp_dir().join(format!("purpose-rich-script-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let mut maker = VideoMaker::at(root.clone()).unwrap();
        let channel = maker.store.data.channels[0].id;
        maker.store.data.collapsed_channels.push(channel);
        let video = maker.store.data.selected_mut().unwrap();
        video.script_body = "첫 문단\n둘째 문단".into();
        video.script_rich.apply(0..2, |s| {
            s.bold = true;
            s.italic = true;
            s.underline = true;
            s.size = Some(23.0);
            s.line_height = Some(1.8);
        });
        video.script_rich.set_kind(&video.script_body, 5..10, rich_script::TextKind::H2);
        video.script_rich.insert_image(55, 5);
        video.script_rich.images_migrated = true;
        let rich = video.script_rich.clone();
        maker.save();
        let loaded = VideoMaker::at(root.clone()).unwrap();
        assert_eq!(loaded.video().unwrap().script_rich, rich);
        assert_eq!(loaded.store.data.collapsed_channels, vec![channel]);
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn document_size_and_line_spacing_survive_reopen() {
        let root = std::env::temp_dir().join(format!("purpose-script-style-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let mut maker = VideoMaker::at(root.clone()).unwrap();
        let video = maker.store.data.selected_mut().unwrap();
        video.script_font_size = 22.0;
        video.script_line_height = 1.9;
        video.script_body = "\t위계\n\t\t본문".into();
        maker.save();
        let loaded = VideoMaker::at(root.clone()).unwrap();
        let video = loaded.video().unwrap();
        assert_eq!(video.script_font_size, 22.0);
        assert_eq!(video.script_line_height, 1.9);
        assert_eq!(video.script_body, "\t위계\n\t\t본문");
        let _ = fs::remove_dir_all(root);
    }
}
