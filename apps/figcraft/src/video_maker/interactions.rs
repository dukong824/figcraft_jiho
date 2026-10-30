//! Shared desktop input and library-to-workspace drag and drop.
use super::{
    VideoMaker,
    model::{AssetKind, Section},
    navigation::Route,
};
use std::{fs, time::Instant};
#[derive(Clone)]
pub(super) struct AssetDrag {
    pub video_id: u64,
    pub asset_id: u64,
}

/// egui's consume_key deliberately permits extra Shift/Alt; document and chat input need exact chords.
pub(super) fn consume_key(input: &mut egui::InputState, modifiers: egui::Modifiers, key: egui::Key) -> bool {
    let mut consumed = false;
    input.events.retain(|event| {
        if let egui::Event::Key { key: event_key, pressed: true, modifiers: event_modifiers, .. } = event
            && *event_key == key
            && event_modifiers.matches_exact(modifiers)
        {
            consumed = true;
            false
        } else {
            true
        }
    });
    consumed
}

#[derive(Clone, Default)]
struct WindowGeometry {
    size: winit::dpi::PhysicalSize<u32>,
    position: Option<winit::dpi::PhysicalPosition<i32>>,
    maximized: bool,
}

impl VideoMaker {
    pub fn raw_input(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        let mut zoom = self.store.data.ui_zoom;
        if !zoom.is_finite() {
            zoom = 1.0;
        }
        let before = zoom;
        let mut pointer = ctx.pointer_latest_pos();
        let canvas = if self.section_is_design() { self.design_canvas } else { None };
        raw.events.retain(|event| match event {
            egui::Event::PointerMoved(pos) => {
                pointer = Some(*pos);
                true
            }
            egui::Event::MouseWheel { unit, delta, modifiers, .. } if modifiers.ctrl || modifiers.mac_cmd => {
                if canvas.is_some_and(|r| pointer.is_some_and(|p| r.contains(p))) {
                    return true;
                }
                let step = match unit {
                    egui::MouseWheelUnit::Line => delta.y * 0.10,
                    egui::MouseWheelUnit::Point => delta.y * 0.0025,
                    egui::MouseWheelUnit::Page => delta.y * 0.20,
                };
                zoom = (zoom * step.clamp(-0.1, 0.1).exp()).clamp(0.7, 2.0);
                false
            }
            egui::Event::Key { key, pressed: true, modifiers, .. }
                if (modifiers.ctrl || modifiers.mac_cmd)
                    && matches!(key, egui::Key::Plus | egui::Key::Equals | egui::Key::Minus | egui::Key::Num0) =>
            {
                zoom = match key {
                    egui::Key::Minus => zoom / 1.1,
                    egui::Key::Num0 => 1.0,
                    _ => zoom * 1.1,
                }
                .clamp(0.7, 2.0);
                false
            }
            egui::Event::Key { key: egui::Key::F11, pressed: true, .. } => {
                ctx.data_mut(|d| d.insert_temp(egui::Id::unique("purpose.fullscreen-request"), true));
                false
            }
            egui::Event::Key { key: egui::Key::Escape, pressed: true, .. }
                if ctx.data(|d| d.get_temp::<bool>(egui::Id::unique("purpose.fullscreen-active")).unwrap_or(false)) =>
            {
                ctx.data_mut(|d| d.insert_temp(egui::Id::unique("purpose.fullscreen-request"), true));
                false
            }
            _ => true,
        });
        if (zoom - before).abs() > 0.001 {
            self.store.data.ui_zoom = zoom;
            ctx.set_zoom_factor(zoom * self.store.data.ui_font_size / 14.0);
            self.zoom_save_due = Some(Instant::now());
            ctx.request_repaint();
        }
    }
    pub fn window_requests(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        let request = ctx.data_mut(|d| d.remove_temp::<bool>(egui::Id::unique("purpose.fullscreen-request"))).unwrap_or(false);
        if let Some(window) = frame.winit_window() {
            log::debug!("Purpose geometry: native={:?} egui={:?} ppp={}", window.inner_size(), ctx.viewport_rect(), ctx.pixels_per_point());
            let current = window.fullscreen().is_some();
            if request {
                let target = !current;
                let key = egui::Id::unique("purpose.window-restore");
                if target {
                    ctx.data_mut(|d| {
                        d.insert_temp(
                            key,
                            WindowGeometry { size: window.inner_size(), position: window.outer_position().ok(), maximized: window.is_maximized() },
                        )
                    });
                    let monitor = window.current_monitor().or_else(|| window.primary_monitor());
                    #[cfg(windows)]
                    {
                        use winit::platform::windows::WindowExtWindows;
                        window.set_undecorated_shadow(false);
                        window.set_border_color(None);
                    }
                    window.set_maximized(false);
                    window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(monitor.clone())));
                    if let Some(monitor) = monitor {
                        let size = monitor.size();
                        let _ = window.request_inner_size(size);
                        window.set_outer_position(monitor.position());
                        ctx.send_viewport_cmd_to(
                            egui::ViewportId::ROOT,
                            egui::ViewportCommand::InnerSize(egui::vec2(size.width as f32, size.height as f32) / ctx.pixels_per_point()),
                        );
                    }
                } else {
                    window.set_fullscreen(None);
                    #[cfg(windows)]
                    {
                        use winit::platform::windows::{Color, WindowExtWindows};
                        window.set_undecorated_shadow(true);
                        window.set_border_color(Some(Color::SYSTEM_DEFAULT));
                    }
                    if let Some(saved) = ctx.data_mut(|d| d.remove_temp::<WindowGeometry>(key)) {
                        let _ = window.request_inner_size(saved.size);
                        if let Some(position) = saved.position {
                            window.set_outer_position(position);
                        }
                        window.set_maximized(saved.maximized);
                        ctx.send_viewport_cmd_to(
                            egui::ViewportId::ROOT,
                            egui::ViewportCommand::InnerSize(egui::vec2(saved.size.width as f32, saved.size.height as f32) / ctx.pixels_per_point()),
                        );
                    }
                }
                ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Fullscreen(target));
                ctx.request_repaint();
            }
            ctx.data_mut(|d| d.insert_temp(egui::Id::unique("purpose.fullscreen-active"), window.fullscreen().is_some()));
        }
    }
    pub(super) fn drop_allowed(&self, payload: &AssetDrag, section: Section) -> bool {
        let asset =
            self.store.data.channels.iter().flat_map(|c| &c.videos).find(|v| v.id == payload.video_id).and_then(|v| v.asset(payload.asset_id));
        asset.is_some_and(|a| match section {
            Section::Library | Section::Script => true,
            Section::Edit => a.kind.can_edit(),
            Section::Design => matches!(a.kind, AssetKind::Image | AssetKind::Design),
        })
    }
    pub(super) fn drop_asset(&mut self, payload: AssetDrag, target: Route) -> Result<(), String> {
        let Some((video_id, section)) = target.video() else { return Err("영상의 탭으로 드롭하세요".into()) };
        if !self.drop_allowed(&payload, section) {
            return Err("이 파일은 해당 탭에서 사용할 수 없습니다".into());
        }
        let source = self
            .store
            .data
            .channels
            .iter()
            .flat_map(|c| &c.videos)
            .find(|v| v.id == payload.video_id)
            .and_then(|v| v.asset(payload.asset_id))
            .cloned()
            .ok_or("원본 파일을 찾을 수 없습니다")?;
        let target_video =
            self.store.data.channels.iter().flat_map(|c| &c.videos).find(|v| v.id == video_id).ok_or("대상 영상을 찾을 수 없습니다")?;
        let extension = std::path::Path::new(&source.file).extension().and_then(|v| v.to_str()).unwrap_or("bin");
        let shared_file = format!("shared-{}-{}.{}", payload.video_id, source.id, extension);
        let existing = target_video.assets.iter().find(|a| a.file == shared_file).map(|a| a.id);
        let asset_id = if payload.video_id == video_id {
            source.id
        } else if let Some(existing) = existing {
            existing
        } else {
            let mut copied = source.clone();
            copied.id = self.store.data.id();
            copied.file = shared_file;
            let path = self.store.asset_path(video_id, &copied);
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            fs::copy(self.store.asset_path(payload.video_id, &source), path).map_err(|e| e.to_string())?;
            let id = copied.id;
            self.store
                .data
                .channels
                .iter_mut()
                .flat_map(|c| &mut c.videos)
                .find(|v| v.id == video_id)
                .ok_or("대상 영상을 찾을 수 없습니다")?
                .assets
                .push(copied);
            id
        };
        self.navigate(target);
        match section {
            Section::Script => self.add_to_script(asset_id),
            Section::Edit => self.append_film_asset(asset_id)?,
            Section::Design => self.pending_design_asset = Some(asset_id),
            Section::Library => {}
        }
        self.save();
        Ok(())
    }
    pub fn place_design_asset(&mut self, fig: &mut figcraft_ui_egui::FigApp) {
        if self.design_loaded != self.video_id() {
            return;
        }
        if let Some(id) = self.pending_design_asset.take()
            && let Some((video_id, asset)) = self.video().and_then(|v| v.asset(id).map(|a| (v.id, a.clone())))
        {
            let path = self.store.asset_path(video_id, &asset);
            if let Err(error) = fig.session.execute("file.import", &serde_json::json!({"path":path})) {
                self.message = format!("디자인에 파일 추가 실패: {error}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canvas_control_wheel_preserves_native_zoom_while_surrounding_panels_change_ui_zoom() {
        let root = std::env::temp_dir().join(format!("purpose-canvas-zoom-{}", std::process::id()));
        let mut maker = VideoMaker::at(root.clone()).unwrap();
        maker.navigate(Route::Video { video_id: maker.video_id().unwrap(), section: Section::Design });
        maker.design_canvas = Some(egui::Rect::from_min_max(egui::pos2(300.0, 100.0), egui::pos2(900.0, 700.0)));
        let ctx = egui::Context::default();
        let wheel = egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, 1.0),
            modifiers: egui::Modifiers::CTRL,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
        };
        let mut raw = egui::RawInput { events: vec![egui::Event::PointerMoved(egui::pos2(400.0, 200.0)), wheel.clone()], ..Default::default() };
        maker.raw_input(&ctx, &mut raw);
        assert_eq!(maker.store.data.ui_zoom, 1.0);
        assert_eq!(raw.events.len(), 2);
        raw.events = vec![egui::Event::PointerMoved(egui::pos2(100.0, 200.0)), wheel];
        maker.raw_input(&ctx, &mut raw);
        assert!(maker.store.data.ui_zoom > 1.0);
        assert_eq!(raw.events.len(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn shift_enter_and_shift_tab_are_not_consumed_as_plain_keys() {
        for key in [egui::Key::Enter, egui::Key::Tab] {
            let mut input = egui::InputState::default();
            input.events.push(egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::SHIFT });
            assert!(!consume_key(&mut input, egui::Modifiers::NONE, key));
            assert_eq!(input.events.len(), 1);
            assert!(consume_key(&mut input, egui::Modifiers::SHIFT, key));
            assert!(input.events.is_empty());
        }
    }
    #[test]
    fn dragging_between_videos_copies_the_file_and_attaches_to_the_target_script() {
        let root = std::env::temp_dir().join(format!("purpose-drop-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let mut maker = VideoMaker::at(root.clone()).unwrap();
        let source_id = maker.video_id().unwrap();
        let file = root.join("source.png");
        image::RgbaImage::from_pixel(16, 16, image::Rgba([20, 80, 160, 255])).save(&file).unwrap();
        maker.import_path(&file).unwrap();
        let asset = maker.video().unwrap().assets[0].clone();
        let channel = maker.store.data.channels[0].id;
        let target = maker.store.data.add_video(channel).unwrap();
        maker.drop_asset(AssetDrag { video_id: source_id, asset_id: asset.id }, Route::Video { video_id: target, section: Section::Script }).unwrap();
        let target_video = maker.video().unwrap();
        assert_eq!(target_video.id, target);
        assert_eq!(target_video.assets.len(), 1);
        assert!(target_video.script_assets.contains(&target_video.assets[0].id));
        assert_eq!(
            fs::read(maker.store.asset_path(source_id, &asset)).unwrap(),
            fs::read(maker.store.asset_path(target, &target_video.assets[0])).unwrap()
        );
        let id = target_video.assets[0].id;
        maker.drop_asset(AssetDrag { video_id: source_id, asset_id: asset.id }, Route::Video { video_id: target, section: Section::Script }).unwrap();
        assert_eq!(maker.video().unwrap().assets.len(), 1);
        maker.drop_asset(AssetDrag { video_id: target, asset_id: id }, Route::Video { video_id: target, section: Section::Script }).unwrap();
        assert_eq!(maker.video().unwrap().assets.len(), 1);
        assert_eq!(maker.video().unwrap().script_assets.len(), 1);
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn control_wheel_changes_the_whole_ui_and_does_not_reach_the_canvas() {
        let root = std::env::temp_dir().join(format!("purpose-zoom-{}", std::process::id()));
        let mut maker = VideoMaker::at(root.clone()).unwrap();
        let ctx = egui::Context::default();
        let mut raw = egui::RawInput::default();
        raw.events.push(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, 1.0),
            modifiers: egui::Modifiers::CTRL,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
        });
        maker.raw_input(&ctx, &mut raw);
        assert!(maker.store.data.ui_zoom > 1.0);
        assert!(raw.events.is_empty());
        maker.store.save().unwrap();
        assert!(super::super::model::Store::load(root.clone()).unwrap().data.ui_zoom > 1.0);
        let _ = fs::remove_dir_all(root);
    }
}
