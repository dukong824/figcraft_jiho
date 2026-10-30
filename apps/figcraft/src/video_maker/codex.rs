//! Use the official CLI's own authentication flow and credential store.
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};

pub struct Login {
    child: Child,
    pub lines: Receiver<String>,
}

pub fn executable(configured: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = configured.filter(|p| p.is_file() && complete_runtime(p)) {
        return Some(path.to_owned());
    }
    let name = if cfg!(windows) { "codex.exe" } else { "codex" };
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let bundled = dir.join("codex").join(name);
        if bundled.is_file() {
            return Some(bundled);
        }
    }
    std::env::var_os("PATH").and_then(|paths| std::env::split_paths(&paths).map(|p| p.join(name)).find(|p| p.is_file() && complete_runtime(p)))
}

fn complete_runtime(path: &Path) -> bool {
    !cfg!(windows) || path.parent().is_some_and(|p| p.join("codex-code-mode-host.exe").is_file())
}

pub fn command(path: &Path) -> Command {
    let mut command = Command::new(path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW; the browser handles sign in.
    }
    // Official tool discovery also searches PATH. Keep helpers from the same CLI release first.
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        let mut paths = vec![parent.to_owned()];
        if let Some(previous) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&previous));
        }
        if let Ok(paths) = std::env::join_paths(paths) {
            command.env("PATH", paths);
        }
    }
    command.stdin(Stdio::null());
    command
}

pub fn status(path: &Path) -> Result<bool, String> {
    command(path)
        .args(["login", "status"])
        .output()
        .map(|out| {
            let text = format!("{} {}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
            out.status.success() && text.to_ascii_lowercase().contains("chatgpt")
        })
        .map_err(|e| e.to_string())
}

impl Login {
    pub fn start(path: &Path, device: bool) -> Result<Self, String> {
        let mut cmd = command(path);
        cmd.arg("login");
        if device {
            cmd.arg("--device-auth");
        }
        let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| e.to_string())?;
        let (tx, lines) = mpsc::channel();
        if let Some(pipe) = child.stdout.take() {
            read_lines(pipe, tx.clone());
        }
        if let Some(pipe) = child.stderr.take() {
            read_lines(pipe, tx);
        }
        Ok(Self { child, lines })
    }

    pub fn finished(&mut self) -> Result<Option<bool>, String> {
        self.child.try_wait().map(|status| status.map(|s| s.success())).map_err(|e| e.to_string())
    }
}

