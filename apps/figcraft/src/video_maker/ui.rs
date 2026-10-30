use super::SideView;
use super::VideoMaker;
use super::media;
use super::model::{Asset, AssetKind, Item, Section};
use super::navigation::Route;
use super::style::{self, Icon};
use egui::{CornerRadius, Rect, RichText, Sense, Stroke, pos2, vec2};
use std::path::Path;

impl VideoMaker {
    /// Draw the shared shell and non-design sections. The caller draws FigCraft in the remaining
    /// area when `section == Design` and handles a requested design export after this returns.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let ctx = ui.ctx().clone();
        self.ui_context = Some(ctx.clone());
        log::debug!("Purpose root: max={:?} clip={:?} content={:?}", ui.max_rect(), ui.clip_rect(), ctx.content_rect());
        if !self.fonts_ready {
            figcraft_ui_egui::theme::install_fonts(&ctx, true, figcraft_ui_egui::cjk::language("ko"));
            self.fonts_ready = true;
            ctx.request_repaint();
            return false; // set_fonts takes effect on the next frame.
        }
        if !self.zoom_ready {
            let zoom = self.store.data.ui_zoom;
            if zoom.is_finite() {
                ctx.set_zoom_factor(zoom.clamp(0.7, 2.0));
            }
            self.zoom_ready = true;
        }
        if !ctx.fonts(|f| f.definitions().families.contains_key(&egui::FontFamily::Name("purpose-script-bold".into()))) {
            super::document::install_fonts(&ctx);
            ctx.request_repaint();
            return false;
        }
        self.appearance(&ctx);
        figcraft_ui_icons::set_enabled(&ctx, true);
        style::configure(&ctx);
        ui.set_style(ctx.global_style());
        #[cfg(not(target_os = "macos"))]
        figcraft_ui_egui::tabs::resize_edges(&ctx);
        self.poll_jobs(&ctx);
        let editor_fullscreen =
            self.section_is_edit() && ctx.data(|d| d.get_temp::<bool>(egui::Id::unique("purpose.fullscreen-active")).unwrap_or(false));
        let export_design = if editor_fullscreen {
            false
        } else {
            self.browser_bar(ui);
            if self.sidebar_open {
                self.sidebar(ui);
            }
            self.side_panel(ui);
            self.header(ui)
        };
        if self.tabs.route().is_none() {
            self.empty_tab(ui);
        } else if self.tabs.route() == Some(Route::History) {
            egui::CentralPanel::default().frame(egui::Frame::NONE.fill(style::SURFACE).inner_margin(32.0)).show(ui, |ui| self.history_page(ui));
        } else if let Some(Route::Guide { channel_id }) = self.tabs.route() {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.fill(style::SURFACE).inner_margin(32.0))
                .show(ui, |ui| self.guide_page(ui, channel_id));
        } else if !matches!(self.section, Section::Design | Section::Edit) {
            egui::CentralPanel::default().frame(egui::Frame::NONE.fill(style::SURFACE).inner_margin(egui::Margin::symmetric(32, 12))).show(
                ui,
                |ui| match self.section {
                    Section::Library => self.library(ui),
                    Section::Script => self.script(ui),
                    Section::Edit => {}
                    Section::Design => {}
                },
            );
        }
        if let Some(payload) = egui::DragAndDrop::payload::<super::interactions::AssetDrag>(&ctx)
            && let Some(pos) = ctx.pointer_hover_pos()
        {
            let name = self
                .store
                .data
                .channels
                .iter()
                .flat_map(|c| &c.videos)
                .find(|v| v.id == payload.video_id)
                .and_then(|v| v.asset(payload.asset_id))
                .map(|a| a.name.clone())
                .unwrap_or_default();
            egui::Area::new(egui::Id::unique("purpose-asset-drag-preview"))
                .order(egui::Order::Tooltip)
                .fixed_pos(pos + vec2(16.0, 16.0))
                .interactable(false)
                .show(&ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_width(180.0);
                        if let Some(texture) = self.thumbnails.get(&payload.asset_id) {
                            ui.add(egui::Image::new(texture).max_size(vec2(180.0, 110.0)));
                        } else {
                            let (r, _) = ui.allocate_exact_size(vec2(180.0, 75.0), Sense::hover());
                            style::paint_icon(ui.painter(), Rect::from_center_size(r.center(), vec2(32.0, 32.0)), Icon::Library, style::MUTED);
                        }
                        ui.add(egui::Label::new(name).truncate());
                    });
                });
        }
        self.settings(&ctx);
        self.trash_window(&ctx);
        if !self.message.is_empty() {
            self.toast = Some((std::mem::take(&mut self.message), std::time::Instant::now()));
        }
        if let Some((message, at)) = &self.toast {
            let seconds = if message.contains("실패") || message.contains("오류") { 5.0 } else { 1.0 };
            if at.elapsed().as_secs_f32() < seconds {
                egui::Area::new(egui::Id::unique("purpose-notification"))
                    .anchor(egui::Align2::CENTER_TOP, vec2(0.0, 64.0))
                    .order(egui::Order::Tooltip)
                    .show(&ctx, |ui| {
                        egui::Frame::popup(ui.style()).inner_margin(egui::Margin::symmetric(16, 10)).show(ui, |ui| {
                            ui.label(message);
                        });
                    });
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            } else {
                self.toast = None;
            }
        }
        export_design
    }

    fn browser_bar(&mut self, ui: &mut egui::Ui) {
        let (shortcut_new, shortcut_close, shortcut_back, shortcut_forward) = ui.ctx().input(|input| {
            (
                input.modifiers.command && input.key_pressed(egui::Key::T),
                input.modifiers.command && input.key_pressed(egui::Key::W),
                input.modifiers.alt && input.key_pressed(egui::Key::ArrowLeft),
                input.modifiers.alt && input.key_pressed(egui::Key::ArrowRight),
            )
        });
        let labels: Vec<_> = self
            .tabs
            .items()
            .map(|(index, _id, route, active)| {
                let label = route.map_or_else(
                    || "새 탭".to_owned(),
                    |route| {
                        let Some((video_id, section)) = route.video() else {
                            return if matches!(route, Route::Guide { .. }) { "Codex 가이드".into() } else { "수정 기록".into() };
                        };
                        let name = self
                            .store
                            .data
                            .channels
                            .iter()
                            .flat_map(|channel| &channel.videos)
                            .find(|video| video.id == video_id)
                            .map_or("없는 영상", |video| video.name.as_str());
                        format!("{}  /  {}", name, section.label())
                    },
                );
                (index, label, active)
            })
            .collect();
        let mut select = None;
        let mut close = None;
        let mut new_tab = shortcut_new;
        let mut back = shortcut_back;
        let mut forward = shortcut_forward;
        egui::Panel::top("purpose-browser-bar")
            .exact_size(47.0)
            .frame(egui::Frame::NONE.fill(style::SIDEBAR).stroke(Stroke::new(1.0, style::BORDER)).inner_margin(egui::Margin::symmetric(12, 5)))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                ui.horizontal(|ui| {
                    if style::icon_button(ui, Icon::Sidebar, 31.0, "사이드바 접기/펼치기").clicked() {
                        self.sidebar_open = !self.sidebar_open;
                    }
                    ui.add_space(5.0);
                    back |=
                        ui.add_enabled_ui(self.tabs.can_back(), |ui| style::icon_button(ui, Icon::Back, 31.0, "뒤로 가기 (Alt+←)")).inner.clicked();
                    forward |= ui
                        .add_enabled_ui(self.tabs.can_forward(), |ui| style::icon_button(ui, Icon::Forward, 31.0, "앞으로 가기 (Alt+→)"))
                        .inner
                        .clicked();
                    ui.add_space(13.0);
                    let tabs_width = (ui.available_width() - 104.0).max(80.0);
                    ui.allocate_ui_with_layout(vec2(tabs_width, 35.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        egui::ScrollArea::horizontal().id_salt("purpose-open-tabs").auto_shrink([false, true]).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for (index, label, active) in &labels {
                                    let fill = if *active { style::SURFACE } else { style::SIDEBAR };
                                    egui::Frame::NONE
                                        .fill(fill)
                                        .corner_radius(CornerRadius::same(6))
                                        .inner_margin(egui::Margin::symmetric(4, 2))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                let tab = ui
                                                    .add_sized(
                                                        [162.0, 29.0],
                                                        egui::Button::new(
                                                            RichText::new(label).color(if *active { style::TEXT } else { style::MUTED }).size(13.0),
                                                        )
                                                        .frame(false)
                                                        .sense(Sense::click_and_drag()),
                                                    )
                                                    .on_hover_text(label);
                                                if tab.clicked() {
                                                    select = Some(*index);
                                                }
                                                if tab.drag_started() {
                                                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                                                }
                                                if tab.double_clicked() {
                                                    let max = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                                                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
                                                }
                                                if style::icon_button(ui, Icon::Close, 22.0, "탭 닫기 (Ctrl+W)").clicked() {
                                                    close = Some(*index);
                                                }
                                            });
                                        });
                                }
                                if style::icon_button(ui, Icon::Plus, 30.0, "새 탭 (Ctrl+T)").clicked() {
                                    new_tab = true;
                                }
                                let blank_width = (tabs_width - labels.len() as f32 * 206.0 - 35.0).max(60.0);
                                let blank = ui.allocate_response(vec2(blank_width, 32.0), Sense::click_and_drag());
                                if blank.drag_started() {
                                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                                }
                                if blank.double_clicked() {
                                    let max = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
                                }
                            });
                        });
                    });
                    let max = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                    if style::icon_button(ui, Icon::Minimize, 30.0, "최소화").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }
                    if style::icon_button(ui, if max { Icon::Restore } else { Icon::Maximize }, 30.0, if max { "이전 크기" } else { "최대화" })
                        .clicked()
                    {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
                    }
                    if style::icon_button(ui, Icon::Close, 30.0, "닫기").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
        if shortcut_close {
            close = self.tabs.items().find(|(_, _, _, active)| *active).map(|(index, _, _, _)| index);
        }
        if let Some(index) = close {
            self.tabs.close(index);
            self.sync_route();
        } else if new_tab {
            self.tabs.new_tab();
            self.sync_route();
        } else if let Some(index) = select {
            self.tabs.select(index);
            self.sync_route();
        } else if back {
            self.tabs.back();
            self.sync_route();
        } else if forward {
            self.tabs.forward();
            self.sync_route();
        }
    }

    fn empty_tab(&mut self, ui: &mut egui::Ui) {
        let mut selected = None;
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(style::SURFACE).inner_margin(egui::Margin::symmetric(40, 50))).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(85.0);
                ui.label(RichText::new("무엇을 만들까요?").size(32.0).strong());
                ui.add_space(8.0);
                ui.label(RichText::new("왼쪽에서 영상을 열거나 새 영상을 만드세요.").color(style::MUTED));
                ui.add_space(32.0);
                for channel in &self.store.data.channels {
                    ui.label(RichText::new(&channel.name).strong());
                    for video in &channel.videos {
                        if ui.button(format!("{} 열기", video.name)).clicked() {
                            selected = Some(video.id);
                        }
                    }
                    ui.add_space(12.0);
                }
            });
        });
        if let Some(video_id) = selected {
            self.navigate(Route::Video { video_id, section: Section::Library });
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let mut new_channel = false;
        let mut new_video = None;
        let mut navigate = None;
        let mut open_in_new_tab = None;
        let mut trashed = None;
        let mut dropped = None;
        let drag = egui::DragAndDrop::payload::<super::interactions::AssetDrag>(ui.ctx());
        let allowed: Vec<_> = Section::ALL.into_iter().filter(|section| drag.as_ref().is_some_and(|p| self.drop_allowed(p, *section))).collect();
        let mut rename = None;
        let mut dirty = false;
        let mut collapsed = self.store.data.collapsed_channels.clone();
        let active_route = self.tabs.route().and_then(Route::video);
        let mut expanded = self.store.data.expanded_videos.clone();
        egui::Panel::left("purpose-sidebar")
            .default_size(252.0)
            .min_size(210.0)
            .max_size(330.0)
            .resizable(true)
            .frame(egui::Frame::NONE.fill(style::SIDEBAR).inner_margin(egui::Margin::symmetric(12, 10)).stroke(Stroke::new(1.0, style::BORDER)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some(logo) = super::branding::texture(ui.ctx()) {
                        ui.image((logo.id(), vec2(26.0, 26.0)));
                    }
                    ui.label(RichText::new("Purpose video maker").size(15.0).strong());
                });
                ui.add_space(14.0);
                style::search_field(ui, &mut self.search, ui.available_width(), "채널 또는 영상 검색");
                ui.add_space(18.0);
                let (workspace, _) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::hover());
                ui.painter().text(workspace.left_center(), egui::Align2::LEFT_CENTER, "워크스페이스", egui::FontId::proportional(12.0), style::MUTED);
                if ui.ctx().pointer_hover_pos().is_some_and(|p| workspace.contains(p)) {
                    let rect = Rect::from_center_size(pos2(workspace.right() - 15.0, workspace.center().y), vec2(24.0, 24.0));
                    let plus = ui.interact(rect, ui.make_persistent_id("add-channel"), Sense::click()).on_hover_text("새 채널");
                    if plus.hovered() {
                        ui.painter().rect_filled(rect, 4, style::SELECTED);
                    }
                    style::paint_icon(ui.painter(), rect.shrink(4.0), Icon::Plus, style::MUTED);
                    new_channel = plus.clicked();
                }
                ui.add_space(6.0);
                egui::ScrollArea::vertical().id_salt("purpose-projects").max_height((ui.available_height() - 224.0).max(30.0)).show(ui, |ui| {
                    let query = self.search.to_lowercase();
                    for channel in &mut self.store.data.channels {
                        let channel_match = query.is_empty() || channel.name.to_lowercase().contains(&query);
                        if !channel_match && !channel.videos.iter().any(|v| v.name.to_lowercase().contains(&query)) {
                            continue;
                        }
                        ui.push_id(("channel", channel.id), |ui| {
                            let open = !collapsed.contains(&channel.id);
                            let (response, toggle) = style::tree_row(ui, &channel.name, open, true);
                            if toggle {
                                if open {
                                    collapsed.push(channel.id);
                                } else {
                                    collapsed.retain(|id| *id != channel.id);
                                }
                            }
                            if response.dnd_hover_payload::<super::interactions::AssetDrag>().is_some() {
                                collapsed.retain(|id| *id != channel.id);
                            }
                            item_actions(ui, &response, Item::Channel(channel.id), &channel.name, &mut rename, &mut trashed);
                            let rect = Rect::from_center_size(pos2(response.rect.right() - 45.0, response.rect.center().y), vec2(24.0, 24.0));
                            let plus = ui.interact(rect, ui.scope_id().with("add"), Sense::click()).on_hover_text("영상 추가");
                            if ui.ctx().pointer_hover_pos().is_some_and(|p| response.rect.contains(p)) {
                                if plus.hovered() {
                                    ui.painter().rect_filled(rect, 4, style::SELECTED);
                                }
                                style::paint_icon(
                                    ui.painter(),
                                    rect.shrink(4.0),
                                    Icon::Plus,
                                    if plus.hovered() { style::TEXT } else { style::MUTED },
                                );
                            }
                            if plus.clicked() {
                                new_video = Some(channel.id);
                            }
                        });
                        if collapsed.contains(&channel.id) && query.is_empty() {
                            continue;
                        }
                        for video in &channel.videos {
                            if !channel_match && !video.name.to_lowercase().contains(&query) {
                                continue;
                            }
                            ui.push_id(("video", video.id), |ui| {
                                let is_open = expanded.contains(&video.id);
                                let (response, toggle) = style::tree_row(ui, &video.name, is_open, false);
                                if toggle {
                                    if is_open {
                                        expanded.retain(|id| *id != video.id);
                                    } else {
                                        expanded.push(video.id);
                                    }
                                } else if response.clicked()
                                    && ui.input(|i| i.pointer.interact_pos()).is_none_or(|p| p.x < response.rect.right() - 34.0)
                                {
                                    let target = Route::Video { video_id: video.id, section: Section::Library };
                                    if ui.input(|i| i.modifiers.command) {
                                        open_in_new_tab = Some(target);
                                    } else {
                                        navigate = Some(target);
                                    }
                                }
                                item_actions(ui, &response, Item::Video(video.id), &video.name, &mut rename, &mut trashed);
                                if response.dnd_hover_payload::<super::interactions::AssetDrag>().is_some()
                                    && !is_open
                                    && !expanded.contains(&video.id)
                                {
                                    expanded.push(video.id);
                                }
                                if is_open || !query.is_empty() {
                                    for section in Section::ALL {
                                        ui.push_id(section.label(), |ui| {
                                            let response = style::nav_row(
                                                ui,
                                                section.label(),
                                                section_icon(section),
                                                active_route == Some((video.id, section)),
                                                28.0,
                                            );
                                            if response.clicked() {
                                                let target = Route::Video { video_id: video.id, section };
                                                if ui.input(|i| i.modifiers.command) {
                                                    open_in_new_tab = Some(target);
                                                } else {
                                                    navigate = Some(target);
                                                }
                                            }
                                            if allowed.contains(&section) && response.dnd_hover_payload::<super::interactions::AssetDrag>().is_some()
                                            {
                                                ui.painter().rect_stroke(response.rect, 5, Stroke::new(1.5, style::BLUE), egui::StrokeKind::Inside);
                                                ui.ctx().set_cursor_icon(egui::CursorIcon::Copy);
                                            }
                                            if allowed.contains(&section)
                                                && let Some(payload) = response.dnd_release_payload::<super::interactions::AssetDrag>()
                                            {
                                                dropped = Some(((*payload).clone(), Route::Video { video_id: video.id, section }));
                                            }
                                        });
                                    }
                                }
                            });
                        }
                        ui.add_space(10.0);
                    }
                });
                ui.add_space((ui.available_height() - 211.0).max(4.0));
                ui.separator();
                if style::nav_row(ui, "수정 기록", Icon::History, self.tabs.route() == Some(Route::History), 0.0).clicked() {
                    self.checkpoint();
                    self.history_video = None;
                    self.navigate(Route::History);
                }
                if style::nav_row(ui, "휴지통", Icon::Trash, self.trash_open, 0.0).clicked() {
                    self.trash_open = true;
                }
                if style::nav_row(ui, "Codex", Icon::Sparkle, self.side_view == Some(SideView::Codex), 0.0).clicked() {
                    self.side_view = if self.side_view.is_some() { None } else { Some(SideView::Codex) };
                    if self.side_view.is_some() {
                        self.refresh_auth();
                    }
                }
                if style::nav_row(ui, "Codex 가이드", Icon::Guide, matches!(self.tabs.route(), Some(Route::Guide { .. })), 0.0).clicked() {
                    let channel_id = self.store.data.selected().map(|(c, _)| c.id);
                    self.navigate(Route::Guide { channel_id });
                }
                if style::nav_row(ui, "설정", Icon::Settings, self.settings_open, 0.0).clicked() {
                    self.settings_open = true;
                }
            });
        if let Some((payload, target)) = dropped
            && let Err(error) = self.drop_asset(payload, target)
        {
            self.message = error;
        }
        if let Some(rename) = rename {
            self.rename_video = Some(rename);
        }
        if collapsed != self.store.data.collapsed_channels {
            self.store.data.collapsed_channels = collapsed;
            dirty = true;
        }
        if expanded != self.store.data.expanded_videos {
            self.store.data.expanded_videos = expanded;
            dirty = true;
        }
        if new_channel {
            self.store.data.add_channel();
            dirty = true;
        }
        if let Some(channel) = new_video {
            self.store.data.collapsed_channels.retain(|id| *id != channel);
            self.checkpoint();
            if let Some(video_id) = self.store.data.add_video(channel) {
                navigate = Some(Route::Video { video_id, section: Section::Script });
            }
            dirty = true;
        }
        if let Some(item) = trashed {
            self.checkpoint();
            let ids = match item {
                Item::Video(id) => {
                    self.store.data.trash_video(id);
                    vec![id]
                }
                Item::Channel(id) => self.store.data.trash_channel(id),
            };
            for id in ids {
                self.tabs.remove_video(id);
            }
            self.sync_route();
            dirty = true;
        }
        if let Some(route) = navigate {
            self.navigate(route);
        }
        if let Some(route) = open_in_new_tab {
            self.tabs.new_tab();
            self.navigate(route);
        }
        let mut renamed = false;
        if let Some((id, name)) = &mut self.rename_video
            && let Some(save) = style::rename_dialog(ui.ctx(), egui::Id::unique("purpose-rename"), "이름 바꾸기", name)
        {
            if save {
                dirty |= self.store.data.rename(*id, name);
            }
            renamed = true;
        }
        if renamed {
            self.rename_video = None;
        }
        if dirty {
            self.save();
        }
    }

    fn header(&mut self, ui: &mut egui::Ui) -> bool {
        let export = false;
        let crumb = self
            .store
            .data
            .selected()
            .filter(|_| self.tabs.route().and_then(Route::video).is_some())
            .map(|(channel, video)| format!("{}  /  {}  /  {}", channel.name, video.name, self.section.label()))
            .unwrap_or_else(|| match self.tabs.route() {
                Some(Route::History) => "수정 기록".into(),
                Some(Route::Guide { .. }) => "Codex 가이드".into(),
                _ => "새 탭".into(),
            });
        egui::Panel::top("purpose-header")
            .exact_size(45.0)
            .frame(egui::Frame::NONE.fill(style::SURFACE).inner_margin(egui::Margin::symmetric(22, 7)).stroke(Stroke::new(1.0, style::BORDER)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(17.0, 24.0), Sense::hover());
                    style::paint_icon(ui.painter(), Rect::from_center_size(rect.center(), vec2(17.0, 17.0)), Icon::Channel, style::MUTED);
                    ui.add(egui::Label::new(RichText::new(crumb).size(13.0)).truncate());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !self.sidebar_open && style::icon_button(ui, Icon::Sparkle, 28.0, "Codex 열기").clicked() {
                            self.side_view = Some(SideView::Codex);
                            self.refresh_auth();
                        }
                    });
                });
            });
        export
    }

    pub(super) fn request_thumbnail(&mut self, asset: &Asset) {
        if self.thumbnails.contains_key(&asset.id) {
            self.thumbnail_recent.retain(|id| *id != asset.id);
            self.thumbnail_recent.push(asset.id);
            return;
        }
        if !matches!(asset.kind, AssetKind::Image | AssetKind::Video)
            || self.thumbnails.contains_key(&asset.id)
            || self.thumbnail_failed.contains(&asset.id)
            || self.thumbnail_jobs.contains_key(&asset.id)
            || self.thumbnail_jobs.len() >= 4
        {
            return;
        }
        let Some(video_id) = self.video_id() else { return };
        let path = self.store.asset_path(video_id, asset);
        let kind = asset.kind;
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = load_thumbnail(&path, kind);
            let _ = tx.send(result);
        });
        self.thumbnail_jobs.insert(asset.id, rx);
    }
}

