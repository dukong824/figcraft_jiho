use super::{VideoMaker, style};
use egui::{RichText, vec2};
impl VideoMaker {
    pub(super) fn settings(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let mut close = false;
        let mut choose = false;
        let mut disable = false;
        let mut now = false;
        let mut reveal = None;
        let content_height = (ctx.content_rect().height() - 150.0).clamp(140.0, 400.0);
        let before = (self.store.data.ui_font.clone(), self.store.data.video_icon.clone(), self.store.data.ui_font_size);
        let output = egui::Modal::new(egui::Id::unique("purpose-settings"))
            .frame(style::modal_frame())
            .backdrop_color(egui::Color32::from_black_alpha(38))
            .show(ctx, |ui| {
                ui.set_width((ctx.content_rect().width() - 80.0).clamp(340.0, 660.0));
                ui.horizontal(|ui| {
                    ui.label(style::heading("설정", 22.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        close = style::icon_button(ui, style::Icon::Close, 28.0, "설정 닫기").clicked();
                    });
                });
                ui.add_space(20.0);
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(vec2(144.0, content_height), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.set_width(144.0);
                        ui.set_min_height((content_height - 20.0).max(100.0));
                        for (index, label, icon) in [(0, "모양", style::Icon::Grid), (1, "저장 및 백업", style::Icon::History)] {
                            if style::nav_row(ui, label, icon, self.settings_section == index, 0.0).clicked() {
                                self.settings_section = index;
                            }
                        }
                    });
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(16.0);
                    let width = ui.available_width();
                    egui::ScrollArea::vertical().id_salt("settings-content").max_height(content_height).show(ui, |ui| {
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                            ui.set_width(width);
                            ui.set_min_height((content_height - 20.0).max(100.0));
                            if self.settings_section == 0 {
                                section_title(ui, "인터페이스");
                                ui.add_space(18.0);
                                ui.label(RichText::new("앱 글꼴").strong());
                                ui.add_space(5.0);
                                let label = if self.store.data.ui_font.is_empty() { "기본 글꼴" } else { &self.store.data.ui_font };
                                let response = style::dropdown(ui, label, (width - 30.0).max(120.0));
                                egui::Popup::menu(&response).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
                                    ui.set_width(280.0);
                                    style::search_field(ui, &mut self.font_query, 280.0, "설치된 글꼴 검색");
                                    if ui.selectable_label(self.store.data.ui_font.is_empty(), "기본 글꼴").clicked() {
                                        self.store.data.ui_font.clear();
                                        ui.close();
                                    }
                                    let query = self.font_query.to_lowercase();
                                    egui::ScrollArea::vertical().max_height(230.0).show(ui, |ui| {
                                        for family in &self.font_names {
                                            if family.to_lowercase().contains(&query)
                                                && ui.selectable_label(self.store.data.ui_font == *family, family).clicked()
                                            {
                                                self.store.data.ui_font = family.clone();
                                                ui.close();
                                            }
                                        }
                                    });
                                });
                                ui.add_space(14.0);
                                ui.horizontal(|ui| {
                                    ui.label("글자 크기");
                                    let response = style::dropdown(ui, &format!("{} pt", self.store.data.ui_font_size), 96.0);
                                    egui::Popup::menu(&response).show(|ui| {
                                        for size in [12.0, 13.0, 14.0, 15.0, 16.0, 18.0, 20.0] {
                                            if ui.selectable_value(&mut self.store.data.ui_font_size, size, format!("{size} pt")).clicked() {
                                                ui.close();
                                            }
                                        }
                                    });
                                });
                                ui.add_space(9.0);
                                ui.label(RichText::new("가나다 ABC 123").size(14.0).color(style::MUTED));
                                ui.add_space(18.0);
                                ui.separator();
                                ui.add_space(14.0);
                                ui.label(RichText::new("영상 아이콘").strong());
                                ui.add_space(10.0);
                                ui.horizontal(|ui| {
                                    for (name, tip) in [("clapperboard", "클래퍼보드"), ("film", "필름"), ("file-video-camera", "영상 파일")]
                                    {
                                        let (rect, response) = ui.allocate_exact_size(vec2(44.0, 40.0), egui::Sense::click());
                                        let selected = self.store.data.video_icon == name;
                                        ui.painter().rect_filled(
                                            rect,
                                            7,
                                            if selected {
                                                style::SELECTED
                                            } else if response.hovered() {
                                                style::HOVER
                                            } else {
                                                style::SURFACE
                                            },
                                        );
                                        ui.painter().rect_stroke(
                                            rect,
                                            7,
                                            egui::Stroke::new(1.0, if selected { style::MUTED } else { style::BORDER }),
                                            egui::StrokeKind::Inside,
                                        );
                                        figcraft_ui_icons::paint(
                                            ui.painter(),
                                            egui::Rect::from_center_size(rect.center(), vec2(21.0, 21.0)),
                                            name,
                                            style::TEXT,
                                        );
                                        if response.on_hover_text(tip).clicked() {
                                            self.store.data.video_icon = name.into();
                                        }
                                    }
                                });
                            } else {
                                section_title(ui, "저장 및 백업");
                                ui.add_space(18.0);
                                ui.label(RichText::new("자동 백업 폴더").strong());
                                ui.add_space(6.0);
                                egui::Frame::NONE.fill(style::SIDEBAR).corner_radius(7).inner_margin(12.0).show(ui, |ui| {
                                    ui.set_width((width - 24.0).max(160.0));
                                    let path = self
                                        .store
                                        .data
                                        .backup_directory
                                        .as_ref()
                                        .map(|p| p.display().to_string())
                                        .unwrap_or_else(|| "백업 폴더를 선택하세요".into());
                                    ui.add(egui::Label::new(RichText::new(&path).size(12.0).color(style::MUTED)).truncate()).on_hover_text(path);
                                    ui.add_space(9.0);
                                    ui.horizontal_wrapped(|ui| {
                                        choose = style::action_button(ui, "폴더 선택…", false).clicked();
                                        if self.store.data.backup_directory.is_some() {
                                            now = style::action_button(ui, "지금 백업", true).clicked();
                                            disable = style::action_button(ui, "자동 백업 끄기", false).clicked();
                                        }
                                    });
                                });
                                if self.backup.busy() {
                                    ui.horizontal(|ui| {
                                        ui.spinner();
                                        ui.label("백업 중…");
                                    });
                                }
                                if let Some(at) = &self.backup.last_ok {
                                    ui.label(RichText::new(format!("마지막 백업 · {at}")).small().color(style::MUTED));
                                }
                                if let Some(path) = &self.backup.location
                                    && ui.link("백업 폴더 열기 ↗").clicked()
                                {
                                    reveal = Some(path.clone());
                                }
                                if !self.backup.error.is_empty() {
                                    ui.label(RichText::new(&self.backup.error).color(egui::Color32::from_rgb(180, 55, 45)));
                                }
                                ui.add_space(24.0);
                                ui.separator();
                                ui.add_space(17.0);
                                ui.label(RichText::new("프로젝트 저장 위치").strong());
                                ui.add_space(5.0);
                                let root = self.store.root.display().to_string();
                                ui.add(egui::Label::new(RichText::new(&root).size(12.0).color(style::MUTED)).truncate()).on_hover_text(root);
                                if ui.link("프로젝트 폴더 열기 ↗").clicked() {
                                    reveal = Some(self.store.root.clone());
                                }
                            }
                        });
                    });
                });
            });
        if close || output.should_close() {
            self.settings_open = false;
            style::release_focus(ctx);
        }
        if before != (self.store.data.ui_font.clone(), self.store.data.video_icon.clone(), self.store.data.ui_font_size) {
            self.save();
        }
        if choose && let Some(path) = rfd::FileDialog::new().set_title("자동 백업 폴더 선택").pick_folder() {
            self.store.data.backup_directory = Some(path);
            if self.store.data.backup_id.is_empty() {
                self.store.data.backup_id = format!("{}-{}", chrono::Utc::now().format("%Y%m%d%H%M%S%f"), std::process::id());
            }
            self.save();
            self.backup.request();
        }
        if disable {
            self.store.data.backup_directory = None;
            self.save();
        }
        if now {
            self.backup.request();
        }
        if let Some(path) = reveal {
            let name = if cfg!(windows) {
                "explorer"
            } else if cfg!(target_os = "macos") {
                "open"
            } else {
                "xdg-open"
            };
            if let Err(error) = std::process::Command::new(name).arg(path).spawn() {
                self.message = error.to_string();
            }
        }
    }
}
fn section_title(ui: &mut egui::Ui, title: &str) {
    ui.label(style::heading(title, 17.0));
}
