//! Trash is a browser page. Permanent deletion is committed before background file cleanup;
//! pending work survives restart and is confined to owned, non-symlink project paths.
use super::{
    VideoMaker,
    model::Workspace,
    style::{self, Icon},
};
use egui::{RichText, vec2};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Component, Path},
    sync::mpsc,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Item {
    Channel(u64),
    Video(u64),
    Asset(u64, u64),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Pending {
    Video(u64),
    Asset { video: u64, file: String },
}
#[derive(Default)]
pub(super) struct Cleanup {
    job: Option<mpsc::Receiver<Result<(), String>>>,
    error: String,
}

fn strip_asset(video: &mut super::model::Video, id: u64) {
    video.assets.retain(|a| a.id != id);
    video.removed_assets.retain(|a| a.id != id);
    video.script_assets.retain(|a| *a != id);
    video.script_rich.images.retain(|i| i.asset_id != id);
    video.script.retain(|b| !matches!(b.kind, super::model::BlockKind::Asset(a) if a == id));
    video.clips.retain(|c| c.asset_id != id);
}
pub(super) fn discard(data: &mut Workspace, item: Item) -> Result<Vec<u64>, String> {
    let mut videos = Vec::new();
    match item {
        Item::Channel(id) => {
            let at = data.trashed_channels.iter().position(|t| t.channel.id == id).ok_or("이미 복원되거나 삭제된 채널입니다")?;
            videos.extend(data.trashed_channels.remove(at).channel.videos.iter().map(|v| v.id));
            for t in &data.trash {
                if t.channel_id == id {
                    videos.push(t.video.id);
                }
            }
            data.trash.retain(|t| t.channel_id != id);
        }
        Item::Video(id) => {
            let at = data.trash.iter().position(|t| t.video.id == id).ok_or("이미 복원되거나 삭제된 영상입니다")?;
            data.trash.remove(at);
            videos.push(id);
        }
        Item::Asset(video, id) => {
            let target = data.channels.iter_mut().flat_map(|c| &mut c.videos).find(|v| v.id == video).ok_or("영상을 찾을 수 없습니다")?;
            let asset = target.removed_assets.iter().find(|a| a.id == id).cloned().ok_or("이미 복원되거나 삭제된 소스입니다")?;
            safe_filename(&asset.file)?;
            strip_asset(target, id);
            for revision in data.revisions.iter_mut().filter(|r| r.video.id == video) {
                strip_asset(&mut revision.video, id);
            }
            data.pending_deletions.push(Pending::Asset { video, file: asset.file.clone() });
            data.purged_assets.push((video, asset.file));
        }
    }
    // Never remove an active, restored video even if a damaged archive repeats its ID.
    videos.retain(|id| !data.channels.iter().flat_map(|c| &c.videos).any(|v| v.id == *id));
    videos.sort_unstable();
    videos.dedup();
    data.revisions.retain(|r| !videos.contains(&r.video.id));
    data.purged_assets.retain(|(video, _)| !videos.contains(video));
    for id in &videos {
        data.pending_deletions.push(Pending::Video(*id));
        data.expanded_videos.retain(|v| v != id);
    }
    Ok(videos)
}
fn safe_filename(file: &str) -> Result<(), String> {
    let mut parts = Path::new(file).components();
    if matches!(parts.next(), Some(Component::Normal(_))) && parts.next().is_none() && !file.contains(['/', '\\']) {
        Ok(())
    } else {
        Err("잘못된 소스 파일 경로입니다".into())
    }
}
fn remove_owned(root: &Path, pending: &Pending) -> Result<(), String> {
    let (video, file) = match pending {
        Pending::Video(id) => (*id, None),
        Pending::Asset { video, file } => {
            safe_filename(file)?;
            (*video, Some(file))
        }
    };
    let mut path = root.to_owned();
    for part in [Some("videos".to_owned()), Some(video.to_string()), file.map(|_| "assets".to_owned()), file.cloned()].into_iter().flatten() {
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => return Err("심볼릭 링크는 영구 삭제할 수 없습니다".into()),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.to_string()),
        }
    }
    if file.is_some() { fs::remove_file(path) } else { fs::remove_dir_all(path) }.map_err(|e| e.to_string())
}
impl VideoMaker {
    pub fn close_deleted_designs(&mut self, fig: &mut figcraft_ui_egui::FigApp) {
        for id in std::mem::take(&mut self.deleted_designs) {
            let directory = self.store.video_dir(id);
            let indices: Vec<_> = fig
                .session
                .docs
                .iter()
                .enumerate()
                .filter(|(_, d)| d.path.as_ref().is_some_and(|p| Path::new(p).starts_with(&directory)))
                .map(|(i, _)| i)
                .collect();
            for index in indices.into_iter().rev() {
                if let Err(error) = fig.session.execute("file.close", &serde_json::json!({"index":index,"force":true})) {
                    self.message = format!("삭제한 디자인 닫기 실패: {error}");
                }
            }
            if self.design_loaded == Some(id) {
                self.design_loaded = None;
            }
        }
    }
    pub(super) fn poll_purge(&mut self, ctx: &egui::Context) {
        let mut result = None;
        if let Some(job) = &self.purge.job {
            match job.try_recv() {
                Ok(value) => result = Some(value),
                Err(mpsc::TryRecvError::Empty) => ctx.request_repaint_after(std::time::Duration::from_millis(100)),
                Err(mpsc::TryRecvError::Disconnected) => result = Some(Err("파일 삭제 작업이 종료되었습니다".into())),
            }
        }
        if let Some(result) = result {
            self.purge.job = None;
            match result {
                Ok(()) => {
                    if !self.store.data.pending_deletions.is_empty() {
                        let pending = self.store.data.pending_deletions.remove(0);
                        if let Err(e) = self.store.save() {
                            self.store.data.pending_deletions.insert(0, pending);
                            self.purge.error = e;
                        }
                    }
                }
                Err(e) => self.purge.error = e,
            }
        }
        if self.purge.job.is_none()
            && self.purge.error.is_empty()
            && !self.imports.busy()
            && !self.download_busy()
            && let Some(pending) = self.store.data.pending_deletions.first().cloned()
        {
            let root = self.store.root.clone();
            let (tx, rx) = mpsc::channel();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let _ = tx.send(remove_owned(&root, &pending));
                ctx.request_repaint();
            });
            self.purge.job = Some(rx);
        }
    }
    fn trash_rows(&self) -> Vec<(Item, String, String, Icon)> {
        let mut rows = Vec::new();
        for t in &self.store.data.trashed_channels {
            rows.push((Item::Channel(t.channel.id), t.channel.name.clone(), format!("채널 · 영상 {}개", t.channel.videos.len()), Icon::Channel));
        }
        for t in &self.store.data.trash {
            rows.push((Item::Video(t.video.id), t.video.name.clone(), t.channel_name.clone(), Icon::Video));
        }
        for c in &self.store.data.channels {
            for v in &c.videos {
                for a in &v.removed_assets {
                    rows.push((Item::Asset(v.id, a.id), a.name.clone(), format!("{} / {}", c.name, v.name), super::ui::asset_icon(a.kind)));
                }
            }
        }
        rows
    }
    pub(super) fn trash_page(&mut self, ui: &mut egui::Ui) {
        let rows = self.trash_rows();
        let mut restore = None;
        egui::ScrollArea::vertical().id_salt("trash-page").auto_shrink([false, false]).show(ui, |ui| {
            let (width, left) = style::page_width(ui, false);
            ui.add_space(style::PAGE_TOP);
            ui.horizontal(|ui| {
                ui.add_space(left);
                ui.allocate_ui_with_layout(vec2(width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(width);
                    ui.label(style::heading("휴지통", style::PAGE_TITLE));
                    ui.add_space(style::TITLE_GAP);
                    style::search_field(ui, &mut self.trash_query, width.min(320.0), "삭제된 항목 검색");
                    ui.add_space(12.0);
                    ui.separator();
                    let query = self.trash_query.to_lowercase();
                    let mut count = 0;
                    for (item, name, path, icon) in &rows {
                        if !query.is_empty() && !name.to_lowercase().contains(&query) && !path.to_lowercase().contains(&query) {
                            continue;
                        }
                        count += 1;
                        ui.push_id(format!("{item:?}"), |ui| {
                            egui::Frame::NONE.inner_margin(egui::Margin::symmetric(8, 12)).show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (rect, _) = ui.allocate_exact_size(vec2(24.0, 32.0), egui::Sense::hover());
                                    style::paint_icon(ui.painter(), rect.shrink(3.0), *icon, style::MUTED);
                                    let text_width = (ui.available_width() - 180.0).max(30.0);
                                    ui.allocate_ui_with_layout(vec2(text_width, 38.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                                        ui.set_min_width(text_width);
                                        ui.add(egui::Label::new(RichText::new(name).strong()).truncate());
                                        ui.add(egui::Label::new(RichText::new(path).small().color(style::MUTED)).truncate());
                                    });
                                    if style::action_button(ui, "복원", false).clicked() {
                                        restore = Some(*item);
                                    }
                                    ui.add_enabled_ui(!self.imports.busy() && !self.download_busy(), |ui| {
                                        if style::action_button(ui, "영구 삭제", false).on_hover_text("이 항목과 저장 파일을 삭제합니다").clicked()
                                        {
                                            self.trash_confirm = Some(*item);
                                        }
                                    });
                                });
                            });
                            ui.separator();
                        });
                    }
                    if count == 0 {
                        ui.add_space(64.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new(if query.is_empty() { "휴지통이 비어 있습니다" } else { "검색 결과가 없습니다" }).color(style::MUTED),
                            );
                        });
                    }
                    if !self.store.data.pending_deletions.is_empty() {
                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            if self.purge.error.is_empty() {
                                ui.spinner();
                                ui.label("저장 파일을 정리하는 중…");
                            } else {
                                ui.label("일부 파일을 삭제하지 못했습니다");
                                if style::action_button(ui, "다시 시도", false).clicked() {
                                    self.purge.error.clear();
                                }
                            }
                        });
                        if !self.purge.error.is_empty() {
                            ui.label(RichText::new(&self.purge.error).small().color(style::MUTED));
                        }
                    }
                });
            });
        });
        if let Some(item) = restore {
            match item {
                Item::Channel(id) => self.store.data.restore_channel(id),
                Item::Video(id) => self.store.data.restore_video(id),
                Item::Asset(video, id) => {
                    if let Some(v) = self.store.data.channels.iter_mut().flat_map(|c| &mut c.videos).find(|v| v.id == video)
                        && let Some(i) = v.removed_assets.iter().position(|a| a.id == id)
                    {
                        v.assets.push(v.removed_assets.remove(i));
                    }
                }
            }
            self.save();
        }
    }
    pub(super) fn trash_confirmation(&mut self, ctx: &egui::Context) {
        let Some(item) = self.trash_confirm else { return };
        let Some((_, name, _, _)) = self.trash_rows().into_iter().find(|(i, _, _, _)| *i == item) else {
            self.trash_confirm = None;
            return;
        };
        let mut cancel = false;
        let mut delete = false;
        let modal = egui::Modal::new(egui::Id::unique("permanent-delete-confirmation")).frame(style::modal_frame()).show(ctx, |ui| {
            ui.set_width(400.0);
            ui.label(style::heading("영구 삭제할까요?", 22.0));
            ui.add_space(10.0);
            ui.label(format!("‘{name}’ 항목과 앱에 저장된 파일을 삭제합니다. 이 작업은 되돌릴 수 없습니다."));
            ui.label(RichText::new("대본과 편집에서 참조하는 소스도 더 이상 사용할 수 없습니다.").small().color(style::MUTED));
            ui.add_space(20.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                delete = ui
                    .add_enabled_ui(!self.imports.busy() && !self.download_busy(), |ui| style::action_button(ui, "영구 삭제", true))
                    .inner
                    .clicked();
                cancel = style::action_button(ui, "취소", false).clicked();
            });
        });
        if delete {
            let previous = self.store.data.clone();
            let result = discard(&mut self.store.data, item).and_then(|ids| {
                self.store.save()?;
                Ok(ids)
            });
            match result {
                Ok(ids) => {
                    for id in ids {
                        self.deleted_designs.push(id);
                        self.tabs.remove_video(id);
                        self.chats.remove(&id);
                        if self.film.as_ref().is_some_and(|f| f.video_id == id)
                            && let Some(mut film) = self.film.take()
                        {
                            film.app.stop();
                        }
                    }
                    self.message = "영구 삭제했습니다".into();
                }
                Err(e) => {
                    self.store.data = previous;
                    self.message = format!("영구 삭제 실패: {e}");
                }
            }
        }
        if delete || cancel || modal.should_close() {
            self.trash_confirm = None;
            style::release_focus(ctx);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn permanent_delete_removes_history_and_files_and_keeps_other_videos() {
        let mut data = Workspace::default();
        let c = data.add_channel();
        let id = data.add_video(c).unwrap();
        let keep = data.add_video(c).unwrap();
        data.selected_video = Some(id);
        data.checkpoint("fixture");
        data.trash_video(id);
        let ids = discard(&mut data, Item::Video(id)).unwrap();
        assert_eq!(ids, vec![id]);
        assert!(data.revisions.iter().all(|r| r.video.id != id));
        assert!(data.channels[0].videos.iter().any(|v| v.id == keep));
        assert!(discard(&mut data, Item::Video(id)).is_err());
        let root = std::env::temp_dir().join(format!("purpose-purge-{}", std::process::id()));
        let dir = root.join("videos").join(id.to_string());
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("sample"), "fixture").unwrap();
        remove_owned(&root, &Pending::Video(id)).unwrap();
        assert!(!dir.exists());
        remove_owned(&root, &Pending::Video(id)).unwrap();
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn asset_deletion_removes_inline_and_historical_references_and_persists_cleanup() {
        let root = std::env::temp_dir().join(format!("purpose-purge-asset-{}", std::process::id()));
        let mut store = super::super::model::Store::load(root.clone()).unwrap();
        let channel = store.data.add_channel();
        let video = store.data.add_video(channel).unwrap();
        let asset = super::super::model::Asset {
            id: 81,
            name: "source.png".into(),
            file: "81.png".into(),
            kind: super::super::model::AssetKind::Image,
            duration: 0.0,
            has_audio: false,
        };
        let v = store.data.selected_mut().unwrap();
        v.removed_assets.push(asset);
        v.script_assets.push(81);
        v.script_rich.insert_image(81, 0);
        store.data.checkpoint("fixture");
        let directory = store.video_dir(video).join("assets");
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("81.png"), "fixture").unwrap();
        fs::write(directory.join("keep.png"), "keep").unwrap();
        discard(&mut store.data, Item::Asset(video, 81)).unwrap();
        store.save().unwrap();
        let restored = super::super::model::Store::load(root.clone()).unwrap();
        assert_eq!(restored.data.purged_assets, vec![(video, "81.png".into())]);
        assert_eq!(restored.data.pending_deletions.len(), 1);
        assert!(restored.data.selected().unwrap().1.script_rich.images.is_empty());
        assert!(restored.data.revisions.iter().all(|r| r.video.asset(81).is_none()));
        remove_owned(&root, &restored.data.pending_deletions[0]).unwrap();
        assert!(!directory.join("81.png").exists());
        assert!(directory.join("keep.png").exists());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn path_traversal_and_symlinks_are_not_deleted() {
        for name in ["../outside", "/tmp/outside", "a/b", "a\\b", ".."] {
            assert!(safe_filename(name).is_err());
        }
        assert!(safe_filename("123.mp4").is_ok());
        #[cfg(unix)]
        {
            let root = std::env::temp_dir().join(format!("purpose-purge-link-{}", std::process::id()));
            fs::create_dir_all(root.join("outside")).unwrap();
            fs::write(root.join("outside/keep"), "fixture").unwrap();
            std::os::unix::fs::symlink(root.join("outside"), root.join("videos")).unwrap();
            assert!(remove_owned(&root, &Pending::Video(1)).is_err());
            assert!(root.join("outside/keep").exists());
            fs::remove_dir_all(root).unwrap();
        }
    }
}