fn section_icon(section: Section) -> Icon {
    match section {
        Section::Library => Icon::Library,
        Section::Script => Icon::Script,
        Section::Design => Icon::Design,
        Section::Edit => Icon::Edit,
    }
}

pub(super) fn asset_icon(kind: AssetKind) -> Icon {
    match kind {
        AssetKind::Video => Icon::Video,
        AssetKind::Audio => Icon::Audio,
        AssetKind::Image => Icon::Image,
        AssetKind::Design => Icon::Design,
    }
}

pub(super) fn asset_detail(asset: &Asset) -> String {
    if matches!(asset.kind, AssetKind::Video | AssetKind::Audio) {
        format!("{} · {:.1}초", asset.kind.label(), asset.duration)
    } else {
        asset.kind.label().to_owned()
    }
}

fn load_thumbnail(path: &Path, kind: AssetKind) -> Result<egui::ColorImage, String> {
    let image = if kind == AssetKind::Video {
        let bytes = media::preview_png(path, 0.1)?;
        image::load_from_memory(&bytes).map_err(|e| e.to_string())?
    } else {
        image::open(path).map_err(|e| e.to_string())?
    };
    let image = image.thumbnail(1280, 960).to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    Ok(egui::ColorImage::from_rgba_unmultiplied(size, &image))
}

