use egui::{Color32, CornerRadius, FontId, Painter, Rect, Response, Sense, Stroke, Ui, pos2, vec2};

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

#[derive(Clone, Copy)]
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
    Rename,
    Fast,
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
    visuals.window_stroke = Stroke::new(1.0, BORDER);
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.menu_corner_radius = CornerRadius::same(8);
    visuals.window_highlight_topmost = false;
    visuals.popup_shadow = egui::epaint::Shadow { offset: [0, 4], blur: 18, spread: 0, color: Color32::from_black_alpha(24) };
    visuals.window_shadow = egui::epaint::Shadow { offset: [0, 8], blur: 28, spread: 0, color: Color32::from_black_alpha(30) };
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
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
    ctx.global_style_mut(|style| {
        style.spacing.item_spacing = vec2(8.0, 7.0);
        style.spacing.button_padding = vec2(9.0, 5.0);
        style.spacing.interact_size.y = 30.0;
        style.spacing.menu_margin = egui::Margin::same(6);
        style.spacing.window_margin = egui::Margin::same(20);
        style.spacing.icon_width = 15.0;
        style.spacing.icon_spacing = 8.0;
        style.spacing.scroll.bar_width = 6.0;
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
        rect.shrink((size - 18.0) / 2.0),
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
    let mut job = egui::text::LayoutJob::simple(label.to_owned(), FontId::proportional(14.0), TEXT, (width - indent - 66.0).max(15.0));
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
        Icon::Video => Some("file-video-camera"),
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
        Icon::Rename => Some("pencil"),
        Icon::Fast => Some("zap"),
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
    ui.painter().text(pos2(rect.left() + 35.0, rect.center().y), egui::Align2::LEFT_CENTER, label, FontId::proportional(13.0), color);
    response
}

pub fn dropdown(ui: &mut Ui, label: &str, max_text_width: f32) -> Response {
    let mut job = egui::text::LayoutJob::simple(label.to_owned(), FontId::proportional(11.0), TEXT, max_text_width);
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

#[cfg(test)]
mod tests {
    use super::*;
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
