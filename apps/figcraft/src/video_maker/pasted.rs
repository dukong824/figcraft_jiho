//! Native clipboard images keep their original target while PNG encoding runs off the UI thread.
use super::{
    SideView, VideoMaker,
    chat::Attachment,
    model::{Asset, AssetKind, Section},
    navigation::Route,
};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, mpsc},
};
pub(super) struct Pending {
    pub video: u64,
    pub attach: bool,
    events: mpsc::Receiver<Result<Asset, String>>,
}
fn store_image(directory: PathBuf, id: u64, name: String, image: Arc<egui::ColorImage>) -> Result<Asset, String> {
    let [w, h] = image.size;
    if w == 0 || h == 0 || w.checked_mul(h).filter(|n| *n <= 40_000_000 && *n == image.pixels.len()).is_none() {
        return Err("붙여넣을 이미지가 너무 크거나 올바르지 않습니다".into());
    }
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let rgba: Vec<_> = image.pixels.iter().flat_map(|p| p.to_srgba_unmultiplied()).collect();
    let file = format!("{id}.png");
    let temp = directory.join(format!(".paste-{id}.png"));
    let result = image::save_buffer_with_format(&temp, &rgba, w as u32, h as u32, image::ColorType::Rgba8, image::ImageFormat::Png)
        .map_err(|e| e.to_string())
        .and_then(|_| fs::rename(&temp, directory.join(&file)).map_err(|e| e.to_string()));
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result?;
    Ok(Asset { id, name, file, kind: AssetKind::Image, duration: 0.0, has_audio: false })
}
impl VideoMaker {
    pub(super) fn paste_images(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        if ctx.memory(|m| m.top_modal_layer().is_some()) {
            return;
        }
        let Some((video, section)) = self.tabs.route().and_then(Route::video) else {
            return;
        };
        let chat_focus = self.side_view == Some(SideView::Codex)
            && (ctx.memory(|m| m.has_focus(egui::Id::unique(("codex-composer", video))))
                || !ctx.text_edit_focused()
                    && !self.library_focused
                    && ctx
                        .data(|d| d.get_temp::<egui::Rect>(egui::Id::unique("purpose-codex-bounds")))
                        .is_some_and(|r| ctx.pointer_latest_pos().is_some_and(|p| r.contains(p))));
        if !chat_focus && section != Section::Library {
            return;
        }
        let mut images = Vec::new();
        raw.events.retain(|event| {
            if let egui::Event::PasteImage(image) = event {
                images.push(image.clone());
                false
            } else {
                true
            }
        });
        if !images.is_empty() {
            raw.events.retain(|event| !matches!(event,egui::Event::Key{key:egui::Key::V,modifiers,..}if modifiers.command||modifiers.ctrl));
        }
        for image in images {
            if let Err(e) = self.queue_pasted_image(video, chat_focus, image, ctx) {
                self.message = e;
            }
        }
    }
    fn queue_pasted_image(&mut self, video: u64, attach: bool, image: Arc<egui::ColorImage>, ctx: &egui::Context) -> Result<(), String> {
        if self.pastes.len() >= 4 {
            return Err("이미지 붙여넣기가 끝난 뒤 다시 시도하세요".into());
        }
        let staged = self.pastes.iter().filter(|p| p.video == video && p.attach).count();
        if attach && self.chat_attachments.get(&video).map_or(0, Vec::len) + staged >= 8 {
            return Err("한 번에 최대 8개의 소스를 첨부할 수 있습니다".into());
        }
        if !self.store.data.channels.iter().flat_map(|c| &c.videos).any(|v| v.id == video) {
            return Err("대상 영상이 없습니다".into());
        }
        let id = self.store.data.id();
        self.store.save()?;
        let dir = self.store.video_dir(video).join("assets");
        let name = chrono::Local::now().format("붙여넣은 이미지 %H%M%S.png").to_string();
        let (tx, events) = mpsc::channel();
        let wake = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(store_image(dir, id, name, image));
            wake.request_repaint();
        });
        self.pastes.push(Pending { video, attach, events });
        Ok(())
    }
    pub(super) fn poll_pastes(&mut self, ctx: &egui::Context) {
        let mut done = Vec::new();
        for (index, job) in self.pastes.iter().enumerate() {
            match job.events.try_recv() {
                Ok(v) => done.push((index, v)),
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => done.push((index, Err("이미지 붙여넣기 작업이 종료되었습니다".into()))),
            }
        }
        for (index, result) in done.into_iter().rev() {
            let job = self.pastes.remove(index);
            match result {
                Ok(asset) => {
                    let id = asset.id;
                    let name = asset.name.clone();
                    let target = self
                        .store
                        .data
                        .channels
                        .iter_mut()
                        .flat_map(|c| &mut c.videos)
                        .chain(self.store.data.trash.iter_mut().map(|t| &mut t.video))
                        .chain(self.store.data.trashed_channels.iter_mut().flat_map(|t| &mut t.channel.videos))
                        .find(|v| v.id == job.video);
                    if let Some(target) = target {
                        target.assets.push(asset);
                        if let Err(e) = self.store.save() {
                            self.message = format!("이미지 저장 실패: {e}");
                        }
                        if job.attach {
                            self.chat_attachments.entry(job.video).or_default().push(Attachment { video: job.video, id, name });
                        }
                    }
                }
                Err(e) => self.message = format!("이미지 붙여넣기 실패: {e}"),
            }
        }
        if !self.pastes.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pasted_image_is_a_real_png_and_remains_attached_to_its_original_video_after_navigation() {
        let root = std::env::temp_dir().join(format!("purpose-paste-{}", std::process::id()));
        let mut maker = VideoMaker::at(root.clone()).unwrap();
        let video = maker.video_id().unwrap();
        let channel = maker.store.data.channels[0].id;
        let next = maker.store.data.add_video(channel).unwrap();
        let ctx = egui::Context::default();
        maker
            .queue_pasted_image(
                video,
                true,
                Arc::new(egui::ColorImage::filled([2, 2], egui::Color32::from_rgba_unmultiplied(80, 120, 160, 128))),
                &ctx,
            )
            .unwrap();
        maker.store.data.selected_video = Some(next);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(4);
        while !maker.pastes.is_empty() && std::time::Instant::now() < deadline {
            maker.poll_pastes(&ctx);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let source = &maker.store.data.channels[0].videos.iter().find(|v| v.id == video).unwrap().assets[0];
        let png = image::open(maker.store.asset_path(video, source)).unwrap().to_rgba8();
        assert_eq!(png.dimensions(), (2, 2));
        assert_eq!(png.get_pixel(0, 0)[3], 128);
        assert!(maker.video().unwrap().assets.is_empty());
        assert_eq!(maker.chat_attachments[&video][0].id, source.id);
        let _ = fs::remove_dir_all(root);
    }
}
