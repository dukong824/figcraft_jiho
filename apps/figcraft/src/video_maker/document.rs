//! Freeform document typography and indentation, using character offsets for CJK text.
use super::{VideoMaker, style};
use egui::{
    FontId, RichText, TextEdit,
    text::{CCursor, CCursorRange},
};
fn byte(text: &str, index: usize) -> usize {
    text.char_indices().nth(index).map_or(text.len(), |(i, _)| i)
}

pub(super) fn indent(text: &mut String, range: CCursorRange, outdent: bool) -> CCursorRange {
    let a = range.primary.index.0.min(range.secondary.index.0);
    let b = range.primary.index.0.max(range.secondary.index.0);
    let start = text[..byte(text, a)].rfind('\n').map_or(0, |i| i + 1);
    let end_at = byte(text, b);
    let end = if b > a && text[..end_at].ends_with('\n') { end_at } else { text[end_at..].find('\n').map_or(text.len(), |i| end_at + i + 1) };
    let mut rebuilt = String::new();
    let mut edits = Vec::new();
    let mut pos = text[..start].chars().count();
    for line in text[start..end].split_inclusive('\n') {
        let remove = if line.starts_with('\t') { 1 } else { line.chars().take_while(|c| *c == ' ').count().min(4) };
        if outdent {
            rebuilt.push_str(&line[byte(line, remove)..]);
            edits.push((pos, -(remove as isize)));
        } else {
            rebuilt.push('\t');
            rebuilt.push_str(line);
            edits.push((pos, 1));
        }
        pos += line.chars().count();
    }
    if start == end && !outdent {
        rebuilt.push('\t');
        edits.push((pos, 1));
    }
    text.replace_range(start..end, &rebuilt);
    let shift = |cursor: CCursor| {
        let mut next = cursor.index.0 as isize;
        for (at, delta) in &edits {
            if cursor.index.0 >= *at {
                next += if *delta < 0 { (*delta).max(-((cursor.index.0 - at) as isize)) } else { *delta };
            }
        }
        CCursor::new(next.max(0) as usize)
    };
    CCursorRange { primary: shift(range.primary), secondary: shift(range.secondary), ..range }
}
fn newline(text: &mut String, range: CCursorRange) -> CCursorRange {
    let a = byte(text, range.primary.index.0.min(range.secondary.index.0));
    let b = byte(text, range.primary.index.0.max(range.secondary.index.0));
    let start = text[..a].rfind('\n').map_or(0, |i| i + 1);
    let leading: String = text[start..a].chars().take_while(|c| *c == '\t' || *c == ' ').collect();
    let insert = format!("\n{leading}");
    let at = text[..a].chars().count() + insert.chars().count();
    text.replace_range(a..b, &insert);
    CCursorRange::one(CCursor::new(at))
}