fn item_actions(
    ui: &mut egui::Ui,
    response: &egui::Response,
    item: Item,
    name: &str,
    rename: &mut Option<(Item, String)>,
    trashed: &mut Option<Item>,
) {
    let menu = |ui: &mut egui::Ui, rename: &mut Option<(Item, String)>, trashed: &mut Option<Item>| {
        ui.set_width(208.0);
        if style::menu_item(ui, "이름 바꾸기", Icon::Rename, false).clicked() {
            *rename = Some((item, name.to_owned()));
            ui.close();
        }
        ui.separator();
        if style::menu_item(ui, "휴지통으로 이동", Icon::Trash, true).clicked() {
            *trashed = Some(item);
            ui.close();
        }
    };
    let menu_id = ui.scope_id().with("item-actions");
    if ui.input(|i| i.pointer.hover_pos()).is_some_and(|p| response.rect.contains(p)) || egui::Popup::is_id_open(ui.ctx(), menu_id) {
        let rect = Rect::from_center_size(pos2(response.rect.right() - 16.0, response.rect.center().y), vec2(27.0, 27.0));
        let more = ui.interact(rect, ui.scope_id().with("more"), Sense::click()).on_hover_text("이름 바꾸기, 휴지통 이동…");
        if more.hovered() {
            ui.painter().rect_filled(rect, 4, style::SELECTED);
        }
        style::paint_icon(ui.painter(), rect.shrink(4.0), Icon::More, style::MUTED);
        egui::Popup::from_toggle_button_response(&more).id(menu_id).show(|ui| menu(ui, rename, trashed));
    }
    response.context_menu(|ui| menu(ui, rename, trashed));
}

