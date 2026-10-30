//! Embedded FilmCraft editor, one native project per video.
use super::{
    VideoMaker,
    film_audio::CpalOut,
    film_audio_in::CpalIn,
    model::{Asset, AssetKind, Store, Video},
};
use filmcraft_engine::Session;
use filmcraft_project::{BinEntry, BinId, ItemId, MediaRef};
use filmcraft_time::Tick;
use filmcraft_ui_egui::FilmcraftApp;
use serde_json::json;
use std::{
    fs,
    path::{Component, Path, PathBuf},
    sync::Once,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub(super) struct Editor {
    pub app: FilmcraftApp,
    pub video_id: u64,
    seen_revision: u64,
    seen_epoch: u64,
    save_due: Option<Instant>,
    gpu_ready: bool,
    library_signature: Vec<(u64, String)>,
    library_sync_after: Instant,
    library_bin: BinId,
    source_observation: std::collections::HashMap<ItemId, (PathBuf, bool, bool)>,
    history_position: (usize, usize),
}

fn execute(session: &mut Session, id: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
    session.execute(id, params).map_err(|e| format!("FilmCraft {id}: {e}"))
}

fn project_path(store: &Store, id: u64) -> PathBuf {
    store.video_dir(id).join("edit.fcproj")
}

/// Write completely before replacing the current project, including on Windows.
fn replace_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temp = path.with_extension("fcproj.restore.tmp");
    let backup = path.with_extension("fcproj.restore.bak");
    fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    if fs::rename(&temp, path).is_err() {
        if backup.exists() {
            return Err("이전 복원 백업이 남아 있습니다. 편집 파일을 교체하지 않았습니다".into());
        }
        fs::rename(path, &backup).map_err(|e| e.to_string())?;
        if let Err(error) = fs::rename(&temp, path) {
            let _ = fs::rename(&backup, path);
            return Err(error.to_string());
        }
        let _ = fs::remove_file(backup);
    }
    Ok(())
}

fn clipboard_media() -> Option<filmcraft_ui_egui::ClipboardMedia> {
    let mut clipboard = arboard::Clipboard::new().ok()?;
    if let Ok(files) = clipboard.get().file_list()
        && !files.is_empty()
    {
        return Some(filmcraft_ui_egui::ClipboardMedia::Files(files.iter().map(|p| p.to_string_lossy().into_owned()).collect()));
    }
    let image = clipboard.get_image().ok()?;
    Some(filmcraft_ui_egui::ClipboardMedia::Image { width: image.width, height: image.height, rgba: image.bytes.into_owned() })
}

pub(super) fn open_path(path: &str, reveal: bool) -> Result<(), String> {
    let mut command = if cfg!(target_os = "windows") {
        let mut cmd = std::process::Command::new("explorer");
        cmd.arg(if reveal { format!("/select,{path}") } else { path.into() });
        cmd
    } else if cfg!(target_os = "macos") {
        let mut cmd = std::process::Command::new("open");
        if reveal {
            cmd.arg("-R");
        }
        cmd.arg(path);
        cmd
    } else {
        let mut cmd = std::process::Command::new("xdg-open");
        let path = Path::new(path);
        cmd.arg(if reveal { path.parent().unwrap_or(path) } else { path });
        cmd
    };
    command.spawn().map(|_| ()).map_err(|e| e.to_string())
}

struct GpuFrameRenderer(filmcraft_gpu::ExportRenderer);
impl filmcraft_export::FrameRenderer for GpuFrameRenderer {
    fn render(
        &mut self,
        project: &filmcraft_project::Project,
        sequence: ItemId,
        time: Tick,
        opts: filmcraft_render::RenderOptions,
        sources: &dyn filmcraft_render::SourceProvider,
    ) -> Option<filmcraft_render::Image> {
        self.0.render(project, sequence, time, opts, sources)
    }
}

fn revision_path(store: &Store, id: u64, name: &str) -> Result<PathBuf, String> {
    let mut components = Path::new(name).components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return Err("잘못된 편집 기록 파일 이름입니다".into());
    }
    Ok(store.video_dir(id).join("edit-revisions").join(name))
}

fn existing_item(session: &Session, path: &Path) -> Option<ItemId> {
    session.project.items.values().find_map(|item| {
        let media = item.as_media()?;
        match &media.media {
            MediaRef::File { path: source } if Path::new(source) == path || session.media.follow(source).is_ok_and(|p| Path::new(&p) == path) => {
                Some(item.id)
            }
            _ => None,
        }
    })
}

fn import_asset(session: &mut Session, store: &Store, video: u64, asset: &Asset) -> Result<ItemId, String> {
    let path = store.asset_path(video, asset);
    if let Some(id) = existing_item(session, &path) {
        if session.project.item(id).and_then(|i| i.as_media()).is_some_and(|m| m.offline) {
            execute(session, "media.relink", json!({"item":id.0,"path":path,"force":true,"relinkOthers":false}))?;
        }
        sync_item_name(session, id, asset)?;
        return Ok(id);
    }
    let detect = session.prefs.media.import_image_sequences;
    session.prefs.media.import_image_sequences = false;
    let result = execute(session, "file.import", json!({"paths":[path],"imageSequence":false}));
    session.prefs.media.import_image_sequences = detect;
    let result = result?;
    let id = result["items"]
        .as_array()
        .and_then(|ids| ids.first())
        .and_then(|id| id.as_u64())
        .map(ItemId)
        .ok_or_else(|| format!("{}: {}", asset.name, result["errors"]))?;
    sync_item_name(session, id, asset)?;
    Ok(id)
}
fn sync_item_name(session: &mut Session, id: ItemId, asset: &Asset) -> Result<(), String> {
    let old = session.project.item(id).map(|item| item.name.clone()).ok_or("편집 소스가 없습니다")?;
    let clips: Vec<_> = session
        .project
        .items
        .values()
        .filter_map(|item| item.as_sequence().map(|seq| (item.id, seq)))
        .flat_map(|(sequence, seq)| {
            seq.video_tracks.iter().chain(&seq.audio_tracks).flat_map(move |track| track.items.iter().map(move |clip| (sequence, clip)))
        })
        .filter(|(_, clip)| clip.item == id && (clip.name == old || clip.name == asset.file) && clip.name != asset.name)
        .map(|(seq, clip)| (seq, clip.id))
        .collect();
    if old != asset.name {
        execute(session, "clip.rename", json!({"item":id.0,"name":asset.name}))?;
    }
    // Use commands for names while preserving the user's active timeline, tabs and selection.
    let state = session.state.clone();
    let event_count = session.events.len();
    let result = (|| {
        for (sequence, clip) in clips {
            if session.state.active_sequence != Some(sequence) {
                execute(session, "sequence.open", json!({"item":sequence.0}))?;
            }
            execute(session, "clip.rename", json!({"clip":clip.0,"name":asset.name}))?;
        }
        Ok(())
    })();
    session.state = state;
    let generated = session.events.split_off(event_count);
    session.events.extend(generated.into_iter().filter(|event| !matches!(event, filmcraft_engine::Event::OpenSequence(_))));
    result
}

