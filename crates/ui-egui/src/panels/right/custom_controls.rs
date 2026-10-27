//! Continuous custom-effect controls: direct number entry plus an accessible slider.
use super::menu::{block, row};
use super::*;

pub(super) fn number(ui: &mut Ui, id: Id, label: &str, value: f64, range: std::ops::RangeInclusive<f64>, step: f64) -> Option<FieldEdit> {
    let mut typed = None;
    row(ui, 28.0, |ui| {
        typed = widgets::number_field(ui, id, Lead::Text(tr(label)), Some(value), "", ui.available_width(), step);
    });
    let mut moved = None;
    block(ui, 20.0, 0.0, |ui| {
        // Frame keeps a useful ten-second track; typing longer/negative times extends it.
        // Never clamp a saved value merely because the panel was opened.
        let range = if label == "Frame (ms)" { value.min(0.0)..=value.max(10_000.0) } else { range };
        let mut next = value;
        let response = ui
            .push_id(id.with("slider"), |ui| {
                ui.spacing_mut().slider_width = ui.available_width();
                ui.spacing_mut().interact_size.y = 18.0;
                ui.add(egui::Slider::new(&mut next, range).clamping(egui::SliderClamping::Edits).step_by(step).show_value(false))
            })
            .inner;
        response.widget_info(|| egui::WidgetInfo::slider(ui.is_enabled(), next, tr(label)));
        let gesture = id.with("slider_gesture");
        let active = ui.data(|data| data.get_temp::<bool>(gesture)).unwrap_or(false);
        let held = ui.input(|input| input.pointer.primary_down());
        if response.changed() {
            if held {
                ui.data_mut(|data| data.insert_temp(gesture, true));
                moved = Some(FieldEdit::Scrub(next, false));
            } else {
                ui.data_mut(|data| data.remove::<bool>(gesture));
                moved = Some(if active { FieldEdit::Scrub(next, true) } else { FieldEdit::Set(next) });
            }
        } else if active && (!held || response.drag_stopped()) {
            // The release frame often has no value change: it must still finish the undo group.
            ui.data_mut(|data| data.remove::<bool>(gesture));
            moved = Some(FieldEdit::Scrub(value, true));
        }
    });
    typed.or(moved)
}
