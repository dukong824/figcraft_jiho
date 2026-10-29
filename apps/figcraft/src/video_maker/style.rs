use egui::{Color32, CornerRadius, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, pos2, vec2};

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
    let width = ui.available_width().max(40.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, 33.0), Sense::click());
    if selected || response.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(6), if selected { SELECTED } else { HOVER });
    }
    let icon_rect = Rect::from_center_size(pos2(rect.left() + indent + 17.0, rect.center().y), vec2(17.0, 17.0));
    paint_icon(ui.painter(), icon_rect, icon, if selected { TEXT } else { MUTED });
    let mut job = egui::text::LayoutJob::simple(label.to_owned(), FontId::proportional(14.0), TEXT, (width - indent - 66.0).max(15.0));
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let position = pos2(rect.left() + indent + 34.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().with_clip_rect(rect).galley(position, galley, TEXT);
    response
}

pub fn tree_row(ui: &mut Ui, label: &str, expanded: bool, channel: bool) -> (Response, bool) {
    let response = nav_row(ui, label, if channel { Icon::Channel } else { Icon::Video }, false, if channel { 0.0 } else { 10.0 });
    let icon = Rect::from_center_size(pos2(response.rect.left() + if channel { 17.0 } else { 27.0 }, response.rect.center().y), vec2(23.0, 25.0));
    let hovered = ui.input(|i| i.pointer.hover_pos()).is_some_and(|p| response.rect.contains(p));
    let toggle = ui.interact(icon, ui.scope_id().with("expand"), Sense::click());
    if hovered {
        ui.painter().rect_filled(icon, CornerRadius::same(3), if response.hovered() { HOVER } else { SIDEBAR });
        let c = icon.center();
        let points = if expanded {
            [pos2(c.x - 4.0, c.y - 2.0), pos2(c.x, c.y + 2.0), pos2(c.x + 4.0, c.y - 2.0)]
        } else {
            [pos2(c.x - 2.0, c.y - 4.0), pos2(c.x + 2.0, c.y), pos2(c.x - 2.0, c.y + 4.0)]
        };
        ui.painter().add(Shape::line(points.to_vec(), Stroke::new(1.5, MUTED)));
    }
    (response, toggle.on_hover_text(if expanded { "접기" } else { "펼치기" }).clicked())
}

fn line(p: &Painter, a: Pos2, b: Pos2, color: Color32) {
    p.line_segment([a, b], Stroke::new(1.6, color));
}

