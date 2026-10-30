use egui::{Color32, Context, CornerRadius, FontId, Id, Painter, Rect, Response, Sense, Stroke, Ui, pos2, vec2};

pub const TEXT: Color32 = Color32::from_rgb(55, 53, 47);
pub const MUTED: Color32 = Color32::from_rgb(120, 119, 116);
pub const FAINT: Color32 = Color32::from_rgb(155, 154, 151);
pub const SIDEBAR: Color32 = Color32::from_rgb(247, 247, 245);
pub const SURFACE: Color32 = Color32::WHITE;
pub const HOVER: Color32 = Color32::from_rgb(238, 238, 235);
pub const SELECTED: Color32 = Color32::from_rgb(232, 232, 229);
pub const BORDER: Color32 = Color32::from_rgb(231, 231, 228);
pub const THUMB: Color32 = Color32::from_rgb(244, 244, 242);
pub const BLUE: Color32 = Color32::from_rgb(35, 131, 226);

pub const PAGE_TOP: f32 = 48.0;
pub const PAGE_TITLE: f32 = 36.0;
pub const TITLE_GAP: f32 = 24.0;
pub const DOCUMENT_WIDTH: f32 = 760.0;
pub const COLLECTION_WIDTH: f32 = 1120.0;
pub fn page_width(ui: &Ui, document: bool) -> (f32, f32) {
    let width = (ui.available_width() - 48.0).max(1.0).min(if document { DOCUMENT_WIDTH } else { COLLECTION_WIDTH });
    (width, ((ui.available_width() - width) / 2.0).max(0.0))
}
pub fn page_frame() -> egui::Frame {
    egui::Frame::NONE.fill(SURFACE).inner_margin(egui::Margin::symmetric(32, 0))
}
pub fn border(ctx: &Context) -> Stroke {
    Stroke::new(1.0 / ctx.pixels_per_point(), BORDER)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Sidebar,
    Back,
    Forward,
    Plus,
    Close,
    Minimize,
    Maximize,
    Restore,
    Channel,
    Video,
    Library,
    Script,
    Design,
    Edit,
    Grid,
    List,
    Image,
    Audio,
    Sparkle,
    History,
    Trash,
    Settings,
    Guide,
    Copy,
    Paste,
    Rename,
    ChevronDown,
    ChevronRight,
    More,
    Send,
    Stop,
    Bold,
    Italic,
    Underline,
    Strike,
    Indent,
    Outdent,
}

pub fn configure(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.override_text_color = Some(TEXT);
    visuals.panel_fill = SURFACE;
    visuals.window_fill = SURFACE;
    visuals.window_stroke = border(ctx);
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.menu_corner_radius = CornerRadius::same(8);
    visuals.window_highlight_topmost = false;
    visuals.popup_shadow = egui::epaint::Shadow { offset: [0, 4], blur: 18, spread: 0, color: Color32::from_black_alpha(24) };
    visuals.window_shadow = egui::epaint::Shadow { offset: [0, 8], blur: 28, spread: 0, color: Color32::from_black_alpha(30) };
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, FAINT);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(5);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(5);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(5);
    visuals.widgets.active.corner_radius = CornerRadius::same(5);
    visuals.text_cursor.stroke = Stroke::new(1.5, TEXT);
    visuals.hyperlink_color = BLUE;
    visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
    visuals.widgets.inactive.weak_bg_fill = SIDEBAR;
    visuals.widgets.hovered.bg_fill = HOVER;
    visuals.widgets.active.bg_fill = SELECTED;
    visuals.widgets.noninteractive.bg_stroke = Stroke::NONE;
    visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    visuals.selection.bg_fill = Color32::from_rgb(196, 220, 244);
    visuals.selection.stroke = Stroke::new(1.0, BLUE);
    ctx.set_visuals(visuals);
    let border = border(ctx);
    ctx.global_style_mut(|style| {
        style.spacing.item_spacing = vec2(8.0, 7.0);
        style.spacing.button_padding = vec2(9.0, 5.0);
        style.spacing.interact_size.y = 30.0;
        style.spacing.menu_margin = egui::Margin::same(6);
        style.spacing.window_margin = egui::Margin::same(20);
        style.spacing.icon_width = 15.0;
        style.spacing.icon_spacing = 8.0;
        style.spacing.scroll.bar_width = 6.0;
        style.visuals.widgets.noninteractive.bg_stroke = border;
        style.text_styles.insert(egui::TextStyle::Small, FontId::proportional(12.0));
        style.text_styles.insert(egui::TextStyle::Body, FontId::proportional(14.0));
        style.text_styles.insert(egui::TextStyle::Button, FontId::proportional(14.0));
        style.text_styles.insert(egui::TextStyle::Heading, FontId::proportional(25.0));
    });
}

