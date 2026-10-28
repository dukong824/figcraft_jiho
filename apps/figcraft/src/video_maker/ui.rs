use super::VideoMaker;
use super::media;
use super::model::{AssetKind, BlockKind, Caption, ScriptBlock, Section};
use super::navigation::Route;
use egui::{Color32, RichText, TextEdit};
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
        ctx.set_visuals(egui::Visuals::light());
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
            egui::CentralPanel::default().frame(egui::Frame::NONE.fill(Color32::WHITE).inner_margin(22.0)).show(ui, |ui| match self.section {
                Section::Library => self.library(ui),
                Section::Script => self.script(ui),
                Section::Edit => self.edit(ui),
                Section::Design => {}
            });
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
                        format!("{} · {}", route.section.label(), name)
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
            .exact_size(52.0)
            .frame(egui::Frame::NONE.fill(Color32::from_rgb(248, 248, 248)).inner_margin(egui::Margin::symmetric(16, 5)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.add_sized([34.0, 34.0], egui::Button::new("◫")).on_hover_text("사이드바 접기/펼치기").clicked() {
                        self.sidebar_open = !self.sidebar_open;
                    }
                    ui.add_space(7.0);
                    back |= ui
                        .add_enabled(self.tabs.can_back(), egui::Button::new(RichText::new("‹").size(25.0)).min_size(egui::vec2(34.0, 34.0)))
                        .on_hover_text("뒤로 가기 (Alt+←)")
                        .clicked();
                    forward |= ui
                        .add_enabled(self.tabs.can_forward(), egui::Button::new(RichText::new("›").size(25.0)).min_size(egui::vec2(34.0, 34.0)))
                        .on_hover_text("앞으로 가기 (Alt+→)")
                        .clicked();
                    ui.add_space(9.0);
                    ui.separator();
                    let tab_width = (ui.available_width() - 146.0).max(40.0);
                    ui.allocate_ui_with_layout(egui::vec2(tab_width, 40.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        egui::ScrollArea::horizontal().id_salt("purpose-open-tabs").show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for (index, label, active) in &labels {
                                    let fill = if *active { Color32::WHITE } else { Color32::from_rgb(244, 244, 244) };
                                    let stroke = egui::Stroke::new(1.0, Color32::from_gray(226));
                                    egui::Frame::NONE.fill(fill).stroke(stroke).corner_radius(egui::CornerRadius::same(6)).show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let tab = ui
                                                .add_sized(
                                                    [180.0, 32.0],
                                                    egui::Button::new(RichText::new(label).strong())
                                                        .frame(false)
                                                        .sense(egui::Sense::click_and_drag()),
                                                )
                                                .on_hover_text(label);
                                            if tab.clicked() {
                                                select = Some(*index);
                                            }
                                            if tab.drag_started() {
                                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                                            }
                                            if tab.double_clicked() {
                                                let maximized = ui.input(|input| input.viewport().maximized.unwrap_or(false));
                                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                                            }
                                            if ui.small_button("×").on_hover_text("탭 닫기 (Ctrl+W)").clicked() {
                                                close = Some(*index);
                                            }
                                        });
                                    });
                                }
                                if ui
                                    .add_sized([34.0, 34.0], egui::Button::new(RichText::new("+").size(23.0)).frame(false))
                                    .on_hover_text("새 탭 (Ctrl+T)")
                                    .clicked()
                                {
                                    new_tab = true;
                                }
                                let blank_width = (tab_width - labels.len() as f32 * 230.0 - 38.0).max(16.0);
                                let blank = ui.allocate_response(egui::vec2(blank_width, 36.0), egui::Sense::click_and_drag());
                                if blank.drag_started() {
                                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                                }
                                if blank.double_clicked() {
                                    let maximized = ui.input(|input| input.viewport().maximized.unwrap_or(false));
                                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                                }
                            });
                        });
                    });
                    let maximized = ui.input(|input| input.viewport().maximized.unwrap_or(false));
                    if ui.add_sized([44.0, 40.0], egui::Button::new("─").frame(false)).on_hover_text("최소화").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }
                    if ui
                        .add_sized([44.0, 40.0], egui::Button::new(if maximized { "❐" } else { "□" }).frame(false))
                        .on_hover_text(if maximized { "이전 크기" } else { "최대화" })
                        .clicked()
                    {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                    }
                    if ui.add_sized([44.0, 40.0], egui::Button::new("×").frame(false)).on_hover_text("닫기").clicked() {
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
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(Color32::WHITE).inner_margin(28.0)).show(ui, |ui| {
            ui.heading("새 탭");
            ui.label("왼쪽에서 영상의 라이브러리, 대본, 디자인 또는 편집을 선택하세요.");
            ui.add_space(18.0);
            for channel in &self.store.data.channels {
                ui.label(RichText::new(&channel.name).strong());
                for video in &channel.videos {
                    if ui.button(format!("{}의 라이브러리 열기", video.name)).clicked() {
                        selected = Some(video.id);
                    }
                }
                ui.add_space(10.0);
            }
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
            .exact_size(264.0)
            .resizable(true)
            .frame(
                egui::Frame::NONE.fill(Color32::from_rgb(250, 250, 249)).inner_margin(12.0).stroke(egui::Stroke::new(1.0, Color32::from_gray(225))),
            )
            .show(ui, |ui| {
                ui.add_space(6.0);
                ui.label(RichText::new("Purpose video maker").size(19.0).strong());
                ui.add_space(14.0);
                ui.add_sized([ui.available_width(), 32.0], TextEdit::singleline(&mut self.search).hint_text("채널이나 영상 검색"));
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("목록").strong());
                    if ui.small_button("+ 채널").clicked() {
                        new_channel = true;
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical().id_salt("purpose-projects").show(ui, |ui| {
                    let query = self.search.to_lowercase();
                    for channel in &mut self.store.data.channels {
                        let channel_match = query.is_empty() || channel.name.to_lowercase().contains(&query);
                        if !channel_match && !channel.videos.iter().any(|v| v.name.to_lowercase().contains(&query)) {
                            continue;
                        }
                        ui.horizontal(|ui| {
                            ui.label("▣");
                            dirty |= ui.add(TextEdit::singleline(&mut channel.name).desired_width(155.0).font(egui::TextStyle::Button)).changed();
                            if ui.small_button("+").on_hover_text("이 채널에 영상 추가").clicked() {
                                new_video = Some(channel.id);
                            }
                        });
                        for video in &channel.videos {
                            if !channel_match && !video.name.to_lowercase().contains(&query) {
                                continue;
                            }
                            ui.indent(("video", video.id), |ui| {
                                let selected = active_route.is_some_and(|route| route.video_id == video.id);
                                if ui.selectable_label(selected, format!("▸ {}", video.name)).on_hover_text("Ctrl+클릭으로 새 탭에서 열기").clicked()
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
                                        let label = match section {
                                            Section::Library => "▦  라이브러리",
                                            Section::Script => "▤  대본",
                                            Section::Design => "◇  디자인",
                                            Section::Edit => "▥  편집",
                                        };
                                        if ui
                                            .selectable_label(
                                                active_route.is_some_and(|route| route.video_id == video.id && route.section == section),
                                                label,
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
                            });
                        }
                        ui.add_space(9.0);
                    }
                });
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.label(RichText::new("로컬 프로젝트 · 자동 저장").small().color(Color32::GRAY));
                    ui.label(RichText::new("✦  Codex").small());
                });
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
        let crumb = route
            .and_then(|route| {
                self.store.data.channels.iter().find_map(|channel| {
                    channel
                        .videos
                        .iter()
                        .find(|video| video.id == route.video_id)
                        .map(|video| format!("{}  /  {}  /  {}", channel.name, video.name, route.section.label()))
                })
            })
            .unwrap_or_else(|| "영상 선택".into());
        egui::Panel::top("purpose-header")
            .exact_size(76.0)
            .frame(
                egui::Frame::NONE
                    .fill(Color32::WHITE)
                    .inner_margin(egui::Margin::symmetric(20, 8))
                    .stroke(egui::Stroke::new(1.0, Color32::from_gray(232))),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if route.is_some() {
                        if let Some(video) = self.store.data.selected_mut() {
                            dirty |= ui.add(TextEdit::singleline(&mut video.name).desired_width(360.0).font(egui::TextStyle::Heading)).changed();
                        }
                    } else {
                        ui.heading("새 탭");
                    }
                    if self.section == Section::Design && ui.button("디자인을 라이브러리에 저장").clicked() {
                        export_design = true;
                    }
                });
                ui.add_space(4.0);
                ui.label(RichText::new(crumb).small().color(Color32::from_gray(100)));
                if !self.message.is_empty() {
                    ui.label(RichText::new(&self.message).small().color(Color32::from_rgb(0, 115, 88)));
                }
            });
        if dirty {
            self.save();
        }
        export_design
    }

    fn library(&mut self, ui: &mut egui::Ui) {
        let mut imported = Vec::new();
        ui.horizontal(|ui| {
            ui.heading("라이브러리");
            if ui.button("+ 내 컴퓨터에서 파일 가져오기").clicked()
                && let Some(files) = rfd::FileDialog::new().pick_files()
            {
                imported = files;
            }
        });
        ui.label("이 영상의 파일은 프로젝트 폴더에 복사됩니다. 대본에 참조하거나 편집 타임라인에 넣을 수 있습니다.");
        imported.extend(ui.ctx().input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect::<Vec<_>>()));
        for path in imported {
            if let Err(e) = self.import_path(&path) {
                self.message = format!("{}: {e}", path.display());
            }
        }
        ui.add_space(12.0);
        let assets = self.video().map(|v| v.assets.clone()).unwrap_or_default();
        if assets.is_empty() {
            ui.label("아직 파일이 없습니다. 영상, 오디오, 이미지 또는 .figcraft 디자인 파일을 추가하세요.");
        }
        let mut script = None;
        let mut timeline = None;
        for asset in assets {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(asset.kind.label()).strong().color(Color32::from_rgb(0, 126, 94)));
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&asset.name).strong());
                        if asset.kind.can_edit() {
                            ui.label(format!("{:.1}초", asset.duration));
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if asset.kind.can_edit() && ui.small_button("타임라인에 추가").clicked() {
                            timeline = Some(asset.id);
                        }
                        if ui.small_button("대본에 첨부").clicked() {
                            script = Some(asset.id);
                        }
                    });
                });
            });
            ui.add_space(5.0);
        }
        if let Some(id) = script {
            self.add_to_script(id);
        }
        if let Some(id) = timeline {
            self.add_to_timeline(id);
        }
    }

    fn script(&mut self, ui: &mut egui::Ui) {
        ui.heading("대본");
        ui.label("문단과 장면을 쌓고 라이브러리 파일을 참조하세요. 변경 사항은 자동 저장됩니다.");
        ui.separator();
        let mut dirty = false;
        let mut remove = None;
        let mut add = None;
        let names = self.video().map(|v| v.assets.iter().map(|a| (a.id, a.name.clone())).collect::<Vec<_>>()).unwrap_or_default();
        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(video) = self.store.data.selected_mut() {
                for (index, block) in video.script.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        let kind = match block.kind {
                            BlockKind::Heading => "제목",
                            BlockKind::Paragraph => "문단",
                            BlockKind::Scene => "장면",
                            BlockKind::Asset(_) => "에셋",
                        };
                        ui.menu_button(kind, |ui| {
                            if ui.button("제목").clicked() {
                                block.kind = BlockKind::Heading;
                                dirty = true;
                                ui.close();
                            }
                            if ui.button("문단").clicked() {
                                block.kind = BlockKind::Paragraph;
                                dirty = true;
                                ui.close();
                            }
                            if ui.button("장면").clicked() {
                                block.kind = BlockKind::Scene;
                                dirty = true;
                                ui.close();
                            }
                        });
                        if ui.small_button("삭제").clicked() {
                            remove = Some(index);
                        }
                    });
                    match block.kind {
                        BlockKind::Asset(id) => {
                            let name = names.iter().find(|(a, _)| *a == id).map_or("없는 에셋", |(_, n)| n.as_str());
                            ui.label(RichText::new(format!("▣  {name}")).color(Color32::from_rgb(0, 122, 94)));
                            dirty |=
                                ui.add_sized([ui.available_width(), 27.0], TextEdit::singleline(&mut block.text).hint_text("에셋 메모")).changed();
                        }
                        _ => {
                            let rows = if matches!(block.kind, BlockKind::Heading) { 1 } else { 4 };
                            dirty |= ui
                                .add_sized(
                                    [ui.available_width(), if rows == 1 { 38.0 } else { 100.0 }],
                                    TextEdit::multiline(&mut block.text).desired_rows(rows).hint_text("내용을 입력하세요…"),
                                )
                                .changed();
                        }
                    }
                    ui.add_space(12.0);
                }
            }
            ui.horizontal(|ui| {
                if ui.button("+ 문단").clicked() {
                    add = Some(BlockKind::Paragraph);
                }
                if ui.button("+ 장면").clicked() {
                    add = Some(BlockKind::Scene);
                }
                if ui.button("+ 제목").clicked() {
                    add = Some(BlockKind::Heading);
                }
            });
        });
        if let Some(video) = self.store.data.selected_mut() {
            if let Some(index) = remove
                && index < video.script.len()
            {
                video.script.remove(index);
                dirty = true;
            }
            if let Some(kind) = add {
                video.script.push(ScriptBlock { kind, text: String::new() });
                dirty = true;
            }
        }
        if dirty {
            self.save();
        }
    }

    fn edit(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("편집");
            if ui.add_enabled(self.export_job.is_none(), egui::Button::new("MP4 내보내기")).clicked() {
                self.start_export();
            }
        });
        let Some(video) = self.video().cloned() else { return };
        if self.selected_clip.is_none() {
            self.selected_clip = video.clips.iter().find(|clip| !clip.audio_track).map(|clip| clip.id);
        }
        ui.label(format!("1280 × 720 · 30fps · 길이 {:.1}초 · 자막은 MP4와 SRT에 저장", video.duration()));
        ui.separator();
        self.preview_ui(ui, &video);
        ui.separator();
        ui.label(RichText::new("영상 타임라인").strong());
        let mut clicked = None;
        egui::ScrollArea::horizontal().id_salt("video-timeline").show(ui, |ui| {
            ui.horizontal(|ui| {
                for clip in video.clips.iter().filter(|c| !c.audio_track) {
                    let name = video.asset(clip.asset_id).map_or("없는 에셋", |a| a.name.as_str());
                    let width = (clip.duration() * 36.0).clamp(95.0, 280.0) as f32;
                    if ui
                        .add_sized(
                            [width, 48.0],
                            egui::Button::new(format!("{}\n{:.1}s", name, clip.duration())).selected(self.selected_clip == Some(clip.id)),
                        )
                        .clicked()
                    {
                        clicked = Some(clip.id);
                    }
                }
            });
        });
        ui.label(RichText::new("오디오 트랙").strong());
        egui::ScrollArea::horizontal().id_salt("audio-timeline").show(ui, |ui| {
            ui.horizontal(|ui| {
                for clip in video.clips.iter().filter(|c| c.audio_track) {
                    let name = video.asset(clip.asset_id).map_or("없는 에셋", |a| a.name.as_str());
                    if ui
                        .add_sized(
                            [170.0, 42.0],
                            egui::Button::new(format!("{}  @ {:.1}s", name, clip.start)).selected(self.selected_clip == Some(clip.id)),
                        )
                        .clicked()
                    {
                        clicked = Some(clip.id);
                    }
                }
            });
        });
        if let Some(id) = clicked {
            self.selected_clip = Some(id);
            self.preview_clip = None;
            self.preview_at = 0.0;
        }
        self.clip_controls(ui, &video);
        ui.separator();
        self.captions(ui);
    }

    fn preview_ui(&mut self, ui: &mut egui::Ui, video: &super::model::Video) {
        if self.preview_clip != self.selected_clip {
            self.refresh_preview(ui.ctx(), video);
        }
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                if let Some(texture) = &self.preview {
                    let size = texture.size_vec2();
                    let factor = (ui.available_width().min(720.0) / size.x).min(310.0 / size.y).min(1.0);
                    ui.image((texture.id(), size * factor));
                } else {
                    ui.allocate_ui_with_layout(egui::vec2(550.0, 250.0), egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                        ui.label("클립을 선택하면 프레임이 표시됩니다");
                    });
                }
            });
            ui.vertical(|ui| {
                if let Some(clip) = video.clips.iter().find(|c| Some(c.id) == self.selected_clip) {
                    ui.label(format!("선택: {}", video.asset(clip.asset_id).map_or("없는 에셋", |a| a.name.as_str())));
                    let response = ui.add(egui::Slider::new(&mut self.preview_at, 0.0..=clip.duration()).text("프레임 위치"));
                    if response.changed() {
                        self.preview_clip = None;
                    }
                    if ui.button("소스 재생 (ffplay)").clicked()
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
                }
            });
        });
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
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("선택한 클립").strong());
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
        ui.horizontal(|ui| {
            ui.label(RichText::new("자막").strong());
            if ui.small_button("+ 자막").clicked() {
                add = true;
            }
        });
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
            .exact_size(350.0)
            .resizable(true)
            .frame(
                egui::Frame::NONE.fill(Color32::from_rgb(251, 251, 250)).inner_margin(16.0).stroke(egui::Stroke::new(1.0, Color32::from_gray(232))),
            )
            .show(ui, |ui| {
                ui.label(RichText::new("✦  Codex").size(18.0).strong());
                ui.label("현재 영상의 대본, 에셋, 타임라인 정보를 바탕으로 답합니다. 결과를 검토한 후 반영하세요.");
                ui.separator();
                if self.ai_job.is_some() {
                    ui.spinner();
                    ui.label("Codex 작업 중…");
                }
                egui::ScrollArea::vertical().max_height(ui.available_height() - 150.0).show(ui, |ui| {
                    ui.label(&self.ai_response);
                    if self.section == Section::Script
                        && !self.ai_response.is_empty()
                        && self.ai_job.is_none()
                        && ui.button("답변을 대본에 새 문단으로 추가").clicked()
                    {
                        insert = true;
                    }
                });
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.horizontal(|ui| {
                        if ui.add_enabled(self.ai_job.is_none(), egui::Button::new("보내기")).clicked() {
                            send = true;
                        }
                        ui.label("Codex CLI 로그인 필요 · 계정 한도 적용");
                    });
                    ui.add_sized(
                        [ui.available_width(), 90.0],
                        TextEdit::multiline(&mut self.prompt).hint_text("대본을 다듬거나 편집 방향을 물어보세요"),
                    );
                });
            });
        if send {
            self.ask_codex(self.prompt.clone());
        }
        if insert && let Some(video) = self.store.data.selected_mut() {
            video.script.push(ScriptBlock { kind: BlockKind::Paragraph, text: self.ai_response.clone() });
            self.save();
        }
    }
}
