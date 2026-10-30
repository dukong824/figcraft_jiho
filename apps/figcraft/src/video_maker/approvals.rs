//! Server-initiated Codex approvals. Only an explicit dialog decision sends a response.
use super::{VideoMaker, style};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::mpsc};
pub(super) struct Request {
    pub id: Value,
    pub method: String,
    pub params: Value,
    pub response: mpsc::Sender<Value>,
    pub answers: BTreeMap<String, String>,
}
impl Request {
    pub fn new(id: Value, method: String, params: Value, response: mpsc::Sender<Value>) -> Self {
        Self { id, method, params, response, answers: BTreeMap::new() }
    }
    fn permissions(&self) -> bool {
        self.method == "item/permissions/requestApproval"
    }
    fn questions(&self) -> bool {
        self.method == "item/tool/requestUserInput"
    }
    fn elicitation(&self) -> bool {
        self.method == "mcpServer/elicitation/request"
    }
    fn form(&self) -> bool {
        self.elicitation() && self.params["mode"] != "url"
    }
    pub fn can_allow(&self) -> bool {
        if self.method == "item/fileChange/requestApproval" {
            return false;
        }
        if self.permissions() {
            let fs = &self.params["permissions"]["fileSystem"];
            return fs["write"].as_array().is_none_or(Vec::is_empty)
                && fs["entries"].as_array().is_none_or(|a| a.iter().all(|e| matches!(e["access"].as_str(), Some("read" | "deny"))));
        }
        if self.elicitation() && self.params["mode"] == "url" {
            return self.params["url"]
                .as_str()
                .and_then(|u| url::Url::parse(u).ok())
                .is_some_and(|u| matches!(u.scheme(), "https" | "http") && u.host_str().is_some());
        }
        if self.form() {
            return self.params["requestedSchema"]["properties"].as_object().is_some_and(|a| {
                a.len() <= 8 && a.values().all(|s| matches!(s["type"].as_str(), Some("string" | "boolean" | "integer" | "number")))
            });
        }
        true
    }
    pub fn result(&self, allow: bool) -> Value {
        let allow = allow && self.can_allow();
        if self.permissions() {
            return if allow { json!({"permissions":self.params["permissions"],"scope":"turn"}) } else { json!({"permissions":{},"scope":"turn"}) };
        }
        if self.questions() {
            let answers: serde_json::Map<String, Value> =
                self.answers.iter().map(|(id, v)| (id.clone(), json!({"answers":if allow{vec![v.clone()]}else{vec![]}}))).collect();
            return json!({"answers":answers});
        }
        if self.elicitation() {
            let mut content = serde_json::Map::new();
            if allow
                && self.form()
                && let Some(props) = self.params["requestedSchema"]["properties"].as_object()
            {
                for (id, schema) in props {
                    let text = self.answers.get(id).cloned().unwrap_or_default();
                    let value = match schema["type"].as_str() {
                        Some("boolean") => json!(text == "true"),
                        Some("integer") => text.parse::<i64>().map_or(Value::Null, |v| json!(v)),
                        Some("number") => text.parse::<f64>().ok().filter(|v| v.is_finite()).map_or(Value::Null, |v| json!(v)),
                        _ => json!(text),
                    };
                    content.insert(id.clone(), value);
                }
            }
            return json!({"action":if allow{"accept"}else{"decline"},"content":if allow&&self.form(){Value::Object(content)}else{Value::Null}});
        }
        json!({"decision":if allow{"accept"}else{"decline"}})
    }
    fn ready(&self) -> bool {
        if !self.can_allow() {
            return false;
        }
        if self.questions() {
            return self.params["questions"]
                .as_array()
                .is_some_and(|q| q.iter().all(|v| self.answers.get(v["id"].as_str().unwrap_or("")).is_some_and(|s| !s.trim().is_empty())));
        }
        if self.form()
            && let Some(props) = self.params["requestedSchema"]["properties"].as_object()
        {
            for (id, schema) in props {
                let text = self.answers.get(id).map(String::as_str).unwrap_or("");
                let required = self.params["requestedSchema"]["required"].as_array().is_some_and(|a| a.iter().any(|s| s == id));
                if required && schema["type"] != "boolean" && text.trim().is_empty() {
                    return false;
                }
                if !text.is_empty() && schema["type"] == "integer" && text.parse::<i64>().is_err() {
                    return false;
                }
                if !text.is_empty() && schema["type"] == "number" && text.parse::<f64>().ok().filter(|v| v.is_finite()).is_none() {
                    return false;
                }
            }
        }
        true
    }
}
impl VideoMaker {
    pub(super) fn approval_dialog(&mut self, ctx: &egui::Context) {
        if self.chat_job.as_ref().is_none_or(|j| j.cancelled()) {
            self.approvals.clear();
            return;
        }
        let context = self
            .chat_job
            .as_ref()
            .and_then(|j| {
                self.store.data.channels.iter().find_map(|c| c.videos.iter().find(|v| v.id == j.video_id).map(|v| format!("{} / {}", c.name, v.name)))
            })
            .unwrap_or_default();
        let count = self.approvals.len();
        let Some(request) = self.approvals.first_mut() else {
            return;
        };
        let mut decision = None;
        let modal = egui::Modal::new(egui::Id::unique("purpose-codex-approval")).frame(style::modal_frame()).show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 80.0).clamp(300.0, 520.0));
            ui.label(style::heading(if request.questions() { "Codex의 확인 요청" } else { "Codex 작업을 승인할까요?" }, 22.0));
            ui.add_space(6.0);
            ui.add(egui::Label::new(egui::RichText::new(&context).small().color(style::MUTED)).truncate()).on_hover_text(context);
            if count > 1 {
                ui.label(egui::RichText::new(format!("대기 중인 요청 {count}개")).small().color(style::MUTED));
            }
            ui.add_space(14.0);
            egui::ScrollArea::vertical().id_salt("approval-details").max_height((ctx.content_rect().height() - 270.0).clamp(120.0, 400.0)).show(
                ui,
                |ui| {
                    if let Some(reason) = request.params["reason"].as_str().or_else(|| request.params["message"].as_str()) {
                        ui.label(reason);
                        ui.add_space(12.0);
                    }
                    if let Some(command) = request.params["command"].as_str() {
                        egui::Frame::NONE.fill(style::SIDEBAR).corner_radius(6).inner_margin(12.0).show(ui, |ui| {
                            ui.add(egui::Label::new(egui::RichText::new(command).monospace()).wrap().selectable(true));
                        });
                    }
                    if request.permissions() {
                        let profile = &request.params["permissions"];
                        if profile["network"]["enabled"] == true {
                            ui.label(egui::RichText::new("인터넷 연결 허용").strong());
                        }
                        let filesystem = &profile["fileSystem"];
                        if let Some(read) = filesystem["read"].as_array() {
                            for path in read.iter().take(30) {
                                ui.add(egui::Label::new(format!("파일 읽기 · {}", path.as_str().unwrap_or(""))).wrap().selectable(true));
                            }
                        }
                        if let Some(entries) = filesystem["entries"].as_array() {
                            for entry in entries.iter().take(30) {
                                let path = entry["path"]["path"].as_str().unwrap_or("요청한 경로");
                                ui.add(
                                    egui::Label::new(format!("{} · {path}", if entry["access"] == "read" { "파일 읽기" } else { "접근 제한" }))
                                        .wrap()
                                        .selectable(true),
                                );
                            }
                        }
                        if filesystem["write"].as_array().is_some_and(|v| !v.is_empty()) {
                            ui.label("프로젝트 파일 직접 쓰기 요청");
                        }
                    }
                    if let Some(network) = request.params.get("networkApprovalContext").filter(|v| !v.is_null()) {
                        ui.label(format!("네트워크: {network}"));
                    }
                    if let Some(cwd) = request.params["cwd"].as_str() {
                        ui.add_space(8.0);
                        ui.add(egui::Label::new(egui::RichText::new(format!("작업 위치 · {cwd}")).small().color(style::MUTED)).wrap());
                    }
                    if request.elicitation()
                        && request.params["mode"] == "url"
                        && let Some(url) = request.params["url"].as_str()
                    {
                        ui.add(egui::Label::new(url).wrap().selectable(true));
                    }
                    if request.questions()
                        && let Some(questions) = request.params["questions"].as_array()
                    {
                        for q in questions.iter().take(8) {
                            let id = q["id"].as_str().unwrap_or("");
                            ui.label(q["question"].as_str().unwrap_or("확인 내용"));
                            let answer = request.answers.entry(id.into()).or_default();
                            if let Some(options) = q["options"].as_array() {
                                for option in options {
                                    let label = option["label"].as_str().unwrap_or("");
                                    ui.radio_value(answer, label.to_owned(), label);
                                }
                            }
                            ui.add(
                                egui::TextEdit::singleline(answer).password(q["isSecret"].as_bool().unwrap_or(false)).desired_width(f32::INFINITY),
                            );
                            ui.add_space(10.0);
                        }
                    }
                    if request.form()
                        && let Some(props) = request.params["requestedSchema"]["properties"].as_object()
                    {
                        for (id, schema) in props.iter().take(8) {
                            ui.label(schema["title"].as_str().unwrap_or(id));
                            let value = request.answers.entry(id.clone()).or_default();
                            if schema["type"] == "boolean" {
                                let mut checked = value == "true";
                                if ui.checkbox(&mut checked, schema["description"].as_str().unwrap_or("허용")).changed() {
                                    *value = checked.to_string();
                                }
                            } else if let Some(options) = schema["enum"].as_array() {
                                for option in options.iter().filter_map(Value::as_str) {
                                    ui.radio_value(value, option.to_owned(), option);
                                }
                            } else {
                                ui.add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY));
                            }
                            ui.add_space(10.0);
                        }
                    }
                    if !request.can_allow() {
                        ui.label(
                            egui::RichText::new(
                                "프로젝트 파일의 직접 쓰기 권한은 제공하지 않습니다. 디자인·편집·라이브러리는 앱 도구를 통해 수정합니다.",
                            )
                            .small()
                            .color(style::MUTED),
                        );
                    }
                },
            );
            ui.add_space(20.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled_ui(request.ready(), |ui| {
                        style::action_button(ui, if request.questions() { "답변 보내기" } else { "이번 요청 승인" }, true)
                    })
                    .inner
                    .clicked()
                {
                    decision = Some(true);
                }
                if style::action_button(ui, "거절", false).clicked() {
                    decision = Some(false);
                }
            });
        });
        if modal.should_close() {
            decision = Some(false);
        }
        if let Some(allow) = decision {
            let request = self.approvals.remove(0);
            if allow
                && request.can_allow()
                && request.elicitation()
                && request.params["mode"] == "url"
                && let Some(url) = request.params["url"].as_str()
            {
                let _ = webbrowser::open(url);
            }
            let _ = request.response.send(request.result(allow));
            style::release_focus(ctx);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn network_grants_are_explicit_and_project_writes_are_not_granted() {
        let (tx, rx) = mpsc::channel();
        let r = Request::new(json!(7), "item/permissions/requestApproval".into(), json!({"permissions":{"network":{"enabled":true}}}), tx);
        assert!(rx.try_recv().is_err());
        assert_eq!(r.result(false)["permissions"], json!({}));
        assert_eq!(r.result(true)["scope"], "turn");
        let (tx, _) = mpsc::channel();
        let r = Request::new(json!(8), "item/permissions/requestApproval".into(), json!({"permissions":{"fileSystem":{"write":["/project"]}}}), tx);
        assert!(!r.can_allow());
        assert_eq!(r.result(true)["permissions"], json!({}));
    }
}