fn library_bin(session: &mut Session, name: &str) -> Result<BinId, String> {
    if let Some(id) = session.project.root.children.iter().find_map(|c| match c {
        BinEntry::Bin(b) if b.name == name => Some(b.id),
        _ => None,
    }) {
        return Ok(id);
    }
    execute(session, "file.newBin", json!({"name":name}))?["bin"].as_u64().map(BinId).ok_or_else(|| "소스 폴더를 만들 수 없습니다".into())
}
fn observe_sources(session: &Session, bin: BinId) -> std::collections::HashMap<ItemId, (PathBuf, bool, bool)> {
    let mut members = Vec::new();
    if let Some(bin) = session.project.root.find_bin(bin) {
        bin.all_items(&mut members);
    }
    session
        .project
        .items
        .values()
        .filter_map(|item| {
            let media = item.as_media()?;
            let MediaRef::File { path } = &media.media else {
                return None;
            };
            Some((item.id, (PathBuf::from(session.media.follow(path).ok()?), !media.offline, members.contains(&item.id))))
        })
        .collect()
}
fn reconcile_library(session: &mut Session, store: &Store, video: &Video) -> Result<BinId, String> {
    // Automatic bookkeeping must not consume the user's native undo/redo steps.
    let history = session.history.clone();
    let result = reconcile_library_inner(session, store, video);
    session.history = history;
    result
}
fn reconcile_library_inner(session: &mut Session, store: &Store, video: &Video) -> Result<BinId, String> {
    let bin = library_bin(session, "라이브러리")?;
    let mut active = Vec::new();
    let mut errors = Vec::new();
    for asset in video.assets.iter().filter(|a| a.kind.can_edit()) {
        match import_asset(session, store, video.id, asset) {
            Ok(id) => active.push(id),
            Err(error) => errors.push(format!("{}: {error}", asset.name)),
        }
    }
    let moved: Vec<_> = active.iter().filter(|id| session.project.root.parent_of(**id) != Some(bin)).map(|id| id.0).collect();
    if !moved.is_empty() {
        execute(session, "project.moveToBin", json!({"items":moved,"bin":bin.0}))?;
    }
    // Keep removed/duplicate media outside the library without offlining existing timeline clips.
    let managed: std::collections::HashSet<_> = video.assets.iter().chain(&video.removed_assets).map(|a| store.asset_path(video.id, a)).collect();
    let archive: Vec<_> = session
        .project
        .items
        .values()
        .filter(|item| !active.contains(&item.id))
        .filter_map(|item| {
            let MediaRef::File { path } = &item.as_media()?.media else {
                return None;
            };
            let path = session.media.follow(path).unwrap_or_else(|_| path.clone());
            managed.contains(Path::new(&path)).then_some(item.id)
        })
        .collect();
    if !archive.is_empty() {
        let folder = library_bin(session, "보관 소스")?;
        let moved: Vec<_> = archive.iter().filter(|id| session.project.root.parent_of(**id) != Some(folder)).map(|id| id.0).collect();
        if !moved.is_empty() {
            execute(session, "project.moveToBin", json!({"items":moved,"bin":folder.0}))?;
        }
    }
    if errors.is_empty() { Ok(bin) } else { Err(format!("편집 소스 동기화 실패: {}", errors.join(" · "))) }
}

fn new_sequence(session: &mut Session, name: &str) -> Result<(), String> {
    execute(session, "file.newSequence", json!({"name":name,"width":1280,"height":720,"fps":30.0,"video":3,"audio":3}))?;
    Ok(())
}

/// Convert only once from the old float-based editor to FilmCraft's integer tick timeline.
fn legacy_session(store: &Store, video: &Video) -> Result<Session, String> {
    let mut session = Session::default();
    execute(&mut session, "file.newProject", json!({"name":video.name}))?;
    new_sequence(&mut session, &format!("{} · 편집 시퀀스", video.name))?;
    let mut cursor = Tick::ZERO;
    for clip in &video.clips {
        let asset = video.asset(clip.asset_id).ok_or_else(|| format!("이전 타임라인의 에셋 {}를 찾을 수 없습니다", clip.asset_id))?;
        if !clip.trim_start.is_finite()
            || !clip.trim_end.is_finite()
            || clip.trim_end <= clip.trim_start
            || !clip.start.is_finite()
            || !clip.volume.is_finite()
        {
            return Err("이전 타임라인에 잘못된 시간 또는 볼륨이 있습니다".into());
        }
        let item = import_asset(&mut session, store, video.id, asset)?;
        let duration = Tick::from_seconds_f64(clip.duration());
        let at = if clip.audio_track { Tick::from_seconds_f64(clip.start) } else { cursor };
        let placed = execute(
            &mut session,
            "timeline.place",
            json!({"item":item.0,"track":"V1","audioTrack":if clip.audio_track {"A2"}else{"A1"},"time":at.0,"sourceIn":Tick::from_seconds_f64(clip.trim_start).0,"duration":duration.0,"video":!clip.audio_track,"audio":clip.audio_track||asset.has_audio,"insert":false}),
        )?;
        if !clip.audio_track {
            execute(&mut session, "clip.fitToFrame", json!({}))?;
        }
        if (clip.volume - 1.0).abs() > f64::EPSILON && (clip.audio_track || asset.has_audio) {
            let db = if clip.volume <= 0.0 { -96.0 } else { (20.0 * clip.volume.log10()).clamp(-96.0, 96.0) };
            execute(&mut session, "clip.audioGain", json!({"clips":placed["clips"],"mode":"set","db":db}))?;
        }
        if !clip.audio_track {
            cursor += duration;
        }
    }
    for caption in &video.captions {
        if !caption.start.is_finite() || !caption.end.is_finite() || caption.end <= caption.start {
            return Err("이전 자막의 시간이 잘못되었습니다".into());
        }
        execute(
            &mut session,
            "captions.add",
            json!({"text":caption.text,"time":Tick::from_seconds_f64(caption.start).0,"durationSeconds":caption.end-caption.start}),
        )?;
    }
    for asset in video.assets.iter().filter(|a| a.kind != AssetKind::Design) {
        // An unsupported unused library file stays in the library; timeline sources must import.
        if let Err(error) = import_asset(&mut session, store, video.id, asset) {
            log::warn!("{error}");
        }
    }
    Ok(session)
}

