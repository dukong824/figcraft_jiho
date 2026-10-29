//! Embedded FilmCraft editor, one native project per video.
use super::{
    VideoMaker,
    film_audio::CpalOut,
    film_audio_in::CpalIn,
    model::{Asset, AssetKind, Store, Video},
};
use filmcraft_engine::Session;
use filmcraft_project::{ItemId, MediaRef};
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
    library_assets: std::collections::HashSet<u64>,
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

fn open_path(path: &str, reveal: bool) -> Result<(), String> {
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
            MediaRef::File { path: source } if Path::new(source) == path => Some(item.id),
            _ => None,
        }
    })
}

fn import_asset(session: &mut Session, store: &Store, video: u64, asset: &Asset) -> Result<ItemId, String> {
    let path = store.asset_path(video, asset);
    if let Some(id) = existing_item(session, &path) {
        return Ok(id);
    }
    let detect = session.prefs.media.import_image_sequences;
    session.prefs.media.import_image_sequences = false;
    let result = execute(session, "file.import", json!({"paths":[path],"imageSequence":false}));
    session.prefs.media.import_image_sequences = detect;
    let result = result?;
    result["items"]
        .as_array()
        .and_then(|ids| ids.first())
        .and_then(|id| id.as_u64())
        .map(ItemId)
        .ok_or_else(|| format!("{}: {}", asset.name, result["errors"]))
}

fn new_sequence(session: &mut Session, name: &str) -> Result<(), String> {
    execute(session, "file.newSequence", json!({"name":name,"width":1280,"height":720,"fps":30.0,"video":3,"audio":3}))?;
    Ok(())
}

/// Convert only once from the old float-based editor to FilmCraft's integer tick timeline.
fn legacy_session(store: &Store, video: &Video) -> Result<Session, String> {
    let mut session = Session::default();
    execute(&mut session, "file.newProject", json!({"name":video.name}))?;
    new_sequence(&mut session, &video.name)?;
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
    pub fn section_is_edit(&self) -> bool {
        self.tabs.route().and_then(super::navigation::Route::video).is_some_and(|(_, section)| section == super::model::Section::Edit)
    }

    fn ensure_film(&mut self) -> Result<(), String> {
        let video = self.video().cloned().ok_or("영상을 먼저 선택하세요")?;
        if let Some(editor) = self.film.as_mut().filter(|editor| editor.video_id == video.id) {
            for asset in video.assets.iter().filter(|a| a.kind.can_edit()) {
                if editor.library_assets.insert(asset.id)
                    && let Err(error) = import_asset(&mut editor.app.session, &self.store, video.id, asset)
                {
                    log::warn!("{error}");
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
        fs::create_dir_all(self.store.video_dir(video.id)).map_err(|e| e.to_string())?;
        let mut config = filmcraft_engine::autosave::AutosaveConfig::new(self.store.video_dir(video.id).join("editor-state"));
        config.journal_debounce = Duration::from_secs(2);
        session.start_autosave(config).map_err(|e| format!("편집 자동 저장 초기화 실패: {e}"))?;
        if session.prefs_path.as_ref().is_none_or(|path| !path.exists()) {
            session.prefs.appearance.appearance_mode = "light".into();
            session.prefs.appearance.light_theme = "light".into();
        }
        session.prefs.general.interface_language = "ko".into();
        for asset in video.assets.iter().filter(|a| a.kind.can_edit()) {
            if let Err(error) = import_asset(&mut session, &self.store, video.id, asset) {
                log::warn!("{error}");
            }
        }
        session.voiceover.input = Some(Box::new(CpalIn::new(&session.prefs.audio_hardware.device_class)));
        let seen_revision = session.revision;
        let seen_epoch = session.document_epoch;
        let mut app = FilmcraftApp::new(session);
        app.embedded = true;
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
            app,
            video_id: video.id,
            seen_revision,
            seen_epoch,
            save_due: None,
            gpu_ready: false,
            library_assets: video.assets.iter().map(|a| a.id).collect(),
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
            new_sequence(&mut editor.app.session, &video.name)?;
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
}
