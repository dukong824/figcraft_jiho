use super::{
    VideoMaker,
    style::{self, Icon},
};
use egui::{RichText, vec2};
#[derive(Clone, Copy)]
enum Item {
    Channel(u64),
    Video(u64),
    Asset(u64, u64),
}
impl VideoMaker {
    pub(super) fn trash_window(&mut self, ctx: &egui::Context) {
        if !self.trash_open {
            return;
        }
        let mut close = false;
        let mut restore = None;
        let mut rows = Vec::new();
        for item in &self.store.data.trashed_channels {
            rows.push((
                Item::Channel(item.channel.id),
                item.channel.name.clone(),
                format!("채널 · 영상 {}개", item.channel.videos.len()),
                Icon::Channel,
            ));
        }
        for item in &self.store.data.trash {
            rows.push((Item::Video(item.video.id), item.video.name.clone(), item.channel_name.clone(), Icon::Video));
        }
        for channel in &self.store.data.channels {
            for video in &channel.videos {
                for asset in &video.removed_assets {
                    rows.push((
                        Item::Asset(video.id, asset.id),
                        asset.name.clone(),
                        format!("{} / {}", channel.name, video.name),
                        super::ui::asset_icon(asset.kind),
                    ));
                }
            }
        }
        let output = egui::Modal::new(egui::Id::unique("purpose-trash-window"))
            .frame(style::modal_frame())
            .backdrop_color(egui::Color32::from_black_alpha(38))
            .show(ctx, |ui| {
                ui.set_width((ctx.content_rect().width() - 100.0).clamp(360.0, 680.0));
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(24.0, 28.0), egui::Sense::hover());
                    style::paint_icon(ui.painter(), r.shrink(3.0), Icon::Trash, style::MUTED);
                    ui.label(style::heading("휴지통", 22.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        close = style::icon_button(ui, Icon::Close, 28.0, "휴지통 닫기").clicked();
                    });
                });
                ui.add_space(17.0);
                style::search_field(ui, &mut self.trash_query, ui.available_width(), "삭제된 항목 검색");
                ui.add_space(14.0);
                ui.separator();
                let query = self.trash_query.to_lowercase();
                egui::ScrollArea::vertical()
                    .id_salt("trash-items")
                    .min_scrolled_height(300.0)
                    .auto_shrink([false, false])
                    .max_height((ctx.content_rect().height() - 270.0).clamp(160.0, 340.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let filtered: Vec<_> = rows
                            .iter()
                            .filter(|(_, name, path, _)| {
                                query.is_empty() || name.to_lowercase().contains(&query) || path.to_lowercase().contains(&query)
                            })
                            .collect();
                        if filtered.is_empty() {
                            ui.add_space(90.0);
                            ui.vertical_centered(|ui| {
                                let (r, _) = ui.allocate_exact_size(vec2(38.0, 38.0), egui::Sense::hover());
                                style::paint_icon(ui.painter(), r, Icon::Trash, style::FAINT);
                                ui.add_space(14.0);
                                ui.label(
                                    RichText::new(if query.is_empty() {
                                        "휴지통이 비어 있습니다"
                                    } else {
                                        "검색 결과가 없습니다"
                                    })
                                    .color(style::MUTED),
                                );
                            });
                        }
                        for (index, (item, name, path, icon)) in filtered.into_iter().enumerate() {
                            ui.push_id(index, |ui| {
                                ui.add_space(5.0);
                                egui::Frame::NONE.inner_margin(egui::Margin::symmetric(8, 10)).show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let (r, _) = ui.allocate_exact_size(vec2(25.0, 30.0), egui::Sense::hover());
                                        style::paint_icon(ui.painter(), r.shrink(4.0), *icon, style::MUTED);
                                        let width = (ui.available_width() - 95.0).max(100.0);
                                        ui.allocate_ui_with_layout(vec2(width, 35.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                                            ui.set_min_width(width);
                                            ui.add(egui::Label::new(RichText::new(name).size(14.0)).truncate());
                                            ui.add(egui::Label::new(RichText::new(path).size(11.0).color(style::MUTED)).truncate());
                                        });
                                        if style::action_button(ui, "복원", false).clicked() {
                                            restore = Some(*item);
                                        }
                                    });
                                });
                                ui.separator();
                            });
                        }
                    });
            });
        if close || output.should_close() {
            self.trash_open = false;
            style::release_focus(ctx);
        }
        if let Some(item) = restore {
            match item {
                Item::Channel(id) => self.store.data.restore_channel(id),
                Item::Video(id) => self.store.data.restore_video(id),
                Item::Asset(video, id) => {
                    if let Some(video) = self.store.data.channels.iter_mut().flat_map(|c| &mut c.videos).find(|v| v.id == video)
                        && let Some(index) = video.removed_assets.iter().position(|a| a.id == id)
                    {
                        video.assets.push(video.removed_assets.remove(index));
                    }
                }
            }
            self.save();
        }
    }
}