impl Drop for Login {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn read_lines(pipe: impl std::io::Read + Send + 'static, tx: mpsc::Sender<String>) {
    std::thread::spawn(move || {
        for line in BufReader::new(pipe).lines().take(300) {
            let Ok(line) = line else { break };
            let clean = strip_ansi(&line);
            if tx.send(clean).is_err() {
                break;
            }
        }
    });
}

fn strip_ansi(text: &str) -> String {
    let mut escape = false;
    text.chars()
        .filter(|c| {
            if *c == '\u{1b}' {
                escape = true;
                return false;
            }
            if escape {
                if c.is_ascii_alphabetic() {
                    escape = false;
                }
                return false;
            }
            !c.is_control() || *c == '\t'
        })
        .collect()
}

impl super::VideoMaker {
    pub(super) fn refresh_auth(&mut self) {
        if self.auth_job.is_some() || self.login.is_some() {
            return;
        }
        let Some(path) = executable(self.store.data.codex_path.as_deref()) else {
            self.authenticated = None;
            self.auth_error = "Codex CLI를 찾을 수 없습니다. 실행 파일을 선택하거나 Windows 배포본을 사용하세요.".into();
            return;
        };
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(status(&path));
        });
        self.auth_job = Some(rx);
        self.auth_error.clear();
    }

    pub(super) fn start_login(&mut self, device: bool) {
        let Some(path) = executable(self.store.data.codex_path.as_deref()) else {
            return;
        };
        self.login_output.clear();
        self.auth_error.clear();
        match Login::start(&path, device) {
            Ok(login) => self.login = Some(login),
            Err(error) => self.auth_error = error,
        }
    }

    pub(super) fn poll_auth(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.auth_job {
            match rx.try_recv() {
                Ok(Ok(auth)) => {
                    self.authenticated = Some(auth);
                    if auth {
                        self.refresh_catalog();
                    }
                    self.auth_job = None;
                }
                Ok(Err(error)) => {
                    self.auth_error = error;
                    self.auth_job = None;
                    self.authenticated = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.auth_job = None;
                    self.auth_error = "연결 확인이 종료되었습니다".into();
                }
                Err(mpsc::TryRecvError::Empty) => ctx.request_repaint_after(std::time::Duration::from_millis(200)),
            }
        }
        if let Some(login) = &mut self.login {
            for line in login.lines.try_iter() {
                if self.login_output.len() < 16000 {
                    self.login_output.push_str(&line);
                    self.login_output.push('\n');
                }
            }
            match login.finished() {
                Ok(Some(success)) => {
                    self.login = None;
                    if success {
                        self.refresh_auth();
                    } else {
                        self.auth_error = "로그인이 완료되지 않았습니다. 다시 시도하세요.".into();
                    }
                }
                Err(error) => {
                    self.login = None;
                    self.auth_error = error;
                }
                Ok(None) => ctx.request_repaint_after(std::time::Duration::from_millis(200)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    #[test]
    fn selected_cli_handles_login_and_chat_output() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("purpose-fake-codex-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let cli = root.join("codex");
        std::fs::write(
            &cli,
            r##"#!/bin/sh
cd "$(dirname "$0")"
if [ "$1" = login ]; then
  if [ "$2" = status ]; then if test -f logged-in; then echo 'Logged in using ChatGPT'; exit 0; else exit 1; fi; fi
  echo 'http://localhost:1455/auth/callback'
  touch logged-in
  exit 0
fi
printf '%s\n' "$@" > args.txt
while [ "$#" -gt 0 ]; do
  if [ "$1" = --output-last-message ]; then shift; output="$1"; fi
  shift
done
cat > request.txt
echo '수정된 대본' > "$output"
"##,
        )
        .unwrap();
        std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(!super::status(&cli).unwrap());
        let mut login = super::Login::start(&cli, false).unwrap();
        assert!(login.lines.recv_timeout(std::time::Duration::from_secs(2)).unwrap().contains("localhost"));
        for _ in 0..100 {
            if login.finished().unwrap() == Some(true) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(super::status(&cli).unwrap());
        let mut maker = super::super::VideoMaker::at(root.join("projects")).unwrap();
        maker.store.data.codex_path = Some(cli.clone());
        maker.authenticated = Some(true);
        maker.store.data.codex_settings = super::super::codex_options::Settings { model: "test-model".into(), effort: "high".into(), fast: true };
        maker.catalog = vec![
            serde_json::from_value(serde_json::json!({"model":"test-model","displayName":"Test model","additionalSpeedTiers":["fast"]})).unwrap(),
        ];
        maker.ask_codex("내 대본 수정".into());
        let id = maker.video_id().unwrap();
        assert_eq!(maker.conversation(id).messages[0].text, "내 대본 수정");
        let ctx = egui::Context::default();
        for _ in 0..200 {
            maker.poll_chat(&ctx);
            if maker.chat_job.is_none() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(maker.chat_job.is_none());
        assert_eq!(maker.conversation(id).messages[1].text.trim(), "수정된 대본");
        assert!(std::fs::read_to_string(root.join("request.txt")).unwrap().contains("내 대본 수정"));
        let args = std::fs::read_to_string(root.join("args.txt")).unwrap();
        assert!(args.contains("--model\ntest-model\n"));
        assert!(args.contains("model_reasoning_effort=\"high\""));
        assert!(args.contains("service_tier=\"fast\""));
        let mut reopened = super::super::VideoMaker::at(root.join("projects")).unwrap();
        assert_eq!(reopened.conversation(id).messages.len(), 2);
        drop(login);
        let _ = std::fs::remove_dir_all(root);
    }
    #[test]
    fn login_output_keeps_urls_and_codes_without_terminal_escapes() {
        assert_eq!(super::strip_ansi("\u{1b}[32mABCD-1234\u{1b}[0m https://auth.openai.com"), "ABCD-1234 https://auth.openai.com");
    }
}