impl VideoMaker {
    pub(super) fn sync_film_name(&mut self, video_id: u64, asset_id: u64) -> Result<(), String> {
        let asset = self
            .store
            .data
            .channels
            .iter()
            .flat_map(|c| &c.videos)
            .find(|v| v.id == video_id)
            .and_then(|v| v.asset(asset_id))
            .cloned()
            .ok_or("자료를 찾을 수 없습니다")?;
        if !asset.kind.can_edit() {
            return Ok(());
        }
        if let Some(editor) = self.film.as_mut().filter(|e| e.video_id == video_id) {
            import_asset(&mut editor.app.session, &self.store, video_id, &asset)?;
            self.save_film()?;
        }
        Ok(())
    }
    fn sync_native_source_actions(&mut self) {
        let mut actions = Vec::new();
        let video;
        {
            let Some(editor) = &mut self.film else {
                return;
            };
            video = editor.video_id;
            let session = &editor.app.session;
            let position = (session.history.undo.len(), session.history.redo.len());
            let label = if position.0 < editor.history_position.0 && position.1 > editor.history_position.1 {
                session.history.redo.last().map(|(label, _)| label.as_str())
            } else {
                session.history.undo.last().map(|(label, _)| label.as_str())
            };
            let observed = observe_sources(session, editor.library_bin);
            if matches!(label, Some("Clear" | "Import")) && (position != editor.history_position || observed != editor.source_observation) {
                for (id, (path, present, member)) in &editor.source_observation {
                    if *member && *present && observed.get(id).is_none_or(|(_, present, _)| !*present) {
                        actions.push((path.clone(), false));
                    }
                }
                for (id, (path, present, member)) in &observed {
                    if *member && *present && editor.source_observation.get(id).is_none_or(|(_, present, _)| !*present) {
                        actions.push((path.clone(), true));
                    }
                }
            }
            editor.source_observation = observed;
            editor.history_position = position;
        }
        if actions.is_empty() {
            return;
        }
        if let Some(target) = self.store.data.channels.iter_mut().flat_map(|c| &mut c.videos).find(|v| v.id == video) {
            let directory = self.store.root.join("videos").join(video.to_string()).join("assets");
            for (path, present) in actions {
                if present {
                    if let Some(index) = target.removed_assets.iter().position(|a| directory.join(&a.file) == path) {
                        target.assets.push(target.removed_assets.remove(index));
                    }
                } else if let Some(index) = target.assets.iter().position(|a| directory.join(&a.file) == path) {
                    target.removed_assets.push(target.assets.remove(index));
                }
            }
        }
        self.save();
    }
    fn sync_native_imports(&mut self) {
        let Some(editor) = &self.film else {
            return;
        };
        let video_id = editor.video_id;
        let Some(video) = self.store.data.channels.iter().flat_map(|c| &c.videos).find(|v| v.id == video_id) else {
            return;
        };
        let managed: std::collections::HashSet<_> =
            video.assets.iter().chain(&video.removed_assets).map(|a| self.store.asset_path(video_id, a)).collect();
        let files: Vec<_> = editor
            .app
            .session
            .project
            .items
            .values()
            .filter_map(|item| {
                let MediaRef::File { path } = &item.as_media()?.media else {
                    return None;
                };
                let path = PathBuf::from(editor.app.session.media.follow(path).ok()?);
                (!managed.contains(&path) && !self.film_import_paths.contains(&(video_id, item.id, path.clone()))).then_some((
                    item.id,
                    path,
                    item.name.clone(),
                ))
            })
            .collect();
        for (item, path, name) in files {
            self.film_import_paths.insert((video_id, item, path.clone()));
            match self.queue_import(video_id, path, Some(name)) {
                Ok(id) => {
                    self.film_imports.insert(id, (video_id, item));
                }
                Err(error) => self.message = error,
            }
        }
    }
    pub(super) fn complete_film_import(&mut self, asset: &Asset) -> Result<(), String> {
        let Some((video, item)) = self.film_imports.remove(&asset.id) else {
            return Ok(());
        };
        let path = self.store.asset_path(video, asset);
        let params = json!({"item":item.0,"path":path,"force":true,"relinkOthers":false});
        if let Some(editor) = self.film.as_mut().filter(|e| e.video_id == video) {
            if editor.app.session.project.item(item).is_some() {
                let history = editor.app.session.history.clone();
                let result =
                    execute(&mut editor.app.session, "media.relink", params).and_then(|_| sync_item_name(&mut editor.app.session, item, asset));
                editor.app.session.history = history;
                result?;
            }
            self.save_film()?;
        } else {
            let project = project_path(&self.store, video);
            if project.exists() {
                let mut session = Session::default();
                execute(&mut session, "file.open", json!({"path":project}))?;
                if session.project.item(item).is_some() {
                    execute(&mut session, "media.relink", params)?;
                    sync_item_name(&mut session, item, asset)?;
                }
                execute(&mut session, "file.save", json!({"path":project}))?;
                let name = format!("edit-{}.fcproj", SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos());
                let snapshot = revision_path(&self.store, video, &name)?;
                if let Some(dir) = snapshot.parent() {
                    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
                }
                fs::copy(project, snapshot).map_err(|e| e.to_string())?;
                if let Some(v) = self.store.data.channels.iter_mut().flat_map(|c| &mut c.videos).find(|v| v.id == video) {
                    v.film_revision = Some(name);
                }
                session.shutdown();
            }
        }
        Ok(())
    }
    pub fn section_is_edit(&self) -> bool {
        self.tabs.route().and_then(super::navigation::Route::video).is_some_and(|(_, section)| section == super::model::Section::Edit)
    }

