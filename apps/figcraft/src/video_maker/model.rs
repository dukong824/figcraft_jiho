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
        self.assets.iter().find(|a| a.id == id)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Channel {
    pub id: u64,
    pub name: String,
    pub videos: Vec<Video>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    #[serde(default)]
    pub revisions: Vec<Revision>,
    #[serde(default)]
    pub trash: Vec<TrashedVideo>,
    #[serde(default)]
    pub expanded_videos: Vec<u64>,
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
            version: 3,
            next_id: 1,
            selected_video: None,
            channels: Vec::new(),
            revisions: Vec::new(),
            trash: Vec::new(),
            expanded_videos: Vec::new(),
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
        self.channels.push(Channel { id, name: format!("채널 {}", self.channels.len() + 1), videos: Vec::new() });
        id
    }

    pub fn add_video(&mut self, channel_id: u64) -> Option<u64> {
        let id = self.id();
        let channel = self.channels.iter_mut().find(|c| c.id == channel_id)?;
        channel.videos.push(Video {
            id,
            name: format!("새 영상 {}", channel.videos.len() + 1),
            assets: Vec::new(),
            script_title: format!("새 영상 {}", channel.videos.len() + 1),
            script_body: String::new(),
            script_font_size: 16.0,
            script_line_height: 1.6,
            film_revision: None,
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
        let Some((_, video)) = self.selected() else { return };
        if self.revisions.iter().rev().find(|r| r.video.id == video.id).is_some_and(|r| r.video == *video) {
            return;
        }
        let video = video.clone();
        self.revisions.push(Revision { at: now(), label: label.into(), video });
        if self.revisions.len() > 100 {
            self.revisions.remove(0);
        }
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
            self.channels.push(Channel { id: item.channel_id, name: item.channel_name, videos: Vec::new() });
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
}

impl Store {
    pub fn load(root: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&root).map_err(|e| format!("프로젝트 폴더를 만들 수 없습니다: {e}"))?;
        let path = root.join("workspace.json");
        let backup = root.join("workspace.json.bak");
        let source = if path.exists() { &path } else { &backup };
        let mut data: Workspace = if source.exists() {
            let bytes = fs::read(source).map_err(|e| format!("프로젝트 목록을 읽을 수 없습니다: {e}"))?;
            serde_json::from_slice(&bytes).map_err(|e| format!("프로젝트 목록이 손상되었습니다: {e}"))?
        } else {
            Workspace::default()
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
        let store = Self { root, data, writable: true };
        if migrated {
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

pub(super) fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let temp = path.with_extension("json.tmp");
    let backup = path.with_extension("json.bak");
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    if fs::rename(&temp, path).is_err() {
        if backup.exists() {
            return Err("이전 백업이 남아 있어 저장을 중단했습니다".into());
        }
        fs::rename(path, &backup).map_err(|e| e.to_string())?;
        if let Err(e) = fs::rename(&temp, path) {
            let _ = fs::rename(&backup, path);
            return Err(e.to_string());
        }
        let _ = fs::remove_file(backup);
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