pub fn icon_button(ui: &mut Ui, icon: Icon, size: f32, tip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(6), HOVER);
    }
    paint_icon(
        ui.painter(),
        rect.shrink((size - if matches!(icon, Icon::Sidebar | Icon::Back | Icon::Forward) { 22.0 } else { 18.0 }) / 2.0),
        icon,
        if !ui.is_enabled() {
            FAINT
        } else if response.hovered() {
            TEXT
        } else {
            MUTED
        },
    );
    response.on_hover_text(tip)
}

pub fn icon_toggle(ui: &mut Ui, icon: Icon, selected: bool, size: f32, tip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    if selected || response.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(6), if selected { SELECTED } else { HOVER });
    }
    paint_icon(ui.painter(), rect.shrink((size - 18.0) / 2.0), icon, if selected { TEXT } else { MUTED });
    response.on_hover_text(tip)
}

pub fn filter_tab(ui: &mut Ui, label: &str, selected: bool) -> Response {
    ui.add(
        egui::Button::new(egui::RichText::new(label).size(14.0).color(if selected { TEXT } else { MUTED }))
            .fill(if selected { SELECTED } else { Color32::TRANSPARENT })
            .corner_radius(5)
            .min_size(vec2(78.0, 30.0)),
    )
}

pub fn nav_row(ui: &mut Ui, label: &str, icon: Icon, selected: bool, indent: f32) -> Response {
    row(ui, label, Some(icon), selected, indent)
}
fn row(ui: &mut Ui, label: &str, icon: Option<Icon>, selected: bool, indent: f32) -> Response {
    let width = ui.available_width().max(40.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, 33.0), Sense::click());
    if selected || response.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(6), if selected { SELECTED } else { HOVER });
    }
    let icon_rect = Rect::from_center_size(pos2(rect.left() + indent + 17.0, rect.center().y), vec2(17.0, 17.0));
    if let Some(icon) = icon {
        paint_icon(ui.painter(), icon_rect, icon, if selected { TEXT } else { MUTED });
    }
    let mut job = egui::text::LayoutJob::simple(
        label.to_owned(),
        FontId::proportional(14.0),
        TEXT,
        (width - indent - if icon.is_none() { 100.0 } else { 44.0 }).max(15.0),
    );
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let position = pos2(rect.left() + indent + 34.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().with_clip_rect(rect).galley(position, galley, TEXT);
    response
}

pub fn tree_row(ui: &mut Ui, label: &str, expanded: bool, channel: bool) -> (Response, bool) {
    let indent = if channel { 0.0 } else { 10.0 };
    let response = row(ui, label, None, false, indent);
    let hit = Rect::from_center_size(pos2(response.rect.left() + indent + 17.0, response.rect.center().y), vec2(23.0, 25.0));
    let hovered = ui.input(|i| i.pointer.hover_pos()).is_some_and(|p| response.rect.contains(p));
    let icon = if hovered {
        if expanded { Icon::ChevronDown } else { Icon::ChevronRight }
    } else if channel {
        Icon::Channel
    } else {
        Icon::Video
    };
    paint_icon(ui.painter(), Rect::from_center_size(hit.center(), vec2(17.0, 17.0)), icon, MUTED);
    let toggle = ui.interact(hit, ui.scope_id().with("expand"), Sense::click()).on_hover_text(if expanded { "접기" } else { "펼치기" });
    (response, toggle.clicked())
}