    pub(super) fn ensure_film(&mut self) -> Result<(), String> {
        let video = self.video().cloned().ok_or("영상을 먼저 선택하세요")?;
        if let Some(editor) = self.film.as_mut().filter(|editor| editor.video_id == video.id) {
            let signature: Vec<_> = video
                .assets
                .iter()
                .map(|a| (a.id, format!("active:{}", a.name)))
                .chain(video.removed_assets.iter().map(|a| (a.id, format!("removed:{}", a.name))))
                .collect();
            if editor.library_signature != signature && Instant::now() >= editor.library_sync_after {
                match reconcile_library(&mut editor.app.session, &self.store, &video) {
                    Ok(bin) => {
                        editor.library_signature = signature;
                        editor.library_sync_after = Instant::now();
                        editor.library_bin = bin;
                        editor.source_observation = observe_sources(&editor.app.session, bin);
                        editor.history_position = (editor.app.session.history.undo.len(), editor.app.session.history.redo.len());
                    }
                    Err(error) => {
                        editor.library_sync_after = Instant::now() + Duration::from_secs(5);
                        self.message = error;
                    }
                }
            }
            return Ok(());
        }
        self.save_film()?;
        if let Some(mut editor) = self.film.take() {
            editor.app.stop();
            editor.app.session.shutdown();
        }
        static MEDIA_INIT: Once = Once::new();
        MEDIA_INIT.call_once(|| {
            let _ = filmcraft_platform::register();
            filmcraft_text::fonts::scan_system();
            filmcraft_export::register_frame_renderer(|| {
                filmcraft_gpu::ExportRenderer::new().map(|r| Box::new(GpuFrameRenderer(r)) as Box<dyn filmcraft_export::FrameRenderer>)
            });
        });
        let path = project_path(&self.store, video.id);
        if !path.exists()
            && let Some(name) = &video.film_revision
        {
            let bytes =
                fs::read(revision_path(&self.store, video.id, name)?).map_err(|e| format!("편집 기록에서 프로젝트를 복구할 수 없습니다: {e}"))?;
            replace_file(&path, &bytes)?;
        }
        let mut session = if path.exists() {
            let mut session = Session::default();
            execute(&mut session, "file.open", json!({"path":path}))?;
            session
        } else {
            legacy_session(&self.store, &video)?
        };
        // A sequence is the editing workspace, never the script document or an imported source.
        let sequences: Vec<_> = session
            .project
            .items
            .values()
            .filter(|item| item.as_sequence().is_some() && (item.name == video.name || item.name == format!("{}의 대본", video.name)))
            .map(|item| item.id)
            .collect();
        for id in sequences {
            execute(&mut session, "clip.rename", json!({"item":id.0,"name":format!("{} · 편집 시퀀스",video.name)}))?;
        }
        fs::create_dir_all(self.store.video_dir(video.id)).map_err(|e| e.to_string())?;
        let mut config = filmcraft_engine::autosave::AutosaveConfig::new(self.store.video_dir(video.id).join("editor-state"));
        config.journal_debounce = Duration::from_secs(2);
        session.start_autosave(config).map_err(|e| format!("편집 자동 저장 초기화 실패: {e}"))?;
        if session.prefs_path.as_ref().is_none_or(|path| !path.exists()) {
            session.prefs.appearance.appearance_mode = "light".into();
            session.prefs.appearance.light_theme = "light".into();
        }
        session.prefs.general.interface_language = "ko".into();
        let library_bin = library_bin(&mut session, "라이브러리")?;
        if let Err(error) = reconcile_library(&mut session, &self.store, &video) {
            self.message = error;
        }
        session.voiceover.input = Some(Box::new(CpalIn::new(&session.prefs.audio_hardware.device_class)));
        let seen_revision = session.revision;
        let seen_epoch = session.document_epoch;
        let mut app = FilmcraftApp::new(session);
        app.embedded = true;
        app.ui.project_panel.bin = Some(library_bin.0);
        app.set_startup_update_check(false);
        app.audio = Some(Box::new(CpalOut::new()));
        app.hooks.open_url = Some(Box::new(|url| webbrowser::open(url).map(|_| ()).map_err(|e| e.to_string())));
        app.hooks.open_path = Some(Box::new(open_path));
        app.hooks.clipboard_media = Some(Box::new(clipboard_media));
        app.hooks.pick_files = Some(Box::new(|_| {
            rfd::FileDialog::new().pick_files().unwrap_or_default().into_iter().map(|p| p.to_string_lossy().into_owned()).collect()
        }));
        app.hooks.pick_open_project =
            Some(Box::new(|| rfd::FileDialog::new().add_filter("FilmCraft", &["fcproj"]).pick_file().map(|p| p.to_string_lossy().into_owned())));
        app.hooks.pick_save = Some(Box::new(|name| rfd::FileDialog::new().set_file_name(name).save_file().map(|p| p.to_string_lossy().into_owned())));
        app.hooks.pick_save_as = Some(Box::new(|filter, extensions, name| {
            rfd::FileDialog::new().add_filter(filter, extensions).set_file_name(name).save_file().map(|p| p.to_string_lossy().into_owned())
        }));
        app.hooks.pick_open_file = Some(Box::new(|filter, extensions| {
            rfd::FileDialog::new().add_filter(filter, extensions).pick_file().map(|p| p.to_string_lossy().into_owned())
        }));
        app.hooks.pick_folder = Some(Box::new(|| rfd::FileDialog::new().pick_folder().map(|p| p.to_string_lossy().into_owned())));
        app.hooks.pick_folder_at =
            Some(Box::new(|path| rfd::FileDialog::new().set_directory(path).pick_folder().map(|p| p.to_string_lossy().into_owned())));
        app.hooks.pick_file_for_relink = Some(Box::new(|_, _| rfd::FileDialog::new().pick_file().map(|p| p.to_string_lossy().into_owned())));
        app.hooks.cursor_screen_position = Some(Box::new(filmcraft_platform::cursor::cursor_screen_position));
        self.film = Some(Editor {
            video_id: video.id,
            seen_revision,
            seen_epoch,
            save_due: None,
            gpu_ready: false,
            source_observation: observe_sources(&app.session, library_bin),
            history_position: (app.session.history.undo.len(), app.session.history.redo.len()),
            library_bin,
            library_signature: Vec::new(),
            library_sync_after: Instant::now(),
            app,
        });
        if !path.exists() || video.film_revision.is_none() {
            self.save_film()?;
        }
        Ok(())
    }

