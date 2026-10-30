use super::{
    SideView, VideoMaker, codex,
    style::{self, Icon},
};
use egui::{RichText, TextEdit, vec2};

impl VideoMaker {
    pub(super) fn side_panel(&mut self, ui: &mut egui::Ui) {
        if self.side_view.is_none() {
            ui.ctx().data_mut(|d| d.remove::<egui::Rect>(egui::Id::unique("purpose-codex-bounds")));
        }
        let Some(view) = self.side_view else { return };
        let mut close = false;
        egui::Panel::right("purpose-side-view")
            .show_separator_line(false)
            .default_size(390.0)
            .min_size(310.0)
            .max_size(650.0)
            .resizable(true)
            .frame(egui::Frame::NONE.fill(style::SURFACE).inner_margin(egui::Margin { left: 18, right: 18, top: 6, bottom: 16 }))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(match view {
                            SideView::Codex => "Codex",
                        })
                        .size(figcraft_ui_icons::font_size(14.0))
                        .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        close = style::icon_button(ui, Icon::Close, 26.0, "사이드 보기 닫기").clicked();
                        if view == SideView::Codex {
                            let more = style::icon_button(ui, Icon::More, 26.0, "Codex 메뉴");
                            egui::Popup::menu(&more).show(|ui| {
                                if style::menu_item(ui, "연결 다시 확인", Icon::History, false).clicked() {
                                    self.refresh_auth();
                                    ui.close();
                                }
                                if style::menu_item(ui, "실행 파일 선택…", Icon::Settings, false).clicked() {
                                    self.select_codex();
                                    ui.close();
                                }
                                if self.authenticated == Some(true) && style::menu_item(ui, "로그아웃", Icon::Close, true).clicked() {
                                    if let Some(path) = codex::executable(self.store.data.codex_path.as_deref()) {
                                        let (tx, rx) = std::sync::mpsc::channel();
                                        std::thread::spawn(move || {
                                            let result = codex::command(&path)
                                                .arg("logout")
                                                .output()
                                                .map_err(|e| e.to_string())
                                                .and_then(|_| codex::status(&path));
                                            let _ = tx.send(result);
                                        });
                                        self.auth_job = Some(rx);
                                    }
                                    ui.close();
                                }
                            });
                        }
                    });
                });
                ui.separator();
                match view {
                    SideView::Codex => self.codex_contents(ui),
                }
                let r = ui.max_rect();
                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::unique("purpose-codex-bounds"), r.expand2(vec2(18.0, 6.0))));
                let x = r.left() - 18.0;
                self.dividers.push((egui::pos2(x, r.top() - 6.0), egui::pos2(x, r.bottom() + 16.0)));
            });
        if close {
            self.side_view = None;
        }
    }

    fn codex_contents(&mut self, ui: &mut egui::Ui) {
        if self.authenticated != Some(true) {
            self.codex_sign_in(ui);
            return;
        }
        let Some(video_id) = self.tabs.route().and_then(super::navigation::Route::video).map(|(id, _)| id) else {
            ui.centered_and_justified(|ui| {
                ui.label("왼쪽에서 영상을 선택하세요");
            });
            return;
        };
        let messages = self.conversation(video_id).messages.clone();
        let running = self.chat_job.as_ref().filter(|job| job.video_id == video_id).map(|job| job.reply);
        let rect = ui.available_rect_before_wrap();
        let text_height = ui.fonts_mut(|f| f.layout_job(composer_text(&self.prompt, (rect.width() - 32.0).max(60.0)))).size().y;
        let drop_hover = self.attachment_drop(ui, video_id, rect);
        let pasting = self.pastes.iter().any(|p| p.video == video_id && p.attach);
        let has_attachments = self.chat_attachments.get(&video_id).is_some_and(|a| !a.is_empty()) || pasting;
        let attachment_height = if has_attachments { 44.0 } else { 0.0 };
        let composer = composer_rect(rect, text_height.max(44.0) + attachment_height);
        let conversation = egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), (composer.top() - 12.0).max(rect.top())));
        ui.scope_builder(egui::UiBuilder::new().max_rect(conversation), |ui| {
            egui::ScrollArea::vertical().id_salt(("codex-conversation", video_id)).auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
                ui.set_width(conversation.width() - 8.0);
                if messages.is_empty() {
                    ui.add_space(48.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("무엇을 함께 만들까요?").size(figcraft_ui_icons::font_size(19.0)).strong());
                    });
                }
                for (index, message) in messages.iter().enumerate() {
                    ui.push_id(index, |ui| {
                        ui.add_space(20.0);
                        if message.role == super::chat::Role::User {
                            let max_width = (ui.available_width() * 0.90 - 24.0).max(80.0);
                            let mut job = egui::text::LayoutJob::default();
                            job.wrap.max_width = max_width;
                            job.append(
                                &format!(
                                    "{}{}",
                                    message.text,
                                    if message.attachments.is_empty() {
                                        String::new()
                                    } else {
                                        format!("\n\n첨부 · {}", message.attachments.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(" · "))
                                    }
                                ),
                                0.0,
                                egui::TextFormat {
                                    font_id: egui::FontId::proportional(figcraft_ui_icons::font_size(14.0)),
                                    color: style::TEXT,
                                    line_height: Some(figcraft_ui_icons::font_size(21.0)),
                                    ..Default::default()
                                },
                            );
                            let galley = ui.fonts_mut(|f| f.layout_job(job.clone()));
                            let size = galley.size() + vec2(24.0, 20.0);
                            let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), size.y), egui::Sense::hover());
                            let bubble = egui::Rect::from_min_size(egui::pos2(row.right() - size.x, row.top()), size);
                            ui.painter().rect_filled(bubble, 12, style::SIDEBAR);
                            ui.scope_builder(
                                egui::UiBuilder::new().max_rect(bubble.shrink2(vec2(12.0, 10.0))).layout(egui::Layout::top_down(egui::Align::Min)),
                                |ui| {
                                    ui.add(egui::Label::new(job).selectable(true).wrap());
                                },
                            );
                            let (time, _) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), egui::Sense::hover());
                            ui.painter().text(
                                time.right_center(),
                                egui::Align2::RIGHT_CENTER,
                                &message.at,
                                egui::FontId::proportional(figcraft_ui_icons::font_size(10.0)),
                                style::FAINT,
                            );
                        } else {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Codex").size(figcraft_ui_icons::font_size(13.0)).strong());
                                if running != Some(index) {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} · {}",
                                            if message.cancelled {
                                                "중지"
                                            } else if message.error {
                                                "오류"
                                            } else {
                                                "완료"
                                            },
                                            super::chat::elapsed(message.elapsed_ms)
                                        ))
                                        .size(figcraft_ui_icons::font_size(11.0))
                                        .color(style::MUTED),
                                    );
                                }
                            });
                            if !message.activities.is_empty() || running == Some(index) {
                                let latest = message.activities.last().map_or("요청 준비 중", |a| a.label.as_str());
                                let title = if running == Some(index) {
                                    format!("{latest} · {}", super::chat::elapsed(message.elapsed_ms))
                                } else {
                                    format!("작업 내역 {}개", message.activities.len())
                                };
                                ui.horizontal(|ui| {
                                    if running == Some(index) {
                                        ui.spinner();
                                    }
                                    egui::CollapsingHeader::new(RichText::new(title).size(figcraft_ui_icons::font_size(12.0)).color(style::MUTED))
                                        .show(ui, |ui| {
                                            for activity in &message.activities {
                                                ui.label(
                                                    RichText::new(format!("{}  {}", super::chat::elapsed(activity.elapsed_ms), activity.label))
                                                        .size(figcraft_ui_icons::font_size(11.0))
                                                        .color(style::MUTED),
                                                );
                                                if !activity.detail.is_empty() {
                                                    ui.add(
                                                        egui::Label::new(RichText::new(&activity.detail).size(figcraft_ui_icons::font_size(12.0)))
                                                            .selectable(true)
                                                            .wrap(),
                                                    );
                                                }
                                                ui.add_space(6.0);
                                            }
                                            if let Some(usage) = &message.usage
                                                && let (Some(input), Some(output)) = (usage["input_tokens"].as_u64(), usage["output_tokens"].as_u64())
                                            {
                                                ui.label(
                                                    RichText::new(format!("입력 {input} · 출력 {output} 토큰"))
                                                        .size(figcraft_ui_icons::font_size(11.0))
                                                        .color(style::MUTED),
                                                );
                                            }
                                        });
                                });
                            }
                            if !message.text.is_empty() {
                                let job = super::chat::markdown(&message.text, ui.available_width(), message.error);
                                ui.add(egui::Label::new(job).selectable(true).wrap());
                                let links = super::chat::links(&message.text);
                                if !links.is_empty() {
                                    ui.horizontal_wrapped(|ui| {
                                        for (label, url) in links {
                                            if ui.link(label).on_hover_text(&url).clicked() {
                                                let _ = webbrowser::open(&url);
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    });
                }
                ui.add_space(12.0);
            });
        });
        let mut send = false;
        ui.painter().rect_filled(composer, 12, style::SURFACE);
        ui.painter().rect_stroke(
            composer,
            12,
            egui::Stroke::new(1.0, if drop_hover { style::BLUE } else { style::BORDER }),
            egui::StrokeKind::Inside,
        );
        let inner = composer.shrink(12.0);
        let controls = egui::Rect::from_min_max(egui::pos2(inner.left(), inner.bottom() - 30.0), inner.max);
        let input = egui::Rect::from_min_max(inner.min + vec2(0.0, attachment_height), egui::pos2(inner.right(), controls.top() - 12.0));
        if has_attachments {
            let chips = egui::Rect::from_min_size(inner.min, vec2(inner.width(), 36.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(chips), |ui| self.attachment_chips(ui, video_id));
        }
        let id = egui::Id::unique(("codex-composer", video_id));
        let enter = self.chat_job.is_none()
            && ui.memory(|m| m.has_focus(id))
            && ui.input_mut(|i| super::interactions::consume_key(i, egui::Modifiers::NONE, egui::Key::Enter));
        ui.scope_builder(egui::UiBuilder::new().max_rect(input), |ui| {
            egui::ScrollArea::vertical()
                .id_salt(("codex-input-scroll", video_id))
                .min_scrolled_height(input.height())
                .max_height(input.height())
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let mut layouter =
                        |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap: f32| ui.fonts_mut(|f| f.layout_job(composer_text(text.as_str(), wrap)));
                    ui.add(
                        TextEdit::multiline(&mut self.prompt)
                            .id(id)
                            .font(egui::FontId::proportional(figcraft_ui_icons::font_size(14.0)))
                            .desired_width(ui.available_width())
                            .desired_rows(2)
                            .margin(egui::Margin::ZERO)
                            .frame(egui::Frame::NONE)
                            .layouter(&mut layouter)
                            .hint_text("메시지 보내기…"),
                    );
                });
        });
        ui.scope_builder(egui::UiBuilder::new().max_rect(controls), |ui| {
            ui.horizontal(|ui| {
                self.attachment_picker(ui, video_id);
                ui.allocate_ui_with_layout(vec2((controls.width() - 76.0).max(80.0), 30.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.set_min_width((controls.width() - 76.0).max(80.0));
                    self.codex_controls(ui);
                });
                if let Some(job) = &self.chat_job {
                    if job.video_id == video_id {
                        if style::round_button(ui, Icon::Stop, true, "응답 중지").clicked() {
                            job.cancel();
                        }
                    } else {
                        style::round_button(ui, Icon::Stop, false, "다른 영상에서 작업 중");
                    }
                } else {
                    let enabled = (!self.prompt.trim().is_empty() || has_attachments) && !pasting;
                    send = style::round_button(ui, Icon::Send, enabled, "보내기 · Enter / 줄바꿈 · Shift+Enter").clicked() && enabled
                        || enter && enabled;
                }
            });
        });
        if send {
            self.ask_codex(if self.prompt.trim().is_empty() { "첨부한 자료를 확인해 주세요.".into() } else { self.prompt.clone() });
        }
    }

    fn codex_sign_in(&mut self, ui: &mut egui::Ui) {
        let height = ui.available_height();
        ui.add_space((height * 0.26).max(20.0));
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("Codex").size(figcraft_ui_icons::font_size(26.0)).strong());
            ui.add_space(12.0);
            if self.auth_job.is_some() {
                ui.spinner();
                ui.label("연결 확인 중…");
                return;
            }
            if self.login.is_some() {
                ui.spinner();
                ui.label("브라우저에서 로그인을 완료하세요");
                if ui.button("취소").clicked() {
                    self.login = None;
                }
            } else if codex::executable(self.store.data.codex_path.as_deref()).is_some() {
                if ui
                    .add(egui::Button::new(RichText::new("ChatGPT로 연결").color(egui::Color32::WHITE)).fill(style::TEXT).min_size(vec2(170.0, 36.0)))
                    .clicked()
                {
                    self.start_login(false);
                }
            } else if ui.button("Codex 실행 파일 선택").clicked() {
                self.select_codex();
            }
            ui.add_space(8.0);
            ui.collapsing("다른 연결 방법", |ui| {
                if ui.button("기기 코드로 로그인").clicked() {
                    self.start_login(true);
                }
                if ui.button("다시 확인").clicked() {
                    self.refresh_auth();
                }
                if ui.button("실행 파일 선택").clicked() {
                    self.select_codex();
                }
            });
        });
        if !self.auth_error.is_empty() {
            ui.add_space(16.0);
            ui.label(RichText::new(&self.auth_error).size(figcraft_ui_icons::font_size(12.0)).color(style::MUTED));
        }
        if !self.login_output.is_empty() {
            egui::ScrollArea::vertical().id_salt("codex-login-output").max_height(180.0).show(ui, |ui| {
                ui.label(RichText::new(&self.login_output).size(figcraft_ui_icons::font_size(12.0)));
                for word in self.login_output.split_whitespace().filter(|s| s.starts_with("https://") || s.starts_with("http://localhost:")) {
                    ui.hyperlink(word);
                }
            });
        }
    }
    fn select_codex(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            self.login = None;
            self.auth_job = None;
            self.store.data.codex_path = Some(path);
            self.authenticated = None;
            self.catalog.clear();
            self.catalog_job = None;
            self.save();
            self.refresh_auth();
        }
    }
}

