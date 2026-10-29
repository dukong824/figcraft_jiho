//! Public CLI model catalog and per-request options. No inference or credential reads.
use super::{VideoMaker, codex};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub model: String,
    pub effort: String,
    pub fast: bool,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub model: String,
    pub display_name: String,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub default_reasoning_effort: String,
    #[serde(default)]
    pub supported_reasoning_efforts: Vec<Effort>,
    #[serde(default)]
    pub service_tiers: Vec<Tier>,
    #[serde(default)]
    pub additional_speed_tiers: Vec<String>,
    #[serde(default)]
    pub hidden: bool,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Effort {
    pub reasoning_effort: String,
    #[serde(default)]
    pub description: String,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Tier {
    pub id: String,
    #[serde(default)]
    pub description: String,
}
impl Model {
    pub fn fast_tier(&self) -> Option<&str> {
        self.service_tiers
            .iter()
            .map(|t| t.id.as_str())
            .chain(self.additional_speed_tiers.iter().map(String::as_str))
            .find(|id| matches!(*id, "priority" | "fast"))
    }
}
impl Settings {
    pub fn configure(&self, command: &mut Command, fast_tier: Option<&str>) {
        if !self.model.trim().is_empty() {
            command.arg("--model").arg(self.model.trim());
        }
        let mut config = |key: &str, value: &str| {
            if let Ok(value) = serde_json::to_string(value) {
                command.arg("-c").arg(format!("{key}={value}"));
            }
        };
        if !self.effort.is_empty() {
            config("model_reasoning_effort", &self.effort);
        }
        config("service_tier", if self.fast { fast_tier.unwrap_or("default") } else { "default" });
    }
}
pub fn catalog(cli: &Path) -> Result<Vec<Model>, String> {
    let mut child = codex::command(cli)
        .args(["-c", "forced_login_method=\"chatgpt\"", "-c", "model_provider=\"openai\"", "app-server", "--listen", "stdio://"])
        .env_remove("OPENAI_API_KEY")
        .env_remove("CODEX_API_KEY")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let result = (|| {
        let mut input = child.stdin.take().ok_or("CLI 입력을 열 수 없습니다")?;
        let output = child.stdout.take().ok_or("CLI 출력을 열 수 없습니다")?;
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output.take(4 * 1024 * 1024)).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        writeln!(input,"{}",json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"purpose_video_maker","title":"Purpose video maker","version":"0.1.7"},"capabilities":{"experimentalApi":true}}})).map_err(|e|e.to_string())?;
        input.flush().map_err(|e| e.to_string())?;
        let deadline = Instant::now() + Duration::from_secs(8);
        while let Some(wait) = deadline.checked_duration_since(Instant::now()) {
            let line = rx.recv_timeout(wait).map_err(|_| "모델 목록을 불러오지 못했습니다".to_owned())?;
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
            if let Some(error) = value.get("error") {
                return Err(error["message"].as_str().unwrap_or("CLI 요청 오류").chars().take(300).collect());
            }
            match value["id"].as_u64() {
                Some(1) => {
                    writeln!(input, "{}", json!({"method":"initialized"})).map_err(|e| e.to_string())?;
                    writeln!(input, "{}", json!({"id":2,"method":"model/list","params":{"limit":100,"includeHidden":false}}))
                        .map_err(|e| e.to_string())?;
                    input.flush().map_err(|e| e.to_string())?;
                }
                Some(2) => {
                    let values = value["result"]["data"].as_array().ok_or("CLI 모델 목록 형식을 읽을 수 없습니다")?;
                    let models = values
                        .iter()
                        .take(128)
                        .filter_map(|v| serde_json::from_value::<Model>(v.clone()).ok())
                        .filter(|m| !m.hidden && m.model.len() <= 200 && !m.model.is_empty())
                        .collect();
                    return Ok(models);
                }
                _ => {}
            }
        }
        Err("모델 목록 응답 시간 초과".into())
    })();
    let _ = child.kill();
    let _ = child.wait();
    result
}
impl VideoMaker {
    pub(super) fn refresh_catalog(&mut self) {
        if self.catalog_job.is_some() {
            return;
        }
        let Some(cli) = codex::executable(self.store.data.codex_path.as_deref()) else {
            return;
        };
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(catalog(&cli));
        });
        self.catalog_job = Some(rx);
        self.catalog_error.clear();
    }
    pub(super) fn poll_catalog(&mut self, ctx: &egui::Context) {
        let result = self.catalog_job.as_ref().and_then(|r| r.try_recv().ok());
        if let Some(result) = result {
            self.catalog_job = None;
            match result {
                Ok(models) => {
                    self.catalog = models;
                    self.catalog_error.clear();
                }
                Err(error) => self.catalog_error = error,
            }
        }
        if self.catalog_job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
    pub(super) fn selected_model(&self) -> Option<&Model> {
        let name = &self.store.data.codex_settings.model;
        if name.is_empty() {
            self.catalog.iter().find(|m| m.is_default).or_else(|| self.catalog.first())
        } else {
            self.catalog.iter().find(|m| &m.model == name)
        }
    }
}
pub fn effort_label(value: &str) -> &str {
    match value {
        "" => "자동",
        "none" => "없음",
        "minimal" => "최소",
        "low" => "낮음",
        "medium" => "보통",
        "high" => "높음",
        "xhigh" => "매우 높음",
        "max" => "최대",
        _ => value,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[test]
    fn public_stdio_handshake_reads_account_models_and_filters_hidden_entries() {
        use std::{fs, os::unix::fs::PermissionsExt};
        let root = std::env::temp_dir().join(format!("purpose-models-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let cli = root.join("codex");
        fs::write(&cli,r##"#!/bin/sh
cd "$(dirname "$0")"
read first
printf '%s\n' "$first" > protocol.txt
echo '{"id":1,"result":{}}'
read initialized
read models
printf '%s\n%s\n' "$initialized" "$models" >> protocol.txt
echo '{"id":2,"result":{"data":[{"model":"available","displayName":"Available","isDefault":true,"supportedReasoningEfforts":[{"reasoningEffort":"high"}],"additionalSpeedTiers":["fast"]},{"model":"hidden","displayName":"Hidden","hidden":true}]}}'
read unused
"##).unwrap();
        fs::set_permissions(&cli, fs::Permissions::from_mode(0o700)).unwrap();
        let models = catalog(&cli).unwrap();
        assert_eq!(models.len(), 1);
        assert!(models[0].is_default);
        assert_eq!(models[0].fast_tier(), Some("fast"));
        let messages: Vec<serde_json::Value> =
            fs::read_to_string(root.join("protocol.txt")).unwrap().lines().map(|v| serde_json::from_str(v).unwrap()).collect();
        assert_eq!(messages[0]["method"], "initialize");
        assert_eq!(messages[1]["method"], "initialized");
        assert_eq!(messages[2]["method"], "model/list");
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn request_options_reach_cli_and_fast_off_overrides_global_fast_config() {
        let settings = Settings { model: "model-a".into(), effort: "high".into(), fast: true };
        let mut c = Command::new("codex");
        settings.configure(&mut c, Some("priority"));
        let args: Vec<_> = c.get_args().map(|a| a.to_string_lossy().to_string()).collect();
        assert!(args.contains(&"model-a".into()));
        assert!(args.contains(&"model_reasoning_effort=\"high\"".into()));
        assert!(args.contains(&"service_tier=\"priority\"".into()));
        let mut c = Command::new("codex");
        Settings::default().configure(&mut c, Some("priority"));
        assert!(c.get_args().any(|v| v == "service_tier=\"default\""));
    }
    #[test]
    fn metadata_controls_reasoning_and_speed_availability() {
        let m:Model=serde_json::from_value(json!({"model":"model-a","displayName":"Model A","supportedReasoningEfforts":[{"reasoningEffort":"high","description":"Careful"}],"serviceTiers":[{"id":"priority","description":"Fast"}]})).unwrap();
        assert_eq!(m.fast_tier(), Some("priority"));
        assert_eq!(m.supported_reasoning_efforts[0].reasoning_effort, "high");
        assert_eq!(Model::default().fast_tier(), None);
    }
}
