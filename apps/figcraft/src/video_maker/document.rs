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

use super::rich_script::{RichScript, TextKind, line_end};
use egui::{Sense, Stroke, pos2, vec2};
use std::{
    ops::Range,
    time::{Duration, Instant},
};

pub(super) fn install_fonts(ctx: &egui::Context) {
    let mut fonts = ctx.fonts(|f| f.definitions().clone());
    // Prefer the platform's familiar sans face while preserving all installed CJK fallbacks.
    if !fonts.font_data.contains_key("inter") {
        for family in ["Segoe UI", "SF Pro Text", "Arial", "Noto Sans", "DejaVu Sans"] {
            if let Some(face) = figcraft_text::find_face(family, "Regular")
                && face.family.eq_ignore_ascii_case(family)
            {
                fonts.font_data.insert("purpose-regular".into(), std::sync::Arc::new(figcraft_ui_egui::theme::font_data(&face)));
                for family in [egui::FontFamily::Proportional, egui::FontFamily::Name("medium".into()), egui::FontFamily::Name("semibold".into())] {
                    if let Some(list) = fonts.families.get_mut(&family) {
                        list.insert(0, "purpose-regular".into());
                    }
                }
                break;
            }
        }
    }
    let mut bold = Vec::new();
    for family in ["Inter", "Segoe UI", "Arial", "DejaVu Sans", "Malgun Gothic", "맑은 고딕", "Noto Sans CJK KR", "Noto Sans CJK SC"] {
        if let Some(face) = figcraft_text::find_face(family, "Bold")
            && face.family.eq_ignore_ascii_case(family)
            && face.style.to_lowercase().contains("bold")
        {
            let name = format!("purpose-bold-{family}");
            fonts.font_data.insert(name.clone(), std::sync::Arc::new(figcraft_ui_egui::theme::font_data(&face)));
            bold.push(name);
        }
    }
    bold.extend(fonts.families.get(&egui::FontFamily::Name("semibold".into())).cloned().unwrap_or_default());
    fonts.families.insert(egui::FontFamily::Name("purpose-script-bold".into()), bold);
    ctx.set_fonts(fonts);
}

