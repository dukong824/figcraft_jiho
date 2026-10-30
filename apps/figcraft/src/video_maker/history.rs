use super::{
    VideoMaker,
    model::{Section, Video},
    navigation::Route,
    style::{self, Icon},
};
use egui::{RichText, Sense, pos2, vec2};

fn display_time(at: &str) -> String {
    chrono::NaiveDateTime::parse_from_str(at, "%Y-%m-%d %H:%M:%S UTC")
        .map(|t| t.and_utc().with_timezone(&chrono::Local).format("%m월 %d일 %H:%M").to_string())
        .unwrap_or_else(|_| at.into())
}
impl VideoMaker {
    pub(super) fn history_page(&mut self, ui: &mut egui::Ui) {
        let mut restore = None;
        let mut open = None;
        egui::ScrollArea::vertical().id_salt("history-page").auto_shrink([false, false]).show(ui, |ui| {
            let (width, left) = style::page_width(ui, false);
            ui.add_space(style::PAGE_TOP);
            ui.horizontal(|ui| {
                ui.add_space(left);
                ui.allocate_ui_with_layout(vec2(width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(width);
                    ui.label(style::heading("수정 기록", style::PAGE_TITLE));
                    ui.add_space(style::TITLE_GAP);
                    ui.horizontal_wrapped(|ui| {
                        if style::filter_tab(ui, "최근 수정", !self.history_all).clicked() {
                            self.history_all = false;
                        }
                        if style::filter_tab(ui, "모든 영상", self.history_all).clicked() {
                            self.history_all = true;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            style::search_field(ui, &mut self.history_query, (width - 200.0).clamp(100.0, 220.0), "영상 또는 채널 검색");
                            style::icon_button(
                                ui,
                                Icon::History,
                                28.0,
                                "보관 기준: 최근 7일은 5분, 8~14일은 1시간, 15~30일은 하루 간격. 30일 이후 삭제",
                            );
                        });
                    });
                    ui.add_space(12.0);
                    ui.separator();
                    let query = self.history_query.trim().to_lowercase();
                    let mut rows: Vec<_> = self
                        .store
                        .data
                        .channels
                        .iter()
                        .flat_map(|c| {
                            c.videos.iter().map(|v| {
                                let records: Vec<_> =
                                    self.store.data.revisions.iter().enumerate().filter(|(_, r)| r.video.id == v.id).map(|(i, _)| i).collect();
                                (v.id, v.name.clone(), c.name.clone(), records)
                            })
                        })
                        .filter(|(_, name, channel, records)| {
                            (self.history_all || !records.is_empty())
                                && (query.is_empty() || name.to_lowercase().contains(&query) || channel.to_lowercase().contains(&query))
                        })
                        .collect();
                    rows.sort_by(|a, b| {
                        b.3.last()
                            .map(|i| &self.store.data.revisions[*i].at)
                            .cmp(&a.3.last().map(|i| &self.store.data.revisions[*i].at))
                            .then(a.1.cmp(&b.1))
                    });
                    if rows.is_empty() {
                        ui.add_space(48.0);
                        ui.label(RichText::new("표시할 수정 기록이 없습니다").color(style::MUTED));
                    }
                    for (id, name, channel, records) in rows {
                        ui.push_id(id, |ui| {
                            let expanded = self.history_expanded.contains(&id);
                            let (rect, response) = ui.allocate_exact_size(vec2(width, 62.0), Sense::click());
                            if response.hovered() {
                                ui.painter().rect_filled(rect, 5, style::HOVER);
                            }
                            style::paint_icon(
                                ui.painter(),
                                egui::Rect::from_center_size(pos2(rect.left() + 12.0, rect.center().y), vec2(16.0, 16.0)),
                                if expanded { Icon::ChevronDown } else { Icon::ChevronRight },
                                style::MUTED,
                            );
                            let text_rect = egui::Rect::from_min_max(rect.min + vec2(32.0, 8.0), rect.max - vec2(76.0, 4.0));
                            ui.scope_builder(egui::UiBuilder::new().max_rect(text_rect), |ui| {
                                ui.add(egui::Label::new(style::heading(&name, 15.0)).truncate());
                                let latest =
                                    records.last().map(|i| display_time(&self.store.data.revisions[*i].at)).unwrap_or_else(|| "기록 없음".into());
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(format!("{channel} · {latest} · {}개 기록", records.len()))
                                            .size(figcraft_ui_icons::font_size(12.0))
                                            .color(style::MUTED),
                                    )
                                    .truncate(),
                                );
                            });
                            let button = egui::Rect::from_center_size(pos2(rect.right() - 24.0, rect.center().y), vec2(30.0, 30.0));
                            let go = ui.interact(button, ui.scope_id().with("open"), Sense::click()).on_hover_text("대본 열기");
                            style::paint_icon(ui.painter(), button.shrink(6.0), Icon::Script, if go.hovered() { style::TEXT } else { style::MUTED });
                            if go.clicked() {
                                open = Some(id);
                            } else if response.clicked() {
                                if expanded {
                                    self.history_expanded.remove(&id);
                                } else {
                                    self.history_expanded.insert(id);
                                }
                            }
                            if expanded {
                                egui::Frame::NONE.fill(style::SIDEBAR).corner_radius(6).inner_margin(egui::Margin::symmetric(20, 12)).show(
                                    ui,
                                    |ui| {
                                        ui.set_width((width - 40.0).max(80.0));
                                        if records.is_empty() {
                                            ui.label(RichText::new("아직 저장된 기록이 없습니다").color(style::MUTED));
                                        }
                                        let limit = self.history_visible.get(&id).copied().unwrap_or(50);
                                        for (index, record) in records.iter().rev().take(limit).enumerate() {
                                            let entry = &self.store.data.revisions[*record];
                                            ui.push_id((&entry.at, &entry.label), |ui| {
                                                if index > 0 {
                                                    ui.add_space(8.0);
                                                    ui.separator();
                                                    ui.add_space(8.0);
                                                }
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.label(RichText::new(display_time(&entry.at)).size(figcraft_ui_icons::font_size(13.0)));
                                                    ui.label(
                                                        RichText::new(&entry.label).size(figcraft_ui_icons::font_size(12.0)).color(style::MUTED),
                                                    );
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        if style::action_button(ui, "복원", false).clicked() {
                                                            restore = Some(entry.video.clone());
                                                        }
                                                    });
                                                });
                                                egui::CollapsingHeader::new("대본 미리보기").id_salt((id, &entry.at, &entry.label)).show(ui, |ui| {
                                                    ui.label(&entry.video.script_body);
                                                });
                                            });
                                        }
                                        if records.len() > limit && style::action_button(ui, "이전 기록 50개 더 보기", false).clicked() {
                                            self.history_visible.insert(id, limit + 50);
                                        }
                                    },
                                );
                                ui.add_space(8.0);
                            }
                            ui.separator();
                        });
                    }
                });
            });
        });
        if let Some(video) = restore {
            self.restore_history(video);
        }
        if let Some(video_id) = open {
            self.navigate(Route::Video { video_id, section: Section::Script });
        }
    }
    fn restore_history(&mut self, video: Video) {
        if let Err(error) = self.save_film() {
            self.message = error;
            return;
        }
        self.store.data.selected_video = Some(video.id);
        self.store.data.checkpoint("복원 전 기록");
        if let Err(error) = self.store.save() {
            self.message = error;
            return;
        }
        if let Err(error) = self.restore_film_revision(&video) {
            self.message = error;
            return;
        }
        let id = video.id;
        self.store.data.selected_video = Some(id);
        if let Some(current) = self.store.data.selected_mut() {
            *current = video;
        }
        self.store.data.checkpoint("기록 복원");
        self.save();
        self.message = "이 버전으로 복원했습니다".into();
    }
}