pub fn paint_icon(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    if icon == Icon::Video {
        let name = painter.ctx().data(|d| d.get_temp::<String>(egui::Id::unique("purpose-video-icon"))).unwrap_or_else(|| "clapperboard".into());
        let _ = figcraft_ui_icons::paint(painter, rect, &name, color);
        return;
    }
    if let Some(name) = lucide(icon) {
        let _ = figcraft_ui_icons::paint(painter, rect, name, color);
    }
}

fn lucide(icon: Icon) -> Option<&'static str> {
    match icon {
        Icon::Sidebar => Some("panel-left"),
        Icon::Back => Some("chevron-left"),
        Icon::Forward => Some("chevron-right"),
        Icon::Plus => Some("plus"),
        Icon::Close => Some("x"),
        Icon::Minimize => Some("minus"),
        Icon::Maximize => Some("square"),
        Icon::Restore => Some("copy"),
        Icon::Channel => Some("tv-minimal-play"),
        Icon::Video => Some("clapperboard"),
        Icon::Library => Some("library-big"),
        Icon::Script => Some("file-text"),
        Icon::Design => Some("pen-tool"),
        Icon::Edit => Some("film"),
        Icon::Grid => Some("layout-grid"),
        Icon::List => Some("list"),
        Icon::Image => Some("image"),
        Icon::Audio => Some("file-music"),
        Icon::Sparkle => Some("sparkles"),
        Icon::History => Some("rotate-ccw-clock"),
        Icon::Trash => Some("trash"),
        Icon::Settings => Some("settings"),
        Icon::Guide => Some("bookmark"),
        Icon::Copy => Some("copy"),
        Icon::Paste => Some("between-horizontal-start"),
        Icon::Rename => Some("pencil"),
        Icon::ChevronDown => Some("chevron-down"),
        Icon::ChevronRight => Some("chevron-right"),
        Icon::More => Some("ellipsis"),
        Icon::Send => Some("arrow-up"),
        Icon::Stop => Some("square"),
        Icon::Bold => Some("letter-b"),
        Icon::Italic => Some("italic"),
        Icon::Underline => Some("underline"),
        Icon::Strike => Some("strikethrough"),
        Icon::Indent => Some("list-indent-increase"),
        Icon::Outdent => Some("list-indent-decrease"),
    }
}

/// Quiet, aligned action rows shared by context menus and overflow menus.
pub fn menu_item(ui: &mut Ui, label: &str, icon: Icon, destructive: bool) -> Response {
    ui.set_max_width(240.0);
    let width = ui.available_width().clamp(180.0, 240.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, 32.0), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 5, HOVER);
    }
    let color = if destructive { Color32::from_rgb(185, 65, 57) } else { TEXT };
    paint_icon(
        ui.painter(),
        Rect::from_center_size(pos2(rect.left() + 16.0, rect.center().y), vec2(16.0, 16.0)),
        icon,
        if destructive { color } else { MUTED },
    );
    let mut job = egui::text::LayoutJob::simple(label.into(), FontId::proportional(14.0), color, (width - 48.0).max(20.0));
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let text = ui.fonts_mut(|f| f.layout_job(job));
    ui.painter().galley(pos2(rect.left() + 35.0, rect.center().y - text.size().y / 2.0), text, color);
    response
}

pub fn dropdown(ui: &mut Ui, label: &str, max_text_width: f32) -> Response {
    let mut job = egui::text::LayoutJob::simple(label.to_owned(), FontId::proportional(13.0), TEXT, max_text_width);
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let text = ui.fonts_mut(|f| f.layout_job(job));
    let (rect, response) = ui.allocate_exact_size(vec2((text.size().x + 28.0).max(48.0), 28.0), Sense::click());
    ui.painter().rect_filled(rect, 5, if response.hovered() { HOVER } else { SIDEBAR });
    ui.painter().galley(pos2(rect.left() + 7.0, rect.center().y - text.size().y / 2.0), text, TEXT);
    paint_icon(ui.painter(), Rect::from_center_size(pos2(rect.right() - 10.0, rect.center().y), vec2(12.0, 12.0)), Icon::ChevronDown, MUTED);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, ui.is_enabled(), label));
    response.on_hover_text(label)
}

