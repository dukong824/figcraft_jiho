//! One font policy for the shared shell and both embedded editors.
use super::VideoMaker;
use egui::{FontFamily, Id};
impl VideoMaker {
    pub(super) fn appearance(&mut self, ctx: &egui::Context) -> bool {
        let size = self.store.data.ui_font_size;
        let size = if size.is_finite() { size.clamp(12.0, 20.0) } else { 14.0 };
        if self.font_size_applied != Some(size) {
            self.store.data.ui_font_size = size;
            ctx.set_zoom_factor(self.store.data.ui_zoom.clamp(0.7, 2.0) * size / 14.0);
            self.font_size_applied = Some(size);
        }
        ctx.data_mut(|d| d.insert_temp(Id::unique("purpose-video-icon"), self.store.data.video_icon.clone()));
        if self.font_applied.as_deref() == Some(self.store.data.ui_font.as_str()) {
            return false;
        }
        let base = self.base_fonts.get_or_insert_with(|| ctx.fonts(|f| f.definitions().clone())).clone();
        let (fonts, found) = definitions(base, &self.store.data.ui_font);
        if !found {
            self.message = "선택한 글꼴을 찾을 수 없어 기본 글꼴을 사용합니다".into();
        }
        ctx.set_fonts(fonts);
        self.font_applied = Some(self.store.data.ui_font.clone());
        ctx.request_repaint();
        true
    }
}
fn definitions(mut fonts: egui::FontDefinitions, family: &str) -> (egui::FontDefinitions, bool) {
    if family.is_empty() {
        return (fonts, true);
    }
    let Some(regular) = figcraft_text::find_face(family, "Regular").filter(|f| f.family.eq_ignore_ascii_case(family)) else { return (fonts, false) };
    let normal = "purpose-ui-custom-regular".to_owned();
    fonts.font_data.insert(normal.clone(), std::sync::Arc::new(figcraft_ui_egui::theme::font_data(&regular)));
    let bold = figcraft_text::find_face(family, "Bold")
        .filter(|f| f.family.eq_ignore_ascii_case(family) && (f.weight >= 600 || f.axes.iter().any(|a| a.tag == "wght" && a.max >= 600.0)));
    let strong = if let Some(face) = bold {
        let name = "purpose-ui-custom-bold".to_owned();
        let mut data = figcraft_ui_egui::theme::font_data(&face);
        if face.axes.iter().any(|a| a.tag == "wght") {
            data.tweak.coords.push(b"wght", 700.0);
        }
        fonts.font_data.insert(name.clone(), std::sync::Arc::new(data));
        Some(name)
    } else {
        None
    };
    for (key, list) in &mut fonts.families {
        let is_bold = matches!(key, FontFamily::Name(name) if name.contains("bold") || name.contains("medium"));
        if is_bold {
            // A regular-only family must never displace the existing bold fallback.
            if let Some(strong) = &strong {
                list.insert(0, strong.clone());
            }
        } else {
            list.insert(0, normal.clone());
        }
    }
    (fonts, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_font_keeps_real_bold_faces_or_the_existing_bold_fallback() {
        figcraft_text::scan_system_fonts();
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| super::super::document::install_fonts(ui.ctx()));
        output.textures_delta.clear();
        let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
        output.textures_delta.clear();
        let base = ctx.fonts(|f| f.definitions().clone());
        for family in figcraft_text::family_names().into_iter().take(100) {
            let (fonts, found) = definitions(base.clone(), &family);
            if !found {
                continue;
            }
            let list = &fonts.families[&FontFamily::Name("purpose-script-bold".into())];
            assert_ne!(list.first().map(String::as_str), Some("purpose-ui-custom-regular"), "{family}");
            if let Some(data) = fonts.font_data.get("purpose-ui-custom-bold") {
                let face = figcraft_text::find_face(&family, "Bold").unwrap();
                assert!(face.weight >= 600 || data.tweak.coords.as_ref().iter().any(|(_, v)| *v >= 600.0));
            }
        }
    }
}
