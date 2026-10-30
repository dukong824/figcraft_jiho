//! Official app-server JSON-RPC transport. The UI remains responsive while server approvals wait.
use super::{
    approvals::Request,
    chat::{Event, RunOptions},
    codex,
};
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    process::{Child, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
const LINE_LIMIT: u64 = 16 * 1024 * 1024;
fn send(input: &mut impl Write, value: Value) -> Result<(), String> {
    serde_json::to_writer(&mut *input, &value).map_err(|e| e.to_string())?;
    input.write_all(b"\n").and_then(|_| input.flush()).map_err(|e| e.to_string())
}
fn policy() -> Value {
    json!({"granular":{"rules":true,"sandbox_approval":false,"mcp_elicitations":true,"request_permissions":true}})
}
fn stop(child: &mut Child) {
    #[cfg(windows)]
    {
        let _ = codex::command(std::path::Path::new("taskkill.exe")).args(["/PID", &child.id().to_string(), "/T", "/F"]).status();
    }
    #[cfg(unix)]
    {
        let _ = std::process::Command::new("kill").args(["-TERM", "--", &format!("-{}", child.id())]).stderr(Stdio::null()).status();
    }
    let _ = child.kill();
    let _ = child.wait();
}
fn supported(method: &str) -> bool {
    matches!(
        method,
        "item/commandExecution/requestApproval"
            | "item/fileChange/requestApproval"
            | "item/permissions/requestApproval"
            | "mcpServer/elicitation/request"
            | "item/tool/requestUserInput"
    )
}
struct Pending {
    id: Value,
    response: mpsc::Receiver<Value>,
    denied: Value,
}
#[derive(Default)]
struct Messages {
    entries: Vec<(String, String)>,
}
impl Messages {
    fn update(&mut self, id: &str, text: &str, delta: bool) {
        if self.entries.len() >= 64 && !self.entries.iter().any(|(i, _)| i == id) {
            return;
        }
        let index = if let Some(i) = self.entries.iter().position(|(i, _)| i == id) {
            i
        } else {
            self.entries.push((id.into(), String::new()));
            self.entries.len() - 1
        };
        if let Some((_, value)) = self.entries.get_mut(index) {
            if !delta {
                value.clear();
            }
            if value.len() < 800_000 {
                value.extend(text.chars().take(200_000 - value.chars().count().min(200_000)));
            }
        }
    }
    fn text(&self) -> String {
        self.entries.iter().filter(|(_, v)| !v.is_empty()).map(|(_, v)| v.as_str()).collect::<Vec<_>>().join("\n\n").chars().take(200_000).collect()
    }
}
pub(super) fn run(
    cli: PathBuf,
    root: PathBuf,
    prompt: String,
    tx: mpsc::Sender<Event>,
    cancel: Arc<AtomicBool>,
    options: RunOptions,
) -> Result<String, String> {
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let mut command = codex::command(&cli);
    command.env_remove("OPENAI_API_KEY").env_remove("CODEX_API_KEY").args([
        "-c",
        "forced_login_method=\"chatgpt\"",
        "-c",
        "model_provider=\"openai\"",
        "-c",
        "features.request_permissions_tool=true",
        "-c",
        "features.default_mode_request_user_input=true",
    ]);
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    options.connection.configure(&mut command, &executable);
    command.args(["app-server", "--listen", "stdio://"]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(|e| format!("Codex를 실행할 수 없습니다: {e}"))?;
    let result = (|| {
        let mut input = child.stdin.take().ok_or("Codex 입력을 열 수 없습니다")?;
        let output = child.stdout.take().ok_or("Codex 출력을 열 수 없습니다")?;
        let errors = child.stderr.take();
        let (err_tx, err_rx) = mpsc::channel();
        let error_reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(mut pipe) = errors {
                let mut chunk = [0u8; 4096];
                loop {
                    match pipe.read(&mut chunk) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let keep = n.min(16000usize.saturating_sub(bytes.len()));
                            bytes.extend_from_slice(&chunk[..keep]);
                        }
                    }
                }
            }
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let _ = err_tx.send(text);
        });
        let (lines, rx) = mpsc::sync_channel(64);
        let reader = std::thread::spawn(move || {
            let mut reader = BufReader::new(output);
            loop {
                let mut line = String::new();
                match reader.by_ref().take(LINE_LIMIT + 1).read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) if line.len() as u64 <= LINE_LIMIT => {
                        if let Ok(v) = serde_json::from_str::<Value>(&line)
                            && lines.send(v).is_err()
                        {
                            break;
                        }
                    }
                    _ => break,
                }
            }
        });
        let result = drive(&mut input, &rx, &root, prompt, &tx, &cancel, options);
        drop(input);
        stop(&mut child);
        drop(rx);
        let _ = reader.join();
        let _ = error_reader.join();
        match result {
            Err(e) if !cancel.load(Ordering::Relaxed) => {
                let detail = err_rx.try_recv().unwrap_or_default();
                Err(if detail.trim().is_empty() { e } else { format!("{e}\n{}", detail.chars().take(3000).collect::<String>()) })
            }
            other => other,
        }
    })();
    // Also covers setup errors before the I/O workers were created.
    if child.try_wait().ok().flatten().is_none() {
        stop(&mut child);
    }
    result
}
fn drive(
    input: &mut impl Write,
    rx: &mpsc::Receiver<Value>,
    root: &std::path::Path,
    prompt: String,
    tx: &mpsc::Sender<Event>,
    cancel: &AtomicBool,
    options: RunOptions,
) -> Result<String, String> {
    send(
        input,
        json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"purpose_video_maker","version":"0.1.12"},"capabilities":{"experimentalApi":true}}}),
    )?;
    let mut phase = 1;
    let mut phase_at = Instant::now();
    let mut thread = String::new();
    let mut pending: Vec<Pending> = Vec::new();
    let mut messages = Messages::default();
    let mut content = vec![json!({"type":"text","text":prompt})];
    content.extend(options.images.iter().map(|p| json!({"type":"localImage","path":p})));
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("작업을 중지했습니다".into());
        }
        if phase < 4 && phase_at.elapsed() > Duration::from_secs(30) {
            return Err("Codex 연결 준비 시간이 초과되었습니다".into());
        }
        let mut replies = Vec::new();
        for (index, request) in pending.iter().enumerate() {
            match request.response.try_recv() {
                Ok(result) => replies.push((index, result)),
                Err(mpsc::TryRecvError::Disconnected) => replies.push((index, request.denied.clone())),
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        for (index, result) in replies.into_iter().rev() {
            let request = pending.remove(index);
            send(input, json!({"id":request.id,"result":result}))?;
            let _ = tx.send(Event::ApprovalClosed(request.id));
        }
        let value = match rx.recv_timeout(Duration::from_millis(40)) {
            Ok(v) => v,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => return Err("Codex 연결이 종료되었습니다".into()),
        };
        if let Some(method) = value["method"].as_str() {
            let params = &value["params"];
            if let Some(id) = value.get("id") {
                if !supported(method) {
                    send(input, json!({"id":id,"error":{"code":-32601,"message":"This client does not implement this request"}}))?;
                    continue;
                }
                if pending.len() >= 32 || pending.iter().any(|p| p.id == *id) {
                    send(input, json!({"id":id,"error":{"code":-32600,"message":"Too many or duplicate pending approval requests"}}))?;
                    continue;
                }
                let (answer, response) = mpsc::channel();
                let request = Request::new(id.clone(), method.into(), params.clone(), answer);
                let denied = request.result(false);
                pending.push(Pending { id: id.clone(), response, denied });
                let _ = tx.send(Event::Activity(
                    "승인 대기 중".into(),
                    params["reason"].as_str().unwrap_or("사용자 확인이 필요합니다").chars().take(2000).collect(),
                ));
                let _ = tx.send(Event::Approval(request));
                continue;
            }
            if method == "serverRequest/resolved" {
                let id = &params["requestId"];
                pending.retain(|p| p.id != *id);
                let _ = tx.send(Event::ApprovalClosed(id.clone()));
                continue;
            }
            if params["threadId"].as_str().is_some_and(|id| id != thread) {
                continue;
            }
            match method {
                "turn/started" => {
                    phase = 4;
                    let _ = tx.send(Event::Activity("요청 처리 중".into(), String::new()));
                }
                "item/agentMessage/delta" => {
                    if let (Some(id), Some(delta)) = (params["itemId"].as_str(), params["delta"].as_str()) {
                        messages.update(id, delta, true);
                        let _ = tx.send(Event::Delta(delta.chars().take(200_000).collect()));
                    }
                }
                "item/started" | "item/completed" => {
                    let item = &params["item"];
                    let completed = method == "item/completed";
                    if item["type"] == "agentMessage" {
                        if let (Some(id), Some(text)) = (item["id"].as_str(), item["text"].as_str()) {
                            messages.update(id, text, false);
                            if completed {
                                let _ = tx.send(Event::Text(messages.text()));
                            }
                        }
                    } else {
                        let (label, detail) = match item["type"].as_str().unwrap_or("") {
                            "commandExecution" => {
                                (if completed { "명령 실행 완료" } else { "명령 실행 중" }, item["command"].as_str().unwrap_or("").to_owned())
                            }
                            "mcpToolCall" => (
                                if completed { "앱 도구 실행 완료" } else { "앱 도구 사용 중" },
                                format!("{} · {}", item["server"].as_str().unwrap_or(""), item["tool"].as_str().unwrap_or("")),
                            ),
                            "webSearch" => ("인터넷 검색", item["query"].as_str().unwrap_or("").to_owned()),
                            "reasoning" => (
                                "계획 정리",
                                item["summary"]
                                    .as_array()
                                    .map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("\n"))
                                    .unwrap_or_default(),
                            ),
                            _ => ("작업 진행 중", String::new()),
                        };
                        let _ = tx.send(Event::Activity(label.into(), detail.chars().take(4000).collect()));
                    }
                }
                "thread/tokenUsage/updated" => {
                    let usage = &params["tokenUsage"]["last"];
                    let _=tx.send(Event::Usage(json!({"input_tokens":usage["inputTokens"],"output_tokens":usage["outputTokens"],"cached_input_tokens":usage["cachedInputTokens"]})));
                }
                "error" => {
                    let error = params["error"]["message"].as_str().unwrap_or("Codex 작업 오류");
                    if params["willRetry"].as_bool().unwrap_or(false) {
                        let _ = tx.send(Event::Activity("다시 시도 중".into(), error.chars().take(2000).collect()));
                    } else {
                        let _ = tx.send(Event::Failure(error.chars().take(4000).collect()));
                    }
                }
                "turn/completed" => {
                    return match params["turn"]["status"].as_str() {
                        Some("completed") => Ok(messages.text()),
                        Some("interrupted") => Err("작업을 중지했습니다".into()),
                        _ => Err(params["turn"]["error"]["message"].as_str().unwrap_or("Codex 요청이 실패했습니다").chars().take(4000).collect()),
                    };
                }
                _ => {}
            }
            continue;
        }
        if let Some(id) = value["id"].as_u64() {
            if let Some(error) = value.get("error") {
                return Err(error["message"].as_str().unwrap_or("Codex 연결 오류").chars().take(4000).collect());
            }
            if id == 1 && phase == 1 {
                send(input, json!({"method":"initialized"}))?;
                let mut params = json!({"cwd":root,"modelProvider":"openai","sandbox":"read-only","approvalPolicy":policy(),"approvalsReviewer":"user","ephemeral":true});
                if !options.settings.model.trim().is_empty() {
                    params["model"] = json!(options.settings.model.trim());
                }
                params["serviceTier"] = json!(if options.settings.fast { options.fast_tier.as_deref().unwrap_or("default") } else { "default" });
                send(input, json!({"id":2,"method":"thread/start","params":params}))?;
                phase = 2;
                phase_at = Instant::now();
            } else if id == 2 && phase == 2 {
                thread = value["result"]["thread"]["id"].as_str().ok_or("Codex 작업 ID를 읽을 수 없습니다")?.into();
                let mut params = json!({"threadId":thread,"input":content,"approvalPolicy":policy(),"approvalsReviewer":"user","sandboxPolicy":{"type":"readOnly","networkAccess":false}});
                if !options.settings.effort.is_empty() {
                    params["effort"] = json!(options.settings.effort);
                }
                send(input, json!({"id":3,"method":"turn/start","params":params}))?;
                phase = 3;
                phase_at = Instant::now();
            } else if id == 3 && phase == 3 {
                phase = 4;
            }
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    #[cfg(unix)]
    pub fn fixture_cli(root: &std::path::Path, mode: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        fs::create_dir_all(root).unwrap();
        let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
        let cli = root.join("codex-fixture");
        fs::write(
            &cli,
            format!(
                r#"#!/bin/sh
cd {root}
if [ "$1" = login ]; then
  if [ "$2" = status ]; then test -f logged-in && echo 'Logged in using ChatGPT'; exit $?; fi
  echo 'http://localhost:1455/auth/callback'
  touch logged-in
  exit 0
fi
printf '%s\n' "$@" > args.txt
export PURPOSE_CODEX_FIXTURE={mode}
export PURPOSE_CODEX_FIXTURE_DIR={root}
exec {exe} --exact video_maker::codex_session::tests::fixture_server --ignored --nocapture
"#,
                root = quote(&root.to_string_lossy()),
                mode = quote(mode),
                exe = quote(&std::env::current_exe().unwrap().to_string_lossy())
            ),
        )
        .unwrap();
        fs::set_permissions(&cli, fs::Permissions::from_mode(0o700)).unwrap();
        cli
    }
    #[test]
    #[ignore = "stdio fixture executed by transport integration tests"]
    fn fixture_server() {
        let Ok(mode) = std::env::var("PURPOSE_CODEX_FIXTURE") else {
            return;
        };
        let root = PathBuf::from(std::env::var("PURPOSE_CODEX_FIXTURE_DIR").unwrap());
        let mut out = std::io::stdout().lock();
        writeln!(out).unwrap();
        out.flush().unwrap();
        let mut captured = fs::OpenOptions::new().create(true).append(true).open(root.join("request.txt")).unwrap();
        let mut pending = Vec::new();
        for line in std::io::stdin().lock().lines() {
            let line = line.unwrap();
            writeln!(captured, "{line}").unwrap();
            captured.flush().unwrap();
            let Ok(v) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            match v["method"].as_str() {
                Some("initialize") => send(&mut out, json!({"id":v["id"],"result":{}})).unwrap(),
                Some("thread/start") => {
                    assert_eq!(v["params"]["sandbox"], "read-only");
                    assert_eq!(v["params"]["approvalsReviewer"], "user");
                    assert_eq!(v["params"]["approvalPolicy"]["granular"]["sandbox_approval"], false);
                    send(&mut out, json!({"id":v["id"],"result":{"thread":{"id":"fixture-thread"}}})).unwrap();
                }
                Some("turn/start") => {
                    send(&mut out, json!({"id":v["id"],"result":{"turn":{"id":"fixture-turn","status":"inProgress"}}})).unwrap();
                    send(
                        &mut out,
                        json!({"method":"turn/started","params":{"threadId":"fixture-thread","turn":{"id":"fixture-turn","status":"inProgress"}}}),
                    )
                    .unwrap();
                    if mode == "stop" {
                        continue;
                    }
                    if mode == "approval" {
                        send(&mut out,json!({"id":"network-request","method":"item/permissions/requestApproval","params":{"threadId":"fixture-thread","turnId":"fixture-turn","itemId":"p1","cwd":root,"reason":"네트워크 연결 확인","permissions":{"network":{"enabled":true}}}})).unwrap();
                        send(&mut out,json!({"id":42,"method":"item/commandExecution/requestApproval","params":{"threadId":"fixture-thread","turnId":"fixture-turn","itemId":"c1","command":"read fixture","cwd":root,"reason":"파일 읽기 확인"}})).unwrap();
                        pending = vec![json!("network-request"), json!(42)];
                        continue;
                    }
                    finish_fixture(&mut out);
                    break;
                }
                _ => {
                    if pending.contains(&v["id"]) {
                        fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(root.join("decisions.jsonl"))
                            .unwrap()
                            .write_all(format!("{v}\n").as_bytes())
                            .unwrap();
                        pending.retain(|id| *id != v["id"]);
                        if pending.is_empty() {
                            finish_fixture(&mut out);
                            break;
                        }
                    }
                }
            }
        }
    }
    fn finish_fixture(out: &mut impl Write) {
        send(out,json!({"method":"item/agentMessage/delta","params":{"threadId":"fixture-thread","turnId":"fixture-turn","itemId":"a1","delta":"수정된 대본"}})).unwrap();
        send(out,json!({"method":"item/completed","params":{"threadId":"fixture-thread","turnId":"fixture-turn","item":{"id":"a1","type":"agentMessage","phase":"final_answer","text":"수정된 대본"}}})).unwrap();
        send(out,json!({"method":"thread/tokenUsage/updated","params":{"threadId":"fixture-thread","turnId":"fixture-turn","tokenUsage":{"last":{"inputTokens":123,"outputTokens":45}}}})).unwrap();
        send(out, json!({"method":"turn/completed","params":{"threadId":"fixture-thread","turn":{"id":"fixture-turn","status":"completed"}}}))
            .unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn real_stdio_roundtrip_waits_for_each_user_decision_and_routes_original_ids() {
        let root = std::env::temp_dir().join(format!("purpose-approval-rpc-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let cli = fixture_cli(&root, "approval");
        let mut maker = super::super::VideoMaker::at(root.join("projects")).unwrap();
        maker.store.data.codex_path = Some(cli);
        maker.authenticated = Some(true);
        maker.ask_codex("인터넷을 확인해".into());
        let ctx = egui::Context::default();
        let until = Instant::now() + Duration::from_secs(5);
        while maker.approvals.len() < 2 && Instant::now() < until {
            maker.poll_chat(&ctx);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(maker.approvals.len(), 2);
        assert!(!root.join("decisions.jsonl").exists());
        let command = maker.approvals.remove(1);
        command.response.send(command.result(false)).unwrap();
        let network = maker.approvals.remove(0);
        network.response.send(network.result(true)).unwrap();
        let until = Instant::now() + Duration::from_secs(5);
        while maker.chat_job.is_some() && Instant::now() < until {
            maker.poll_chat(&ctx);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(maker.chat_job.is_none());
        let responses: Vec<Value> =
            fs::read_to_string(root.join("decisions.jsonl")).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect();
        assert_eq!(responses.iter().find(|v| v["id"] == 42).unwrap()["result"]["decision"], "decline");
        assert_eq!(responses.iter().find(|v| v["id"] == "network-request").unwrap()["result"]["permissions"]["network"]["enabled"], true);
        let video = maker.video_id().unwrap();
        assert_eq!(maker.conversation(video).messages[1].text, "수정된 대본");
        assert_eq!(maker.conversation(video).messages[1].usage.as_ref().unwrap()["output_tokens"], 45);
        let _ = fs::remove_dir_all(root);
    }
}
