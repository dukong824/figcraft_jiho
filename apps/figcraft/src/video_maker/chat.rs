//! Persistent per-video conversations and the CLI's public JSON event stream.
use super::{VideoMaker, codex};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    time::{Duration, Instant},
};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) enum Role {
    User,
    Assistant,
}
#[derive(Clone, Serialize, Deserialize, Default)]
pub(super) struct Activity {
    pub label: String,
    pub detail: String,
    pub elapsed_ms: u64,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(super) struct Attachment {
    pub video: u64,
    pub id: u64,
    pub name: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Message {
    pub role: Role,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    pub text: String,
    pub at: String,
    #[serde(default)]
    pub elapsed_ms: u64,
    #[serde(default)]
    pub activities: Vec<Activity>,
    #[serde(default)]
    pub error: bool,
    #[serde(default)]
    pub cancelled: bool,
    #[serde(default)]
    pub usage: Option<Value>,
}
#[derive(Default, Serialize, Deserialize)]
pub(super) struct Conversation {
    pub messages: Vec<Message>,
}
pub(super) enum Event {
    Activity(String, String),
    Text(String),
    Usage(Value),
    Failure(String),
    Finished(Result<String, String>, bool),
}
pub(super) struct Job {
    pub video_id: u64,
    pub reply: usize,
    pub started: Instant,
    pub events: Receiver<Event>,
    cancel: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Job {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub(super) fn elapsed(ms: u64) -> String {
    let s = ms / 1000;
    if s >= 60 { format!("{}분 {:02}초", s / 60, s % 60) } else { format!("{s}초") }
}

pub(super) fn decode_event(value: &Value) -> Option<Event> {
    let kind = value["type"].as_str()?;
    if kind == "turn.started" {
        return Some(Event::Activity("요청 처리 중".into(), String::new()));
    }
    if kind == "turn.completed" {
        return Some(Event::Usage(value["usage"].clone()));
    }
    if kind == "error" || kind == "turn.failed" {
        return Some(Event::Failure(value["message"].as_str().or_else(|| value["error"]["message"].as_str()).unwrap_or("요청 처리 실패").into()));
    }
    if !matches!(kind, "item.started" | "item.updated" | "item.completed") {
        return None;
    }
    let item = &value["item"];
    let complete = kind == "item.completed";
    let (label, detail) = match item["type"].as_str()? {
        "agent_message" => return Some(Event::Text(item["text"].as_str()?.chars().take(200_000).collect())),
        "reasoning" => ("계획 정리", item["text"].as_str().unwrap_or_default().to_owned()),
        "command_execution" => (if complete { "명령 실행 완료" } else { "명령 실행 중" }, item["command"].as_str().unwrap_or_default().to_owned()),
        "mcp_tool_call" => (
            if complete { "도구 실행 완료" } else { "도구 사용 중" },
            format!("{} · {}", item["server"].as_str().unwrap_or_default(), item["tool"].as_str().unwrap_or_default()),
        ),
        "web_search" => ("자료 검색", item["query"].as_str().unwrap_or_default().to_owned()),
        "file_change" => (
            "파일 변경",
            item["changes"].as_array().map(|a| a.iter().filter_map(|v| v["path"].as_str()).collect::<Vec<_>>().join("\n")).unwrap_or_default(),
        ),
        "todo_list" => (
            "작업 목록",
            item["items"].as_array().map(|a| a.iter().filter_map(|v| v["text"].as_str()).collect::<Vec<_>>().join("\n")).unwrap_or_default(),
        ),
        _ => return None,
    };
    Some(Event::Activity(label.into(), detail.chars().take(4000).collect()))
}

struct RunOptions {
    settings: super::codex_options::Settings,
    fast_tier: Option<String>,
    connection: super::agent_transport::Connection,
    images: Vec<PathBuf>,
}
fn run(cli: PathBuf, root: PathBuf, prompt: String, tx: mpsc::Sender<Event>, cancel: Arc<AtomicBool>, options: RunOptions) -> Result<String, String> {
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let output = root.join(format!(
        ".codex-response-{}.txt",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos()
    ));
    let mut command = codex::command(&cli);
    command
        .env_remove("OPENAI_API_KEY")
        .env_remove("CODEX_API_KEY")
        .args([
            "-c",
            "forced_login_method=\"chatgpt\"",
            "-c",
            "model_provider=\"openai\"",
            "-c",
            "approval_policy=\"never\"",
            "exec",
            "--json",
            "--sandbox",
            "read-only",
            "--ephemeral",
            "--skip-git-repo-check",
            "--output-last-message",
        ])
        .arg(&output)
        .arg("-C")
        .arg(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    options.settings.configure(&mut command, options.fast_tier.as_deref());
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    options.connection.configure(&mut command, &executable);
    for image in options.images {
        command.arg("--image").arg(image);
    }
    command.arg("-");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(|e| format!("Codex를 실행할 수 없습니다: {e}"))?;
    if let Some(mut input) = child.stdin.take() {
        let text = prompt;
        std::thread::spawn(move || {
            let _ = input.write_all(text.as_bytes());
        });
    }
    let events = tx.clone();
    let reader = child.stdout.take().map(|out| {
        std::thread::spawn(move || {
            for line in BufReader::new(out).lines() {
                let Ok(line) = line else { break };
                if let Ok(value) = serde_json::from_str::<Value>(&line)
                    && let Some(event) = decode_event(&value)
                {
                    let _ = events.send(event);
                }
            }
        })
    });
    let errors = child.stderr.take().map(|pipe| {
        std::thread::spawn(move || {
            let mut errors = String::new();
            for line in BufReader::new(pipe).lines() {
                let Ok(line) = line else { break };
                if errors.len() < 16_000 {
                    errors.push_str(&line);
                    errors.push('\n');
                }
            }
            errors
        })
    });
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e.to_string());
            }
        }
        if cancel.load(Ordering::Relaxed) {
            #[cfg(windows)]
            {
                let _ = codex::command(std::path::Path::new("taskkill.exe")).args(["/PID", &child.id().to_string(), "/T", "/F"]).status();
            }
            #[cfg(unix)]
            {
                let _ = std::process::Command::new("kill").args(["-TERM", "--", &format!("-{}", child.id())]).status();
            }
            let _ = child.kill();
            let _ = child.wait();
            let _ = fs::remove_file(output);
            return Err("작업을 중지했습니다".into());
        }
        std::thread::sleep(Duration::from_millis(80));
    };
    if let Some(reader) = reader {
        let _ = reader.join();
    }
    let stderr = errors.and_then(|thread| thread.join().ok()).unwrap_or_default();
    let answer = fs::read_to_string(&output).unwrap_or_default();
    let _ = fs::remove_file(output);
    if !status.success() {
        return Err(if stderr.trim().is_empty() { "Codex 요청이 실패했습니다".into() } else { stderr.chars().take(4000).collect() });
    }
    Ok(answer)
}

impl VideoMaker {
    pub(super) fn conversation(&mut self, id: u64) -> &mut Conversation {
        let path = self.store.video_dir(id).join("codex-chat.json");
        self.chats.entry(id).or_insert_with(|| {
            let mut chat: Conversation = fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
            if let Some(last) = chat.messages.last_mut()
                && last.role == Role::Assistant
                && last.text.is_empty()
            {
                last.text = "이전 작업이 완료되기 전에 앱이 종료되었습니다".into();
                last.error = true;
            }
            chat
        })
    }
    pub(super) fn save_chat(&mut self, id: u64) {
        if let Some(chat) = self.chats.get(&id)
            && let Err(error) = super::model::write_json(&self.store.video_dir(id).join("codex-chat.json"), chat)
        {
            self.message = format!("대화 저장 실패: {error}");
        }
    }
    pub(super) fn start_chat(&mut self, request: String, context: String) {
        let Some(video) = self.video_id() else { return };
        let Some(cli) = codex::executable(self.store.data.codex_path.as_deref()) else {
            self.authenticated = Some(false);
            self.auth_error = "Codex 실행 파일을 찾을 수 없습니다".into();
            return;
        };
        if self.chat_job.is_some() || self.authenticated != Some(true) || request.trim().is_empty() {
            return;
        }
        let attachments = self.chat_attachments.get(&video).cloned().unwrap_or_default();
        let (attachment_context, images) = match self.attachment_context(&attachments) {
            Ok(value) => value,
            Err(error) => {
                self.message = error;
                return;
            }
        };
        let bridge = match super::agent_transport::Bridge::start(video, self.ui_context.clone().unwrap_or_default()) {
            Ok(bridge) => bridge,
            Err(error) => {
                self.message = format!("앱 도구 연결 실패: {error}");
                return;
            }
        };
        let connection = bridge.connection.clone();
        self.agent = Some(bridge);
        let settings = self.store.data.codex_settings.clone();
        let fast_tier = self.selected_model().and_then(|m| m.fast_tier()).map(str::to_owned);
        let chat = self.conversation(video);
        let history = chat
            .messages
            .iter()
            .rev()
            .take(12)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|m| {
                format!(
                    "{}: {}\n첨부: {}",
                    if m.role == Role::User { "사용자" } else { "Codex" },
                    m.text.chars().take(12000).collect::<String>(),
                    m.attachments.iter().map(|a| format!("{} (영상 {}, 자료 {})", a.name, a.video, a.id)).collect::<Vec<_>>().join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        let now = chrono::Local::now().format("%H:%M").to_string();
        chat.messages.push(Message {
            role: Role::User,
            attachments,
            text: request.clone(),
            at: now.clone(),
            elapsed_ms: 0,
            activities: vec![],
            error: false,
            cancelled: false,
            usage: None,
        });
        let reply = chat.messages.len();
        chat.messages.push(Message {
            role: Role::Assistant,
            attachments: Vec::new(),
            text: String::new(),
            at: now,
            elapsed_ms: 0,
            activities: vec![],
            error: false,
            cancelled: false,
            usage: None,
        });
        self.save_chat(video);
        let guide = self.guide_text(video).unwrap_or_default();
        let prompt = format!(
            "{context}\n\n작업 전에 반드시 읽고 준수할 가이드:\n{guide}\n이전 대화:\n{history}\n사용자 요청:\n{request}\n첨부한 라이브러리 소스:{attachment_context}"
        );
        let (tx, events) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let stop = cancel.clone();
        let root = self.store.video_dir(video);
        let worker = std::thread::spawn(move || {
            let result = run(cli, root, prompt, tx.clone(), stop.clone(), RunOptions { settings, fast_tier, connection, images });
            let _ = tx.send(Event::Finished(result, stop.load(Ordering::Relaxed)));
        });
        self.chat_job = Some(Job { video_id: video, reply, started: Instant::now(), events, cancel, worker: Some(worker) });
        self.prompt.clear();
        self.chat_attachments.remove(&video);
    }
    pub fn shutdown_chat(&mut self) {
        self.agent = None;
        let Some(job) = self.chat_job.take() else { return };
        let id = job.video_id;
        let reply = job.reply;
        let elapsed = job.started.elapsed().as_millis().min(u64::MAX as u128) as u64;
        drop(job);
        if let Some(message) = self.chats.get_mut(&id).and_then(|c| c.messages.get_mut(reply)) {
            message.elapsed_ms = elapsed;
            message.cancelled = true;
            if !message.text.is_empty() {
                message.text.push_str("\n\n");
            }
            message.text.push_str("앱을 닫아 작업을 중지했습니다");
        }
        self.save_chat(id);
    }
    pub(super) fn poll_chat(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.chat_job else { return };
        let id = job.video_id;
        let index = job.reply;
        let ms = job.started.elapsed().as_millis().min(u64::MAX as u128) as u64;
        let mut events = Vec::new();
        loop {
            match job.events.try_recv() {
                Ok(event) => events.push(event),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    if !events.iter().any(|e| matches!(e, Event::Finished(..))) {
                        events.push(Event::Finished(Err("Codex 연결이 종료되었습니다".into()), false));
                    }
                    break;
                }
            }
        }
        let Some(reply) = self.chats.get_mut(&id).and_then(|c| c.messages.get_mut(index)) else { return };
        reply.elapsed_ms = ms;
        let mut finished = false;
        for event in events {
            match event {
                Event::Activity(label, detail) => {
                    if reply.activities.last().is_none_or(|a| a.label != label || a.detail != detail) {
                        reply.activities.push(Activity { label, detail, elapsed_ms: ms });
                    }
                    if reply.activities.len() > 100 {
                        reply.activities.remove(0);
                    }
                }
                Event::Text(text) => {
                    reply.text = text;
                    reply.error = false;
                }
                Event::Failure(error) => {
                    reply.text = error;
                    reply.error = true;
                }
                Event::Usage(usage) => reply.usage = Some(usage),
                Event::Finished(result, cancelled) => {
                    finished = true;
                    reply.cancelled = cancelled;
                    match result {
                        Ok(text) if !text.trim().is_empty() => {
                            reply.text = text;
                            reply.error = false;
                        }
                        Ok(_) if !reply.text.is_empty() => {}
                        Ok(_) => {
                            reply.error = true;
                            reply.text = "Codex가 빈 응답을 보냈습니다".into();
                        }
                        Err(error) => {
                            reply.error = !cancelled;
                            if !reply.text.is_empty() {
                                reply.text.push_str("\n\n");
                            }
                            reply.text.push_str(&error);
                        }
                    }
                }
            }
        }
        if finished {
            self.agent = None;
            self.chat_job = None;
            self.save_chat(id);
        } else {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}

/// Render the CLI's Markdown using egui text, without web content or remote image fetching.
pub(super) fn markdown(text: &str, width: f32, error: bool) -> egui::text::LayoutJob {
    use pulldown_cmark::{Event as Md, HeadingLevel, Tag, TagEnd};
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = width;
    let mut strong = false;
    let mut italic = false;
    let mut code = false;
    let mut heading = 0;
    let mut lists: Vec<Option<u64>> = Vec::new();
    for event in pulldown_cmark::Parser::new(text) {
        let mut content = String::new();
        let mut inline_code = false;
        match event {
            Md::Start(Tag::Strong) => strong = true,
            Md::End(TagEnd::Strong) => strong = false,
            Md::Start(Tag::Emphasis) => italic = true,
            Md::End(TagEnd::Emphasis) => italic = false,
            Md::Start(Tag::Heading { level, .. }) => {
                heading = match level {
                    HeadingLevel::H1 => 22,
                    HeadingLevel::H2 => 19,
                    _ => 16,
                }
            }
            Md::End(TagEnd::Heading(_)) => {
                heading = 0;
                content.push_str("\n\n");
            }
            Md::Start(Tag::CodeBlock(_)) => code = true,
            Md::End(TagEnd::CodeBlock) => {
                code = false;
                content.push('\n');
            }
            Md::Start(Tag::List(start)) => lists.push(start),
            Md::End(TagEnd::List(_)) => {
                lists.pop();
                content.push('\n');
            }
            Md::Start(Tag::Item) => {
                content.push_str(&"    ".repeat(lists.len().saturating_sub(1)));
                if let Some(Some(number)) = lists.last_mut() {
                    content.push_str(&format!("{number}. "));
                    *number += 1;
                } else {
                    content.push_str("• ");
                }
            }
            Md::End(TagEnd::Item) => content.push('\n'),
            Md::End(TagEnd::Paragraph) => content.push_str(if lists.is_empty() { "\n\n" } else { "\n" }),
            Md::Text(value) | Md::Html(value) | Md::InlineHtml(value) => content.push_str(&value),
            Md::Code(value) => {
                inline_code = true;
                content.push_str(&value);
            }
            Md::SoftBreak => content.push('\n'),
            Md::HardBreak => content.push('\n'),
            Md::Rule => content.push_str("────────\n"),
            _ => {}
        }
        if !content.is_empty() {
            let mono = code || inline_code;
            let size = if heading > 0 { heading as f32 } else { 14.0 };
            let format = egui::TextFormat {
                font_id: if mono {
                    egui::FontId::monospace(figcraft_ui_icons::font_size(13.0))
                } else if strong || heading > 0 {
                    egui::FontId::new(size, egui::FontFamily::Name("semibold".into()))
                } else {
                    egui::FontId::proportional(size)
                },
                italics: italic,
                color: if error { egui::Color32::from_rgb(180, 50, 50) } else { super::style::TEXT },
                background: if mono { super::style::SIDEBAR } else { egui::Color32::TRANSPARENT },
                line_height: Some(size * 1.5),
                ..Default::default()
            };
            job.append(&content, 0.0, format);
        }
    }
    let end = job.text.trim_end_matches('\n').len();
    job.text.truncate(end);
    job.sections.retain(|section| section.byte_range.start.0 < end);
    for section in &mut job.sections {
        section.byte_range.end = section.byte_range.end.min(end.into());
    }
    job
}

pub(super) fn links(text: &str) -> Vec<(String, String)> {
    use pulldown_cmark::{Event as Md, Tag, TagEnd};
    let mut result = Vec::new();
    let mut active: Option<(String, String)> = None;
    for event in pulldown_cmark::Parser::new(text) {
        match event {
            Md::Start(Tag::Link { dest_url, .. }) | Md::Start(Tag::Image { dest_url, .. }) => active = Some((String::new(), dest_url.to_string())),
            Md::Text(text) | Md::Code(text) => {
                if let Some((label, _)) = &mut active {
                    label.push_str(&text);
                }
            }
            Md::End(TagEnd::Link | TagEnd::Image) => {
                if let Some((label, url)) = active.take()
                    && (url.starts_with("https://") || url.starts_with("http://"))
                    && !result.iter().any(|(_, u)| u == &url)
                {
                    result.push((if label.is_empty() { url.clone() } else { label }, url));
                }
            }
            _ => {}
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn markdown_keeps_korean_line_breaks_and_removes_format_markers() {
        let job = markdown("**도입**\n첫 문장\n둘째 문장", 300.0, false);
        assert_eq!(job.text, "도입\n첫 문장\n둘째 문장");
    }
    #[test]
    fn cli_json_stream_keeps_public_activity_text_and_real_usage() {
        let event = decode_event(&json!({"type":"item.started","item":{"type":"command_execution","command":"rg intro script.txt"}})).unwrap();
        assert!(matches!(event,Event::Activity(label,detail) if label.contains("실행 중")&&detail=="rg intro script.txt"));
        assert!(
            matches!(decode_event(&json!({"type":"item.completed","item":{"type":"agent_message","text":"답변"}})),Some(Event::Text(t)) if t=="답변")
        );
        assert!(
            matches!(decode_event(&json!({"type":"turn.completed","usage":{"input_tokens":123,"output_tokens":45}})),Some(Event::Usage(u)) if u["output_tokens"]==45)
        );
        assert!(decode_event(&json!({"type":"future.event"})).is_none());
        assert_eq!(elapsed(125_000), "2분 05초");
    }
    #[cfg(unix)]
    #[test]
    fn cancelling_a_request_stops_the_cli_and_keeps_the_user_message() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("purpose-chat-stop-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let cli = root.join("codex");
        fs::write(&cli, "#!/bin/sh\necho '{\"type\":\"turn.started\"}'\nsleep 20\n").unwrap();
        fs::set_permissions(&cli, fs::Permissions::from_mode(0o700)).unwrap();
        let mut maker = VideoMaker::at(root.join("project")).unwrap();
        maker.store.data.codex_path = Some(cli);
        maker.authenticated = Some(true);
        maker.ask_codex("중지 테스트".into());
        let id = maker.video_id().unwrap();
        let ctx = egui::Context::default();
        std::thread::sleep(Duration::from_millis(150));
        maker.poll_chat(&ctx);
        assert!(!maker.conversation(id).messages[1].activities.is_empty());
        maker.chat_job.as_ref().unwrap().cancel();
        for _ in 0..100 {
            maker.poll_chat(&ctx);
            if maker.chat_job.is_none() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(maker.chat_job.is_none());
        assert_eq!(maker.conversation(id).messages[0].text, "중지 테스트");
        assert!(maker.conversation(id).messages[1].text.contains("중지"));
        let _ = fs::remove_dir_all(root);
    }
}
