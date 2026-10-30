//! A streaming, incremental backup mirror. Project media is copied once, not on every keystroke.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::mpsc,
    time::{Duration, Instant, SystemTime},
};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Stamp {
    len: u64,
    modified: Option<SystemTime>,
}
#[derive(Default)]
pub struct Controller {
    job: Option<mpsc::Receiver<Result<PathBuf, String>>>,
    due: Option<Instant>,
    pub location: Option<PathBuf>,
    pub last_ok: Option<String>,
    pub error: String,
}
impl Controller {
    pub fn request(&mut self) {
        self.due = Some(Instant::now());
    }
    pub fn finish(&mut self, root: &Path, directory: Option<&Path>, id: &str) -> Result<(), String> {
        if let Some(rx) = self.job.take() {
            let _ = rx.recv();
        }
        if let Some(directory) = directory {
            mirror(root, &destination(directory, id))?;
        }
        Ok(())
    }
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
    pub fn poll(&mut self, root: &Path, directory: Option<&Path>, id: &str, ctx: &egui::Context) {
        if let Some(result) = self.job.as_ref().and_then(|r| r.try_recv().ok()) {
            self.job = None;
            match result {
                Ok(path) => {
                    self.location = Some(path);
                    self.last_ok = Some(chrono::Local::now().format("%H:%M:%S").to_string());
                    self.error.clear();
                }
                Err(error) => self.error = error,
            }
            self.due = Some(Instant::now() + Duration::from_secs(10));
        }
        if directory.is_none() {
            self.due = None;
            return;
        }
        if self.job.is_none() && self.due.is_none() {
            self.request();
        }
        if self.job.is_none()
            && self.due.is_some_and(|at| Instant::now() >= at)
            && let Some(directory) = directory
        {
            self.due = None;
            let source = root.to_owned();
            let target = destination(directory, id);
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                let _ = tx.send(mirror(&source, &target));
            });
            self.job = Some(rx);
        }
        if self.job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(200));
        } else if let Some(due) = self.due {
            ctx.request_repaint_after(due.saturating_duration_since(Instant::now()));
        }
    }
}
fn destination(directory: &Path, id: &str) -> PathBuf {
    let id: String = id.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')).take(80).collect();
    directory.join(format!("Purpose-backup-{}", if id.is_empty() { "workspace" } else { &id }))
}
fn skipped(name: &str) -> bool {
    name.starts_with("Purpose-backup-")
        || name.ends_with(".tmp")
        || name.starts_with(".codex-response-")
        || name == "editor-state"
        || name.ends_with(".lock")
}
fn copy(source: &Path, target: &Path, relative: &Path, index: &mut BTreeMap<String, Stamp>) -> Result<(), String> {
    let metadata = fs::metadata(source).map_err(|e| e.to_string())?;
    let stamp = Stamp { len: metadata.len(), modified: metadata.modified().ok() };
    let key = relative.to_string_lossy().to_string();
    if index.get(&key) == Some(&stamp) && fs::metadata(target).is_ok_and(|m| m.len() == stamp.len) {
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temp = target.with_file_name(format!(".{}.tmp", target.file_name().unwrap_or_default().to_string_lossy()));
    fs::copy(source, &temp).map_err(|e| format!("백업 복사 실패 ({}): {e}", relative.display()))?;
    // The destination keeps one previous workspace manifest if Windows cannot replace it directly.
    if fs::rename(&temp, target).is_err() {
        let previous = target.with_extension("backup-previous");
        if previous.exists() {
            fs::remove_file(&previous).map_err(|e| e.to_string())?;
        }
        if target.exists() {
            fs::rename(target, &previous).map_err(|e| e.to_string())?;
        }
        if let Err(error) = fs::rename(&temp, target) {
            if previous.exists() {
                let _ = fs::rename(&previous, target);
            }
            return Err(error.to_string());
        }
        if previous.exists() {
            let _ = fs::remove_file(previous);
        }
    }
    index.insert(key, stamp);
    Ok(())
}
fn walk(root: &Path, source: &Path, target: &Path, index: &mut BTreeMap<String, Stamp>) -> Result<(), String> {
    for entry in fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if path.starts_with(target) || kind.is_symlink() || skipped(&entry.file_name().to_string_lossy()) {
            continue;
        }
        let relative = path.strip_prefix(root).map_err(|e| e.to_string())?;
        if relative == Path::new("workspace.json") {
            continue;
        }
        if kind.is_dir() {
            walk(root, &path, target, index)?;
        } else if kind.is_file() {
            copy(&path, &target.join(relative), relative, index)?;
        }
    }
    Ok(())
}
fn mirror(root: &Path, target: &Path) -> Result<PathBuf, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    fs::create_dir_all(target).map_err(|e| format!("백업 폴더를 만들 수 없습니다: {e}"))?;
    let target = target.canonicalize().map_err(|e| e.to_string())?;
    if root == target || root.starts_with(&target) {
        return Err("프로젝트 원본을 백업 대상으로 사용할 수 없습니다".into());
    }
    let manifest = target.join("backup-index.json");
    let mut bytes = Vec::new();
    if let Ok(file) = fs::File::open(&manifest) {
        let _ = file.take(16 * 1024 * 1024).read_to_end(&mut bytes);
    }
    let mut index: BTreeMap<String, Stamp> = serde_json::from_slice(&bytes).unwrap_or_default();
    walk(&root, &root, &target, &mut index)?;
    copy(&root.join("workspace.json"), &target.join("workspace.json"), Path::new("workspace.json"), &mut index)?;
    super::model::write_json(&manifest, &index)?;
    if let Err(error) = super::retention::collect_snapshots(&target) {
        log::warn!("백업 기록 정리 보류: {error}");
    }
    Ok(target)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persisted_backup_identifiers_cannot_escape_the_selected_directory() {
        let directory = Path::new("backups");
        for id in ["../../escape", "C:\\outside", "/absolute", ""] {
            let path = destination(directory, id);
            assert_eq!(path.parent(), Some(directory));
            assert!(path.file_name().unwrap().to_string_lossy().starts_with("Purpose-backup-"));
        }
    }
    #[test]
    fn mirror_preserves_media_and_revisions_and_skips_its_own_directory() {
        let root = std::env::temp_dir().join(format!("purpose-backup-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let target = root.join("backups/mirror");
        fs::create_dir_all(root.join("videos/2/assets")).unwrap();
        fs::write(root.join("workspace.json"), b"{\"version\":3}").unwrap();
        fs::write(root.join("videos/2/assets/3.png"), b"source").unwrap();
        fs::write(root.join("videos/2/design.figcraft"), b"design").unwrap();
        mirror(&root, &target).unwrap();
        let file = target.join("videos/2/assets/3.png");
        assert_eq!(fs::read(&file).unwrap(), b"source");
        let before = fs::metadata(&file).unwrap().modified().unwrap();
        mirror(&root, &target).unwrap();
        assert_eq!(fs::metadata(&file).unwrap().modified().unwrap(), before);
        assert!(!target.join("backups/mirror").exists());
        fs::write(root.join("workspace.json"), b"new state").unwrap();
        mirror(&root, &target).unwrap();
        assert_eq!(fs::read(target.join("workspace.json")).unwrap(), b"new state");
        assert_eq!(fs::read(target.join("videos/2/design.figcraft")).unwrap(), b"design");
        let _ = fs::remove_dir_all(root);
    }
}
