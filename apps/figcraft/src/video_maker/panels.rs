use super::{
    SideView, VideoMaker, codex,
    style::{self, Icon},
};
use egui::{RichText, TextEdit, vec2};

impl VideoMaker {
    pub(super) fn side_panel(&mut self, ui: &mut egui::Ui) {
        let Some(view) = self.side_view else { return };
        let mut close = false;
        egui::Panel::right("purpose-side-view")
            .default_size(330.0)
            .min_size(280.0)
            .max_size(520.0)
            .resizable(true)
            .frame(egui::Frame::NONE.fill(style::SURFACE).stroke(egui::Stroke::new(1.0, style::BORDER)).inner_margin(16.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(match view {
                            SideView::Codex => "Codex",
                            SideView::Trash => "휴지통",
                        })
                        .size(16.0)
                        .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        close = style::icon_button(ui, Icon::Close, 26.0, "사이드 보기 닫기").clicked();
                    });
                });
                ui.separator();
                match view {
                    SideView::Codex => self.codex_contents(ui),
                    SideView::Trash => self.trash_contents(ui),
                }
            });
        if close {
            self.side_view = None;
        }
    }

    fn codex_contents(&mut self, ui: &mut egui::Ui) {
        let path = codex::executable(self.store.data.codex_path.as_deref());
        ui.label(
            RichText::new(if self.auth_job.is_some() {
                "연결 확인 중…"
            } else if self.authenticated == Some(true) {
                "계정 연결됨"
            } else {
                "ChatGPT 계정을 연결하세요"
            })
            .size(13.0)
            .color(style::MUTED),
        );
        ui.collapsing("계정 및 CLI 연결", |ui| {
            if let Some(path) = &path {
                ui.label(RichText::new(path.to_string_lossy()).size(11.0));
            }
            if ui.button("CLI 실행 파일 선택").clicked()
                && let Some(path) = rfd::FileDialog::new().pick_file()
            {
                self.login = None;
                self.auth_job = None;
                self.store.data.codex_path = Some(path);
                self.authenticated = None;
                self.save();
                self.refresh_auth();
            }
            if self.login.is_some() {
                ui.spinner();
                ui.label("브라우저에서 로그인을 마치세요.");
                if ui.button("로그인 취소").clicked() {
                    self.login = None;
                }
            } else if path.is_some() {
                ui.horizontal_wrapped(|ui| {
                    if self.authenticated != Some(true) {
                        if ui.button("ChatGPT로 로그인").clicked() {
                            self.start_login(false);
                        }
                        if ui.button("기기 코드로 로그인").clicked() {
                            self.start_login(true);
                        }
                    }
                    if ui.button("연결 확인").clicked() {
                        self.refresh_auth();
                    }
                    if self.authenticated == Some(true)
                        && ui.button("로그아웃").clicked()
                        && let Some(path) = path.clone()
                    {
                        let (tx, rx) = std::sync::mpsc::channel();
                        std::thread::spawn(move || {
                            let result = codex::command(&path).arg("logout").output().map_err(|e| e.to_string()).and_then(|_| codex::status(&path));
                            let _ = tx.send(result);
                        });
                        self.auth_job = Some(rx);
                    }
                });
            } else {
                ui.hyperlink_to("Codex CLI 다운로드", "https://github.com/openai/codex/releases");
            }
        });
        // Show sign-in actions immediately on the first disconnected visit.
        if self.authenticated != Some(true) && self.login.is_none() && path.is_some() && ui.button("ChatGPT 계정 연결").clicked() {
            self.start_login(false);
        }
        if !self.auth_error.is_empty() {
            ui.label(RichText::new(&self.auth_error).color(style::BLUE).size(12.0));
        }
        if !self.login_output.is_empty() && self.authenticated != Some(true) {
            egui::ScrollArea::vertical().id_salt("codex-login-output").max_height(150.0).show(ui, |ui| {
                ui.label(RichText::new(&self.login_output).size(12.0));
                for word in self.login_output.split_whitespace().filter(|s| s.starts_with("https://") || s.starts_with("http://localhost:")) {
                    ui.hyperlink(word);
                }
            });
        }
        ui.add_space(8.0);
        ui.label(RichText::new("열려 있는 영상의 대본·에셋·타임라인을 참고합니다.").size(12.0).color(style::MUTED));
        let response_height = (ui.available_height() - 155.0).max(60.0);
        egui::ScrollArea::vertical().id_salt("purpose-ai-output").max_height(response_height).show(ui, |ui| {
            if self.ai_job.is_some() {
                ui.spinner();
                ui.label("Codex 작업 중…");
            }
            ui.label(&self.ai_response);
            if !self.ai_response.is_empty() && self.ai_job.is_none() && self.video().is_some() && ui.button("대본에 추가").clicked() {
                if let Some(video) = self.store.data.selected_mut() {
                    if !video.script_body.is_empty() {
                        video.script_body.push_str("\n\n");
                    }
                    video.script_body.push_str(&self.ai_response);
                }
                self.save();
            }
        });
        ui.add_space((ui.available_height() - 145.0).max(4.0));
        egui::Frame::NONE.stroke(egui::Stroke::new(1.0, style::BORDER)).corner_radius(8).inner_margin(9.0).show(ui, |ui| {
            ui.add_sized(
                [ui.available_width(), 60.0],
                TextEdit::multiline(&mut self.prompt).desired_rows(2).frame(egui::Frame::NONE).hint_text("무엇을 도와드릴까요?"),
            );
            let enabled = self.authenticated == Some(true)
                && self.ai_job.is_none()
                && self.tabs.route().and_then(super::navigation::Route::video).is_some()
                && !self.prompt.trim().is_empty();
            if ui.add_enabled(enabled, egui::Button::new("보내기").fill(style::SIDEBAR).min_size(vec2(64.0, 28.0))).clicked() {
                self.ask_codex(self.prompt.clone());
            }
        });
        ui.label(RichText::new("연결된 계정의 Codex 이용 한도가 적용됩니다.").size(11.0).color(style::FAINT));
    }

    fn trash_contents(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("영상의 오른쪽 클릭 메뉴에서 휴지통으로 이동할 수 있습니다. 파일은 복원할 수 있도록 보관됩니다.")
                .size(12.0)
                .color(style::MUTED),
        );
        let entries = self.store.data.trash.clone();
        if entries.is_empty() {
            ui.add_space(20.0);
            ui.label("휴지통이 비어 있습니다.");
        }
        let mut restore = None;
        egui::ScrollArea::vertical().id_salt("purpose-trash").show(ui, |ui| {
            for entry in entries.iter().rev() {
                ui.add_space(12.0);
                ui.label(RichText::new(&entry.video.name).strong());
                ui.label(RichText::new(format!("{} · {}", entry.channel_name, entry.deleted_at)).small());
                if ui.button("복원").clicked() {
                    restore = Some(entry.video.id);
                }
                ui.separator();
            }
        });
        if let Some(id) = restore {
            self.store.data.restore_video(id);
            self.save();
        }
    }
}
