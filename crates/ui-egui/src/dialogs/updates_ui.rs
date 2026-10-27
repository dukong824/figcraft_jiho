//! The personal fork's release channel. Upstream installers must never replace this build.
use crate::FigApp;
use crate::i18n::tr;

pub(crate) const RELEASES_URL: &str = "https://github.com/dukong824/figcraft_jiho/releases";

#[derive(Default)]
pub(crate) struct Runtime {
    pub(crate) manual_window_open: bool,
    pub(crate) status_message: Option<String>,
}

impl FigApp {
    /// The fork uses manually downloaded releases until it has its own signed updater.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_startup_update_check(&mut self, _on: bool) {}
}

pub(crate) fn check(app: &mut FigApp) {
    app.updates_rt.manual_window_open = true;
    app.updates_rt.status_message = Some(format!(
        "figcraft_jiho · FigCraft {}\nDownload this personal build's updates from its GitHub Releases page. Close the app before replacing its files.",
        env!("CARGO_PKG_VERSION")
    ));
}

pub(crate) fn tick(_app: &mut FigApp, _ctx: &egui::Context) {}

pub(crate) fn show(app: &mut FigApp, ctx: &egui::Context) {
    if !app.updates_rt.manual_window_open {
        return;
    }
    let mut open = true;
    let mut close = false;
    egui::Window::new(tr("Check for Updates"))
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            if let Some(message) = &app.updates_rt.status_message {
                ui.label(message);
            }
            ui.add_space(8.0);
            ui.hyperlink_to("figcraft_jiho · GitHub Releases", RELEASES_URL);
            ui.add_space(8.0);
            if ui.button(tr("Close")).clicked() {
                close = true;
            }
        });
    if !open || close {
        app.updates_rt.manual_window_open = false;
        app.updates_rt.status_message = None;
    }
}
