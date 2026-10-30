//! Human-authored common and channel guides, edited together on one document page.
use super::{VideoMaker, style};
use egui::{FontId, TextEdit, vec2};
impl VideoMaker {
    pub(super) fn guide_text(&self, video: u64) -> Result<String, String> {
        let channel = self.store.data.channels.iter().find(|c| c.videos.iter().any(|v| v.id == video)).ok_or("영상의 채널을 찾을 수 없습니다")?;
        Ok(format!("공통 가이드:\n{}\n\n채널: {}\n채널 가이드:\n{}", self.store.data.codex_guide, channel.name, channel.codex_guide))
    }
    pub(super) fn guide_page(&mut self, ui: &mut egui::Ui, _selected: Option<u64>) {
        let mut changed = false;
        let mut guides = vec![(None, "공통 가이드".to_owned())];
        guides.extend(self.store.data.channels.iter().map(|c| (Some(c.id), c.name.clone())));
        egui::ScrollArea::vertical().id_salt("purpose-all-guides").auto_shrink([false, false]).show(ui, |ui| {
            let (width, left) = style::page_width(ui, true);
            ui.add_space(style::DOCUMENT_TOP);
            ui.horizontal(|ui| {
                ui.add_space(left);
                ui.allocate_ui_with_layout(vec2(width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_width(width);
                    ui.label(style::heading("Codex 가이드", style::PAGE_TITLE));
                    ui.add_space(style::TITLE_GAP);
                    for (id, name) in guides {
                        ui.push_id(("guide-section", id), |ui| {
                            let open = self.guide_open.contains(&id);
                            let (rect, response) =
                                ui.allocate_exact_size(vec2(width, 44.0_f32.max(figcraft_ui_icons::font_size(32.0))), egui::Sense::click());
                            if response.hovered() {
                                ui.painter().rect_filled(rect, 5, style::SIDEBAR);
                            }
                            style::paint_icon(
                                ui.painter(),
                                egui::Rect::from_center_size(egui::pos2(rect.left() + 12.0, rect.center().y), vec2(18.0, 18.0)),
                                if open { style::Icon::ChevronDown } else { style::Icon::ChevronRight },
                                style::TEXT,
                            );
                            let galley = ui.fonts_mut(|f| {
                                let mut job = egui::text::LayoutJob::simple(
                                    name.clone(),
                                    egui::FontId::new(figcraft_ui_icons::font_size(23.0), egui::FontFamily::Name("purpose-script-bold".into())),
                                    style::TEXT,
                                    (width - 34.0).max(20.0),
                                );
                                job.wrap.max_rows = 1;
                                f.layout_job(job)
                            });
                            ui.painter().galley(egui::pos2(rect.left() + 34.0, rect.center().y - galley.size().y / 2.0), galley, style::TEXT);
                            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &name));
                            response.clone().on_hover_text(&name);
                            if response.clicked() {
                                if open {
                                    self.guide_open.remove(&id);
                                } else {
                                    self.guide_open.insert(id);
                                }
                            }
                            if self.guide_open.contains(&id) {
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    ui.add_space(34.0);
                                    ui.vertical(|ui| {
                                        let text = if let Some(id) = id {
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
                                                for s in &mut job.sections {
                                                    s.format.line_height = Some(figcraft_ui_icons::font_size(27.0));
                                                }
                                                ui.fonts_mut(|f| f.layout_job(job))
                                            };
                                            changed |= TextEdit::multiline(text)
                                                .id(egui::Id::unique(("guide-body", id)))
                                                .frame(egui::Frame::NONE)
                                                .desired_width((width - 34.0).max(20.0))
                                                .desired_rows(2)
                                                .lock_focus(true)
                                                .layouter(&mut layout)
                                                .hint_text(if id.is_none() {
                                                    "모든 채널에 적용할 지침을 적으세요…"
                                                } else {
                                                    "채널의 설명, 말투, 목표와 편집 원칙을 적으세요…"
                                                })
                                                .show(ui)
                                                .response
                                                .changed();
                                        }
                                    });
                                });
                            }
                            ui.add_space(26.0);
                        });
                    }
                });
            });
        });
        if changed {
            self.save();
        }
    }
}