    pub(super) fn append_film_asset(&mut self, id: u64) -> Result<(), String> {
        self.ensure_film()?;
        let video = self.video().cloned().ok_or("영상을 선택하세요")?;
        let asset = video.asset(id).ok_or("라이브러리 파일이 없습니다")?;
        let editor = self.film.as_mut().ok_or("편집기를 열 수 없습니다")?;
        let item = import_asset(&mut editor.app.session, &self.store, video.id, asset)?;
        if editor.app.session.active_sequence().is_none() {
            new_sequence(&mut editor.app.session, &format!("{} · 편집 시퀀스", video.name))?;
        }
        let at = editor.app.session.active_sequence().map(|s| s.duration()).unwrap_or(Tick::ZERO);
        execute(
            &mut editor.app.session,
            "timeline.place",
            json!({"item":item.0,"track":"V1","audioTrack":if asset.kind==AssetKind::Audio {"A2"}else{"A1"},"time":at.0,"duration":Tick::from_seconds_f64(asset.duration).0,"video":asset.kind!=AssetKind::Audio,"audio":asset.kind==AssetKind::Audio||asset.has_audio,"insert":false}),
        )?;
        if asset.kind != AssetKind::Audio {
            execute(&mut editor.app.session, "clip.fitToFrame", json!({}))?;
        }
        self.save_film()
    }

    pub(super) fn save_film(&mut self) -> Result<(), String> {
        let Some(editor) = self.film.as_mut() else { return Ok(()) };
        let path = project_path(&self.store, editor.video_id);
        let has_snapshot =
            self.store.data.channels.iter().flat_map(|c| &c.videos).find(|v| v.id == editor.video_id).is_some_and(|v| v.film_revision.is_some());
        let bound_to_video = editor.app.session.path.as_deref().is_some_and(|saved| Path::new(saved) == path);
        if !editor.app.session.is_dirty() && editor.save_due.is_none() && bound_to_video && path.exists() && has_snapshot {
            return Ok(());
        }
        // If saving the native file succeeds but its history copy fails, retry the copy.
        editor.save_due = Some(Instant::now());
        execute(&mut editor.app.session, "file.save", json!({"path":path}))?;
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
        let name = format!("edit-{stamp}.fcproj");
        let snapshot = revision_path(&self.store, editor.video_id, &name)?;
        if let Some(dir) = snapshot.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        fs::copy(&path, &snapshot).map_err(|e| format!("편집 기록 저장 실패: {e}"))?;
        editor.seen_revision = editor.app.session.revision;
        editor.seen_epoch = editor.app.session.document_epoch;
        editor.save_due = None;
        if let Some(video) = self.store.data.channels.iter_mut().flat_map(|c| &mut c.videos).find(|v| v.id == editor.video_id) {
            video.film_revision = Some(name);
        }
        self.save();
        Ok(())
    }

    pub(super) fn restore_film_revision(&mut self, video: &Video) -> Result<(), String> {
        self.save_film()?;
        if let Some(mut editor) = self.film.take() {
            editor.app.stop();
            editor.app.session.shutdown();
        }
        let path = project_path(&self.store, video.id);
        if let Some(name) = &video.film_revision {
            let bytes = fs::read(revision_path(&self.store, video.id, name)?).map_err(|e| format!("편집 기록을 읽을 수 없습니다: {e}"))?;
            replace_file(&path, &bytes)?;
        } else if path.exists() {
            let mut session = legacy_session(&self.store, video)?;
            execute(&mut session, "file.save", json!({"path":path}))?;
        }
        Ok(())
    }

    pub fn film_logic(&mut self, ctx: &egui::Context) {
        let active = self.section_is_edit();
        let mut save = false;
        if let Some(editor) = &mut self.film {
            if active {
                editor.app.logic_inner(ctx);
            } else {
                editor.app.session.pump_jobs(Duration::from_millis(3));
                editor.app.session.poll_persistence();
            }
            if editor.seen_epoch != editor.app.session.document_epoch {
                editor.library_signature.clear();
                editor.library_sync_after = Instant::now();
            }
            if editor.seen_revision != editor.app.session.revision || editor.seen_epoch != editor.app.session.document_epoch {
                editor.seen_revision = editor.app.session.revision;
                editor.seen_epoch = editor.app.session.document_epoch;
                editor.save_due = Some(Instant::now());
            }
            if let Some(due) = editor.save_due {
                if due.elapsed() >= Duration::from_secs(2) {
                    save = true;
                } else {
                    ctx.request_repaint_after(Duration::from_millis(250));
                }
            }
        }
        self.sync_native_source_actions();
        self.sync_native_imports();
        if save && let Err(error) = self.save_film() {
            self.message = error;
        }
    }