#[derive(Clone)]
struct Selection {
    editor: egui::Id,
    local: CCursorRange,
    start: usize,
    range: Range<usize>,
    position: egui::Pos2,
}
#[derive(Clone)]
struct ImageDrag {
    video: u64,
    image: u64,
}
#[derive(Clone, PartialEq)]
struct Snapshot {
    text: String,
    rich: RichScript,
}
#[derive(Clone, Default)]
struct History {
    past: Vec<Snapshot>,
    future: Vec<Snapshot>,
    typed_at: Option<Instant>,
}
#[derive(Clone, Copy)]
enum Mark {
    Bold,
    Italic,
    Underline,
    Strike,
}
impl Mark {
    fn current(self, s: &super::rich_script::InlineStyle) -> bool {
        match self {
            Self::Bold => s.bold,
            Self::Italic => s.italic,
            Self::Underline => s.underline,
            Self::Strike => s.strike,
        }
    }
    fn set(self, s: &mut super::rich_script::InlineStyle, value: bool) {
        match self {
            Self::Bold => s.bold = value,
            Self::Italic => s.italic = value,
            Self::Underline => s.underline = value,
            Self::Strike => s.strike = value,
        }
    }
}
#[derive(Clone, Copy)]
enum Format {
    Mark(Mark),
    Kind(TextKind),
    Size(f32),
    Leading(f32),
    Indent(bool),
}
fn snapshot(video: &super::model::Video) -> Snapshot {
    Snapshot { text: video.script_body.clone(), rich: video.script_rich.clone() }
}
fn edit_layout(text: &str, full: &str, start: usize, rich: &RichScript, base: f32, leading: f32, width: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = width;
    let mut run = String::new();
    let mut previous = None;
    let mut run_leading = 0.0;
    let mut kind = rich.kind(full, start);
    let mut at_line_start = super::rich_script::line_start(full, start) == start;
    for (i, c) in text.chars().enumerate() {
        let at = start + i;
        if at_line_start {
            kind = rich.paragraphs.iter().find(|p| p.at == at).map_or(TextKind::Text, |p| p.kind);
        }
        let marks = rich.style(at);
        let size = marks.size.filter(|v| v.is_finite()).unwrap_or_else(|| kind.size(base)).clamp(10.0, 48.0);
        let bold = marks.bold || kind.heading();
        let mut format = egui::TextFormat {
            font_id: if bold { FontId::new(size, egui::FontFamily::Name("purpose-script-bold".into())) } else { FontId::proportional(size) },
            color: style::TEXT,
            italics: marks.italic,
            line_height: Some(size * marks.line_height.filter(|v| v.is_finite()).unwrap_or(leading).clamp(1.0, 2.5)),
            underline: if marks.underline { Stroke::new(1.0, style::TEXT) } else { Stroke::NONE },
            strikethrough: if marks.strike { Stroke::new(1.0, style::TEXT) } else { Stroke::NONE },
            ..Default::default()
        };
        if bold {
            format.coords.push(b"wght", 700.0);
        }
        let indent = if kind == TextKind::Bullet && at_line_start { 20.0 } else { 0.0 };
        if previous.as_ref().is_some_and(|f| f != &format) || indent > 0.0 {
            if let Some(old) = previous.take() {
                job.append(&run, run_leading, old);
                run.clear();
            }
            run_leading = indent;
        }
        previous = Some(format);
        run.push(c);
        at_line_start = c == '\n';
    }
    if let Some(format) = previous {
        job.append(&run, run_leading, format);
    }
    job
}
fn floating_menu(ctx: &egui::Context, selection: &Selection, video: &super::model::Video) -> Option<Format> {
    let mut action = None;
    let current = video.script_rich.style(selection.range.start);
    let screen = ctx.content_rect();
    let position = pos2(
        selection.position.x.clamp(screen.left() + 8.0, (screen.right() - 525.0).max(screen.left() + 8.0)),
        (selection.position.y + 8.0).clamp(screen.top() + 53.0, (screen.bottom() - 100.0).max(screen.top() + 53.0)),
    );
    let popup = egui::Area::new(egui::Id::unique(("script-format", video.id))).order(egui::Order::Foreground).fixed_pos(position).show(ctx, |ui| {
        egui::Frame::popup(ui.style()).inner_margin(7.0).show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            ui.horizontal(|ui| {
                let kind = video.script_rich.kind(&video.script_body, selection.range.start);
                egui::ComboBox::from_id_salt("text-kind").height(260.0).selected_text(kind.label()).width(115.0).show_ui(ui, |ui| {
                    for candidate in TextKind::ALL {
                        if ui.selectable_label(kind == candidate, candidate.label()).clicked() {
                            action = Some(Format::Kind(candidate));
                        }
                    }
                });
                ui.separator();
                for (mark, icon, tip) in [
                    (Mark::Bold, style::Icon::Bold, "굵게 · Ctrl+B"),
                    (Mark::Italic, style::Icon::Italic, "기울임 · Ctrl+I"),
                    (Mark::Underline, style::Icon::Underline, "밑줄 · Ctrl+U"),
                    (Mark::Strike, style::Icon::Strike, "취소선"),
                ] {
                    if style::icon_toggle(ui, icon, video.script_rich.all(selection.range.clone(), |s| mark.current(s)), 26.0, tip).clicked() {
                        action = Some(Format::Mark(mark));
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("글자").small().color(style::MUTED));
                let mut size = current.size.unwrap_or(video.script_rich.kind(&video.script_body, selection.range.start).size(video.script_font_size));
                if ui.add(egui::DragValue::new(&mut size).range(10.0..=48.0).speed(0.25).suffix(" px")).changed() {
                    action = Some(Format::Size(size));
                }
                ui.add_space(6.0);
                ui.label(RichText::new("행간").small().color(style::MUTED));
                let mut height = current.line_height.unwrap_or(video.script_line_height);
                if ui.add(egui::DragValue::new(&mut height).range(1.0..=2.5).speed(0.05).fixed_decimals(1)).changed() {
                    action = Some(Format::Leading(height));
                }
                if style::icon_button(ui, style::Icon::Outdent, 26.0, "내어쓰기 · Shift+Tab").clicked() {
                    action = Some(Format::Indent(true));
                }
                if style::icon_button(ui, style::Icon::Indent, 26.0, "들여쓰기 · Tab").clicked() {
                    action = Some(Format::Indent(false));
                }
            });
        });
    });
    ctx.data_mut(|d| d.insert_temp(egui::Id::unique(("script-format-rect", video.id)), popup.response.rect));
    action
}
impl VideoMaker {
    pub(super) fn script(&mut self, ui: &mut egui::Ui) {
        let Some(mut video) = self.video().cloned() else { return };
        if !video.script_font_size.is_finite() {
            video.script_font_size = 16.0;
        }
        if !video.script_line_height.is_finite() {
            video.script_line_height = 1.6;
        }
        let before = snapshot(&video);
        let selection_id = egui::Id::unique(("script-selection", video.id));
        let focus_id = egui::Id::unique(("script-focus", video.id));
        let history_id = egui::Id::unique(("script-history", video.id));
        let mut selection = ui.ctx().data(|d| d.get_temp::<Selection>(selection_id));
        let mut history = ui.ctx().data(|d| d.get_temp::<History>(history_id)).unwrap_or_default();
        let mut restored = false;
        let focused = ui.ctx().data(|d| d.get_temp::<egui::Id>(focus_id)).is_some_and(|id| ui.memory(|m| m.has_focus(id)));
        if focused {
            let redo =
                ui.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z)));
            let undo = !redo && ui.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Z)));
            let state = if redo {
                history.future.pop().inspect(|_| history.past.push(snapshot(&video)))
            } else if undo {
                history.past.pop().inspect(|_| history.future.push(snapshot(&video)))
            } else {
                None
            };
            if let Some(state) = state {
                video.script_body = state.text;
                video.script_rich = state.rich;
                selection = None;
                restored = true;
                history.typed_at = None;
            }
        }
        if !video.script_rich.images_migrated {
            for id in &video.script_assets {
                if video.asset(*id).is_some_and(|a| a.kind == super::model::AssetKind::Image)
                    && !video.script_rich.images.iter().any(|i| i.asset_id == *id)
                {
                    video.script_rich.insert_image(*id, video.script_body.chars().count());
                }
            }
            video.script_rich.images_migrated = true;
        }
        video.script_rich.normalize(&video.script_body);
        let dragged = egui::DragAndDrop::payload::<ImageDrag>(ui.ctx()).filter(|d| d.video == video.id);
        let mut destination = None;
        let mut remove = None;
        let mut typing = false;
        let mut pointer_in_text = false;
        let mut action = None;
        let mut keyboard_action = None;
        if focused && selection.is_some() {
            for (key, mark) in [(egui::Key::B, Mark::Bold), (egui::Key::I, Mark::Italic), (egui::Key::U, Mark::Underline)] {
                if ui.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, key))) {
                    keyboard_action = Some(Format::Mark(mark));
                }
            }
        }
        ui.visuals_mut().selection.stroke.color = egui::Color32::BLACK;
        egui::ScrollArea::vertical().id_salt(("purpose-script-scroll", video.id)).auto_shrink([false, false]).show(ui, |ui| {
            let width = (ui.available_width() - 104.0).clamp(180.0, 760.0);
            let left = ((ui.available_width() - width) / 2.0).max(0.0);
            ui.add_space(60.0);
            ui.horizontal(|ui| {
                ui.add_space(left);
                ui.allocate_ui_with_layout(vec2(width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_max_width(width);
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.add(
                        TextEdit::singleline(&mut video.name)
                            .font(FontId::new(38.0, egui::FontFamily::Name("purpose-script-bold".into())))
                            .desired_width(width)
                            .frame(egui::Frame::NONE)
                            .hint_text("제목 없음"),
                    );
                    ui.add_space(22.0);
                    let images = video.script_rich.images.clone();
                    let mut start = 0;
                    for slot in 0..=images.len() {
                        let image = images.get(slot);
                        let end = image
                            .and_then(|i| video.script_rich.images.iter().find(|v| v.id == i.id))
                            .map_or_else(|| video.script_body.chars().count(), |i| i.at)
                            .max(start)
                            .min(video.script_body.chars().count());
                        let mut text = video.script_body[byte(&video.script_body, start)..byte(&video.script_body, end)].to_owned();
                        let old = text.clone();
                        let id = ui.make_persistent_id(("script-text", video.id, image.map(|i| i.id)));
                        if restored && let Some(mut state) = TextEdit::load_state(ui.ctx(), id) {
                            state.clear_undoer();
                            state.store(ui.ctx(), id);
                        }
                        if ui.memory(|m| m.has_focus(id)) {
                            let tab = ui.input_mut(|i| super::interactions::consume_key(i, egui::Modifiers::NONE, egui::Key::Tab));
                            let back = ui.input_mut(|i| super::interactions::consume_key(i, egui::Modifiers::SHIFT, egui::Key::Tab));
                            let enter = ui.input_mut(|i| super::interactions::consume_key(i, egui::Modifiers::NONE, egui::Key::Enter));
                            if (tab || back || enter)
                                && let Some(mut state) = TextEdit::load_state(ui.ctx(), id)
                            {
                                let range = state.cursor.char_range().unwrap_or_else(|| CCursorRange::one(CCursor::new(text.chars().count())));
                                let next = if enter { newline(&mut text, range) } else { indent(&mut text, range, back) };
                                state.cursor.set_char_range(Some(next));
                                state.store(ui.ctx(), id);
                            }
                        }
                        let rich = video.script_rich.clone();
                        let full = video.script_body.clone();
                        let mut layouter = |ui: &egui::Ui, buffer: &dyn egui::TextBuffer, wrap: f32| {
                            ui.fonts_mut(|fonts| {
                                fonts.layout_job(edit_layout(
                                    buffer.as_str(),
                                    &full,
                                    start,
                                    &rich,
                                    video.script_font_size,
                                    video.script_line_height,
                                    wrap,
                                ))
                            })
                        };
                        let output = TextEdit::multiline(&mut text)
                            .id(id)
                            .lock_focus(true)
                            .font(FontId::proportional(video.script_font_size))
                            .layouter(&mut layouter)
                            .frame(egui::Frame::NONE)
                            .desired_width(width)
                            .desired_rows(1)
                            .hint_text(if images.is_empty() { "대본을 작성하세요…" } else { "" })
                            .show(ui);
                        if old != text {
                            let previous = video.script_body.clone();
                            video.script_body.replace_range(byte(&previous, start)..byte(&previous, end), &text);
                            video.script_rich.text_changed(&previous, &video.script_body);
                            typing = true;
                        }
                        let next = start + text.chars().count();
                        for paragraph in &video.script_rich.paragraphs {
                            if paragraph.kind == TextKind::Bullet && paragraph.at >= start && paragraph.at < next {
                                let r = output.galley.pos_from_cursor(CCursor::new(paragraph.at - start)).translate(output.galley_pos.to_vec2());
                                ui.painter().circle_filled(pos2(output.galley_pos.x + 6.0, r.center().y), 2.5, style::TEXT);
                            }
                        }
                        pointer_in_text |= ui.ctx().pointer_hover_pos().is_some_and(|p| output.response.rect.contains(p));
                        if output.response.has_focus() {
                            ui.ctx().data_mut(|d| d.insert_temp(focus_id, id));
                            if let Some(range) = output.cursor_range {
                                let a = range.primary.index.0.min(range.secondary.index.0);
                                let b = range.primary.index.0.max(range.secondary.index.0);
                                if a < b {
                                    let r = output.galley.pos_from_cursor(CCursor::new(b)).translate(output.galley_pos.to_vec2());
                                    selection =
                                        Some(Selection { editor: id, local: range, start, range: start + a..start + b, position: r.right_bottom() });
                                } else {
                                    selection = None;
                                }
                            }
                        }
                        if dragged.is_some()
                            && let Some(pos) = ui.ctx().pointer_hover_pos()
                            && output.response.rect.expand2(vec2(0.0, 8.0)).contains(pos)
                        {
                            let local = output.galley.cursor_from_pos(pos - output.galley_pos).index.0;
                            let at = line_end(&video.script_body, (start + local).min(next)).min(next);
                            let r = output.galley.pos_from_cursor(CCursor::new(at.saturating_sub(start + 1))).translate(output.galley_pos.to_vec2());
                            destination = Some((at, None));
                            ui.painter().hline(output.response.rect.x_range(), r.bottom() + 3.0, Stroke::new(2.0, style::BLUE));
                        }
                        start = next;
                        if let Some(image) = image {
                            let asset = video.asset(image.asset_id);
                            let resize_id = egui::Id::unique(("script-image-resize", video.id, image.id));
                            let mut resizing = ui.ctx().data(|d| d.get_temp::<bool>(resize_id).unwrap_or(false));
                            let image_width = image.width.filter(|w| w.is_finite()).unwrap_or(width.min(720.0)).clamp(48.0, width.max(48.0));
                            let response = if let Some(texture) = self.thumbnails.get(&image.asset_id) {
                                ui.add(
                                    egui::Image::new(texture)
                                        .fit_to_exact_size(vec2(image_width, image_width * texture.size_vec2().y / texture.size_vec2().x.max(1.0)))
                                        .sense(Sense::click_and_drag()),
                                )
                            } else {
                                let (r, response) = ui.allocate_exact_size(vec2(image_width, 220.0), Sense::click_and_drag());
                                ui.painter().rect_filled(r, 4, style::THUMB);
                                ui.painter().text(
                                    r.center(),
                                    egui::Align2::CENTER_CENTER,
                                    asset.map_or("이미지를 찾을 수 없습니다", |a| a.name.as_str()),
                                    FontId::proportional(14.0),
                                    style::MUTED,
                                );
                                response
                            };
                            if ui.is_rect_visible(response.rect)
                                && let Some(asset) = asset
                            {
                                self.request_thumbnail(asset);
                            }
                            if response.double_clicked() {
                                resizing = !resizing;
                                ui.ctx().data_mut(|d| d.insert_temp(resize_id, resizing));
                            }
                            if resizing {
                                ui.painter().rect_stroke(response.rect, 2, Stroke::new(1.5, style::BLUE), egui::StrokeKind::Inside);
                                let handle_rect = egui::Rect::from_center_size(response.rect.right_bottom() - vec2(7.0, 7.0), vec2(18.0, 18.0));
                                let handle =
                                    ui.interact(handle_rect, resize_id.with("handle"), Sense::drag()).on_hover_cursor(egui::CursorIcon::ResizeNwSe);
                                ui.painter().rect_filled(handle_rect.shrink(4.0), 2, style::BLUE);
                                if handle.drag_started() {
                                    let origin = ui.input(|i| i.pointer.press_origin()).map_or(response.rect.right(), |p| p.x);
                                    ui.ctx().data_mut(|d| d.insert_temp(resize_id.with("width"), (image_width, origin)));
                                }
                                if handle.dragged() {
                                    let (initial, origin) = ui
                                        .ctx()
                                        .data(|d| d.get_temp::<(f32, f32)>(resize_id.with("width")).unwrap_or((image_width, response.rect.right())));
                                    let pointer = ui.input(|i| i.pointer.interact_pos()).map_or(origin, |p| p.x);
                                    if let Some(current) = video.script_rich.images.iter_mut().find(|i| i.id == image.id) {
                                        current.width = Some((initial + pointer - origin).clamp(48.0, width.max(48.0)));
                                    }
                                }
                                if ui.input(|i| i.key_pressed(egui::Key::Escape))
                                    || ui.input(|i| i.pointer.any_pressed())
                                        && ui.ctx().pointer_hover_pos().is_some_and(|p| !response.rect.expand(12.0).contains(p))
                                {
                                    ui.ctx().data_mut(|d| d.insert_temp(resize_id, false));
                                }
                            } else {
                                response.dnd_set_drag_payload(ImageDrag { video: video.id, image: image.id });
                            }
                            response.clone().on_hover_cursor(egui::CursorIcon::Grab).context_menu(|ui| {
                                if style::menu_item(ui, "대본에서 이미지 제거", style::Icon::Trash, true).clicked() {
                                    remove = Some(image.id);
                                    ui.close();
                                }
                            });
                            if dragged.is_some() && response.contains_pointer() {
                                destination = Some((image.at, Some(image.id)));
                                ui.painter().hline(response.rect.x_range(), response.rect.top() - 3.0, Stroke::new(2.0, style::BLUE));
                            }
                            ui.add_space(14.0);
                        }
                    }
                    let (tail, _) = ui.allocate_exact_size(vec2(width, 100.0), Sense::hover());
                    if dragged.is_some() && ui.ctx().pointer_hover_pos().is_some_and(|p| tail.contains(p)) {
                        destination = Some((video.script_body.chars().count(), None));
                        ui.painter().hline(tail.x_range(), tail.top(), Stroke::new(2.0, style::BLUE));
                    }
                    // Non-image media remain links, images live in the document flow above.
                    for asset in video.assets.iter().filter(|a| a.kind != super::model::AssetKind::Image && video.script_assets.contains(&a.id)) {
                        ui.label(RichText::new(&asset.name).small().color(style::BLUE));
                    }
                });
            });
        });
        if !pointer_in_text && ui.input(|i| i.pointer.any_pressed()) && !egui::Popup::is_any_open(ui.ctx()) {
            let inside = ui
                .ctx()
                .data(|d| d.get_temp::<egui::Rect>(egui::Id::unique(("script-format-rect", video.id))))
                .is_some_and(|r| ui.ctx().pointer_hover_pos().is_some_and(|p| r.contains(p)));
            if !inside {
                if let Some(selected) = &selection {
                    ui.ctx().memory_mut(|m| m.surrender_focus(selected.editor));
                }
                selection = None;
            }
        }
        if let Some(selection) = &selection {
            action = floating_menu(ui.ctx(), selection, &video).or(keyboard_action);
        }
        if let (Some(format), Some(selected)) = (action, selection.as_mut()) {
            let range = selected.range.clone();
            match format {
                Format::Mark(mark) => {
                    let enable = !video.script_rich.all(range.clone(), |s| mark.current(s));
                    video.script_rich.apply(range, |s| mark.set(s, enable));
                }
                Format::Kind(kind) => video.script_rich.set_kind(&video.script_body, range, kind),
                Format::Size(size) => video.script_rich.apply(range, |s| s.size = Some(size)),
                Format::Leading(height) => video.script_rich.apply(range, |s| s.line_height = Some(height)),
                Format::Indent(out) => {
                    let old = video.script_body.clone();
                    let global = CCursorRange::two(CCursor::new(range.start), CCursor::new(range.end));
                    let next = indent(&mut video.script_body, global, out);
                    video.script_rich.text_changed(&old, &video.script_body);
                    selected.range = next.primary.index.0.min(next.secondary.index.0)..next.primary.index.0.max(next.secondary.index.0);
                    selected.local = CCursorRange::two(
                        CCursor::new(selected.range.start.saturating_sub(selected.start)),
                        CCursor::new(selected.range.end.saturating_sub(selected.start)),
                    );
                }
            }
            if let Some(mut state) = TextEdit::load_state(ui.ctx(), selected.editor) {
                state.cursor.set_char_range(Some(selected.local));
                state.store(ui.ctx(), selected.editor);
            }
            if !matches!(format, Format::Size(_) | Format::Leading(_)) {
                ui.ctx().memory_mut(|m| m.request_focus(selected.editor));
            }
        }
        if let (Some(drag), Some((at, before_image))) = (dragged, destination)
            && ui.input(|i| i.pointer.any_released())
        {
            if before_image != Some(drag.image)
                && let Some(index) = video.script_rich.images.iter().position(|i| i.id == drag.image)
            {
                let mut image = video.script_rich.images.remove(index);
                image.at = at;
                let insert = before_image
                    .and_then(|id| video.script_rich.images.iter().position(|i| i.id == id))
                    .unwrap_or_else(|| video.script_rich.images.iter().position(|i| i.at > at).unwrap_or(video.script_rich.images.len()));
                video.script_rich.images.insert(insert, image);
            }
            egui::DragAndDrop::clear_payload(ui.ctx());
            selection = None;
        }
        if let Some(id) = remove {
            video.script_rich.images.retain(|i| i.id != id);
        }
        if before != snapshot(&video) {
            if !restored {
                if !typing || history.typed_at.is_none_or(|at| at.elapsed() > Duration::from_millis(600)) {
                    history.past.push(before);
                    if history.past.len() > 100 {
                        history.past.remove(0);
                    }
                }
                history.future.clear();
                history.typed_at = typing.then(Instant::now);
            }
            if let Some(target) = self.store.data.selected_mut() {
                target.script_body = video.script_body;
                target.script_rich = video.script_rich;
            }
            self.save();
        }
        if self.video().is_some_and(|v| v.name != video.name) {
            if let Some(target) = self.store.data.selected_mut() {
                target.name = video.name;
            }
            self.save();
        }
        ui.ctx().data_mut(|d| {
            if let Some(selection) = selection {
                d.insert_temp(selection_id, selection);
            } else {
                d.remove::<Selection>(selection_id);
            }
            d.insert_temp(history_id, history);
        });
    }
}