impl VideoMaker {
    fn codex_controls(&mut self, ui: &mut egui::Ui) {
        let before = self.store.data.codex_settings.clone();
        let model = self.selected_model().cloned();
        let label = model.as_ref().map(|m| m.display_name.as_str()).filter(|s| !s.is_empty()).unwrap_or(if before.model.is_empty() {
            "모델"
        } else {
            &before.model
        });
        let model_button = style::dropdown(ui, label, (ui.available_width() - 188.0).max(42.0));
        egui::Popup::menu(&model_button).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| {
            ui.set_min_width(210.0);
            if self.catalog_job.is_some() {
                ui.label("모델 불러오는 중…");
            }
            if ui.selectable_label(before.model.is_empty(), "자동 선택").clicked() {
                self.store.data.codex_settings.model.clear();
                ui.close();
            }
            for model in &self.catalog {
                if ui.selectable_label(before.model == model.model, &model.display_name).on_hover_text(&model.model).clicked() {
                    self.store.data.codex_settings.model = model.model.clone();
                    self.store.data.codex_settings.effort = model.default_reasoning_effort.clone();
                    self.store.data.codex_settings.fast &= model.fast_tier().is_some();
                    ui.close();
                }
            }
            ui.separator();
            ui.collapsing("모델 ID 직접 입력", |ui| {
                ui.text_edit_singleline(&mut self.store.data.codex_settings.model);
            });
            if !self.catalog_error.is_empty() {
                ui.label(RichText::new(&self.catalog_error).small().color(style::MUTED));
            }
            if ui.button("목록 새로고침").clicked() {
                self.refresh_catalog();
            }
        });
        let effort_button = style::dropdown(ui, super::codex_options::effort_label(&before.effort), 60.0);
        egui::Popup::menu(&effort_button).show(|ui| {
            if ui.selectable_label(before.effort.is_empty(), "자동").clicked() {
                self.store.data.codex_settings.effort.clear();
                ui.close();
            }
            if let Some(model) = &model {
                for effort in &model.supported_reasoning_efforts {
                    if ui
                        .selectable_label(before.effort == effort.reasoning_effort, super::codex_options::effort_label(&effort.reasoning_effort))
                        .on_hover_text(&effort.description)
                        .clicked()
                    {
                        self.store.data.codex_settings.effort = effort.reasoning_effort.clone();
                        ui.close();
                    }
                }
            } else {
                ui.label(RichText::new("모델 목록을 불러오면 선택할 수 있습니다").small());
            }
        });
        let fast = model.as_ref().is_some_and(|m| m.fast_tier().is_some());
        let tip = model
            .as_ref()
            .and_then(|m| m.service_tiers.iter().find(|t| Some(t.id.as_str()) == m.fast_tier()))
            .map(|t| t.description.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(if fast { "빠른 생성" } else { "이 모델에서 빠른 생성을 지원하지 않습니다" });
        if ui.add_enabled_ui(fast, |ui| style::fast_toggle(ui, before.fast, 26.0, tip)).inner.clicked() {
            self.store.data.codex_settings.fast = !before.fast;
        }
        if before.model != self.store.data.codex_settings.model {
            if let Some(model) = self.selected_model().cloned() {
                self.store.data.codex_settings.reconcile(&model);
            } else {
                self.store.data.codex_settings.effort.clear();
                self.store.data.codex_settings.fast = false;
            }
        }
        if before != self.store.data.codex_settings {
            self.save();
        }
    }
}

fn composer_text(text: &str, width: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::simple(text.into(), egui::FontId::proportional(figcraft_ui_icons::font_size(14.0)), style::TEXT, width);
    for section in &mut job.sections {
        section.format.line_height = Some(figcraft_ui_icons::font_size(22.0));
    }
    job
}
fn composer_rect(bounds: egui::Rect, text_height: f32) -> egui::Rect {
    let height = (text_height.max(44.0) + 66.0).min((bounds.height() * 0.55).clamp(110.0, 320.0));
    egui::Rect::from_min_max(egui::pos2(bounds.left(), bounds.bottom() - height), bounds.max)
}
#[cfg(test)]
mod layout_tests {
    use super::*;
    #[test]
    fn composer_grows_upward_and_keeps_controls_fixed_when_a_third_line_appears() {
        let bounds = egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(350.0, 700.0));
        let two = composer_rect(bounds, 44.0);
        let three = composer_rect(bounds, 66.0);
        assert_eq!(two.bottom(), three.bottom());
        assert_eq!(two.top() - three.top(), 22.0);
        assert_eq!(composer_rect(bounds, 2000.0).height(), 320.0);
    }
}
