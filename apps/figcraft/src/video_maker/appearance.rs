//! One font policy for the shared shell and both embedded editors.
use super::VideoMaker;
use egui::{FontFamily, Id};
impl VideoMaker {
    pub(super) fn appearance(&mut self, ctx: &egui::Context) -> bool {
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
    let bold = figcraft_text::find_face(family, "Bold").filter(|f| f.family.eq_ignore_ascii_case(family) && f.weight >= 600);
    let strong = if let Some(face) = bold {
        let name = "purpose-ui-custom-bold".to_owned();
        fonts.font_data.insert(name.clone(), std::sync::Arc::new(figcraft_ui_egui::theme::font_data(&face)));
        name
    } else {
        normal.clone()
    };
    for (key, list) in &mut fonts.families {
        let is_bold = matches!(key, FontFamily::Name(name) if name.contains("bold") || name.contains("medium"));
        list.insert(0, if is_bold { strong.clone() } else { normal.clone() });
    }
    (fonts, true)
}
