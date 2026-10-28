use super::LibraryView;
use super::VideoMaker;
use super::media;
use super::model::{Asset, AssetKind, Caption, Section};
use super::navigation::Route;
use super::style::{self, Icon};
use egui::{Color32, CornerRadius, FontId, Rect, RichText, Sense, Stroke, TextEdit, pos2, vec2};
use std::path::Path;
use std::process::Command;

impl VideoMaker {
    /// Draw the shared shell and non-design sections. The caller draws FigCraft in the remaining
    /// area when `section == Design` and handles a requested design export after this returns.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let ctx = ui.ctx().clone();
        if !self.fonts_ready {
            figcraft_ui_egui::theme::install_fonts(&ctx, true, figcraft_ui_egui::cjk::language("ko"));
            self.fonts_ready = true;
            ctx.request_repaint();
        }
        style::configure(&ctx);
        #[cfg(not(target_os = "macos"))]
        figcraft_ui_egui::tabs::resize_edges(&ctx);
        self.poll_jobs(&ctx);
        self.browser_bar(ui);
        if self.sidebar_open {
            self.sidebar(ui);
        }
        let export_design = self.header(ui);
        if self.tabs.route().is_none() {
            self.empty_tab(ui);
        } else if self.section != Section::Design {
            if matches!(self.section, Section::Script | Section::Edit) {
                self.ai_panel(ui);
            }
            egui::CentralPanel::default().frame(egui::Frame::NONE.fill(style::SURFACE).inner_margin(egui::Margin::symmetric(32, 12))).show(
                ui,
                |ui| match self.section {
                    Section::Library => self.library(ui),
                    Section::Script => self.script(ui),
                    Section::Edit => self.edit(ui),
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
                        let name = self
                            .store
                            .data
                            .channels
                            .iter()
                            .flat_map(|channel| &channel.videos)
                            .find(|video| video.id == route.video_id)
                            .map_or("없는 영상", |video| video.name.as_str());
                        format!("{}  /  {}", name, route.section.label())
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
                                let blank = ui.allocate_response(vec2(60.0, 32.0), Sense::click_and_drag());
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
            self.navigate(Route { video_id, section: Section::Library });
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let mut new_channel = false;
        let mut new_video = None;
        let mut navigate = None;
        let mut open_in_new_tab = None;
        let mut dirty = false;
        let active_route = self.tabs.route();
        egui::Panel::left("purpose-sidebar")
            .default_size(252.0)
            .min_size(210.0)
            .max_size(330.0)
            .resizable(true)
            .frame(egui::Frame::NONE.fill(style::SIDEBAR).inner_margin(egui::Margin::symmetric(12, 10)).stroke(Stroke::new(1.0, style::BORDER)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Purpose").size(17.0).strong());
                    ui.label(RichText::new("video maker").size(13.0).color(style::MUTED));
                });
                ui.add_space(22.0);
                egui::Frame::NONE.fill(style::HOVER).corner_radius(CornerRadius::same(7)).inner_margin(egui::Margin::symmetric(8, 3)).show(
                    ui,
                    |ui| {
                        ui.horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                            style::paint_icon(ui.painter(), rect, Icon::Search, style::MUTED);
                            ui.add(
                                TextEdit::singleline(&mut self.search)
                                    .desired_width(ui.available_width() - 6.0)
                                    .frame(egui::Frame::NONE)
                                    .hint_text("채널 또는 영상 검색"),
                            );
                        });
                    },
                );
                ui.add_space(24.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("워크스페이스").size(12.0).strong().color(style::MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if style::icon_button(ui, Icon::Plus, 24.0, "새 채널").clicked() {
                            new_channel = true;
                        }
                    });
                });
                ui.add_space(8.0);
                egui::ScrollArea::vertical().id_salt("purpose-projects").max_height((ui.available_height() - 60.0).max(40.0)).show(ui, |ui| {
                    let query = self.search.to_lowercase();
                    for channel in &mut self.store.data.channels {
                        let channel_match = query.is_empty() || channel.name.to_lowercase().contains(&query);
                        if !channel_match && !channel.videos.iter().any(|v| v.name.to_lowercase().contains(&query)) {
                            continue;
                        }
                        ui.horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(vec2(18.0, 20.0), Sense::hover());
                            style::paint_icon(ui.painter(), rect, Icon::Channel, style::TEXT);
                            dirty |= ui
                                .add(
                                    TextEdit::singleline(&mut channel.name)
                                        .desired_width((ui.available_width() - 38.0).max(40.0))
                                        .font(egui::TextStyle::Button)
                                        .frame(egui::Frame::NONE),
                                )
                                .changed();
                            if style::icon_button(ui, Icon::Plus, 22.0, "이 채널에 영상 추가").clicked() {
                                new_video = Some(channel.id);
                            }
                        });
                        ui.add_space(3.0);
                        for video in &channel.videos {
                            if !channel_match && !video.name.to_lowercase().contains(&query) {
                                continue;
                            }
                            let selected = active_route.is_some_and(|route| route.video_id == video.id);
                            if style::nav_row(ui, &video.name, Icon::Video, false, 10.0).on_hover_text("Ctrl+클릭으로 새 탭에서 열기").clicked()
                            {
                                let target = Route { video_id: video.id, section: Section::Library };
                                if ui.input(|input| input.modifiers.command) {
                                    open_in_new_tab = Some(target);
                                } else {
                                    navigate = Some(target);
                                }
                            }
                            if selected {
                                for section in Section::ALL {
                                    let icon = section_icon(section);
                                    if style::nav_row(
                                        ui,
                                        section.label(),
                                        icon,
                                        active_route.is_some_and(|route| route.video_id == video.id && route.section == section),
                                        28.0,
                                    )
                                    .on_hover_text("Ctrl+클릭으로 새 탭에서 열기")
                                    .clicked()
                                    {
                                        let target = Route { video_id: video.id, section };
                                        if ui.input(|input| input.modifiers.command) {
                                            open_in_new_tab = Some(target);
                                        } else {
                                            navigate = Some(target);
                                        }
                                    }
                                }
                            }
                        }
                        ui.add_space(15.0);
                    }
                });
                ui.add_space((ui.available_height() - 56.0).max(6.0));
                ui.separator();
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                    style::paint_icon(ui.painter(), rect, Icon::Sparkle, style::MUTED);
                    ui.label(RichText::new("Codex").color(style::MUTED).size(13.0));
                });
                ui.label(RichText::new("로컬 프로젝트 · 자동 저장").color(style::FAINT).size(11.0));
            });
        if new_channel {
            self.store.data.add_channel();
            dirty = true;
        }
        if let Some(channel) = new_video {
            if let Some(video_id) = self.store.data.add_video(channel) {
                navigate = Some(Route { video_id, section: Section::Library });
            }
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
        let mut export_design = false;
        let mut dirty = false;
        let route = self.tabs.route();
        let channel_name = route.and_then(|route| {
            self.store
                .data
                .channels
                .iter()
                .find(|channel| channel.videos.iter().any(|video| video.id == route.video_id))
                .map(|channel| channel.name.clone())
        });
        egui::Panel::top("purpose-header")
            .exact_size(45.0)
            .frame(egui::Frame::NONE.fill(style::SURFACE).inner_margin(egui::Margin::symmetric(22, 7)).stroke(Stroke::new(1.0, style::BORDER)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some(channel) = &channel_name {
                        let (rect, _) = ui.allocate_exact_size(vec2(17.0, 17.0), Sense::hover());
                        style::paint_icon(ui.painter(), rect, Icon::Channel, style::TEXT);
                        ui.label(RichText::new(channel).color(style::MUTED).size(13.0));
                        ui.label(RichText::new("/").color(style::FAINT));
                        if let Some(video) = self.store.data.selected_mut() {
                            dirty |= ui
                                .add(
                                    TextEdit::singleline(&mut video.name)
                                        .desired_width(172.0)
                                        .font(FontId::proportional(13.0))
                                        .frame(egui::Frame::NONE),
                                )
                                .changed();
                        }
                        ui.label(RichText::new("/").color(style::FAINT));
                        ui.label(RichText::new(self.section.label()).size(13.0).strong());
                    } else {
                        ui.label(RichText::new("새 탭").size(13.0).strong());
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.section == Section::Design && route.is_some() && ui.button("디자인을 라이브러리에 저장").clicked() {
                            export_design = true;
                        }
                        if !self.message.is_empty() {
                            ui.label(RichText::new(&self.message).size(12.0).color(style::BLUE));
                        }
                    });
                });
            });
        if dirty {
            self.save();
        }
        export_design
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
                                TextEdit::singleline(&mut video.script_title)
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

    fn edit(&mut self, ui: &mut egui::Ui) {
        let Some(video) = self.video().cloned() else { return };
        if self.selected_clip.is_none() {
            self.selected_clip = video.clips.iter().find(|clip| !clip.audio_track).map(|clip| clip.id);
        }
        egui::ScrollArea::vertical().id_salt("purpose-edit-scroll").show(ui, |ui| {
            ui.add_space(22.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("편집").size(32.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let button =
                        egui::Button::new(RichText::new("MP4 내보내기").color(Color32::WHITE)).fill(style::TEXT).corner_radius(CornerRadius::same(6));
                    if ui.add_enabled(self.export_job.is_none(), button).clicked() {
                        self.start_export();
                    }
                });
            });
            ui.label(RichText::new(format!("1280 × 720 · 30 fps · 영상 길이 {:.1}초", video.duration())).size(13.0).color(style::MUTED));
            ui.add_space(22.0);
            self.preview_ui(ui, &video);
            ui.add_space(28.0);
            ui.label(RichText::new("타임라인").size(19.0).strong());
            ui.add_space(9.0);
            let mut clicked = None;
            egui::Frame::NONE
                .fill(style::SIDEBAR)
                .stroke(Stroke::new(1.0, style::BORDER))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::same(13))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("영상").size(12.0).strong().color(style::MUTED));
                        ui.add_space(8.0);
                        egui::ScrollArea::horizontal().id_salt("video-timeline").show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for clip in video.clips.iter().filter(|c| !c.audio_track) {
                                    let name = video.asset(clip.asset_id).map_or("없는 에셋", |a| a.name.as_str());
                                    let width = (clip.duration() * 36.0).clamp(115.0, 280.0) as f32;
                                    let selected = self.selected_clip == Some(clip.id);
                                    let button = egui::Button::new(format!("{}\n{:.1}초", name, clip.duration()))
                                        .fill(if selected { Color32::from_rgb(210, 229, 246) } else { style::SURFACE })
                                        .stroke(Stroke::new(1.0, if selected { style::BLUE } else { style::BORDER }))
                                        .corner_radius(CornerRadius::same(5));
                                    if ui.add_sized([width, 48.0], button).clicked() {
                                        clicked = Some(clip.id);
                                    }
                                }
                                if video.clips.iter().all(|c| c.audio_track) {
                                    ui.label(RichText::new("라이브러리에서 영상을 추가하세요").size(12.0).color(style::FAINT));
                                }
                            });
                        });
                    });
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("오디오").size(12.0).strong().color(style::MUTED));
                        egui::ScrollArea::horizontal().id_salt("audio-timeline").show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for clip in video.clips.iter().filter(|c| c.audio_track) {
                                    let name = video.asset(clip.asset_id).map_or("없는 에셋", |a| a.name.as_str());
                                    let selected = self.selected_clip == Some(clip.id);
                                    let button = egui::Button::new(format!("{}  ·  {:.1}초", name, clip.start))
                                        .fill(if selected { Color32::from_rgb(210, 229, 246) } else { style::SURFACE })
                                        .stroke(Stroke::new(1.0, if selected { style::BLUE } else { style::BORDER }))
                                        .corner_radius(CornerRadius::same(5));
                                    if ui.add_sized([180.0, 42.0], button).clicked() {
                                        clicked = Some(clip.id);
                                    }
                                }
                                if video.clips.iter().all(|c| !c.audio_track) {
                                    ui.label(RichText::new("오디오 파일을 추가하면 여기에 표시됩니다").size(12.0).color(style::FAINT));
                                }
                            });
                        });
                    });
                });
            if let Some(id) = clicked {
                self.selected_clip = Some(id);
                self.preview_clip = None;
                self.preview_at = 0.0;
            }
            ui.add_space(20.0);
            self.clip_controls(ui, &video);
            ui.add_space(20.0);
            self.captions(ui);
            ui.add_space(28.0);
        });
    }

    fn preview_ui(&mut self, ui: &mut egui::Ui, video: &super::model::Video) {
        if self.preview_clip != self.selected_clip {
            self.refresh_preview(ui.ctx(), video);
        }
        let width = ui.available_width().min(690.0);
        egui::Frame::NONE.fill(Color32::from_rgb(31, 31, 31)).corner_radius(CornerRadius::same(7)).show(ui, |ui| {
            ui.allocate_ui_with_layout(vec2(width, 360.0), egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                if let Some(texture) = &self.preview {
                    let size = texture.size_vec2();
                    let factor = (width / size.x).min(360.0 / size.y).min(1.0);
                    ui.image((texture.id(), size * factor));
                } else {
                    ui.label(RichText::new("클립을 선택하면 미리보기가 표시됩니다").color(Color32::from_gray(200)).size(13.0));
                }
            });
        });
        ui.add_space(9.0);
        if let Some(clip) = video.clips.iter().find(|clip| Some(clip.id) == self.selected_clip) {
            ui.horizontal(|ui| {
                ui.label(RichText::new(video.asset(clip.asset_id).map_or("없는 에셋", |asset| asset.name.as_str())).size(12.0).color(style::MUTED));
                ui.add_space(11.0);
                let response = ui.add_sized([230.0, 25.0], egui::Slider::new(&mut self.preview_at, 0.0..=clip.duration()).text("위치"));
                if response.changed() {
                    self.preview_clip = None;
                }
                ui.add_space(5.0);
                if ui.add(egui::Button::new("소스 재생").small().fill(style::SIDEBAR)).clicked()
                    && let Some(asset) = video.asset(clip.asset_id)
                {
                    let path = self.store.asset_path(video.id, asset);
                    match Command::new(super::media::tool("ffplay"))
                        .args(["-autoexit", "-loglevel", "error", "-ss"])
                        .arg(format!("{:.3}", clip.trim_start))
                        .args(["-t"])
                        .arg(format!("{:.3}", clip.duration()))
                        .arg(path)
                        .spawn()
                    {
                        Ok(_) => self.message = "ffplay에서 소스를 재생합니다".into(),
                        Err(e) => self.message = format!("ffplay를 실행할 수 없습니다: {e}"),
                    }
                }
            });
        }
    }

    fn refresh_preview(&mut self, ctx: &egui::Context, video: &super::model::Video) {
        self.preview = None;
        self.preview_clip = self.selected_clip;
        let Some(clip) = video.clips.iter().find(|c| Some(c.id) == self.selected_clip) else { return };
        let Some(asset) = video.asset(clip.asset_id) else { return };
        if asset.kind == AssetKind::Audio {
            return;
        }
        let path = self.store.asset_path(video.id, asset);
        let png = if asset.kind == AssetKind::Image {
            std::fs::read(&path).map_err(|e| e.to_string())
        } else {
            media::preview_png(&path, clip.trim_start + self.preview_at)
        };
        match png.and_then(|bytes| image::load_from_memory(&bytes).map_err(|e| e.to_string())) {
            Ok(frame) => {
                let frame = frame.to_rgba8();
                let size = [frame.width() as usize, frame.height() as usize];
                self.preview =
                    Some(ctx.load_texture("purpose-preview", egui::ColorImage::from_rgba_unmultiplied(size, &frame), egui::TextureOptions::LINEAR));
            }
            Err(e) => self.message = format!("미리보기 실패: {e}"),
        }
    }

    fn clip_controls(&mut self, ui: &mut egui::Ui, snapshot: &super::model::Video) {
        let Some(id) = self.selected_clip else { return };
        let Some(index) = snapshot.clips.iter().position(|c| c.id == id) else { return };
        let asset_duration = snapshot.asset(snapshot.clips[index].asset_id).map_or(0.0, |a| a.duration);
        let mut dirty = false;
        let mut remove = false;
        let mut move_by = 0isize;
        egui::Frame::NONE
            .fill(style::SURFACE)
            .stroke(Stroke::new(1.0, style::BORDER))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new("선택한 클립").size(16.0).strong());
                    if ui.small_button("←").clicked() {
                        move_by = -1;
                    }
                    if ui.small_button("→").clicked() {
                        move_by = 1;
                    }
                    if ui.small_button("삭제").clicked() {
                        remove = true;
                    }
                });
                if let Some(video) = self.store.data.selected_mut()
                    && let Some(clip) = video.clips.get_mut(index)
                {
                    ui.horizontal(|ui| {
                        ui.label("시작 트림");
                        dirty |= ui.add(egui::DragValue::new(&mut clip.trim_start).speed(0.1).range(0.0..=asset_duration)).changed();
                        ui.label("끝 트림");
                        dirty |= ui.add(egui::DragValue::new(&mut clip.trim_end).speed(0.1).range(0.0..=asset_duration)).changed();
                        ui.label("볼륨");
                        dirty |= ui.add(egui::Slider::new(&mut clip.volume, 0.0..=4.0).text("×")).changed();
                        if clip.audio_track {
                            ui.label("타임라인 시작");
                            dirty |= ui.add(egui::DragValue::new(&mut clip.start).speed(0.1).range(0.0..=86_400.0)).changed();
                        }
                    });
                    if clip.trim_end <= clip.trim_start + 0.04 {
                        clip.trim_end = (clip.trim_start + 0.04).min(asset_duration);
                        dirty = true;
                    }
                }
            });
        if let Some(video) = self.store.data.selected_mut() {
            if remove {
                video.clips.remove(index);
                self.selected_clip = None;
                self.preview_clip = None;
                dirty = true;
            } else if move_by != 0 {
                let dest = index as isize + move_by;
                if dest >= 0 && (dest as usize) < video.clips.len() && video.clips[dest as usize].audio_track == video.clips[index].audio_track {
                    video.clips.swap(index, dest as usize);
                    dirty = true;
                }
            }
        }
        if dirty {
            self.preview_clip = None;
            self.save();
        }
    }

    fn captions(&mut self, ui: &mut egui::Ui) {
        let mut add = false;
        let mut remove = None;
        let mut dirty = false;
        egui::Frame::NONE
            .fill(style::SURFACE)
            .stroke(Stroke::new(1.0, style::BORDER))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new("자막").size(16.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new("+ 자막").small().fill(style::SIDEBAR)).clicked() {
                            add = true;
                        }
                    });
                });
                ui.add_space(8.0);
                if let Some(video) = self.store.data.selected_mut() {
                    for (index, caption) in video.captions.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            dirty |= ui.add(egui::DragValue::new(&mut caption.start).speed(0.1).range(0.0..=86_400.0)).changed();
                            ui.label("–");
                            dirty |= ui.add(egui::DragValue::new(&mut caption.end).speed(0.1).range(0.0..=86_400.0)).changed();
                            dirty |= ui.add(TextEdit::singleline(&mut caption.text).desired_width(360.0).hint_text("자막 내용")).changed();
                            if ui.small_button("삭제").clicked() {
                                remove = Some(index);
                            }
                        });
                    }
                }
            });
        if add {
            let id = self.store.data.id();
            if let Some(video) = self.store.data.selected_mut() {
                let start = video.duration();
                video.captions.push(Caption { id, start, end: start + 3.0, text: String::new() });
                dirty = true;
            }
        }
        if let Some(index) = remove
            && let Some(video) = self.store.data.selected_mut()
        {
            video.captions.remove(index);
            dirty = true;
        }
        if dirty {
            self.save();
        }
    }

    fn ai_panel(&mut self, ui: &mut egui::Ui) {
        let mut send = false;
        let mut insert = false;
        egui::Panel::right("purpose-codex")
            .default_size(330.0)
            .min_size(265.0)
            .max_size(420.0)
            .resizable(true)
            .frame(egui::Frame::NONE.fill(style::SURFACE).stroke(Stroke::new(1.0, style::BORDER)).inner_margin(egui::Margin::symmetric(17, 16)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
                    style::paint_icon(ui.painter(), rect, Icon::Sparkle, style::TEXT);
                    ui.label(RichText::new("Codex").size(16.0).strong());
                });
                ui.add_space(5.0);
                ui.label(RichText::new("현재 영상의 대본, 라이브러리, 타임라인을 참고해 도와줍니다.").size(12.0).color(style::MUTED));
                ui.add_space(15.0);
                ui.separator();
                ui.add_space(11.0);
                if self.ai_job.is_some() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new("Codex 작업 중…").color(style::MUTED));
                    });
                }
                egui::ScrollArea::vertical().id_salt("purpose-ai-output").max_height((ui.available_height() - 155.0).max(70.0)).show(ui, |ui| {
                    if self.ai_response.is_empty() && self.ai_job.is_none() {
                        ui.add_space(22.0);
                        ui.label(RichText::new("대본을 다듬거나 편집 아이디어를 물어보세요.").size(14.0).color(style::MUTED));
                        ui.add_space(8.0);
                        ui.label(RichText::new("답변은 검토한 뒤 대본에 반영할 수 있습니다.").size(12.0).color(style::FAINT));
                    } else {
                        ui.label(RichText::new(&self.ai_response).size(14.0));
                        if self.section == Section::Script && !self.ai_response.is_empty() && self.ai_job.is_none() {
                            ui.add_space(12.0);
                            insert = ui.add(egui::Button::new("대본에 추가").fill(style::SIDEBAR)).clicked();
                        }
                    }
                });
                ui.add_space((ui.available_height() - 170.0).max(8.0));
                egui::Frame::NONE
                    .fill(style::SURFACE)
                    .stroke(Stroke::new(1.0, style::BORDER))
                    .corner_radius(CornerRadius::same(9))
                    .inner_margin(egui::Margin::symmetric(9, 7))
                    .show(ui, |ui| {
                        ui.add_sized(
                            [ui.available_width(), 72.0],
                            TextEdit::multiline(&mut self.prompt).frame(egui::Frame::NONE).hint_text("무엇을 도와드릴까요?"),
                        );
                        ui.horizontal(|ui| {
                            ui.add_space((ui.available_width() - 65.0).max(0.0));
                            let button = egui::Button::new(RichText::new("보내기").color(Color32::WHITE))
                                .fill(style::BLUE)
                                .corner_radius(CornerRadius::same(6));
                            send = ui.add_enabled(self.ai_job.is_none(), button).clicked();
                        });
                    });
                ui.add_space(6.0);
                ui.label(RichText::new("Codex CLI 로그인 · 사용 중인 계정 한도 적용").size(11.0).color(style::FAINT));
            });
        if send {
            self.ask_codex(self.prompt.clone());
        }
        if insert && let Some(video) = self.store.data.selected_mut() {
            if !video.script_body.trim().is_empty() {
                video.script_body.push_str("\n\n");
            }
            video.script_body.push_str(&self.ai_response);
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
