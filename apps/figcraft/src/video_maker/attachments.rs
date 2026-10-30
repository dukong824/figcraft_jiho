//! Library references in a conversation; image sources also become native CLI vision inputs.
use super::{
    SideView, VideoMaker,
    chat::Attachment,
    interactions::AssetDrag,
    style::{self, Icon},
};
use egui::{RichText, vec2};
impl VideoMaker {
    pub(super) fn attach_source(&mut self, target: u64, source: u64, id: u64) {
        let asset = self.store.data.channels.iter().flat_map(|c| &c.videos).find(|v| v.id == source).and_then(|v| v.asset(id)).cloned();
        let Some(asset) = asset else {
            self.message = "자료를 찾을 수 없습니다".into();
            return;
        };
        let list = self.chat_attachments.entry(target).or_default();
        if list.iter().any(|a| a.video == source && a.id == id) {
            return;
        }
        if list.len() >= 8 {
            self.message = "한 번에 최대 8개의 소스를 첨부할 수 있습니다".into();
            return;
        }
        list.push(Attachment { video: source, id, name: asset.name });
        self.side_view = Some(SideView::Codex);
        if self.authenticated.is_none() {
            self.refresh_auth();
        }
    }
    pub(super) fn attachment_context(&self, attachments: &[Attachment]) -> Result<(String, Vec<std::path::PathBuf>), String> {
        let mut text = String::new();
        let mut images = Vec::new();
        for a in attachments {
            let video = self.store.data.channels.iter().flat_map(|c| &c.videos).find(|v| v.id == a.video).ok_or("첨부한 영상이 휴지통에 있습니다")?;
            let asset = video.asset(a.id).ok_or("첨부한 자료를 찾을 수 없습니다")?;
            let path = self.store.asset_path(a.video, asset);
            if !path.is_file() {
                return Err(format!("첨부 파일을 찾을 수 없습니다: {}", a.name));
            }
            text.push_str(&format!(
                "\n- {} · {} · 영상 {} · 자료 ID {} · 길이 {:.2}초 · 파일: {}",
                asset.name,
                asset.kind.label(),
                video.name,
                asset.id,
                asset.duration,
                path.display()
            ));
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
            if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") {
                images.push(path);
            }
        }
        Ok((text, images))
    }
    pub(super) fn attachment_drop(&mut self, ui: &egui::Ui, video: u64, rect: egui::Rect) -> bool {
        let inside = ui.ctx().pointer_hover_pos().is_some_and(|p| rect.contains(p));
        let payload = egui::DragAndDrop::payload::<AssetDrag>(ui.ctx());
        if inside && let Some(payload) = payload {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Copy);
            if ui.input(|i| i.pointer.any_released()) {
                self.attach_source(video, payload.video_id, payload.asset_id);
                egui::DragAndDrop::clear_payload(ui.ctx());
            }
            return true;
        }
        false
    }
    pub(super) fn attachment_picker(&mut self, ui: &mut egui::Ui, video: u64) {
        let response = style::icon_button(ui, Icon::Plus, 28.0, "라이브러리 소스 첨부");
        let mut add = None;
        egui::Popup::menu(&response).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            ui.set_width(280.0);
            style::search_field(ui, &mut self.attachment_query, 280.0, "소스 검색");
            let query = self.attachment_query.to_lowercase();
            egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                if let Some(v) = self.store.data.channels.iter().flat_map(|c| &c.videos).find(|v| v.id == video) {
                    if v.assets.is_empty() {
                        ui.label(RichText::new("라이브러리에 소스가 없습니다").color(style::MUTED));
                    }
                    for asset in &v.assets {
                        if !asset.name.to_lowercase().contains(&query) {
                            continue;
                        }
                        let selected = self.chat_attachments.get(&video).is_some_and(|a| a.iter().any(|a| a.id == asset.id && a.video == video));
                        if ui.add(egui::Button::selectable(selected, &asset.name).truncate()).on_hover_text(&asset.name).clicked() {
                            add = Some((asset.id, selected));
                        }
                    }
                }
            });
        });
        if let Some((id, selected)) = add {
            if selected {
                if let Some(list) = self.chat_attachments.get_mut(&video) {
                    list.retain(|a| a.id != id || a.video != video);
                }
            } else {
                self.attach_source(video, video, id);
            }
        }
    }
    pub(super) fn attachment_chips(&mut self, ui: &mut egui::Ui, video: u64) {
        let attachments = self.chat_attachments.get(&video).cloned().unwrap_or_default();
        let mut remove = None;
        egui::ScrollArea::horizontal()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .id_salt(("attachment-chips", video))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if self.pastes.iter().any(|p| p.video == video && p.attach) {
                        ui.spinner();
                        ui.label("이미지 준비 중…");
                    }
                    for (index, a) in attachments.iter().enumerate() {
                        egui::Frame::NONE.fill(style::SIDEBAR).corner_radius(6).inner_margin(egui::Margin::symmetric(7, 3)).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if let Some(texture) = self.thumbnails.get(&a.id) {
                                    ui.add(egui::Image::new(texture).fit_to_exact_size(vec2(24.0, 24.0)));
                                } else {
                                    let (r, _) = ui.allocate_exact_size(vec2(18.0, 24.0), egui::Sense::hover());
                                    style::paint_icon(ui.painter(), r.shrink(1.0), Icon::Library, style::MUTED);
                                }
                                ui.add_sized(
                                    [120.0, 24.0],
                                    egui::Label::new(RichText::new(&a.name).size(figcraft_ui_icons::font_size(12.0))).truncate(),
                                )
                                .on_hover_text(&a.name);
                                if style::icon_button(ui, Icon::Close, 22.0, "첨부 제거").clicked() {
                                    remove = Some(index);
                                }
                            });
                        });
                    }
                });
            });
        if let Some(index) = remove
            && let Some(list) = self.chat_attachments.get_mut(&video)
        {
            list.remove(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[test]
    fn attachments_reach_real_cli_arguments_prompt_and_persisted_conversation() {
        use std::{fs, time::Duration};
        let root = std::env::temp_dir().join(format!("purpose-attach-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let mut maker = VideoMaker::at(root.clone()).unwrap();
        let video = maker.video_id().unwrap();
        let dir = maker.store.video_dir(video).join("assets");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("3.png");
        image::RgbaImage::from_pixel(8, 8, image::Rgba([30, 40, 50, 255])).save(&file).unwrap();
        maker.store.data.selected_mut().unwrap().assets.push(super::super::model::Asset {
            id: 3,
            name: "장면 레퍼런스.png".into(),
            file: "3.png".into(),
            kind: super::super::model::AssetKind::Image,
            duration: 5.0,
            has_audio: false,
        });
        let cli = super::super::codex_session::tests::fixture_cli(&root, "normal");
        maker.store.data.codex_path = Some(cli);
        maker.authenticated = Some(true);
        maker.attach_source(video, video, 3);
        maker.attach_source(video, video, 3);
        assert_eq!(maker.chat_attachments[&video].len(), 1);
        maker.ask_codex("이 이미지로 오프닝을 제안해".into());
        let ctx = egui::Context::default();
        for _ in 0..200 {
            maker.poll_chat(&ctx);
            if maker.chat_job.is_none() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(maker.chat_job.is_none());
        let args = fs::read_to_string(root.join("args.txt")).unwrap();
        assert!(args.lines().any(|s| s == "app-server"));
        let protocol = fs::read_to_string(root.join("request.txt")).unwrap();
        assert!(protocol.contains("localImage") && protocol.contains(file.to_str().unwrap()));
        assert!(fs::read_to_string(root.join("request.txt")).unwrap().contains("장면 레퍼런스.png"));
        maker.chats.clear();
        assert_eq!(maker.conversation(video).messages[0].attachments[0].id, 3);
        assert!(!maker.chat_attachments.contains_key(&video));
        fs::remove_dir_all(root).unwrap();
    }
}
