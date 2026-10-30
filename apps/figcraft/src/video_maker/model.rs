use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Section {
    #[default]
    Library,
    Script,
    Design,
    Edit,
}

impl Section {
    pub const ALL: [Self; 4] = [Self::Library, Self::Script, Self::Design, Self::Edit];

    pub fn label(self) -> &'static str {
        match self {
            Self::Library => "라이브러리",
            Self::Script => "대본",
            Self::Design => "디자인",
            Self::Edit => "편집",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetKind {
    Video,
    Audio,
    Image,
    Design,
}

impl AssetKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Video => "영상",
            Self::Audio => "오디오",
            Self::Image => "이미지",
            Self::Design => "디자인",
        }
    }

    pub fn can_edit(self) -> bool {
        matches!(self, Self::Video | Self::Audio | Self::Image)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub id: u64,
    pub name: String,
    /// A file name in this video's assets directory, never an external absolute path.
    pub file: String,
    pub kind: AssetKind,
    pub duration: f64,
    pub has_audio: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BlockKind {
    Heading,
    Paragraph,
    Scene,
    Asset(u64),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScriptBlock {
    pub kind: BlockKind,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    pub id: u64,
    pub asset_id: u64,
    pub trim_start: f64,
    pub trim_end: f64,
    /// Audio clips start at this time. Video clips play sequentially in list order.
    pub start: f64,
    pub volume: f64,
    pub audio_track: bool,
}

impl Clip {
    pub fn duration(&self) -> f64 {
        (self.trim_end - self.trim_start).max(0.0)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Caption {
    pub id: u64,
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Video {
    pub id: u64,
    pub name: String,
    pub assets: Vec<Asset>,
    #[serde(default)]
    pub removed_assets: Vec<Asset>,
    #[serde(default)]
    pub script_title: String,
    #[serde(default)]
    pub script_body: String,
    #[serde(default = "default_script_size")]
    pub script_font_size: f32,
    #[serde(default = "default_line_height")]
    pub script_line_height: f32,
    #[serde(default)]
    pub film_revision: Option<String>,
    #[serde(default)]
    pub script_rich: super::rich_script::RichScript,
    #[serde(default)]
    pub script_assets: Vec<u64>,
    /// Read only during migration from the original block editor.
    #[serde(default, skip_serializing)]
    pub script: Vec<ScriptBlock>,
    pub clips: Vec<Clip>,
    pub captions: Vec<Caption>,
}

impl Video {
    fn migrate_script(&mut self) {
        let legacy = std::mem::take(&mut self.script);
        if self.script_title.is_empty() {
            self.script_title = legacy
                .iter()
                .find(|block| matches!(block.kind, BlockKind::Heading) && !block.text.trim().is_empty())
                .map_or_else(|| format!("{}의 대본", self.name), |block| block.text.clone());
        }
        if self.script_body.is_empty() {
            let mut title_used = false;
            let mut paragraphs = Vec::new();
            for block in legacy {
                match block.kind {
                    BlockKind::Heading if !title_used && block.text == self.script_title => title_used = true,
                    BlockKind::Asset(id) => {
                        if !self.script_assets.contains(&id) {
                            self.script_assets.push(id);
                        }
                        if !block.text.trim().is_empty() {
                            paragraphs.push(block.text);
                        }
                    }
                    _ if !block.text.trim().is_empty() => paragraphs.push(block.text),
                    _ => {}
                }
            }
            self.script_body = paragraphs.join("\n\n");
        } else {
            for block in legacy {
                if let BlockKind::Asset(id) = block.kind
                    && !self.script_assets.contains(&id)
                {
                    self.script_assets.push(id);
                }
            }
        }
    }

    #[cfg(test)]
    pub fn duration(&self) -> f64 {
        self.clips.iter().filter(|c| !c.audio_track).map(Clip::duration).sum()
    }

    pub fn asset(&self, id: u64) -> Option<&Asset> {
        self.assets.iter().chain(&self.removed_assets).find(|a| a.id == id)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Channel {
    pub id: u64,
    pub name: String,
    pub videos: Vec<Video>,
    #[serde(default)]
    pub codex_guide: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    #[serde(default)]
    pub pending_deletions: Vec<super::trash::Pending>,
    #[serde(default)]
    pub purged_assets: Vec<(u64, String)>,
    #[serde(default)]
    pub codex_guide: String,
    #[serde(default)]
    pub ui_font: String,
    #[serde(default = "default_font_size")]
    pub ui_font_size: f32,
    #[serde(default = "default_video_icon")]
    pub video_icon: String,
    #[serde(default)]
    pub trashed_channels: Vec<TrashedChannel>,
    #[serde(default)]
    pub codex_settings: super::codex_options::Settings,
    #[serde(default)]
    pub backup_directory: Option<PathBuf>,
    #[serde(default)]
    pub backup_id: String,
    #[serde(default)]
    pub revisions: Vec<Revision>,
    #[serde(default)]
    pub trash: Vec<TrashedVideo>,
    #[serde(default)]
    pub expanded_videos: Vec<u64>,
    #[serde(default)]
    pub collapsed_channels: Vec<u64>,
    #[serde(default)]
    pub codex_path: Option<PathBuf>,
    #[serde(default = "default_zoom")]
    pub ui_zoom: f32,
    pub version: u32,
    pub next_id: u64,
    pub selected_video: Option<u64>,
    pub channels: Vec<Channel>,
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            pending_deletions: Vec::new(),
            purged_assets: Vec::new(),
            codex_guide: String::new(),
            ui_font: String::new(),
            ui_font_size: default_font_size(),
            video_icon: default_video_icon(),
            trashed_channels: Vec::new(),
            codex_settings: Default::default(),
            backup_directory: None,
            backup_id: String::new(),
            version: 3,
            next_id: 1,
            selected_video: None,
            channels: Vec::new(),
            revisions: Vec::new(),
            trash: Vec::new(),
            expanded_videos: Vec::new(),
            collapsed_channels: Vec::new(),
            codex_path: None,
            ui_zoom: 1.0,
        }
    }
}

impl Workspace {
    pub fn id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn selected(&self) -> Option<(&Channel, &Video)> {
        self.channels.iter().find_map(|c| c.videos.iter().find(|v| Some(v.id) == self.selected_video).map(|v| (c, v)))
    }

    pub fn selected_mut(&mut self) -> Option<&mut Video> {
        let id = self.selected_video?;
        self.channels.iter_mut().find_map(|c| c.videos.iter_mut().find(|v| v.id == id))
    }

    pub fn add_channel(&mut self) -> u64 {
        let id = self.id();
        self.channels.push(Channel { id, name: format!("채널 {}", self.channels.len() + 1), videos: Vec::new(), codex_guide: String::new() });
        id
    }

    pub fn add_video(&mut self, channel_id: u64) -> Option<u64> {
        let id = self.id();
        let channel = self.channels.iter_mut().find(|c| c.id == channel_id)?;
        channel.videos.push(Video {
            id,
            name: format!("새 영상 {}", channel.videos.len() + 1),
            assets: Vec::new(),
            removed_assets: Vec::new(),
            script_title: format!("새 영상 {}", channel.videos.len() + 1),
            script_body: String::new(),
            script_font_size: 16.0,
            script_line_height: 1.6,
            film_revision: None,
            script_rich: Default::default(),
            script_assets: Vec::new(),
            script: Vec::new(),
            clips: Vec::new(),
            captions: Vec::new(),
        });
        self.expanded_videos.push(id);
        self.selected_video = Some(id);
        Some(id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Channel(u64),
    Video(u64),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrashedChannel {
    pub deleted_at: String,
    pub channel: Channel,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Revision {
    pub at: String,
    pub label: String,
    pub video: Video,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrashedVideo {
    pub channel_id: u64,
    pub channel_name: String,
    pub deleted_at: String,
    pub video: Video,
}

fn now() -> String {
    chrono::DateTime::<chrono::Utc>::from(std::time::SystemTime::now()).format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

impl Workspace {
    pub fn checkpoint(&mut self, label: &str) {
        self.prune_revisions(chrono::Utc::now());
        let Some((_, video)) = self.selected() else { return };
        if self.revisions.iter().rev().find(|r| r.video.id == video.id).is_some_and(|r| r.video == *video) {
            return;
        }
        let video = video.clone();
        self.revisions.push(Revision { at: now(), label: label.into(), video });
        self.prune_revisions(chrono::Utc::now());
    }

    pub fn rename(&mut self, item: Item, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() {
            return false;
        }
        match item {
            Item::Channel(id) => {
                if let Some(channel) = self.channels.iter_mut().find(|c| c.id == id) {
                    channel.name = name.to_owned();
                    return true;
                }
            }
            Item::Video(id) => {
                if let Some(video) = self.channels.iter_mut().flat_map(|c| &mut c.videos).find(|v| v.id == id) {
                    video.name = name.to_owned();
                    video.script_title = name.to_owned();
                    return true;
                }
            }
        }
        false
    }
    pub fn trash_channel(&mut self, id: u64) -> Vec<u64> {
        let Some(index) = self.channels.iter().position(|c| c.id == id) else { return Vec::new() };
        let channel = self.channels.remove(index);
        let ids: Vec<_> = channel.videos.iter().map(|v| v.id).collect();
        if self.selected_video.is_some_and(|id| ids.contains(&id)) {
            self.selected_video = None;
        }
        if let Some(previous) = self.trashed_channels.iter_mut().find(|c| c.channel.id == id) {
            previous.channel.name = channel.name;
            previous.channel.codex_guide = channel.codex_guide;
            previous.deleted_at = now();
            for video in channel.videos {
                if !previous.channel.videos.iter().any(|v| v.id == video.id) {
                    previous.channel.videos.push(video);
                }
            }
        } else {
            self.trashed_channels.push(TrashedChannel { deleted_at: now(), channel });
        }
        ids
    }
    pub fn restore_channel(&mut self, id: u64) {
        let Some(index) = self.trashed_channels.iter().position(|c| c.channel.id == id) else {
            return;
        };
        let item = self.trashed_channels.remove(index);
        if let Some(channel) = self.channels.iter_mut().find(|c| c.id == id) {
            channel.name = item.channel.name;
            channel.codex_guide = item.channel.codex_guide;
            for video in item.channel.videos {
                if !channel.videos.iter().any(|v| v.id == video.id) {
                    channel.videos.push(video);
                }
            }
        } else {
            self.channels.push(item.channel);
        }
        self.collapsed_channels.retain(|c| *c != id);
    }
    pub fn trash_video(&mut self, id: u64) {
        for channel in &mut self.channels {
            if let Some(index) = channel.videos.iter().position(|v| v.id == id) {
                let video = channel.videos.remove(index);
                self.trash.push(TrashedVideo { channel_id: channel.id, channel_name: channel.name.clone(), deleted_at: now(), video });
                if self.selected_video == Some(id) {
                    self.selected_video = None;
                }
                return;
            }
        }
    }

    pub fn restore_video(&mut self, id: u64) {
        let Some(index) = self.trash.iter().position(|t| t.video.id == id) else { return };
        let item = self.trash.remove(index);
        if !self.channels.iter().any(|c| c.id == item.channel_id) {
            self.channels.push(Channel { id: item.channel_id, name: item.channel_name, videos: Vec::new(), codex_guide: String::new() });
        }
        if let Some(channel) = self.channels.iter_mut().find(|c| c.id == item.channel_id) {
            channel.videos.push(item.video);
        }
    }
}

pub struct Store {
    pub root: PathBuf,
    pub data: Workspace,
    pub writable: bool,
    pub recovered: Option<String>,
}

impl Store {
    pub fn load(root: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&root).map_err(|e| format!("프로젝트 폴더를 만들 수 없습니다: {e}"))?;
        let path = root.join("workspace.json");
        let backup = root.join("workspace.json.bak");
        let previous = root.join("workspace.json.previous");
        let mut recovered = None;
        let mut errors = Vec::new();
        let mut loaded = None;
        for source in [&path, &backup, &previous] {
            if !source.exists() {
                continue;
            }
            let result =
                fs::read(source).map_err(|e| e.to_string()).and_then(|bytes| serde_json::from_slice::<Workspace>(&bytes).map_err(|e| e.to_string()));
            match result {
                Ok(data) => {
                    if source != &path {
                        if path.exists() {
                            let damaged = root.join(format!("workspace.corrupt-{}.json", chrono::Utc::now().format("%Y%m%d%H%M%S%f")));
                            fs::copy(&path, damaged).map_err(|e| format!("손상된 원본을 보존할 수 없습니다: {e}"))?;
                        }
                        recovered = Some("이전 정상 저장본에서 프로젝트를 복구했습니다".to_owned());
                    }
                    loaded = Some(data);
                    break;
                }
                Err(error) => errors.push(format!("{}: {error}", source.display())),
            }
        }
        let mut data = match loaded {
            Some(data) => data,
            None if errors.is_empty() => Workspace::default(),
            None => return Err(format!("프로젝트 목록과 복구본을 읽을 수 없습니다. 원본 파일은 보존되어 있습니다.\n{}", errors.join("\n"))),
        };
        let migrated = data.version < 3;
        let old_version = data.version;
        if migrated {
            for channel in &mut data.channels {
                for video in &mut channel.videos {
                    if old_version < 2 {
                        video.migrate_script();
                    }
                    if !video.script_title.trim().is_empty() && video.script_title != format!("{}의 대본", video.name) {
                        video.name = video.script_title.clone();
                    }
                    video.script_title = video.name.clone();
                }
            }
            data.version = 3;
            if let Some(id) = data.selected_video {
                data.expanded_videos.push(id);
            }
        }
        let store = Self { root, data, writable: true, recovered };
        if migrated || store.recovered.is_some() {
            store.save()?;
        }
        Ok(store)
    }

    pub fn save(&self) -> Result<(), String> {
        if !self.writable {
            return Err("프로젝트 저장소를 사용할 수 없습니다".into());
        }
        write_json(&self.root.join("workspace.json"), &self.data)
    }

    pub fn video_dir(&self, id: u64) -> PathBuf {
        self.root.join("videos").join(id.to_string())
    }

    pub fn design_path(&self, id: u64) -> PathBuf {
        self.video_dir(id).join("design.figcraft")
    }

    pub fn asset_path(&self, video_id: u64, asset: &Asset) -> PathBuf {
        self.video_dir(video_id).join("assets").join(&asset.file)
    }
}

fn default_script_size() -> f32 {
    16.0
}
fn default_line_height() -> f32 {
    1.6
}
fn default_zoom() -> f32 {
    1.0
}

fn default_font_size() -> f32 {
    14.0
}
fn default_video_icon() -> String {
    "clapperboard".into()
}

pub(super) fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let temp = path.with_extension("json.tmp");
    let previous = path.with_extension("json.previous");
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    // Keep a verified, rolling recovery copy. A damaged current file must never replace it.
    if let Ok(current) = fs::read(path)
        && serde_json::from_slice::<serde_json::Value>(&current).is_ok()
    {
        let backup = path.with_extension("json.bak");
        let pending = path.with_extension("json.bak.tmp");
        fs::write(&pending, current).map_err(|e| e.to_string())?;
        replace_json(&pending, &backup, &path.with_extension("json.bak.previous"))?;
    }
    replace_json(&temp, path, &previous)
}

fn replace_json(temp: &Path, path: &Path, previous: &Path) -> Result<(), String> {
    if fs::rename(temp, path).is_err() {
        if previous.exists() {
            if !path.exists() {
                return Err("이전 저장 복구본을 먼저 복구해야 합니다".into());
            }
            fs::remove_file(previous).map_err(|e| e.to_string())?;
        }
        fs::rename(path, previous).map_err(|e| e.to_string())?;
        if let Err(e) = fs::rename(temp, path) {
            let _ = fs::rename(previous, path);
            return Err(e.to_string());
        }
        let _ = fs::remove_file(previous);
    }
    Ok(())
}

pub fn file_name(path: &Path) -> String {
    path.file_name().and_then(|s| s.to_str()).unwrap_or("파일").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn corrupt_primary_recovers_verified_backup_and_preserves_the_damaged_file() {
        let root = std::env::temp_dir().join(format!("purpose-recover-manifest-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let mut store = Store::load(root.clone()).unwrap();
        let channel = store.data.add_channel();
        store.data.add_video(channel);
        store.data.selected_mut().unwrap().script_body = "정상 대본".into();
        store.save().unwrap();
        store.save().unwrap();
        fs::write(root.join("workspace.json"), b"{broken").unwrap();
        let recovered = Store::load(root.clone()).unwrap();
        assert!(recovered.recovered.is_some());
        assert_eq!(recovered.data.selected().unwrap().1.script_body, "정상 대본");
        assert!(fs::read_dir(&root).unwrap().filter_map(Result::ok).any(|f| f.file_name().to_string_lossy().starts_with("workspace.corrupt-")));
        let restored = Store::load(root.clone()).unwrap();
        assert!(restored.recovered.is_none());
        assert_eq!(restored.data.selected().unwrap().1.script_body, "정상 대본");
        let backup: Workspace = serde_json::from_slice(&fs::read(root.join("workspace.json.bak")).unwrap()).unwrap();
        assert_eq!(backup.selected().unwrap().1.script_body, "정상 대본");
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn no_valid_recovery_copy_does_not_overwrite_any_source() {
        let root = std::env::temp_dir().join(format!("purpose-recover-invalid-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("workspace.json"), b"{primary").unwrap();
        fs::write(root.join("workspace.json.bak"), b"{backup").unwrap();
        assert!(Store::load(root.clone()).is_err());
        assert_eq!(fs::read(root.join("workspace.json")).unwrap(), b"{primary");
        assert_eq!(fs::read(root.join("workspace.json.bak")).unwrap(), b"{backup");
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn channel_rename_trash_and_mixed_restore_keep_every_video_and_settings() {
        let mut ws = Workspace::default();
        let channel = ws.add_channel();
        let first = ws.add_video(channel).unwrap();
        let second = ws.add_video(channel).unwrap();
        assert!(ws.rename(Item::Channel(channel), " renamed channel "));
        assert!(!ws.rename(Item::Channel(channel), "  "));
        ws.selected_mut().unwrap().script_rich.insert_image(42, 0);
        ws.selected_mut().unwrap().script_rich.images[0].width = Some(320.0);
        ws.codex_settings = super::super::codex_options::Settings { model: "model-test".into(), effort: "high".into(), fast: true };
        ws.backup_directory = Some(PathBuf::from("backup"));
        ws.backup_id = "test".into();
        ws.trash_video(first);
        assert_eq!(ws.trash_channel(channel), vec![second]);
        assert!(ws.channels.is_empty());
        let mut ws: Workspace = serde_json::from_slice(&serde_json::to_vec(&ws).unwrap()).unwrap();
        ws.restore_video(first);
        ws.restore_channel(channel);
        ws.restore_channel(channel);
        assert_eq!(ws.channels.len(), 1);
        assert_eq!(ws.channels[0].name, "renamed channel");
        assert_eq!(ws.channels[0].videos.len(), 2);
        assert_eq!(ws.channels[0].videos.iter().find(|v| v.id == second).unwrap().script_rich.images[0].width, Some(320.0));
        assert_eq!(ws.codex_settings.model, "model-test");
        assert!(ws.codex_settings.fast);
        assert_eq!(ws.backup_id, "test");
        assert!(ws.trash.is_empty() && ws.trashed_channels.is_empty());
    }

    #[test]
    fn history_and_trash_round_trip_with_independent_expansion() {
        let mut ws = Workspace::default();
        let channel = ws.add_channel();
        let first = ws.add_video(channel).unwrap();
        ws.selected_mut().unwrap().script_body = "이전 대본".into();
        ws.checkpoint("첫 기록");
        ws.selected_mut().unwrap().script_body = "수정한 대본".into();
        ws.checkpoint("두 번째 기록");
        ws.checkpoint("중복 없음");
        assert_eq!(ws.revisions.len(), 2);
        let second = ws.add_video(channel).unwrap();
        assert!(ws.expanded_videos.contains(&first) && ws.expanded_videos.contains(&second));
        ws.trash_video(first);
        let bytes = serde_json::to_vec(&ws).unwrap();
        let mut loaded: Workspace = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(loaded.trash[0].video.script_body, "수정한 대본");
        assert_eq!(loaded.revisions[0].video.script_body, "이전 대본");
        loaded.restore_video(first);
        assert!(loaded.trash.is_empty());
        assert_eq!(loaded.channels[0].videos.iter().find(|v| v.id == first).unwrap().script_body, "수정한 대본");
    }

    #[test]
    fn videos_are_selected_and_clips_measure_the_timeline() {
        let mut ws = Workspace::default();
        let channel = ws.add_channel();
        let video = ws.add_video(channel).unwrap();
        assert_eq!(ws.selected().map(|(_, v)| v.id), Some(video));
        let v = ws.selected_mut().unwrap();
        v.clips.push(Clip { id: 3, asset_id: 4, trim_start: 2.0, trim_end: 7.0, start: 0.0, volume: 1.0, audio_track: false });
        v.clips.push(Clip { id: 5, asset_id: 6, trim_start: 0.0, trim_end: 10.0, start: 0.0, volume: 1.0, audio_track: true });
        assert_eq!(v.duration(), 5.0);
    }

    #[test]
    fn old_blocks_become_a_freeform_document_without_losing_asset_references() {
        let root = std::env::temp_dir().join(format!("purpose-script-migration-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("workspace.json"),
            serde_json::to_vec(&json!({
                "version": 1,
                "next_id": 4,
                "selected_video": 2,
                "channels": [{"id": 1, "name": "채널", "videos": [{
                    "id": 2, "name": "영상", "assets": [{"id": 3, "name": "사진.png", "file": "3.png", "kind": "Image", "duration": 5.0, "has_audio": false}],
                    "script": [
                        {"kind": "Heading", "text": "나의 대본"},
                        {"kind": "Paragraph", "text": "첫 문단"},
                        {"kind": "Scene", "text": "두 번째 장면"},
                        {"kind": {"Asset": 3}, "text": "참고 이미지"}
                    ], "clips": [], "captions": []
                }]}]
            }))
            .unwrap(),
        )
        .unwrap();
        let store = Store::load(root.clone()).unwrap();
        let (_, video) = store.data.selected().unwrap();
        assert_eq!(video.script_title, "나의 대본");
        assert_eq!(video.script_body, "첫 문단\n\n두 번째 장면\n\n참고 이미지");
        assert_eq!(video.script_assets, [3]);
        assert!(video.script.is_empty());
        let saved = fs::read_to_string(root.join("workspace.json")).unwrap();
        assert!(!saved.contains("\"script\""));
        assert_eq!(Store::load(root.clone()).unwrap().data.version, 3);
        let _ = fs::remove_dir_all(root);
    }
}
