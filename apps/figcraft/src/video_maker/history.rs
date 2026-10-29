use super::{
    VideoMaker,
    model::Section,
    navigation::Route,
    style::{self, Icon},
};
use egui::{Align2, FontId, Rect, RichText, Sense, pos2, vec2};

impl VideoMaker {
    pub(super) fn history_page(&mut self, ui: &mut egui::Ui) {
        if let Some(id) = self.history_video {
            self.history_detail(ui, id);
            return;
        }
        ui.add_space(16.0);
        ui.label(RichText::new("수정 기록").size(32.0).strong());
        ui.label(RichText::new("영상별로 최근 수정 내역을 찾아보고 이전 버전을 복원하세요.").size(14.0).color(style::MUTED));
        ui.add_space(24.0);
        ui.horizontal(|ui| {
            if ui.selectable_label(!self.history_all, "최근 수정").clicked() {
                self.history_all = false;
            }
            if ui.selectable_label(self.history_all, "모든 영상").clicked() {
                self.history_all = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(egui::TextEdit::singleline(&mut self.history_query).desired_width(245.0).hint_text("영상 또는 채널 검색"));
            });
        });
        ui.add_space(12.0);
        let query = self.history_query.trim().to_lowercase();
        let mut rows: Vec<_> = self
            .store
            .data
            .channels
            .iter()
            .flat_map(|channel| {
                channel.videos.iter().map(|video| {
                    let records: Vec<_> = self.store.data.revisions.iter().filter(|r| r.video.id == video.id).collect();
                    (video.id, video.name.clone(), channel.name.clone(), records.last().map(|r| r.at.clone()).unwrap_or_default(), records.len())
                })
            })
            .filter(|(_, name, channel, at, _)| {
                (self.history_all || !at.is_empty())
                    && (query.is_empty() || name.to_lowercase().contains(&query) || channel.to_lowercase().contains(&query))
            })
            .collect();
        rows.sort_by(|a, b| b.3.cmp(&a.3).then(a.1.cmp(&b.1)));
        let width = ui.available_width();
        let title = (width * 0.37).max(180.0);
        let channel = (width * 0.20).max(100.0);
        let time = (width * 0.25).max(150.0);
        let (header, _) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
        for (x, text) in [
            (header.left() + 10.0, "영상 이름"),
            (header.left() + title, "채널"),
            (header.left() + title + channel, "마지막 수정 시간"),
            (header.left() + title + channel + time, "기록"),
        ] {
            ui.painter().text(pos2(x, header.center().y), Align2::LEFT_CENTER, text, FontId::proportional(12.0), style::MUTED);
        }
        ui.painter().line_segment([header.left_bottom(), header.right_bottom()], egui::Stroke::new(1.0, style::BORDER));
        let mut open = None;
        let mut detail = None;
        egui::ScrollArea::vertical().id_salt("history-page-list").show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (id, name, channel_name, at, count) in rows {
                ui.push_id(id, |ui| {
                    let (rect, response) = ui.allocate_exact_size(vec2(width, 47.0), Sense::click());
                    if response.hovered() {
                        ui.painter().rect_filled(rect, 3, style::HOVER);
                    }
                    style::paint_icon(
                        ui.painter(),
                        Rect::from_center_size(pos2(rect.left() + 15.0, rect.center().y), vec2(17.0, 17.0)),
                        Icon::Video,
                        style::MUTED,
                    );
                    for (x, w, text, color) in [
                        (rect.left() + 33.0, title - 40.0, name.as_str(), style::TEXT),
                        (rect.left() + title, channel - 12.0, channel_name.as_str(), style::MUTED),
                        (rect.left() + title + channel, time - 12.0, if at.is_empty() { "기록 없음" } else { at.as_str() }, style::MUTED),
                    ] {
                        let clip = Rect::from_min_max(pos2(x, rect.top()), pos2(x + w, rect.bottom()));
                        ui.painter().with_clip_rect(clip).text(
                            pos2(x, rect.center().y),
                            Align2::LEFT_CENTER,
                            text,
                            FontId::proportional(14.0),
                            color,
                        );
                    }
                    let action =
                        Rect::from_min_max(pos2(rect.left() + title + channel + time, rect.top() + 5.0), pos2(rect.right(), rect.bottom() - 5.0));
                    let action_response = ui.interact(action, ui.scope_id().with("records"), Sense::click());
                    ui.painter().text(
                        action.left_center(),
                        Align2::LEFT_CENTER,
                        format!("{}개 · 기록 보기", count),
                        FontId::proportional(12.0),
                        style::BLUE,
                    );
                    if action_response.clicked() {
                        detail = Some(id);
                    } else if response.clicked() {
                        open = Some(id);
                    }
                    ui.painter().line_segment([rect.left_bottom(), rect.right_bottom()], egui::Stroke::new(1.0, style::BORDER));
                });
            }
        });
        if let Some(id) = detail {
            self.history_video = Some(id);
        }
        if let Some(video_id) = open {
            self.navigate(Route::Video { video_id, section: Section::Script });
        }
    }

    fn history_detail(&mut self, ui: &mut egui::Ui, id: u64) {
        if ui.button("← 모든 영상의 수정 기록").clicked() {
            self.history_video = None;
            return;
        }
        let name = self
            .store
            .data
            .channels
            .iter()
            .flat_map(|c| &c.videos)
            .find(|v| v.id == id)
            .map(|v| v.name.clone())
            .unwrap_or_else(|| "삭제된 영상".into());
        ui.add_space(20.0);
        ui.label(RichText::new(name).size(30.0).strong());
        ui.label(RichText::new("대본·에셋 목록·FilmCraft 편집 프로젝트의 저장 기록").color(style::MUTED));
        let entries: Vec<_> = self.store.data.revisions.iter().rev().filter(|r| r.video.id == id).cloned().collect();
        let mut restore = None;
        egui::ScrollArea::vertical().id_salt(("history-page-detail", id)).show(ui, |ui| {
            if entries.is_empty() {
                ui.label("아직 저장된 기록이 없습니다.");
            }
            for (index, entry) in entries.into_iter().enumerate() {
                ui.push_id(index, |ui| {
                    ui.add_space(20.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&entry.at).strong());
                        ui.label(RichText::new(&entry.label).small().color(style::MUTED));
                        if ui.button("이 버전으로 복원").clicked() {
                            restore = Some(entry.video.clone());
                        }
                    });
                    ui.label(&entry.video.name);
                    egui::CollapsingHeader::new("대본 미리보기").show(ui, |ui| {
                        ui.label(&entry.video.script_body);
                    });
                    ui.separator();
                });
            }
        });
        if let Some(video) = restore {
            if let Err(error) = self.restore_film_revision(&video) {
                self.message = error;
                return;
            }
            self.checkpoint();
            self.store.data.selected_video = Some(id);
            self.store.data.checkpoint("복원 전 기록");
            if let Some(current) = self.store.data.selected_mut() {
                *current = video;
            }
            self.store.data.checkpoint("기록 복원");
            self.save();
            self.navigate(Route::Video { video_id: id, section: Section::Script });
        }
    }
}
