//! Library interaction, selection and logical removal (source bytes stay available to edits).
use super::{
    LibraryView, VideoMaker,
    interactions::AssetDrag,
    model::Asset,
    style::{self, Icon},
    ui::{asset_detail, asset_icon},
};
use egui::{Color32, FontId, Rect, RichText, Sense, Stroke, pos2, vec2};

#[derive(Clone, Copy)]
enum Action {
    Rename(u64),
    Remove,
    Copy,
    Paste,
    Open(u64),
    Attach(u64),
}
impl VideoMaker {
    pub(super) fn library(&mut self, ui: &mut egui::Ui) {
        let mut imported: Vec<_> = ui.ctx().input(|i| i.raw.dropped_files.iter().map(|file| file.path().to_path_buf()).collect());
        let mut action = None;
        if ui.input(|i| i.pointer.any_pressed()) && !egui::Popup::is_any_open(ui.ctx()) {
            self.library_focused = ui.ctx().pointer_hover_pos().is_some_and(|p| ui.max_rect().intersect(ui.clip_rect()).contains(p));
        }
        if self.library_focused && !ui.ctx().text_edit_focused() && ui.memory(|m| m.top_modal_layer().is_none()) {
            let copy = ui.input_mut(|i| {
                let copy = i.events.iter().any(|event| matches!(event, egui::Event::Copy));
                i.events.retain(|event| !matches!(event, egui::Event::Copy));
                copy || i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::C))
            });
            let paste = ui.input_mut(|i| {
                let paste = i.events.iter().any(|event| matches!(event, egui::Event::Paste(_)));
                i.events.retain(|event| !matches!(event, egui::Event::Paste(_)));
                paste || i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::V))
            });
            if copy {
                action = Some(Action::Copy);
            }
            if paste {
                action = Some(Action::Paste);
            }
            if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
                action = Some(Action::Remove);
            }
            if ui.input(|i| i.key_pressed(egui::Key::F2))
                && let Some(id) = self.library_selection.first()
            {
                action = Some(Action::Rename(*id));
            }
        }
        egui::ScrollArea::vertical().id_salt("purpose-library-scroll").auto_shrink([false, false]).show(ui, |ui| {
            let (width, left) = style::page_width(ui, false);
            ui.add_space(style::PAGE_TOP);
            ui.horizontal(|ui| {
                ui.add_space(left);
                ui.allocate_ui_with_layout(vec2(width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(width);
                    let choose = if width >= 360.0 {
                        ui.horizontal(|ui| {
                            ui.label(style::heading("라이브러리", style::PAGE_TITLE));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                style::action_button(ui, "파일 가져오기", true).clicked()
                            })
                            .inner
                        })
                        .inner
                    } else {
                        ui.label(style::heading("라이브러리", style::PAGE_TITLE));
                        ui.add_space(12.0);
                        style::action_button(ui, "파일 가져오기", true).clicked()
                    };
                    if choose && let Some(files) = rfd::FileDialog::new().pick_files() {
                        imported.extend(files);
                    }
                    ui.add_space(style::TITLE_GAP);
                    ui.horizontal_wrapped(|ui| {
                        let count = self.video().map_or(0, |v| v.assets.len());
                        let editable = self.video().map_or(0, |v| v.assets.iter().filter(|a| a.kind.can_edit()).count());
                        let counts = if editable == count {
                            format!("모든 파일 · {count}")
                        } else {
                            format!("모든 파일 · {count} / 편집 소스 · {editable}")
                        };
                        ui.label(RichText::new(counts).size(figcraft_ui_icons::font_size(13.0)).color(style::MUTED));
                        if !self.library_selection.is_empty() {
                            ui.label(
                                RichText::new(format!("{}개 선택", self.library_selection.len()))
                                    .size(figcraft_ui_icons::font_size(12.0))
                                    .color(style::BLUE),
                            );
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if style::icon_toggle(ui, Icon::List, self.library_view == LibraryView::List, 30.0, "목록 보기").clicked() {
                                self.library_view = LibraryView::List;
                            }
                            if style::icon_toggle(ui, Icon::Grid, self.library_view == LibraryView::Tiles, 30.0, "타일 보기").clicked() {
                                self.library_view = LibraryView::Tiles;
                            }
                            let search_width = (width - 230.0).clamp(90.0, 200.0);
                            style::search_field(ui, &mut self.library_query, search_width, "파일 검색");
                        });
                    });
                    self.import_progress(ui);
                    if !self.pastes.is_empty() {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("이미지 붙여넣는 중…");
                        });
                    }
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(14.0);
                    let assets = self.video().map(|v| v.assets.clone()).unwrap_or_default();
                    let query = self.library_query.trim().to_lowercase();
                    let visible: Vec<_> = assets.into_iter().filter(|a| query.is_empty() || a.name.to_lowercase().contains(&query)).collect();
                    let order: Vec<_> = visible.iter().map(|a| a.id).collect();
                    let canvas_top = ui.cursor().top();
                    let mut rects = Vec::new();
                    if visible.is_empty() {
                        ui.add_space(84.0);
                        ui.vertical_centered(|ui| {
                            let (rect, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                            style::paint_icon(ui.painter(), rect, Icon::Library, style::FAINT);
                            ui.add_space(14.0);
                            ui.label(
                                RichText::new(if query.is_empty() { "파일을 여기에 놓으세요" } else { "검색 결과가 없습니다" })
                                    .size(figcraft_ui_icons::font_size(15.0))
                                    .color(style::MUTED),
                            );
                        });
                    } else {
                        let columns = if self.library_view == LibraryView::Tiles { ((width + 14.0) / 224.0).floor().max(1.0) as usize } else { 1 };
                        let tile_width = (width - 14.0 * columns.saturating_sub(1) as f32) / columns as f32;
                        for row in visible.chunks(columns) {
                            ui.horizontal_top(|ui| {
                                ui.spacing_mut().item_spacing.x = 14.0;
                                for asset in row {
                                    let response = self.library_item(ui, asset, tile_width);
                                    rects.push((asset.id, response.rect));
                                    if response.clicked() {
                                        response.request_focus();
                                        self.select_asset(asset.id, &order, ui.input(|i| i.modifiers));
                                    }
                                    if response.double_clicked() {
                                        action = Some(Action::Open(asset.id));
                                    }
                                    if response.secondary_clicked() && !self.library_selection.contains(&asset.id) {
                                        response.request_focus();
                                        self.library_selection = vec![asset.id];
                                        self.library_anchor = Some(asset.id);
                                    }
                                    if response.drag_started() && !self.library_selection.contains(&asset.id) {
                                        self.library_selection = vec![asset.id];
                                        self.library_anchor = Some(asset.id);
                                    }
                                    if let Some(video_id) = self.video_id() {
                                        response.dnd_set_drag_payload(AssetDrag { video_id, asset_id: asset.id });
                                    }
                                    response.context_menu(|ui| {
                                        if style::menu_item(ui, "열기", asset_icon(asset.kind), false).clicked() {
                                            action = Some(Action::Open(asset.id));
                                            ui.close();
                                        }
                                        if style::menu_item(ui, "이름 바꾸기", Icon::Rename, false).clicked() {
                                            action = Some(Action::Rename(asset.id));
                                            ui.close();
                                        }
                                        if style::menu_item(ui, "Codex에 첨부", Icon::Sparkle, false).clicked() {
                                            action = Some(Action::Attach(asset.id));
                                            ui.close();
                                        }
                                        ui.separator();
                                        if style::menu_item(ui, "복사 · Ctrl+C", Icon::Copy, false).clicked() {
                                            action = Some(Action::Copy);
                                            ui.close();
                                        }
                                        if style::menu_item(ui, "붙여넣기 · Ctrl+V", Icon::Paste, false).clicked() {
                                            style::release_focus(ui.ctx());
                                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                                            ui.close();
                                        }
                                        ui.separator();
                                        if style::menu_item(ui, "휴지통으로 이동", Icon::Trash, true).clicked() {
                                            action = Some(Action::Remove);
                                            ui.close();
                                        }
                                    });
                                }
                            });
                            ui.add_space(14.0);
                        }
                    }
                    let bottom = ui.clip_rect().bottom().max(ui.cursor().top() + 100.0);
                    let canvas = Rect::from_min_max(pos2(ui.min_rect().left(), canvas_top), pos2(ui.min_rect().left() + width, bottom));
                    let pointer = ui.ctx().pointer_hover_pos();
                    if ui.input(|i| i.pointer.primary_pressed())
                        && let Some(p) = pointer.filter(|p| canvas.contains(*p))
                        && !rects.iter().any(|(_, r)| r.contains(p))
                        && !egui::Popup::is_any_open(ui.ctx())
                    {
                        let base = if ui.input(|i| i.modifiers.shift || i.modifiers.command) { self.library_selection.clone() } else { Vec::new() };
                        self.library_selection = base.clone();
                        self.library_marquee = Some((p, base));
                    }
                    if let Some((start, base)) = &self.library_marquee
                        && let Some(p) = pointer
                    {
                        let rect = Rect::from_two_pos(*start, p);
                        self.library_selection = base.clone();
                        for (id, bounds) in &rects {
                            if rect.intersects(*bounds) && !self.library_selection.contains(id) {
                                self.library_selection.push(*id);
                            }
                        }
                        ui.painter().rect_filled(rect.intersect(canvas), 2, Color32::from_rgba_unmultiplied(35, 131, 226, 22));
                        ui.painter().rect_stroke(rect.intersect(canvas), 2, Stroke::new(1.0, style::BLUE), egui::StrokeKind::Inside);
                    }
                    if ui.input(|i| i.pointer.any_released()) {
                        self.library_marquee = None;
                    }
                    let (blank, response) = ui.allocate_exact_size(vec2(width, 120.0), Sense::click());
                    if response.secondary_clicked() {
                        action = None;
                    }
                    response.context_menu(|ui| {
                        if style::menu_item(ui, "붙여넣기 · Ctrl+V", Icon::Paste, false).clicked() {
                            style::release_focus(ui.ctx());
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                            ui.close();
                        }
                    });
                    let _ = blank;
                });
            });
        });
        for source in imported {
            if let Some(video) = self.video_id()
                && let Err(error) = self.queue_import(video, source, None)
            {
                self.message = error;
            }
        }
        if let Some(action) = action {
            self.library_action(action);
        }
        let mut renamed = None;
        if let Some((id, name)) = &mut self.library_rename
            && let Some(save) = style::rename_dialog(ui.ctx(), egui::Id::unique("library-rename"), "파일 이름 바꾸기", name)
        {
            if save {
                renamed = Some((*id, name.clone()));
            }
            self.library_rename = None;
        }
        if let Some((id, name)) = renamed
            && let Some(video) = self.video_id()
            && let Err(error) = self.rename_asset(video, id, &name)
        {
            self.message = error;
        }
    }
    fn library_item(&mut self, ui: &mut egui::Ui, asset: &Asset, width: f32) -> egui::Response {
        let selected = self.library_selection.contains(&asset.id);
        let frame = egui::Frame::NONE
            .fill(if selected { Color32::from_rgb(238, 245, 252) } else { style::SURFACE })
            .stroke(Stroke::new(1.0, if selected { style::BLUE } else { style::BORDER }))
            .corner_radius(8)
            .inner_margin(10.0);
        let response = frame
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width((width - 20.0).max(60.0));
                    if self.library_view == LibraryView::Tiles {
                        let (rect, _) = ui.allocate_exact_size(vec2(width - 20.0, 124.0), Sense::hover());
                        self.paint_thumbnail(ui, asset, rect);
                        ui.add_space(9.0);
                        let mut job = egui::text::LayoutJob::simple(
                            asset.name.clone(),
                            FontId::proportional(figcraft_ui_icons::font_size(13.0)),
                            style::TEXT,
                            width - 20.0,
                        );
                        job.wrap.max_rows = 2;
                        job.wrap.break_anywhere = true;
                        let galley = ui.fonts_mut(|f| f.layout_job(job));
                        let (rect, response) = ui.allocate_exact_size(vec2(width - 20.0, 36.0), Sense::hover());
                        ui.painter().galley(rect.min, galley, style::TEXT);
                        response.on_hover_text(&asset.name);
                        ui.label(RichText::new(asset_detail(asset)).size(figcraft_ui_icons::font_size(11.0)).color(style::MUTED));
                    } else {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(vec2(24.0, 26.0), Sense::hover());
                            style::paint_icon(ui.painter(), r.shrink(3.0), asset_icon(asset.kind), style::MUTED);
                            ui.add(egui::Label::new(&asset.name).truncate()).on_hover_text(&asset.name);
                        });
                        if ui.is_rect_visible(ui.min_rect()) {
                            self.request_thumbnail(asset);
                        }
                    }
                });
            })
            .response;
        ui.interact(response.rect, ui.make_persistent_id(("asset", asset.id)), Sense::click_and_drag()).on_hover_cursor(egui::CursorIcon::Grab)
    }
    fn paint_thumbnail(&mut self, ui: &mut egui::Ui, asset: &Asset, rect: Rect) {
        if ui.is_rect_visible(rect) {
            self.request_thumbnail(asset);
        }
        ui.painter().rect_filled(rect, 5, style::THUMB);
        if let Some(texture) = self.thumbnails.get(&asset.id) {
            let scale = (rect.width() / texture.size_vec2().x).min(rect.height() / texture.size_vec2().y);
            let image = Rect::from_center_size(rect.center(), texture.size_vec2() * scale);
            ui.painter().image(texture.id(), image, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        } else {
            style::paint_icon(ui.painter(), Rect::from_center_size(rect.center(), vec2(30.0, 30.0)), asset_icon(asset.kind), style::FAINT);
        }
    }
    fn select_asset(&mut self, id: u64, order: &[u64], modifiers: egui::Modifiers) {
        if modifiers.shift {
            if let (Some(a), Some(b)) = (self.library_anchor.and_then(|a| order.iter().position(|id| *id == a)), order.iter().position(|i| *i == id))
            {
                self.library_selection = order[a.min(b)..=a.max(b)].to_vec();
                return;
            }
        } else if modifiers.command {
            if self.library_selection.contains(&id) {
                self.library_selection.retain(|i| *i != id);
            } else {
                self.library_selection.push(id);
            }
            self.library_anchor = Some(id);
            return;
        }
        self.library_selection = vec![id];
        self.library_anchor = Some(id);
    }
    fn library_action(&mut self, action: Action) {
        let Some(video) = self.video_id() else {
            return;
        };
        match action {
            Action::Rename(id) => {
                if let Some(asset) = self.video().and_then(|v| v.asset(id)) {
                    self.library_rename = Some((id, asset.name.clone()));
                }
            }
            Action::Copy => {
                if self.library_selection.is_empty() {
                    return;
                }
                self.library_clipboard = self.library_selection.iter().map(|id| (video, *id)).collect();
                self.library_clipboard_marker = serde_json::json!({"purposeLibrary":self.library_clipboard,"workspace":self.store.root}).to_string();
                if let Some(ctx) = &self.ui_context {
                    ctx.copy_text(self.library_clipboard_marker.clone());
                }
            }
            Action::Paste => {
                let mut clipboard = arboard::Clipboard::new().ok();
                let text = clipboard.as_mut().and_then(|c| c.get_text().ok());
                let internal = self.ui_context.is_none() || text.as_deref() == Some(self.library_clipboard_marker.as_str());
                let sources: Vec<_> = if internal {
                    self.library_clipboard
                        .iter()
                        .filter_map(|(video, id)| {
                            self.store
                                .data
                                .channels
                                .iter()
                                .flat_map(|c| &c.videos)
                                .find(|v| v.id == *video)
                                .and_then(|v| v.asset(*id))
                                .map(|a| (self.store.asset_path(*video, a), Some(a.name.clone())))
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                let sources = if sources.is_empty() {
                    clipboard.as_mut().and_then(|c| c.get().file_list().ok()).unwrap_or_default().into_iter().map(|p| (p, None)).collect()
                } else {
                    sources
                };
                if sources.is_empty() {
                    self.message = "붙여넣을 파일이 없습니다".into();
                }
                for (path, name) in sources {
                    if let Err(error) = self.queue_import(video, path, name) {
                        self.message = error;
                    }
                }
            }
            Action::Remove => {
                let ids = self.library_selection.clone();
                if let Err(error) = self.remove_assets(video, &ids) {
                    self.message = error;
                }
                self.library_selection.clear();
            }
            Action::Attach(id) => {
                let ids = if self.library_selection.contains(&id) { self.library_selection.clone() } else { vec![id] };
                for id in ids {
                    self.attach_source(video, video, id);
                }
            }
            Action::Open(id) => {
                if let Some(asset) = self.video().and_then(|v| v.asset(id))
                    && let Err(error) = super::film::open_path(&self.store.asset_path(video, asset).display().to_string(), false)
                {
                    self.message = error;
                }
            }
        }
    }
    pub(super) fn rename_asset(&mut self, video: u64, id: u64, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("이름을 입력하세요".into());
        }
        let target = self.store.data.channels.iter_mut().flat_map(|c| &mut c.videos).find(|v| v.id == video).ok_or("영상을 찾을 수 없습니다")?;
        target.assets.iter_mut().find(|a| a.id == id).ok_or("파일을 찾을 수 없습니다")?.name = name.into();
        self.sync_film_name(video, id)?;
        self.save();
        Ok(())
    }
    pub(super) fn remove_assets(&mut self, video: u64, ids: &[u64]) -> Result<(), String> {
        let target = self.store.data.channels.iter_mut().flat_map(|c| &mut c.videos).find(|v| v.id == video).ok_or("영상을 찾을 수 없습니다")?;
        for id in ids {
            if let Some(index) = target.assets.iter().position(|a| a.id == *id) {
                target.removed_assets.push(target.assets.remove(index));
            }
        }
        self.save();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn copied_assets_import_into_the_chosen_video_even_after_switching_tabs() {
        let root = std::env::temp_dir().join(format!("purpose-library-copy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut maker = VideoMaker::at(root.clone()).unwrap();
        let source = maker.video_id().unwrap();
        let asset = maker.store.data.id();
        let directory = maker.store.video_dir(source).join("assets");
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("internal.figcraft"), b"original design bytes").unwrap();
        maker.store.data.selected_mut().unwrap().assets.push(Asset {
            id: asset,
            name: "사용자 디자인.figcraft".into(),
            file: "internal.figcraft".into(),
            kind: super::super::model::AssetKind::Design,
            duration: 0.0,
            has_audio: false,
        });
        let ctx = egui::Context::default();
        let mut fonts = egui::FontDefinitions::default();
        fonts.families.insert(egui::FontFamily::Name("purpose-script-bold".into()), fonts.families[&egui::FontFamily::Proportional].clone());
        ctx.set_fonts(fonts);
        maker.library_selection = vec![asset];
        maker.library_focused = true;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1200.0, 800.0))),
                events: vec![egui::Event::Copy],
                ..Default::default()
            },
            |ui| maker.library(ui),
        );
        output.textures_delta.clear();
        assert_eq!(maker.library_clipboard, vec![(source, asset)]);
        let channel = maker.store.data.channels[0].id;
        let target = maker.store.data.add_video(channel).unwrap();
        maker.navigate(super::super::navigation::Route::Video { video_id: target, section: super::super::model::Section::Library });
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1200.0, 800.0))),
                events: vec![egui::Event::Paste(maker.library_clipboard_marker.clone())],
                ..Default::default()
            },
            |ui| maker.library(ui),
        );
        output.textures_delta.clear();
        assert!(maker.imports.busy());
        assert!(maker.video().unwrap().assets.is_empty());
        maker.navigate(super::super::navigation::Route::Video { video_id: source, section: super::super::model::Section::Library });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while maker.imports.busy() {
            maker.poll_imports(&ctx);
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let target = maker.store.data.channels[0].videos.iter().find(|v| v.id == target).unwrap();
        assert_eq!(target.assets.len(), 1);
        assert_eq!(target.assets[0].name, "사용자 디자인.figcraft");
        assert_eq!(std::fs::read(maker.store.asset_path(target.id, &target.assets[0])).unwrap(), b"original design bytes");
        assert_eq!(maker.video_id(), Some(source));
        let _ = std::fs::remove_dir_all(root);
    }
}
