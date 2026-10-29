use super::{VideoMaker, style};
use egui::RichText;
impl VideoMaker {
    pub(super) fn settings(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let mut open = true;
        let mut choose = false;
        let mut disable = false;
        let mut now = false;
        let mut reveal = None;
        egui::Window::new("설정")
            .id(egui::Id::unique("purpose-settings"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(460.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label(RichText::new("저장 및 백업").size(18.0).strong());
                ui.add_space(12.0);
                ui.label("자동 백업 폴더");
                if let Some(path) = &self.store.data.backup_directory {
                    ui.add(egui::Label::new(path.to_string_lossy()).truncate()).on_hover_text(path.display().to_string());
                } else {
                    ui.label(RichText::new("폴더를 선택하면 자동 백업을 시작합니다.").color(style::MUTED));
                }
                ui.horizontal(|ui| {
                    choose = ui.button("폴더 선택…").clicked();
                    if self.store.data.backup_directory.is_some() {
                        now = ui.button("지금 백업").clicked();
                        disable = ui.button("자동 백업 끄기").clicked();
                    }
                });
                ui.add_space(10.0);
                ui.label(RichText::new("대본·디자인·편집 프로젝트와 라이브러리를 별도 폴더에 보관합니다.").size(12.0).color(style::MUTED));
                if self.backup.busy() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("백업 중…");
                    });
                }
                if let Some(at) = &self.backup.last_ok {
                    ui.label(RichText::new(format!("마지막 백업 {at}")).small().color(style::MUTED));
                }
                if let Some(path) = &self.backup.location
                    && ui.button("백업 폴더 열기").clicked()
                {
                    reveal = Some(path.clone());
                }
                if !self.backup.error.is_empty() {
                    ui.label(RichText::new(&self.backup.error).color(egui::Color32::from_rgb(180, 55, 45)));
                }
                ui.add_space(20.0);
                ui.separator();
                ui.label(RichText::new("프로젝트 저장 위치").strong());
                ui.add(egui::Label::new(self.store.root.display().to_string()).truncate()).on_hover_text(self.store.root.display().to_string());
                if ui.button("프로젝트 폴더 열기").clicked() {
                    reveal = Some(self.store.root.clone());
                }
            });
        self.settings_open = open;
        if choose && let Some(path) = rfd::FileDialog::new().set_title("자동 백업 폴더 선택").pick_folder() {
            self.store.data.backup_directory = Some(path);
            if self.store.data.backup_id.is_empty() {
                self.store.data.backup_id = format!("{}-{}", chrono::Utc::now().format("%Y%m%d%H%M%S%f"), std::process::id());
            }
            self.save();
            self.backup.request();
        }
        if disable {
            self.store.data.backup_directory = None;
            self.save();
        }
        if now {
            self.backup.request();
        }
        if let Some(path) = reveal {
            let name = if cfg!(windows) {
                "explorer"
            } else if cfg!(target_os = "macos") {
                "open"
            } else {
                "xdg-open"
            };
            if let Err(error) = std::process::Command::new(name).arg(path).spawn() {
                self.message = error.to_string();
            }
        }
    }
}
