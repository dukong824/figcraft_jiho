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
            .default_size(390.0)
            .min_size(310.0)
            .max_size(650.0)
            .resizable(true)
            .frame(egui::Frame::NONE.fill(style::SURFACE).stroke(egui::Stroke::new(1.0, style::BORDER)).inner_margin(egui::Margin {
                left: 18,
                right: 18,
                top: 6,
                bottom: 16,
            }))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(match view {
                            SideView::Codex => "Codex",
                            SideView::Trash => "휴지통",
                        })
                        .size(14.0)
                        .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        close = style::icon_button(ui, Icon::Close, 26.0, "사이드 보기 닫기").clicked();
                        if view == SideView::Codex {
                            ui.menu_button(RichText::new("⋯").size(18.0).color(style::MUTED), |ui| {
                                if ui.button("연결 다시 확인").clicked() {
                                    self.refresh_auth();
                                    ui.close();
                                }
                                if ui.button("실행 파일 선택…").clicked() {
                                    self.select_codex();
                                    ui.close();
                                }
                                if self.authenticated == Some(true) && ui.button("로그아웃").clicked() {
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
                    SideView::Trash => self.trash_contents(ui),
                }
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
        let text_height =
            ui.fonts_mut(|f| f.layout(self.prompt.clone(), egui::FontId::proportional(14.0), style::TEXT, (rect.width() - 26.0).max(80.0))).size().y;
        let composer_height = (text_height + 74.0).clamp(124.0, (rect.height() * 0.55).clamp(124.0, 320.0));
        let conversation =
            egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), (rect.bottom() - composer_height - 12.0).max(rect.top() + 40.0)));
        let composer = egui::Rect::from_min_max(egui::pos2(rect.left(), conversation.bottom() + 10.0), rect.max);
        let mut append = None;
        ui.scope_builder(egui::UiBuilder::new().max_rect(conversation), |ui| {
            egui::ScrollArea::vertical().id_salt(("codex-conversation", video_id)).auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
                ui.set_width(conversation.width() - 8.0);
                if messages.is_empty() {
                    ui.add_space(48.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("무엇을 함께 만들까요?").size(19.0).strong());
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
                                &message.text,
                                0.0,
                                egui::TextFormat {
                                    font_id: egui::FontId::proportional(14.0),
                                    color: style::TEXT,
                                    line_height: Some(21.0),
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
                                egui::FontId::proportional(10.0),
                                style::FAINT,
                            );
                        } else {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Codex").size(13.0).strong());
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
                                        .size(11.0)
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
                                    egui::CollapsingHeader::new(RichText::new(title).size(12.0).color(style::MUTED)).show(ui, |ui| {
                                        for activity in &message.activities {
                                            ui.label(
                                                RichText::new(format!("{}  {}", super::chat::elapsed(activity.elapsed_ms), activity.label))
                                                    .size(11.0)
                                                    .color(style::MUTED),
                                            );
                                            if !activity.detail.is_empty() {
                                                ui.add(egui::Label::new(RichText::new(&activity.detail).size(12.0)).selectable(true).wrap());
                                            }
                                            ui.add_space(6.0);
                                        }
                                        if let Some(usage) = &message.usage
                                            && let (Some(input), Some(output)) = (usage["input_tokens"].as_u64(), usage["output_tokens"].as_u64())
                                        {
                                            ui.label(RichText::new(format!("입력 {input} · 출력 {output} 토큰")).size(11.0).color(style::MUTED));
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
                            if running != Some(index) && !message.text.is_empty() {
                                ui.horizontal(|ui| {
                                    if ui.small_button("복사").clicked() {
                                        ui.ctx().copy_text(message.text.clone());
                                    }
                                    if !message.error && !message.cancelled && ui.small_button("대본에 추가").clicked() {
                                        append = Some(message.text.clone());
                                    }
                                });
                            }
                        }
                    });
                }
                ui.add_space(12.0);
            });
        });
        let mut send = false;
        ui.scope_builder(egui::UiBuilder::new().max_rect(composer), |ui| {
            egui::Frame::NONE.fill(style::SURFACE).stroke(egui::Stroke::new(1.0, style::BORDER)).corner_radius(14).inner_margin(12.0).show(
                ui,
                |ui| {
                    let id = ui.make_persistent_id(("codex-composer", video_id));
                    let enter = self.chat_job.is_none()
                        && ui.memory(|m| m.has_focus(id))
                        && ui.input_mut(|i| super::interactions::consume_key(i, egui::Modifiers::NONE, egui::Key::Enter));
                    egui::ScrollArea::vertical()
                        .id_salt(("codex-input-scroll", video_id))
                        .max_height(composer_height - 66.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.add(
                                TextEdit::multiline(&mut self.prompt)
                                    .id(id)
                                    .font(egui::FontId::proportional(14.0))
                                    .desired_width(ui.available_width())
                                    .desired_rows(2)
                                    .frame(egui::Frame::NONE)
                                    .hint_text("메시지 보내기…"),
                            );
                        });
                    ui.horizontal(|ui| {
                        self.codex_controls(ui);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if let Some(job) = &self.chat_job {
                                if job.video_id == video_id {
                                    if ui.button("중지 ■").clicked() {
                                        job.cancel();
                                    }
                                } else {
                                    ui.label(RichText::new("다른 영상에서 작업 중").size(11.0).color(style::MUTED));
                                }
                            } else {
                                let enabled = !self.prompt.trim().is_empty();
                                send = ui
                                    .add_enabled(
                                        enabled,
                                        egui::Button::new(RichText::new("↑").size(19.0).color(egui::Color32::WHITE))
                                            .fill(style::TEXT)
                                            .min_size(vec2(32.0, 30.0))
                                            .corner_radius(16),
                                    )
                                    .on_hover_text("보내기 · Enter / 줄바꿈 · Shift+Enter")
                                    .clicked()
                                    || enter && enabled;
                            }
                        });
                    });
                },
            );
        });
        if let Some(text) = append {
            if let Some(video) = self.store.data.selected_mut() {
                if !video.script_body.is_empty() {
                    video.script_body.push_str("\n\n");
                }
                video.script_body.push_str(&text);
            }
            self.save();
        }
        if send {
            self.ask_codex(self.prompt.clone());
        }
    }

    fn codex_sign_in(&mut self, ui: &mut egui::Ui) {
        let height = ui.available_height();
        ui.add_space((height * 0.26).max(20.0));
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("Codex").size(26.0).strong());
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
            ui.label(RichText::new(&self.auth_error).size(12.0).color(style::MUTED));
        }
        if !self.login_output.is_empty() {
            egui::ScrollArea::vertical().id_salt("codex-login-output").max_height(180.0).show(ui, |ui| {
                ui.label(RichText::new(&self.login_output).size(12.0));
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

    fn trash_contents(&mut self, ui: &mut egui::Ui) {
        let entries = self.store.data.trash.clone();
        let channels = self.store.data.trashed_channels.clone();
        let mut restore_channel = None;
        if entries.is_empty() && channels.is_empty() {
            ui.add_space(20.0);
            ui.label("휴지통이 비어 있습니다.");
        }
        let mut restore = None;
        egui::ScrollArea::vertical().id_salt("purpose-trash").show(ui, |ui| {
            for entry in channels.iter().rev() {
                ui.add_space(12.0);
                ui.label(RichText::new(&entry.channel.name).strong());
                ui.label(RichText::new(format!("채널 · 영상 {}개 · {}", entry.channel.videos.len(), entry.deleted_at)).small().color(style::MUTED));
                if ui.button("복원").clicked() {
                    restore_channel = Some(entry.channel.id);
                }
                ui.separator();
            }
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
        if let Some(id) = restore_channel {
            self.store.data.restore_channel(id);
            self.save();
        }
        if let Some(id) = restore {
            self.store.data.restore_video(id);
            self.save();
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
        let mut label_job = egui::text::LayoutJob::simple(
            format!("{label} ⌄"),
            egui::FontId::proportional(11.0),
            style::TEXT,
            (ui.available_width() - 154.0).max(64.0),
        );
        label_job.wrap.max_rows = 1;
        label_job.wrap.break_anywhere = true;
        ui.menu_button(label_job, |ui| {
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
        ui.menu_button(RichText::new(super::codex_options::effort_label(&before.effort)).size(11.0), |ui| {
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
        if ui.add_enabled_ui(fast, |ui| style::icon_toggle(ui, Icon::Fast, before.fast, 26.0, tip)).inner.clicked() {
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