pub fn paint_icon(p: &Painter, r: Rect, icon: Icon, color: Color32) {
    let x = r.center().x;
    let y = r.center().y;
    let l = r.left();
    let t = r.top();
    let w = r.width();
    let h = r.height();
    let stroke = Stroke::new(1.5, color);
    match icon {
        Icon::History => {
            p.circle_stroke(r.center(), 7.0, stroke);
            line(p, pos2(x, y - 4.0), pos2(x, y), color);
            line(p, pos2(x, y), pos2(x + 3.0, y + 2.0), color);
        }
        Icon::Settings => {
            p.circle_stroke(r.center(), r.width() * 0.34, stroke);
            p.circle_stroke(r.center(), r.width() * 0.12, stroke);
            for i in 0..8 {
                let angle = i as f32 * std::f32::consts::TAU / 8.0;
                let v = vec2(angle.cos(), angle.sin());
                p.line_segment([r.center() + v * r.width() * 0.34, r.center() + v * r.width() * 0.47], stroke);
            }
        }
        Icon::Trash => {
            p.rect_stroke(r.shrink2(vec2(4.0, 3.0)).translate(vec2(0.0, 2.0)), CornerRadius::same(1), stroke, StrokeKind::Inside);
            line(p, pos2(l + 2.0, t + 4.0), pos2(l + w - 2.0, t + 4.0), color);
            line(p, pos2(x - 2.0, t + 1.0), pos2(x + 2.0, t + 1.0), color);
        }
        Icon::Sidebar => {
            p.rect_stroke(r.shrink(1.0), CornerRadius::same(2), stroke, StrokeKind::Inside);
            line(p, pos2(l + w * 0.38, t + 1.0), pos2(l + w * 0.38, t + h - 1.0), color);
        }
        Icon::Back | Icon::Forward => {
            let dir = if matches!(icon, Icon::Back) { 1.0 } else { -1.0 };
            line(p, pos2(x + dir * 3.0, y - 5.0), pos2(x - dir * 2.0, y), color);
            line(p, pos2(x - dir * 2.0, y), pos2(x + dir * 3.0, y + 5.0), color);
        }
        Icon::Plus => {
            line(p, pos2(x - 5.0, y), pos2(x + 5.0, y), color);
            line(p, pos2(x, y - 5.0), pos2(x, y + 5.0), color);
        }
        Icon::Close => {
            line(p, pos2(x - 4.5, y - 4.5), pos2(x + 4.5, y + 4.5), color);
            line(p, pos2(x + 4.5, y - 4.5), pos2(x - 4.5, y + 4.5), color);
        }
        Icon::Minimize => line(p, pos2(x - 5.0, y + 3.0), pos2(x + 5.0, y + 3.0), color),
        Icon::Maximize => {
            p.rect_stroke(r.shrink(3.0), CornerRadius::ZERO, stroke, StrokeKind::Inside);
        }
        Icon::Restore => {
            p.rect_stroke(r.translate(vec2(2.0, -2.0)).shrink(4.0), CornerRadius::ZERO, stroke, StrokeKind::Inside);
            p.rect_stroke(r.translate(vec2(-2.0, 2.0)).shrink(4.0), CornerRadius::ZERO, stroke, StrokeKind::Inside);
        }
        Icon::Channel | Icon::Video => {
            p.rect_stroke(r.shrink2(vec2(1.0, 3.0)), CornerRadius::same(2), stroke, StrokeKind::Inside);
            p.add(Shape::convex_polygon(vec![pos2(x - 2.0, y - 3.0), pos2(x + 3.0, y), pos2(x - 2.0, y + 3.0)], color, Stroke::NONE));
            if matches!(icon, Icon::Video) {
                line(p, pos2(l + 3.0, t + 1.0), pos2(l + 3.0, t + h - 1.0), color);
            }
        }
        Icon::Library | Icon::Grid => {
            for dx in [1.0, 9.0] {
                for dy in [1.0, 9.0] {
                    p.rect_stroke(Rect::from_min_size(pos2(l + dx, t + dy), vec2(6.0, 6.0)), CornerRadius::same(1), stroke, StrokeKind::Inside);
                }
            }
        }
        Icon::Script => {
            p.rect_stroke(r.shrink2(vec2(3.0, 1.0)), CornerRadius::same(1), stroke, StrokeKind::Inside);
            for dy in [6.0, 10.0, 14.0] {
                line(p, pos2(l + 6.0, t + dy), pos2(l + w - 5.0, t + dy), color);
            }
        }
        Icon::Rename => {
            p.add(Shape::line(
                vec![
                    pos2(l + 3.0, t + h - 3.0),
                    pos2(l + 4.0, t + h - 7.0),
                    pos2(l + w - 5.0, t + 2.0),
                    pos2(l + w - 1.0, t + 6.0),
                    pos2(l + 7.0, t + h - 4.0),
                    pos2(l + 3.0, t + h - 3.0),
                ],
                stroke,
            ));
        }
        Icon::Fast => {
            p.add(Shape::line(
                vec![
                    pos2(x + 2.0, t + 1.0),
                    pos2(l + 3.0, y + 1.0),
                    pos2(x + 1.0, y + 1.0),
                    pos2(x - 2.0, t + h - 1.0),
                    pos2(l + w - 3.0, y - 2.0),
                    pos2(x - 1.0, y - 2.0),
                    pos2(x + 2.0, t + 1.0),
                ],
                stroke,
            ));
        }
        Icon::Design => {
            p.rect_stroke(r.shrink(3.0), CornerRadius::same(2), stroke, StrokeKind::Inside);
            p.circle_filled(pos2(x + 3.0, y - 3.0), 2.0, color);
        }
        Icon::Edit => {
            line(p, pos2(l + 2.0, t + 4.0), pos2(l + w - 2.0, t + 4.0), color);
            line(p, pos2(l + 2.0, t + 9.0), pos2(l + w - 2.0, t + 9.0), color);
            line(p, pos2(l + 2.0, t + 14.0), pos2(l + w - 2.0, t + 14.0), color);
            p.circle_filled(pos2(x + 2.0, t + 9.0), 2.0, color);
        }
        Icon::List => {
            for dy in [3.0, 8.0, 13.0] {
                p.circle_filled(pos2(l + 2.0, t + dy), 1.0, color);
                line(p, pos2(l + 6.0, t + dy), pos2(l + w - 1.0, t + dy), color);
            }
        }
        Icon::Image => {
            p.rect_stroke(r.shrink(2.0), CornerRadius::same(2), stroke, StrokeKind::Inside);
            p.circle_filled(pos2(l + 6.0, t + 6.0), 1.5, color);
            line(p, pos2(l + 4.0, t + h - 4.0), pos2(l + 9.0, t + 10.0), color);
            line(p, pos2(l + 9.0, t + 10.0), pos2(l + w - 3.0, t + h - 4.0), color);
        }
        Icon::Audio => {
            line(p, pos2(x - 2.0, y - 6.0), pos2(x - 2.0, y + 3.0), color);
            p.circle_filled(pos2(x - 4.0, y + 4.0), 2.5, color);
            line(p, pos2(x - 2.0, y - 6.0), pos2(x + 5.0, y - 8.0), color);
            line(p, pos2(x + 5.0, y - 8.0), pos2(x + 5.0, y + 1.0), color);
            p.circle_filled(pos2(x + 3.0, y + 2.0), 2.5, color);
        }
        Icon::Sparkle => {
            line(p, pos2(x, t + 1.0), pos2(x, t + h - 1.0), color);
            line(p, pos2(l + 1.0, y), pos2(l + w - 1.0, y), color);
            line(p, pos2(x - 4.0, y - 4.0), pos2(x + 4.0, y + 4.0), color);
            line(p, pos2(x + 4.0, y - 4.0), pos2(x - 4.0, y + 4.0), color);
        }
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