impl VideoMaker {
    pub(super) fn script(&mut self, ui: &mut egui::Ui) {
        let mut dirty = false;
        let assets = self.video().map(|v| v.assets.clone()).unwrap_or_default();
        let Some(video) = self.store.data.selected_mut() else { return };
        if !video.script_font_size.is_finite() {
            video.script_font_size = 16.0;
        }
        if !video.script_line_height.is_finite() {
            video.script_line_height = 1.6;
        }
        let id = ui.make_persistent_id(("purpose-script-body", video.id));
        let mut indent_action = None;
        ui.horizontal(|ui| {
            ui.label(RichText::new("글자").size(12.0).color(style::MUTED));
            dirty |= ui.add(egui::DragValue::new(&mut video.script_font_size).range(12.0..=36.0).suffix(" px").speed(0.25)).changed();
            ui.add_space(10.0);
            ui.label(RichText::new("행간").size(12.0).color(style::MUTED));
            dirty |= ui.add(egui::DragValue::new(&mut video.script_line_height).range(1.0..=2.5).fixed_decimals(1).speed(0.05)).changed();
            ui.separator();
            if ui.small_button("←").on_hover_text("내어쓰기 · Shift+Tab").clicked() {
                indent_action = Some(true);
            }
            if ui.small_button("→").on_hover_text("들여쓰기 · Tab").clicked() {
                indent_action = Some(false);
            }
        });
        let focused = ui.memory(|m| m.has_focus(id));
        let tab = focused && ui.input_mut(|i| super::interactions::consume_key(i, egui::Modifiers::NONE, egui::Key::Tab));
        let backtab = focused && ui.input_mut(|i| super::interactions::consume_key(i, egui::Modifiers::SHIFT, egui::Key::Tab));
        let enter = focused && ui.input_mut(|i| super::interactions::consume_key(i, egui::Modifiers::NONE, egui::Key::Enter));
        if (tab || backtab || enter || indent_action.is_some())
            && let Some(mut state) = TextEdit::load_state(ui.ctx(), id)
        {
            let range = state.cursor.char_range().unwrap_or_else(|| CCursorRange::one(CCursor::new(video.script_body.chars().count())));
            let next = if enter {
                newline(&mut video.script_body, range)
            } else {
                indent(&mut video.script_body, range, backtab || indent_action == Some(true))
            };
            state.cursor.set_char_range(Some(next));
            state.store(ui.ctx(), id);
            ui.memory_mut(|m| m.request_focus(id));
            dirty = true;
        }
        egui::ScrollArea::vertical().id_salt(("purpose-script-scroll", video.id)).auto_shrink([false, false]).show(ui, |ui| {
            let width = ui.available_width().min(810.0);
            let left = ((ui.available_width() - width) / 2.0).max(0.0);
            ui.add_space(60.0);
            ui.horizontal(|ui| {
                ui.add_space(left);
                ui.allocate_ui_with_layout(egui::vec2(width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_max_width(width);
                    dirty |= ui
                        .add(
                            TextEdit::singleline(&mut video.name)
                                .font(FontId::proportional(38.0))
                                .desired_width(width)
                                .frame(egui::Frame::NONE)
                                .hint_text("제목 없음"),
                        )
                        .changed();
                    ui.add_space(22.0);
                    let font = FontId::proportional(video.script_font_size.clamp(12.0, 36.0));
                    let color = style::TEXT;
                    let line_height = font.size * video.script_line_height.clamp(1.0, 2.5);
                    let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, width: f32| {
                        let mut job = egui::text::LayoutJob::default();
                        job.wrap.max_width = width;
                        job.append(
                            text.as_str(),
                            0.0,
                            egui::TextFormat { font_id: font.clone(), color, line_height: Some(line_height), ..Default::default() },
                        );
                        ui.fonts_mut(|f| f.layout_job(job))
                    };
                    let height = ((video.script_body.lines().count().max(1) as f32) * line_height + 60.0).max(ui.available_height().max(420.0));
                    dirty |= ui
                        .add_sized(
                            [width, height],
                            TextEdit::multiline(&mut video.script_body)
                                .id(id)
                                .lock_focus(true)
                                .layouter(&mut layouter)
                                .frame(egui::Frame::NONE)
                                .desired_width(width)
                                .hint_text("대본을 작성하세요…"),
                        )
                        .changed();
                    if !video.script_assets.is_empty() {
                        ui.add_space(18.0);
                        ui.label(RichText::new("첨부 파일").small().color(style::MUTED));
                        for asset in assets.iter().filter(|a| video.script_assets.contains(&a.id)) {
                            ui.label(RichText::new(&asset.name).size(13.0).color(style::BLUE));
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn indent_and_outdent_multiple_korean_lines_preserve_selection_and_text() {
        let mut text = "첫 문단\n두 번째\n마지막".to_owned();
        let range = CCursorRange::two(CCursor::new(0), CCursor::new(10));
        let next = indent(&mut text, range, false);
        assert_eq!(text, "\t첫 문단\n\t두 번째\n마지막");
        let restored = indent(&mut text, next, true);
        assert_eq!(text, "첫 문단\n두 번째\n마지막");
        assert_eq!(restored, range);
    }
    #[test]
    fn enter_continues_nested_text_and_outdent_handles_spaces() {
        let mut text = "\t\t중첩".to_owned();
        let next = newline(&mut text, CCursorRange::one(CCursor::new(4)));
        assert_eq!(text, "\t\t중첩\n\t\t");
        assert_eq!(next.primary.index.0, 7);
        let mut text = "    가나다".to_owned();
        let next = indent(&mut text, CCursorRange::one(CCursor::new(6)), true);
        assert_eq!(text, "가나다");
        assert_eq!(next.primary.index.0, 2);
    }
}