pub fn round_button(ui: &mut Ui, icon: Icon, enabled: bool, tip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(32.0, 30.0), Sense::click());
    ui.painter().rect_filled(
        rect,
        16,
        if !enabled {
            FAINT
        } else if response.hovered() {
            Color32::from_rgb(75, 73, 67)
        } else {
            TEXT
        },
    );
    paint_icon(ui.painter(), Rect::from_center_size(rect.center(), vec2(19.0, 19.0)), icon, Color32::WHITE);
    response.on_hover_text(tip)
}

pub fn modal_frame() -> egui::Frame {
    egui::Frame::NONE.fill(SURFACE).stroke(Stroke::new(1.0, BORDER)).corner_radius(12).inner_margin(24.0).shadow(egui::epaint::Shadow {
        offset: [0, 12],
        blur: 36,
        spread: 0,
        color: Color32::from_black_alpha(32),
    })
}
pub fn action_button(ui: &mut Ui, label: &str, primary: bool) -> Response {
    ui.add(
        egui::Button::new(egui::RichText::new(label).size(14.0).color(if primary { Color32::WHITE } else { TEXT }))
            .fill(if primary { TEXT } else { SURFACE })
            .stroke(Stroke::new(1.0, if primary { TEXT } else { BORDER }))
            .corner_radius(6)
            .min_size(vec2(78.0, 33.0)),
    )
}
pub fn rename_dialog(ctx: &Context, id: Id, heading: &str, name: &mut String) -> Option<bool> {
    let mut save = false;
    let mut cancel = false;
    let output = egui::Modal::new(id).frame(modal_frame()).backdrop_color(Color32::from_black_alpha(38)).show(ctx, |ui| {
        ui.set_width(360.0);
        ui.horizontal(|ui| {
            ui.label(self::heading(heading, 22.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                cancel = icon_button(ui, Icon::Close, 26.0, "닫기").clicked();
            });
        });
        ui.add_space(18.0);
        ui.label(egui::RichText::new("이름").size(12.0).color(MUTED));
        let edit = egui::TextEdit::singleline(name).id(id.with("name")).desired_width(f32::INFINITY).margin(vec2(11.0, 10.0)).show(ui);
        let first = !ui.is_sizing_pass()
            && ctx.data_mut(|d| {
                let seen = d.get_temp::<bool>(id.with("focused")).unwrap_or(false);
                d.insert_temp(id.with("focused"), true);
                !seen
            });
        if first {
            edit.response.request_focus();
            let mut state = edit.state;
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(0), egui::text::CCursor::new(name.chars().count()))));
            state.store(ctx, edit.response.id);
        }
        ui.add_space(22.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            save = ui.add_enabled_ui(!name.trim().is_empty(), |ui| action_button(ui, "저장", true)).inner.clicked();
            cancel |= action_button(ui, "취소", false).clicked();
        });
        save |= (edit.response.has_focus() || edit.response.lost_focus()) && !name.trim().is_empty() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    });
    if save || cancel || output.should_close() {
        ctx.memory_mut(|m| m.surrender_focus(id.with("name")));
        ctx.data_mut(|d| d.remove::<bool>(id.with("focused")));
        Some(save && !cancel)
    } else {
        None
    }
}

pub fn fast_toggle(ui: &mut Ui, selected: bool, size: f32, tip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 5, HOVER);
    }
    let icon = Rect::from_center_size(rect.center(), vec2(17.0, 17.0));
    let color = if !ui.is_enabled() {
        FAINT
    } else if selected {
        TEXT
    } else {
        MUTED
    };
    if selected {
        figcraft_ui_icons::paint_solid(ui.painter(), icon, "zap", color);
    } else {
        figcraft_ui_icons::paint(ui.painter(), icon, "zap", color);
    }
    response.on_hover_text(tip)
}