#[cfg(test)]
mod shell_tests {
    use super::*;
    #[test]
    fn every_side_view_starts_at_the_browser_bar_and_covers_the_breadcrumb_row() {
        let root = std::env::temp_dir().join(format!("purpose-pane-height-{}", std::process::id()));
        let mut maker = VideoMaker::at(root.clone()).unwrap();
        {
            let view = SideView::Codex;
            let ctx = egui::Context::default();
            maker.side_view = Some(view);
            let mut output = ctx.run_ui(
                egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1280.0, 800.0))), ..Default::default() },
                |ui| {
                    let scope = ui.scope_id();
                    maker.browser_bar(ui);
                    maker.sidebar(ui);
                    maker.side_panel(ui);
                    maker.header(ui);
                    let panel = |salt| egui::containers::panel::PanelState::load(&ctx, scope.with(salt)).unwrap().outer_rect;
                    let browser = panel("purpose-browser-bar");
                    let side = panel("purpose-side-view");
                    let header = panel("purpose-header");
                    assert!((side.top() - browser.bottom()).abs() < 1.0);
                    assert!((side.bottom() - 800.0).abs() < 1.0);
                    assert!((side.top() - header.top()).abs() < 1.0);
                    assert!(side.left() >= header.right() - 1.0);
                },
            );
            output.textures_delta.clear();
        }
        let _ = std::fs::remove_dir_all(root);
    }
}
