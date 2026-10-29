use super::VideoMaker;
use super::media;
use super::model::{Asset, AssetKind, Section};
use super::navigation::Route;
use super::style::{self, Icon};
use super::{LibraryView, SideView};
use egui::{Color32, CornerRadius, FontId, Rect, RichText, Sense, Stroke, TextEdit, pos2, vec2};
use std::path::Path;

impl VideoMaker {
    /// Draw the shared shell and non-design sections. The caller draws FigCraft in the remaining
    /// area when `section == Design` and handles a requested design export after this returns.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let ctx = ui.ctx().clone();
        if !self.fonts_ready {
            figcraft_ui_egui::theme::install_fonts(&ctx, true, figcraft_ui_egui::cjk::language("ko"));
            self.fonts_ready = true;
            ctx.request_repaint();
            return false; // set_fonts takes effect on the next frame.
        }
        style::configure(&ctx);
        ui.set_style(ctx.global_style());
        #[cfg(not(target_os = "macos"))]
        figcraft_ui_egui::tabs::resize_edges(&ctx);
        self.poll_jobs(&ctx);
        self.browser_bar(ui);
        if self.sidebar_open {
            self.sidebar(ui);
        }
        let export_design = self.header(ui);
        self.side_panel(ui);
        if self.tabs.route().is_none() {
            self.empty_tab(ui);
        } else if self.tabs.route() == Some(Route::History) {
            egui::CentralPanel::default().frame(egui::Frame::NONE.fill(style::SURFACE).inner_margin(32.0)).show(ui, |ui| self.history_page(ui));
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
                        let Some((video_id, section)) = route.video() else { return "수정 기록".to_owned() };
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
        let mut dirty = false;
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
                ui.add(TextEdit::singleline(&mut self.search).desired_width(ui.available_width()).hint_text("채널 또는 영상 검색"));
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("워크스페이스").size(12.0).color(style::MUTED));
                    if style::icon_button(ui, Icon::Plus, 24.0, "새 채널").clicked() {
                        new_channel = true;
                    }
                });
                ui.add_space(6.0);
                egui::ScrollArea::vertical().id_salt("purpose-projects").max_height((ui.available_height() - 147.0).max(30.0)).show(ui, |ui| {
                    let query = self.search.to_lowercase();
                    for channel in &mut self.store.data.channels {
                        let channel_match = query.is_empty() || channel.name.to_lowercase().contains(&query);
                        if !channel_match && !channel.videos.iter().any(|v| v.name.to_lowercase().contains(&query)) {
                            continue;
                        }
                        ui.push_id(("channel", channel.id), |ui| {
                            let response = style::nav_row(ui, &channel.name, Icon::Channel, false, 0.0);
                            response.context_menu(|ui| {
                                dirty |= ui.text_edit_singleline(&mut channel.name).changed();
                                if ui.button("영상 추가").clicked() {
                                    new_video = Some(channel.id);
                                    ui.close();
                                }
                            });
                            let rect = Rect::from_center_size(pos2(response.rect.right() - 15.0, response.rect.center().y), vec2(24.0, 24.0));
                            let plus = ui.interact(rect, ui.scope_id().with("add"), Sense::click()).on_hover_text("영상 추가");
                            style::paint_icon(ui.painter(), rect.shrink(4.0), Icon::Plus, style::MUTED);
                            if plus.clicked() {
                                new_video = Some(channel.id);
                            }
                        });
                        for video in &channel.videos {
                            if !channel_match && !video.name.to_lowercase().contains(&query) {
                                continue;
                            }
                            ui.push_id(("video", video.id), |ui| {
                                let is_open = expanded.contains(&video.id);
                                let (response, toggle) = style::tree_row(ui, &video.name, is_open);
                                if toggle {
                                    if is_open {
                                        expanded.retain(|id| *id != video.id);
                                    } else {
                                        expanded.push(video.id);
                                    }
                                } else if response.clicked() {
                                    let target = Route::Video { video_id: video.id, section: Section::Library };
                                    if ui.input(|i| i.modifiers.command) {
                                        open_in_new_tab = Some(target);
                                    } else {
                                        navigate = Some(target);
                                    }
                                }
                                response.context_menu(|ui| {
                                    if ui.button("휴지통으로 이동").clicked() {
                                        trashed = Some(video.id);
                                        ui.close();
                                    }
                                });
                                if is_open || !query.is_empty() {
                                    for section in Section::ALL {
                                        ui.push_id(section.label(), |ui| {
                                            if style::nav_row(
                                                ui,
                                                section.label(),
                                                section_icon(section),
                                                active_route == Some((video.id, section)),
                                                28.0,
                                            )
                                            .clicked()
                                            {
                                                let target = Route::Video { video_id: video.id, section };
                                                if ui.input(|i| i.modifiers.command) {
                                                    open_in_new_tab = Some(target);
                                                } else {
                                                    navigate = Some(target);
                                                }
                                            }
                                        });
                                    }
                                }
                            });
                        }
                        ui.add_space(10.0);
                    }
                });
                ui.add_space((ui.available_height() - 134.0).max(4.0));
                ui.separator();
                if style::nav_row(ui, "수정 기록", Icon::History, self.tabs.route() == Some(Route::History), 0.0).clicked() {
                    self.checkpoint();
                    self.history_video = None;
                    self.navigate(Route::History);
                }
                for (view, label, icon) in [(SideView::Trash, "휴지통", Icon::Trash), (SideView::Codex, "Codex", Icon::Sparkle)] {
                    ui.push_id(label, |ui| {
                        if style::nav_row(ui, label, icon, self.side_view == Some(view), 0.0).clicked() {
                            self.side_view = if self.side_view == Some(view) { None } else { Some(view) };
                            if self.side_view == Some(SideView::Codex) {
                                self.refresh_auth();
                            }
                        }
                    });
                }
            });
        if expanded != self.store.data.expanded_videos {
            self.store.data.expanded_videos = expanded;
            dirty = true;
        }
        if new_channel {
            self.store.data.add_channel();
            dirty = true;
        }
        if let Some(channel) = new_video {
            self.checkpoint();
            if let Some(video_id) = self.store.data.add_video(channel) {
                navigate = Some(Route::Video { video_id, section: Section::Script });
            }
            dirty = true;
        }
        if let Some(id) = trashed {
            self.checkpoint();
            self.store.data.trash_video(id);
            self.tabs.remove_video(id);
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
        if dirty {
            self.save();
        }
    }

    fn header(&mut self, ui: &mut egui::Ui) -> bool {
        let mut export = false;
        let crumb = self
            .store
            .data
            .selected()
            .filter(|_| self.tabs.route().and_then(Route::video).is_some())
            .map(|(channel, video)| format!("{}  /  {}  /  {}", channel.name, video.name, self.section.label()))
            .unwrap_or_else(|| if self.tabs.route() == Some(Route::History) { "수정 기록".into() } else { "새 탭".into() });
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
                        if self.section == Section::Design && self.tabs.route().is_some() && ui.button("디자인을 라이브러리에 저장").clicked()
                        {
                            export = true;
                        }
                    });
                });
            });
        if !self.message.is_empty() {
            egui::Panel::bottom("purpose-status").exact_size(25.0).frame(egui::Frame::NONE.fill(style::SIDEBAR)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&self.message).size(12.0));
                    if ui.small_button("닫기").clicked() {
                        self.message.clear();
                    }
                });
            });
        }
        export
    }

    fn library(&mut self, ui: &mut egui::Ui) {
        let mut imported = Vec::new();
        imported.extend(ui.ctx().input(|i| i.raw.dropped_files.iter().map(|file| file.path().to_path_buf()).collect::<Vec<_>>()));
        let mut attach = None;
        let mut timeline = None;
        egui::ScrollArea::vertical().id_salt("purpose-library-scroll").show(ui, |ui| {
            let viewport_width = ui.clip_rect().width();
            let width = viewport_width.min(1100.0);
            let left = ((viewport_width - width) / 2.0).max(0.0);
            ui.horizontal(|ui| {
                ui.add_space(left);
                ui.allocate_ui_with_layout(vec2(width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_max_width(width);
                    ui.add_space(41.0);
                    ui.label(RichText::new("라이브러리").size(34.0).strong().color(style::TEXT));
                    ui.add_space(5.0);
                    ui.label(RichText::new("이 영상에서 사용할 파일과 디자인을 한곳에 모으세요.").size(14.0).color(style::MUTED));
                    ui.add_space(33.0);
                    ui.horizontal(|ui| {
                        let import = egui::Button::new(RichText::new("파일 가져오기").color(Color32::WHITE).size(13.0))
                            .fill(style::TEXT)
                            .corner_radius(CornerRadius::same(6))
                            .min_size(vec2(110.0, 32.0));
                        if ui.add(import).clicked()
                            && let Some(files) = rfd::FileDialog::new().pick_files()
                        {
                            imported = files;
                        }
                        ui.add_space(8.0);
                        ui.label(RichText::new("파일을 창에 끌어다 놓을 수도 있습니다").size(12.0).color(style::FAINT));
                    });
                    ui.add_space(30.0);
                    let assets = self.video().map(|v| v.assets.clone()).unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("모든 파일  {}", assets.len())).size(14.0).strong().color(style::TEXT));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if style::icon_toggle(ui, Icon::List, self.library_view == LibraryView::List, 28.0, "목록 보기").clicked() {
                                self.library_view = LibraryView::List;
                            }
                            if style::icon_toggle(ui, Icon::Grid, self.library_view == LibraryView::Tiles, 28.0, "타일 보기").clicked() {
                                self.library_view = LibraryView::Tiles;
                            }
                            ui.add_space(11.0);
                            egui::Frame::NONE
                                .fill(style::SIDEBAR)
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(egui::Margin::symmetric(8, 2))
                                .show(ui, |ui| {
                                    ui.add(
                                        TextEdit::singleline(&mut self.library_query)
                                            .frame(egui::Frame::NONE)
                                            .hint_text("파일 검색")
                                            .desired_width(155.0),
                                    );
                                });
                        });
                    });
                    ui.add_space(5.0);
                    ui.separator();
                    ui.add_space(10.0);
                    let query = self.library_query.trim().to_lowercase();
                    let visible: Vec<_> = assets.into_iter().filter(|asset| query.is_empty() || asset.name.to_lowercase().contains(&query)).collect();
                    if visible.is_empty() {
                        ui.add_space(100.0);
                        ui.vertical_centered(|ui| {
                            let (rect, _) = ui.allocate_exact_size(vec2(38.0, 38.0), Sense::hover());
                            style::paint_icon(ui.painter(), rect, Icon::Library, style::FAINT);
                            ui.add_space(14.0);
                            let text = if query.is_empty() { "아직 파일이 없습니다" } else { "검색 결과가 없습니다" };
                            ui.label(RichText::new(text).strong());
                            ui.label(RichText::new("이미지, 영상, 오디오 또는 FigCraft 파일을 추가하세요.").size(13.0).color(style::MUTED));
                        });
                    } else if self.library_view == LibraryView::Tiles {
                        let gap = 13.0;
                        let count = ((ui.available_width() + gap) / (218.0 + gap)).floor().max(1.0) as usize;
                        let tile_width = (ui.available_width() - gap * (count.saturating_sub(1) as f32)) / count as f32;
                        for row in visible.chunks(count) {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = gap;
                                for asset in row {
                                    self.asset_tile(ui, asset, tile_width, &mut attach, &mut timeline);
                                }
                            });
                            ui.add_space(6.0);
                        }
                    } else {
                        for asset in &visible {
                            self.asset_list_row(ui, asset, &mut attach, &mut timeline);
                        }
                    }
                });
            });
        });
        for path in imported {
            if let Err(e) = self.import_path(&path) {
                self.message = format!("{}: {e}", path.display());
            }
        }
        if let Some(id) = attach {
            self.add_to_script(id);
        }
        if let Some(id) = timeline {
            self.add_to_timeline(id);
        }
    }

    fn asset_tile(&mut self, ui: &mut egui::Ui, asset: &Asset, width: f32, attach: &mut Option<u64>, timeline: &mut Option<u64>) {
        egui::Frame::NONE
            .fill(style::SURFACE)
            .stroke(Stroke::new(1.0, style::BORDER))
            .corner_radius(CornerRadius::same(7))
            .inner_margin(egui::Margin::same(9))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_width(width - 18.0);
                    self.asset_thumbnail(ui, asset, width - 18.0, 124.0);
                    ui.add_space(8.0);
                    ui.label(RichText::new(&asset.name).size(13.0).strong());
                    ui.label(RichText::new(asset_detail(asset)).size(11.0).color(style::MUTED));
                    ui.add_space(7.0);
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new("대본에 첨부").small().fill(style::SIDEBAR)).clicked() {
                            *attach = Some(asset.id);
                        }
                        if asset.kind.can_edit() && ui.add(egui::Button::new("편집에 추가").small().fill(style::SIDEBAR)).clicked() {
                            *timeline = Some(asset.id);
                        }
                    });
                });
            });
    }

    fn asset_list_row(&mut self, ui: &mut egui::Ui, asset: &Asset, attach: &mut Option<u64>, timeline: &mut Option<u64>) {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
            style::paint_icon(ui.painter(), rect, asset_icon(asset.kind), style::MUTED);
            ui.add_space(8.0);
            ui.label(RichText::new(&asset.name).size(14.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if asset.kind.can_edit() && ui.add(egui::Button::new("편집에 추가").small().fill(style::SIDEBAR)).clicked() {
                    *timeline = Some(asset.id);
                }
                if ui.add(egui::Button::new("대본에 첨부").small().fill(style::SIDEBAR)).clicked() {
                    *attach = Some(asset.id);
                }
            });
        });
        ui.separator();
    }

    fn asset_thumbnail(&mut self, ui: &mut egui::Ui, asset: &Asset, width: f32, height: f32) {
        self.request_thumbnail(asset);
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        ui.painter().rect_filled(rect, CornerRadius::same(5), style::THUMB);
        if let Some(texture) = self.thumbnails.get(&asset.id) {
            let size = texture.size_vec2();
            let scale = (width / size.x).min(height / size.y);
            let image_rect = Rect::from_center_size(rect.center(), size * scale);
            ui.painter().image(texture.id(), image_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        } else {
            let icon = asset_icon(asset.kind);
            let icon_rect = Rect::from_center_size(rect.center(), vec2(26.0, 26.0));
            style::paint_icon(ui.painter(), icon_rect, icon, style::FAINT);
        }
    }

    fn request_thumbnail(&mut self, asset: &Asset) {
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

    fn script(&mut self, ui: &mut egui::Ui) {
        let mut dirty = false;
        let assets = self.video().map(|video| video.assets.clone()).unwrap_or_default();
        egui::ScrollArea::vertical().id_salt("purpose-script-scroll").auto_shrink([false, false]).show(ui, |ui| {
            let viewport_width = ui.clip_rect().width();
            let width = viewport_width.min(810.0);
            ui.add_space(77.0);
            let left = ((viewport_width - width) / 2.0).max(0.0);
            let document_height = ui.available_height().max(430.0);
            ui.horizontal(|ui| {
                ui.add_space(left);
                ui.allocate_ui_with_layout(vec2(width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_max_width(width);
                    if let Some(video) = self.store.data.selected_mut() {
                        dirty |= ui
                            .add(
                                TextEdit::singleline(&mut video.name)
                                    .font(FontId::proportional(38.0))
                                    .desired_width(width)
                                    .frame(egui::Frame::NONE)
                                    .hint_text("제목 없음"),
                            )
                            .changed();
                        ui.add_space(18.0);
                        let line_count = video.script_body.lines().count().max(1) as f32;
                        let height = (line_count * 24.0 + 60.0).max(document_height);
                        dirty |= ui
                            .add_sized(
                                [width, height],
                                TextEdit::multiline(&mut video.script_body)
                                    .font(FontId::proportional(16.0))
                                    .frame(egui::Frame::NONE)
                                    .desired_width(width)
                                    .hint_text("대본을 작성하세요…"),
                            )
                            .changed();
                        if !video.script_assets.is_empty() {
                            ui.add_space(18.0);
                            ui.label(RichText::new("첨부한 파일").size(12.0).strong().color(style::MUTED));
                            for id in &video.script_assets {
                                if let Some(asset) = assets.iter().find(|asset| asset.id == *id) {
                                    ui.label(RichText::new(format!("{}  {}", asset.kind.label(), asset.name)).size(13.0).color(style::BLUE));
                                }
                            }
                        }
                    }
                });
            });
        });
        if dirty {
            self.save();
        }
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

fn asset_icon(kind: AssetKind) -> Icon {
    match kind {
        AssetKind::Video => Icon::Video,
        AssetKind::Audio => Icon::Audio,
        AssetKind::Image => Icon::Image,
        AssetKind::Design => Icon::Design,
    }
}

fn asset_detail(asset: &Asset) -> String {
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
    let image = image.thumbnail(380, 240).to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    Ok(egui::ColorImage::from_rgba_unmultiplied(size, &image))
}