pub fn heading(text: &str, size: f32) -> egui::RichText {
    egui::RichText::new(text).font(FontId::new(size, egui::FontFamily::Name("purpose-script-bold".into())))
}
pub fn release_focus(ctx: &Context) {
    ctx.memory_mut(|m| {
        if let Some(id) = m.focused() {
            m.surrender_focus(id);
        }
    });
}

pub fn search_field(ui: &mut Ui, text: &mut String, width: f32, hint: &str) -> Response {
    egui::Frame::NONE
        .fill(SIDEBAR)
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(8, 0))
        .show(ui, |ui| {
            ui.allocate_ui_with_layout(vec2((width - 16.0).max(60.0), 30.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                let (rect, _) = ui.allocate_exact_size(vec2(15.0, 30.0), Sense::hover());
                figcraft_ui_icons::paint(ui.painter(), Rect::from_center_size(rect.center(), vec2(14.0, 14.0)), "search", MUTED);
                ui.add_sized(
                    [(width - 36.0).max(40.0), 30.0],
                    egui::TextEdit::singleline(text)
                        .frame(egui::Frame::NONE)
                        .margin(egui::Margin::ZERO)
                        .align(egui::Align2::LEFT_CENTER)
                        .font(FontId::proportional(13.0))
                        .hint_text(hint),
                )
            })
            .inner
        })
        .inner
}

#[cfg(test)]
mod tests {
    #[test]
    fn shared_style_configuration_does_not_reenter_the_context_style_lock() {
        let ctx = egui::Context::default();
        super::configure(&ctx);
        assert_eq!(ctx.global_style().spacing.interact_size.y, 30.0);
    }

    use super::*;
    #[test]
    fn rename_enter_commits_even_when_the_single_line_editor_releases_focus() {
        let ctx = Context::default();
        let mut fonts = egui::FontDefinitions::default();
        fonts.families.insert(egui::FontFamily::Name("purpose-script-bold".into()), fonts.families[&egui::FontFamily::Proportional].clone());
        ctx.set_fonts(fonts);
        let mut name = "old name".to_owned();
        let id = Id::unique("rename-enter-fixture");
        for _ in 0..4 {
            let mut output = ctx.run_ui(
                egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(900.0, 600.0))), ..Default::default() },
                |_| {
                    assert_eq!(rename_dialog(&ctx, id, "이름 바꾸기", &mut name), None);
                },
            );
            output.textures_delta.clear();
        }
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(900.0, 600.0))),
                events: vec![egui::Event::Text("새 이름".into())],
                ..Default::default()
            },
            |_| {
                rename_dialog(&ctx, id, "이름 바꾸기", &mut name);
            },
        );
        output.textures_delta.clear();
        assert_eq!(name, "새 이름");
        let mut committed = None;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(900.0, 600.0))),
                events: vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |_| {
                committed = rename_dialog(&ctx, id, "이름 바꾸기", &mut name);
            },
        );
        output.textures_delta.clear();
        assert_eq!(committed, Some(true));
    }
    #[test]
    fn tree_hover_draws_one_glyph_and_the_toggle_still_receives_clicks() {
        let ctx = egui::Context::default();
        let pointer = pos2(17.0, 16.0);
        let events = vec![
            vec![],
            vec![egui::Event::PointerMoved(pointer)],
            vec![egui::Event::PointerButton { pos: pointer, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE }],
            vec![egui::Event::PointerButton { pos: pointer, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE }],
        ];
        let mut clicked = false;
        for events in events {
            let mut output = ctx.run_ui(
                egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(400.0, 100.0))), events, ..Default::default() },
                |ui| {
                    let (response, toggled) = tree_row(ui, "채널", true, true);
                    assert_eq!(response.rect.height(), 33.0);
                    clicked |= toggled;
                },
            );
            assert_eq!(
                output.shapes.iter().filter(|shape| matches!(shape.shape, egui::Shape::Mesh(_))).count(),
                1,
                "hover must replace the glyph, never draw over it"
            );
            output.textures_delta.clear();
        }
        assert!(clicked);
    }
}
