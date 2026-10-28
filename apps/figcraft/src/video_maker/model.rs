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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Asset {
    pub id: u64,
    pub name: String,
    /// A file name in this video's assets directory, never an external absolute path.
    pub file: String,
    pub kind: AssetKind,
    pub duration: f64,
    pub has_audio: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BlockKind {
    Heading,
    Paragraph,
    Scene,
    Asset(u64),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScriptBlock {
    pub kind: BlockKind,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Caption {
    pub id: u64,
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Video {
    pub id: u64,
    pub name: String,
    pub assets: Vec<Asset>,
    pub script: Vec<ScriptBlock>,
    pub clips: Vec<Clip>,
    pub captions: Vec<Caption>,
}

impl Video {
    pub fn duration(&self) -> f64 {
        self.clips.iter().filter(|c| !c.audio_track).map(Clip::duration).sum()
    }

    pub fn asset(&self, id: u64) -> Option<&Asset> {
        self.assets.iter().find(|a| a.id == id)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Channel {
    pub id: u64,
    pub name: String,
    pub videos: Vec<Video>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub version: u32,
    pub next_id: u64,
    pub selected_video: Option<u64>,
    pub channels: Vec<Channel>,
}

impl Default for Workspace {
    fn default() -> Self {
        Self { version: 1, next_id: 1, selected_video: None, channels: Vec::new() }
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
            script: vec![
                ScriptBlock { kind: BlockKind::Heading, text: "영상 제목".into() },
                ScriptBlock { kind: BlockKind::Paragraph, text: String::new() },
            ],
            clips: Vec::new(),
            captions: Vec::new(),
        });
        self.selected_video = Some(id);
        Some(id)
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
        let data = if source.exists() {
            let bytes = fs::read(source).map_err(|e| format!("프로젝트 목록을 읽을 수 없습니다: {e}"))?;
            serde_json::from_slice(&bytes).map_err(|e| format!("프로젝트 목록이 손상되었습니다: {e}"))?
        } else {
            Workspace::default()
        };
        Ok(Self { root, data, writable: true })
    }

    pub fn save(&self) -> Result<(), String> {
        if !self.writable {
            return Err("프로젝트 저장소를 사용할 수 없습니다".into());
        }
        let path = self.root.join("workspace.json");
        let temp = self.root.join("workspace.json.tmp");
        let backup = self.root.join("workspace.json.bak");
        let bytes = serde_json::to_vec_pretty(&self.data).map_err(|e| e.to_string())?;
        fs::write(&temp, bytes).map_err(|e| e.to_string())?;
        if fs::rename(&temp, &path).is_err() {
            // Windows cannot always rename over an existing destination. Keep the old
            // file recoverable until the new file has taken its place.
            if backup.exists() {
                return Err("이전 프로젝트 백업이 남아 있어 저장을 중단했습니다".into());
            }
            fs::rename(&path, &backup).map_err(|e| e.to_string())?;
            if let Err(e) = fs::rename(&temp, &path) {
                let _ = fs::rename(&backup, &path);
                return Err(e.to_string());
            }
            let _ = fs::remove_file(&backup);
        }
        Ok(())
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

pub fn file_name(path: &Path) -> String {
    path.file_name().and_then(|s| s.to_str()).unwrap_or("파일").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
