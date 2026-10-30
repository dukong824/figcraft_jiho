//! Human-authored common and channel-specific operating instructions.
use super::{VideoMaker, navigation::Route, style};
use egui::{FontFamily, FontId, RichText, TextEdit, vec2};
impl VideoMaker {
    pub(super) fn guide_text(&self, video: u64) -> Result<String, String> {
        let channel = self.store.data.channels.iter().find(|c| c.videos.iter().any(|v| v.id == video)).ok_or("영상의 채널을 찾을 수 없습니다")?;
        Ok(format!("공통 가이드:\n{}\n\n채널: {}\n채널 가이드:\n{}", self.store.data.codex_guide, channel.name, channel.codex_guide))
    }
    pub(super) fn guide_page(&mut self, ui: &mut egui::Ui, selected: Option<u64>) {
        let (width, left) = style::page_width(ui, true);
        let mut next = selected;
        let mut changed = false;
        egui::ScrollArea::vertical().id_salt(("purpose-guide-scroll", selected)).show(ui, |ui| {
            ui.add_space(style::PAGE_TOP + 12.0);
            ui.horizontal(|ui| {
                ui.add_space(left);
                ui.allocate_ui_with_layout(vec2(width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(width);
                    ui.label(
                        RichText::new("Codex 가이드")
                            .font(FontId::new(figcraft_ui_icons::font_size(style::PAGE_TITLE), FontFamily::Name("purpose-script-bold".into()))),
                    );
                    ui.add_space(style::TITLE_GAP);
                    let label = selected
                        .and_then(|id| self.store.data.channels.iter().find(|c| c.id == id))
                        .map_or("모든 채널 · 공통 가이드", |c| c.name.as_str());
                    let response = style::dropdown(ui, label, 240.0);
                    egui::Popup::menu(&response).show(|ui| {
                        if ui.selectable_value(&mut next, None, "모든 채널 · 공통 가이드").clicked() {
                            ui.close();
                        }
                        for channel in &self.store.data.channels {
                            if ui.selectable_value(&mut next, Some(channel.id), &channel.name).clicked() {
                                ui.close();
                            }
                        }
                    });
                    ui.add_space(26.0);
                    let text = if let Some(id) = selected {
                        self.store.data.channels.iter_mut().find(|c| c.id == id).map(|c| &mut c.codex_guide)
                    } else {
                        Some(&mut self.store.data.codex_guide)
                    };
                    if let Some(text) = text {
                        let mut layout = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap: f32| {
                            let mut job = egui::text::LayoutJob::simple(
                                text.as_str().into(),
                                FontId::proportional(figcraft_ui_icons::font_size(16.0)),
                                style::TEXT,
                                wrap,
                            );
                            for section in &mut job.sections {
                                section.format.line_height = Some(figcraft_ui_icons::font_size(27.0));
                            }
                            ui.fonts_mut(|f| f.layout_job(job))
                        };
                        changed = TextEdit::multiline(text)
                            .id(egui::Id::unique(("guide-body", selected)))
                            .frame(egui::Frame::NONE)
                            .desired_width(width)
                            .desired_rows(16)
                            .lock_focus(true)
                            .layouter(&mut layout)
                            .hint_text("채널의 설명, 시청자, 말투, 제작 목표와 편집 원칙을 적어주세요…")
                            .show(ui)
                            .response
                            .changed();
                    }
                });
            });
        });
        if changed {
            self.save();
        }
        if next != selected {
            self.navigate(Route::Guide { channel_id: next });
        }
    }
}