    pub fn film_raw_input(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        if self.section_is_edit()
            && let Some(editor) = &mut self.film
        {
            eframe::App::raw_input_hook(&mut editor.app, ctx, input);
        }
    }

    pub fn film_ui(&mut self, ui: &mut egui::Ui, gpu: Option<eframe::egui_wgpu::RenderState>) {
        if let Err(error) = self.ensure_film() {
            self.message = error.clone();
            egui::CentralPanel::default().show(ui, |ui| {
                ui.label(error);
            });
            return;
        }
        if !self.film_ime_ready {
            filmcraft_ui_egui::install_ime_reset_callbacks(ui.ctx());
            self.film_ime_ready = true;
        }
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| {
            if let Some(editor) = &mut self.film {
                if !editor.gpu_ready
                    && let Some(gpu) = gpu
                {
                    editor.app.set_wgpu(gpu);
                    editor.gpu_ready = true;
                }
                editor.app.embedded_ui(ui);
            }
        });
    }

    pub fn shutdown_film(&mut self) {
        if let Some(mut editor) = self.film.take() {
            editor.app.stop();
            editor.app.session.shutdown();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::{Caption, Clip};
    use super::*;

    fn fixture(label: &str) -> (Store, Video) {
        let root = std::env::temp_dir().join(format!("purpose-film-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let mut store = Store::load(root).unwrap();
        let channel = store.data.add_channel();
        let id = store.data.add_video(channel).unwrap();
        let dir = store.video_dir(id).join("assets");
        fs::create_dir_all(&dir).unwrap();
        image::RgbaImage::from_pixel(32, 24, image::Rgba([25, 120, 220, 255])).save(dir.join("image.png")).unwrap();
        let audio: Vec<f32> = (0..48000).map(|i| ((i as f32 * 440.0 * std::f32::consts::TAU / 48000.0).sin()) * 0.1).collect();
        fs::write(dir.join("audio.wav"), filmcraft_export::write_wav(&audio, 1, 48000, filmcraft_export::SampleFormat::Int16).unwrap()).unwrap();
        let video = store.data.selected_mut().unwrap();
        video.assets = vec![
            Asset { id: 101, name: "image.png".into(), file: "image.png".into(), kind: AssetKind::Image, duration: 5.0, has_audio: false },
            Asset { id: 102, name: "audio.wav".into(), file: "audio.wav".into(), kind: AssetKind::Audio, duration: 1.0, has_audio: true },
        ];
        video.clips = vec![
            Clip { id: 201, asset_id: 101, trim_start: 0.0, trim_end: 1.0, start: 0.0, volume: 1.0, audio_track: false },
            Clip { id: 202, asset_id: 101, trim_start: 1.0, trim_end: 2.0, start: 0.0, volume: 1.0, audio_track: false },
            Clip { id: 203, asset_id: 102, trim_start: 0.1, trim_end: 0.6, start: 0.3, volume: 0.5, audio_track: true },
        ];
        video.captions = vec![Caption { id: 301, start: 0.1, end: 0.8, text: "Synthetic caption".into() }];
        let video = video.clone();
        (store, video)
    }

    fn library_count(maker: &VideoMaker) -> usize {
        let app = &maker.film.as_ref().unwrap().app;
        let bin = app.ui.project_panel.bin.unwrap();
        app.session.project.root.find_bin(BinId(bin)).unwrap().children.iter().filter(|c| matches!(c, BinEntry::Item(_))).count()
    }
    #[test]
    fn library_counts_survive_reopens_removal_restore_and_duplicate_native_imports() {
        let (store, video) = fixture("library-sync");
        store.save().unwrap();
        let mut maker = VideoMaker::at(store.root.clone()).unwrap();
        maker.ensure_film().unwrap();
        assert_eq!(library_count(&maker), 2);
        let path = store.asset_path(video.id, &video.assets[0]);
        execute(&mut maker.film.as_mut().unwrap().app.session, "file.import", json!({"paths":[path],"imageSequence":false})).unwrap();
        maker.save_film().unwrap();
        maker.shutdown_film();
        for _ in 0..3 {
            maker.ensure_film().unwrap();
            assert_eq!(library_count(&maker), 2);
            maker.save_film().unwrap();
            maker.shutdown_film();
        }
        maker.ensure_film().unwrap();
        let media_count = maker.film.as_ref().unwrap().app.session.project.items.values().filter(|i| i.as_media().is_some()).count();
        maker.remove_assets(video.id, &[101]).unwrap();
        maker.ensure_film().unwrap();
        assert_eq!(library_count(&maker), 1);
        let session = &maker.film.as_ref().unwrap().app.session;
        assert!(session.active_sequence().unwrap().video_tracks[0].items.iter().all(|c| session.project.item(c.item).is_some()));
        assert!(!session.project.item(existing_item(session, &path).unwrap()).unwrap().as_media().unwrap().offline);
        let asset = maker.store.data.selected_mut().unwrap().removed_assets.remove(0);
        maker.store.data.selected_mut().unwrap().assets.push(asset);
        maker.ensure_film().unwrap();
        assert_eq!(library_count(&maker), 2);
        assert_eq!(maker.film.as_ref().unwrap().app.session.project.items.values().filter(|i| i.as_media().is_some()).count(), media_count);
        maker.shutdown_film();
        fs::remove_dir_all(store.root).unwrap();
    }
    #[test]
    fn stored_relative_media_paths_reuse_the_original_native_item() {
        let (store, video) = fixture("relative-media");
        store.save().unwrap();
        let mut maker = VideoMaker::at(store.root.clone()).unwrap();
        maker.ensure_film().unwrap();
        maker.save_film().unwrap();
        let path = store.asset_path(video.id, &video.assets[0]);
        let original = existing_item(&maker.film.as_ref().unwrap().app.session, &path).unwrap();
        maker.shutdown_film();
        let project = project_path(&store, video.id);
        let mut data: serde_json::Value = serde_json::from_slice(&fs::read(&project).unwrap()).unwrap();
        fn relative(value: &mut serde_json::Value, absolute: &str) {
            match value {
                serde_json::Value::Object(map) => {
                    if map.get("path").and_then(|p| p.as_str()) == Some(absolute) {
                        map.insert("path".into(), json!("assets/image.png"));
                    }
                    for v in map.values_mut() {
                        relative(v, absolute);
                    }
                }
                serde_json::Value::Array(values) => {
                    for v in values {
                        relative(v, absolute);
                    }
                }
                _ => {}
            }
        }
        relative(&mut data, path.to_str().unwrap());
        fs::write(project, serde_json::to_vec(&data).unwrap()).unwrap();
        maker.ensure_film().unwrap();
        assert_eq!(existing_item(&maker.film.as_ref().unwrap().app.session, &path), Some(original));
        assert_eq!(library_count(&maker), 2);
        maker.shutdown_film();
        fs::remove_dir_all(store.root).unwrap();
    }
    #[test]
    fn native_clear_undo_and_redo_move_library_metadata_without_losing_source_files() {
        let (store, video) = fixture("clear-sync");
        store.save().unwrap();
        let mut maker = VideoMaker::at(store.root.clone()).unwrap();
        maker.ensure_film().unwrap();
        let path = store.asset_path(video.id, &video.assets[0]);
        let item = existing_item(&maker.film.as_ref().unwrap().app.session, &path).unwrap();
        execute(&mut maker.film.as_mut().unwrap().app.session, "project.delete", json!({"items":[item.0]})).unwrap();
        maker.sync_native_source_actions();
        assert_eq!(maker.video().unwrap().assets.len(), 1);
        maker.ensure_film().unwrap();
        assert_eq!(library_count(&maker), 1);
        assert!(path.exists());
        maker.film.as_mut().unwrap().app.session.undo().unwrap();
        maker.sync_native_source_actions();
        assert_eq!(maker.video().unwrap().assets.len(), 2);
        maker.ensure_film().unwrap();
        assert_eq!(library_count(&maker), 2);
        maker.film.as_mut().unwrap().app.session.redo().unwrap();
        maker.sync_native_source_actions();
        assert_eq!(maker.video().unwrap().assets.len(), 1);
        maker.ensure_film().unwrap();
        assert_eq!(library_count(&maker), 1);
        assert!(path.exists());
        maker.shutdown_film();
        fs::remove_dir_all(store.root).unwrap();
    }
    #[test]
    fn native_imports_are_copied_to_library_and_relink_existing_timeline_sources() {
        let (store, video) = fixture("native-import");
        store.save().unwrap();
        let external = store.root.join("external.png");
        image::RgbaImage::from_pixel(20, 20, image::Rgba([45, 50, 55, 255])).save(&external).unwrap();
        let mut maker = VideoMaker::at(store.root.clone()).unwrap();
        maker.ensure_film().unwrap();
        let result =
            execute(&mut maker.film.as_mut().unwrap().app.session, "file.import", json!({"paths":[external],"imageSequence":false})).unwrap();
        let item = ItemId(result["items"][0].as_u64().unwrap());
        maker.sync_native_imports();
        let ctx = egui::Context::default();
        for _ in 0..200 {
            maker.poll_imports(&ctx);
            if !maker.imports.busy() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(maker.video().unwrap().assets.len(), 3);
        let asset = maker.video().unwrap().assets.iter().find(|a| a.name == "external.png").unwrap();
        let owned = maker.store.asset_path(video.id, asset);
        assert!(owned.exists() && external.exists());
        let session = &maker.film.as_ref().unwrap().app.session;
        assert_eq!(existing_item(session, &owned), Some(item));
        maker.ensure_film().unwrap();
        assert_eq!(library_count(&maker), 3);
        maker.shutdown_film();
        fs::remove_dir_all(store.root).unwrap();
    }
    #[test]
    fn source_renames_reach_existing_timeline_clips_and_preserve_custom_clip_names() {
        let (store, video) = fixture("source-names");
        store.save().unwrap();
        let mut maker = VideoMaker::at(store.root.clone()).unwrap();
        maker.ensure_film().unwrap();
        let active = maker.film.as_ref().unwrap().app.session.state.active_sequence;
        let first = maker.film.as_ref().unwrap().app.session.active_sequence().unwrap().video_tracks[0].items[0].id;
        execute(&mut maker.film.as_mut().unwrap().app.session, "clip.rename", json!({"clip":first.0,"name":"사용자가 붙인 클립 이름"})).unwrap();
        maker.rename_asset(video.id, 101, "오프닝 그래픽.png").unwrap();
        let editor = maker.film.as_ref().unwrap();
        assert_eq!(editor.app.session.state.active_sequence, active);
        let clips = &editor.app.session.active_sequence().unwrap().video_tracks[0].items;
        assert_eq!(clips[0].name, "사용자가 붙인 클립 이름");
        assert_eq!(clips[1].name, "오프닝 그래픽.png");
        assert!(editor.app.session.project.items.values().any(|item| item.name == "오프닝 그래픽.png"));
        maker.shutdown_film();
        maker.ensure_film().unwrap();
        assert_eq!(maker.film.as_ref().unwrap().app.session.active_sequence().unwrap().video_tracks[0].items[1].name, "오프닝 그래픽.png");
        maker.shutdown_film();
        let _ = fs::remove_dir_all(store.root);
    }
    #[test]
    fn legacy_timeline_preserves_trims_audio_gain_captions_and_native_roundtrip() {
        let (store, video) = fixture("migration");
        let mut session = legacy_session(&store, &video).unwrap();
        let seq = session.active_sequence().unwrap();
        assert_eq!(seq.video_tracks[0].items.len(), 2);
        assert_eq!(seq.video_tracks[0].items[1].start, Tick::from_seconds_f64(1.0));
        assert_eq!(seq.video_tracks[0].items[1].source_in, Tick::from_seconds_f64(1.0));
        let audio = &seq.audio_tracks[1].items[0];
        assert_eq!(audio.start, Tick::from_seconds_f64(0.3));
        assert_eq!(audio.duration, Tick::from_seconds_f64(0.5));
        assert!((audio.gain_db + 6.020599913).abs() < 0.00001);
        assert_eq!(seq.caption_tracks[0].captions[0].text, "Synthetic caption");
        let path = project_path(&store, video.id);
        execute(&mut session, "file.save", json!({"path":path})).unwrap();
        let mut reopened = Session::default();
        execute(&mut reopened, "file.open", json!({"path":path})).unwrap();
        assert_eq!(reopened.active_sequence().unwrap(), session.active_sequence().unwrap());
        let _ = fs::remove_dir_all(store.root);
    }

    #[test]
    fn native_export_produces_a_decodable_h264_mp4() {
        let (store, video) = fixture("export");
        let mut session = legacy_session(&store, &video).unwrap();
        let path = store.root.join("native-export.mp4");
        execute(&mut session,"file.exportMedia",json!({"path":path,"format":"h264","width":64,"height":48,"fps":30,"audio":true,"hardwareEncoding":"off","range":"custom","startSeconds":0.0,"endSeconds":0.5,"wait":true})).unwrap();
        assert!(fs::metadata(&path).unwrap().len() > 500);
        // External decoder is the oracle, rather than reading our own output with FilmCraft.
        if let Ok(output) = std::process::Command::new("ffprobe").args(["-v", "error", "-show_streams", "-of", "json"]).arg(&path).output() {
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
            let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            let streams = parsed["streams"].as_array().unwrap();
            assert!(streams.iter().any(|s| s["codec_name"] == "h264" && s["width"] == 64 && s["height"] == 48));
            assert!(streams.iter().any(|s| s["codec_type"] == "audio"));
            let decoded = std::process::Command::new("ffmpeg")
                .args(["-v", "error", "-i"])
                .arg(&path)
                .args(["-frames:v", "1", "-pix_fmt", "rgb24", "-f", "rawvideo", "-"])
                .output()
                .unwrap();
            assert!(decoded.status.success(), "{}", String::from_utf8_lossy(&decoded.stderr));
            let offset = (24 * 64 + 16) * 3;
            let pixel = &decoded.stdout[offset..offset + 3];
            assert!(pixel[0] < 60 && pixel[1] > 80 && pixel[2] > 180, "source must fit the sequence, including away from its centre: {pixel:?}");
        }
        let _ = fs::remove_dir_all(store.root);
    }

    #[test]
    fn opening_an_external_project_is_saved_into_the_video_even_when_it_is_clean() {
        let (store, _) = fixture("external-open");
        store.save().unwrap();
        let mut maker = VideoMaker::at(store.root.clone()).unwrap();
        maker.ensure_film().unwrap();
        let mut external = Session::default();
        new_sequence(&mut external, "Replacement timeline").unwrap();
        let external_path = store.root.join("external.fcproj");
        execute(&mut external, "file.save", json!({"path":external_path})).unwrap();
        execute(&mut maker.film.as_mut().unwrap().app.session, "file.open", json!({"path":external_path})).unwrap();
        assert!(!maker.film.as_ref().unwrap().app.session.is_dirty());
        maker.save_film().unwrap();
        maker.shutdown_film();
        maker.ensure_film().unwrap();
        assert!(maker.film.as_ref().unwrap().app.session.project.items.values().any(|item| item.name == "Replacement timeline"));
        maker.shutdown_film();
        let _ = fs::remove_dir_all(store.root);
    }

    #[test]
    fn restoring_a_revision_restores_the_native_timeline_and_handles_a_missing_current_file() {
        let (store, video) = fixture("restore");
        store.save().unwrap();
        let mut maker = VideoMaker::at(store.root.clone()).unwrap();
        maker.ensure_film().unwrap();
        let original = maker.video().unwrap().clone();
        execute(
            &mut maker.film.as_mut().unwrap().app.session,
            "captions.add",
            json!({"text":"New caption","time":Tick::from_seconds_f64(1.0).0,"durationSeconds":0.5}),
        )
        .unwrap();
        maker.save_film().unwrap();
        assert_ne!(maker.video().unwrap().film_revision, original.film_revision);
        maker.restore_film_revision(&original).unwrap();
        *maker.store.data.selected_mut().unwrap() = original.clone();
        maker.ensure_film().unwrap();
        assert_eq!(maker.film.as_ref().unwrap().app.session.active_sequence().unwrap().caption_tracks[0].captions.len(), 1);
        maker.shutdown_film();
        fs::remove_file(project_path(&maker.store, video.id)).unwrap();
        maker.ensure_film().unwrap();
        assert_eq!(maker.film.as_ref().unwrap().app.session.active_sequence().unwrap().video_tracks[0].items.len(), 2);
        maker.shutdown_film();
        let _ = fs::remove_dir_all(store.root);
    }
    #[test]
    fn the_embedded_fullscreen_button_remains_clickable_with_a_narrow_codex_workspace() {
        fn draw(ctx: &egui::Context, app: &mut FilmcraftApp, width: f32, events: Vec<egui::Event>) {
            app.auto.begin_frame();
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 100.0))),
                events,
                ..Default::default()
            };
            let mut output = ctx
                .run_ui(input, |ui| filmcraft_ui_egui::header::show(app, ui, egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 38.0))));
            output.textures_delta.clear();
        }
        for width in [550.0, 850.0, 1188.0] {
            let ctx = egui::Context::default();
            let mut app = FilmcraftApp::new(Session::default());
            app.embedded = true;
            draw(&ctx, &mut app, width, Vec::new());
            let r = app.auto.find("header.fullscreen").expect("the original fullscreen button is visible").rect;
            assert!(r[0] >= 0.0 && r[0] + r[2] <= width);
            let pos = egui::pos2(r[0] + r[2] / 2.0, r[1] + r[3] / 2.0);
            for pressed in [true, false] {
                draw(
                    &ctx,
                    &mut app,
                    width,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE },
                    ],
                );
            }
            assert_eq!(ctx.data_mut(|d| d.remove_temp::<bool>(egui::Id::unique("purpose.fullscreen-request"))), Some(true));
            app.stop();
            app.session.shutdown();
        }
    }
}
