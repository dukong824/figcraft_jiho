//! Per-request, loopback capability channel and a small standard MCP stdio adapter.
//! The adapter has no project-write path: all mutations go through the live app's command layer.
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
const LIMIT: u64 = 16 * 1024 * 1024;
#[derive(Clone)]
pub struct Connection {
    pub address: String,
    pub token: String,
}
pub struct Pending {
    pub name: String,
    pub arguments: Value,
    pub reply: mpsc::Sender<Result<Value, String>>,
    pub cancelled: Arc<AtomicBool>,
    pub deadline: Instant,
}
pub struct Bridge {
    pub connection: Connection,
    pub video: u64,
    pub guide: Option<String>,
    receiver: mpsc::Receiver<Pending>,
    stopped: Arc<AtomicBool>,
}
impl Drop for Bridge {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
    }
}
impl Bridge {
    pub fn start(video: u64, ctx: egui::Context) -> Result<Self, String> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let address = listener.local_addr().map_err(|e| e.to_string())?.to_string();
        let mut random = [0u8; 32];
        getrandom::fill(&mut random).map_err(|e| e.to_string())?;
        let token = random.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let connection = Connection { address, token: token.clone() };
        let (tx, receiver) = mpsc::sync_channel::<Pending>(8);
        let stopped = Arc::new(AtomicBool::new(false));
        let stop = stopped.clone();
        std::thread::spawn(move || {
            let active = Arc::new(AtomicUsize::new(0));
            while !stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if active.try_update(Ordering::Relaxed, Ordering::Relaxed, |n| (n < 4).then_some(n + 1)).is_err() {
                            continue;
                        }
                        let (tx, token, stop, wake, count) = (tx.clone(), token.clone(), stop.clone(), ctx.clone(), active.clone());
                        std::thread::spawn(move || {
                            serve(stream, &token, &tx, &stop, &wake);
                            count.fetch_sub(1, Ordering::Relaxed);
                        });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(15)),
                    Err(_) => break,
                }
            }
        });
        Ok(Self { connection, video, guide: None, receiver, stopped })
    }
    pub fn take_pending(&self) -> Vec<Pending> {
        self.receiver.try_iter().take(4).collect()
    }
}
fn line(reader: impl Read) -> Result<String, String> {
    let mut text = String::new();
    BufReader::new(reader.take(LIMIT + 1)).read_line(&mut text).map_err(|e| e.to_string())?;
    if text.len() as u64 > LIMIT || !text.ends_with('\n') {
        return Err("요청이 너무 크거나 완료되지 않았습니다".into());
    }
    Ok(text)
}
fn serve(mut stream: TcpStream, token: &str, tx: &mpsc::SyncSender<Pending>, stopped: &AtomicBool, ctx: &egui::Context) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let result = (|| {
        let request: Value = serde_json::from_str(&line(&stream)?).map_err(|e| e.to_string())?;
        if stopped.load(Ordering::Relaxed) || request["token"].as_str() != Some(token) {
            return Err("작업 연결이 종료되었거나 유효하지 않습니다".into());
        }
        let name = request["name"].as_str().ok_or("도구 이름이 필요합니다")?.to_owned();
        let (reply, result) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        tx.try_send(Pending {
            name,
            arguments: request["arguments"].clone(),
            reply,
            cancelled: cancelled.clone(),
            deadline: Instant::now() + Duration::from_secs(60),
        })
        .map_err(|_| "앱 작업 대기열이 가득 찼습니다".to_owned())?;
        ctx.request_repaint();
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            match result.recv_timeout(Duration::from_millis(100)) {
                Ok(result) => return result,
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err("앱 연결이 종료되었습니다".into()),
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if stopped.load(Ordering::Relaxed) || Instant::now() >= deadline {
                        cancelled.store(true, Ordering::Relaxed);
                        return Err("앱 작업이 취소되었거나 응답 시간이 초과되었습니다".into());
                    }
                }
            }
        }
    })();
    let response = match result {
        Ok(value) => json!({"result":value}),
        Err(error) => json!({"error":error}),
    };
    let _ = writeln!(stream, "{response}");
}
fn forward(connection: &Connection, name: &str, arguments: Value) -> Result<Value, String> {
    let address: std::net::SocketAddr = connection.address.parse().map_err(|_| "잘못된 앱 연결 주소")?;
    if !address.ip().is_loopback() {
        return Err("앱 연결은 로컬 주소여야 합니다".into());
    }
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(3)).map_err(|e| e.to_string())?;
    stream.set_read_timeout(Some(Duration::from_secs(65))).map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
    writeln!(stream, "{}", json!({"token":connection.token,"name":name,"arguments":arguments})).map_err(|e| e.to_string())?;
    let response: Value = serde_json::from_str(&line(stream)?).map_err(|e| e.to_string())?;
    if let Some(error) = response["error"].as_str() { Err(error.into()) } else { Ok(response["result"].clone()) }
}
fn tool(name: &str, description: &str, properties: Value, required: &[&str], readonly: bool) -> Value {
    json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},"annotations":{"readOnlyHint":readonly,"destructiveHint":false,"openWorldHint":false}})
}
pub fn tools() -> Vec<Value> {
    vec![
        tool(
            "read_guide",
            "First call for every task. Read common and channel instructions, the fixed target video and script read-only policy.",
            json!({}),
            &[],
            true,
        ),
        tool(
            "inspect_project",
            "Read live design layers, FilmCraft project, library assets and script. Also reports asynchronous imports.",
            json!({"target":{"type":"string","enum":["all","design","edit","library","script"]}}),
            &[],
            true,
        ),
        tool(
            "list_commands",
            "Discover actual command ids and parameter documentation before editing.",
            json!({"target":{"type":"string","enum":["design","edit"]},"query":{"type":"string"}}),
            &["target"],
            true,
        ),
        tool(
            "run_command",
            "Execute a live FigCraft or FilmCraft editing command. Changes persist in the app and use the editor's undo history. Script modification is unavailable.",
            json!({"target":{"type":"string","enum":["design","edit"]},"command":{"type":"string"},"params":{"type":"object"}}),
            &["target", "command"],
            false,
        ),
        tool(
            "library_import",
            "Import local files asynchronously. Returns reserved asset ids; inspect_project reports completion. Never changes the script.",
            json!({"paths":{"type":"array","items":{"type":"string"},"minItems":1,"maxItems":64}}),
            &["paths"],
            false,
        ),
        tool(
            "library_rename",
            "Change an asset's display name in both the library and editing workspace.",
            json!({"id":{"type":"integer"},"name":{"type":"string"}}),
            &["id", "name"],
            false,
        ),
        tool(
            "library_remove",
            "Move selected library assets to the trash. Files used by existing edits remain intact.",
            json!({"ids":{"type":"array","items":{"type":"integer"},"minItems":1,"maxItems":128}}),
            &["ids"],
            false,
        ),
        tool(
            "library_copy",
            "Duplicate library assets asynchronously, preserving their display names.",
            json!({"ids":{"type":"array","items":{"type":"integer"},"minItems":1,"maxItems":128}}),
            &["ids"],
            false,
        ),
        tool(
            "library_restore",
            "Restore library assets from the trash.",
            json!({"ids":{"type":"array","items":{"type":"integer"},"minItems":1,"maxItems":128}}),
            &["ids"],
            false,
        ),
        tool(
            "library_place",
            "Place a library source into the design canvas or append it to the editing timeline. Does not attach to the script.",
            json!({"id":{"type":"integer"},"target":{"type":"string","enum":["design","edit"]}}),
            &["id", "target"],
            false,
        ),
        tool(
            "export_design",
            "Export selected design layers, or the page when nothing is selected, to the video library using their own names.",
            json!({}),
            &[],
            false,
        ),
        tool(
            "design_preview",
            "Render the current page or given layer ids to a PNG for visual verification.",
            json!({"ids":{"type":"array","items":{"type":"integer"}}}),
            &[],
            true,
        ),
    ]
}
fn rpc(request: &Value, connection: &Connection) -> Value {
    let id = request["id"].clone();
    let protocol =
        request["params"]["protocolVersion"].as_str().filter(|v| ["2025-06-18", "2025-03-26", "2024-11-05"].contains(v)).unwrap_or("2025-06-18");
    let result = match request["method"].as_str().unwrap_or_default() {
        "initialize" => Ok(
            json!({"protocolVersion":protocol,"serverInfo":{"name":"purpose_video_maker","version":"0.1.9"},"capabilities":{"tools":{}},"instructions":"Your first tool call must be read_guide. Follow the common and channel guide before every action. Design, editing and library changes are tied to the video that started this task. The script and guide are read-only to the agent; suggest script revisions in your response. Discover command schemas, edit through these tools, and inspect/preview the result."}),
        ),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools":tools()})),
        "resources/list" => Ok(json!({"resources":[]})),
        "prompts/list" => Ok(json!({"prompts":[]})),
        "tools/call" => {
            let name = request["params"]["name"].as_str().unwrap_or_default();
            if !tools().iter().any(|t| t["name"] == name) {
                return json!({"jsonrpc":"2.0","id":id,"error":{"code":-32602,"message":"Unknown tool"}});
            }
            let value = forward(connection, name, request["params"].get("arguments").cloned().unwrap_or(json!({})));
            Ok(match value {
                Ok(value) => {
                    if let Some(data) = value["_image"].as_str() {
                        json!({"content":[{"type":"image","mimeType":"image/png","data":data}],"isError":false})
                    } else {
                        json!({"content":[{"type":"text","text":value.to_string()}],"isError":false})
                    }
                }
                Err(error) => json!({"content":[{"type":"text","text":error}],"isError":true}),
            })
        }
        _ => Err("Unknown MCP method"),
    };
    match result {
        Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
        Err(message) => json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":message}}),
    }
}
pub fn stdio() -> Result<(), String> {
    let connection = Connection {
        address: std::env::var("PURPOSE_AGENT_ADDR").map_err(|_| "앱 연결 주소가 없습니다")?,
        token: std::env::var("PURPOSE_AGENT_TOKEN").map_err(|_| "앱 연결 키가 없습니다")?,
    };
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    loop {
        let mut bytes = Vec::new();
        if input.by_ref().take(LIMIT + 1).read_until(b'\n', &mut bytes).map_err(|e| e.to_string())? == 0 {
            break;
        }
        if bytes.len() as u64 > LIMIT {
            return Err("MCP request too large".into());
        }
        let request: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if request.get("id").is_none() {
            continue;
        }
        writeln!(output, "{}", rpc(&request, &connection)).map_err(|e| e.to_string())?;
        output.flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}
impl Connection {
    pub fn configure(&self, command: &mut std::process::Command, executable: &std::path::Path) {
        command.env("PURPOSE_AGENT_ADDR", &self.address).env("PURPOSE_AGENT_TOKEN", &self.token);
        let path = executable.to_string_lossy();
        if let Ok(path) = serde_json::to_string(path.as_ref()) {
            command.arg("-c").arg(format!("mcp_servers.purpose.command={path}"));
        }
        command.args([
            "-c",
            "mcp_servers.purpose.args=[\"--purpose-agent-mcp\"]",
            "-c",
            "mcp_servers.purpose.env_vars=[\"PURPOSE_AGENT_ADDR\",\"PURPOSE_AGENT_TOKEN\"]",
            "-c",
            "mcp_servers.purpose.enabled=true",
        ]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loopback_bridge_requires_its_capability_and_returns_actual_app_results() {
        let bridge = Bridge::start(73, egui::Context::default()).unwrap();
        let bad = Connection { token: "wrong".into(), address: bridge.connection.address.clone() };
        assert!(forward(&bad, "read_guide", json!({})).is_err());
        assert!(bridge.take_pending().is_empty());
        let connection = bridge.connection.clone();
        let worker = std::thread::spawn(move || forward(&connection, "read_guide", json!({})));
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(request) = bridge.take_pending().pop() {
                request.reply.send(Ok(json!({"video":73,"guide":"fixture instructions"}))).unwrap();
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(worker.join().unwrap().unwrap()["video"], 73);
    }
    #[test]
    fn mcp_catalog_has_no_script_write_tool_and_initializes_without_a_gui() {
        let connection = Connection { address: "127.0.0.1:1".into(), token: "fixture".into() };
        let response = rpc(&json!({"id":1,"method":"initialize"}), &connection);
        assert_eq!(response["result"]["serverInfo"]["name"], "purpose_video_maker");
        assert!(tools().iter().all(|t| !t["name"].as_str().unwrap().contains("script")));
        assert!(tools().iter().any(|t| t["name"] == "run_command"));
    }
}
