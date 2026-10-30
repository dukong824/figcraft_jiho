//! Bounded, time-based history. Media and current projects are never garbage collected.
use super::{VideoMaker, model::Workspace};
use chrono::{DateTime, NaiveDateTime, Utc};
use std::{
    collections::HashSet,
    fs,
    path::Path,
    time::{Duration, Instant, SystemTime},
};

fn timestamp(at: &str) -> Option<i64> {
    NaiveDateTime::parse_from_str(at, "%Y-%m-%d %H:%M:%S UTC").ok().map(|at| at.and_utc().timestamp())
}
impl Workspace {
    pub(super) fn prune_revisions(&mut self, now: DateTime<Utc>) {
        let mut seen = HashSet::new();
        // Most recent version wins in each per-video time bucket, regardless of input order.
        self.revisions.sort_by_key(|r| timestamp(&r.at).unwrap_or(i64::MIN));
        self.revisions.reverse();
        self.revisions.retain(|r| {
            let Some(at) = timestamp(&r.at) else {
                return seen.insert((r.video.id, 0, i64::MIN, String::new()));
            };
            if at > now.timestamp().saturating_add(300) {
                return seen.insert((r.video.id, 0, i64::MAX, String::new()));
            }
            let age = now.timestamp().saturating_sub(at).max(0);
            let interval = match age {
                0..=604_800 => 300,
                604_801..=1_209_600 => 3_600,
                1_209_601..=2_592_000 => 86_400,
                _ => return false,
            };
            let event = if matches!(r.label.as_str(), "자동 저장" | "프로젝트 열기") { String::new() } else { r.label.clone() };
            seen.insert((r.video.id, interval, at.min(now.timestamp()).div_euclid(interval), event))
        });
        self.revisions.reverse();
    }
    fn snapshot_refs(&self, output: &mut HashSet<(u64, String)>) {
        let videos = self
            .channels
            .iter()
            .flat_map(|c| &c.videos)
            .chain(self.trashed_channels.iter().flat_map(|c| &c.channel.videos))
            .chain(self.trash.iter().map(|t| &t.video))
            .chain(self.revisions.iter().map(|r| &r.video));
        for video in videos {
            if let Some(name) = &video.film_revision {
                output.insert((video.id, name.clone()));
            }
        }
    }
}
pub(super) fn collect_snapshots(root: &Path) -> Result<usize, String> {
    let mut referenced = HashSet::new();
    for name in ["workspace.json", "workspace.json.bak", "workspace.json.previous"] {
        let path = root.join(name);
        if path.exists() {
            let data: Workspace = serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("기록 정리를 보류했습니다: {}: {e}", path.display()))?;
            data.snapshot_refs(&mut referenced);
        }
    }
    let videos = root.join("videos");
    if !videos.exists() || fs::symlink_metadata(&videos).map_err(|e| e.to_string())?.file_type().is_symlink() {
        return Ok(0);
    }
    let mut removed = 0;
    for dir in fs::read_dir(videos).map_err(|e| e.to_string())? {
        let dir = dir.map_err(|e| e.to_string())?;
        if !dir.file_type().map_err(|e| e.to_string())?.is_dir() {
            continue;
        }
        let Some(id) = dir.file_name().to_str().and_then(|s| s.parse::<u64>().ok()) else { continue };
        let snapshots = dir.path().join("edit-revisions");
        if !snapshots.is_dir() || fs::symlink_metadata(&snapshots).map_err(|e| e.to_string())?.file_type().is_symlink() {
            continue;
        }
        for file in fs::read_dir(snapshots).map_err(|e| e.to_string())? {
            let file = file.map_err(|e| e.to_string())?;
            if !file.file_type().map_err(|e| e.to_string())?.is_file() {
                continue;
            }
            let name = file.file_name().to_string_lossy().into_owned();
            let valid = name
                .strip_prefix("edit-")
                .and_then(|s| s.strip_suffix(".fcproj"))
                .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()));
            let old = file
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| SystemTime::now().duration_since(t).ok())
                .is_some_and(|age| age > Duration::from_secs(3600));
            if valid && old && !referenced.contains(&(id, name)) {
                fs::remove_file(file.path()).map_err(|e| e.to_string())?;
                removed += 1;
            }
        }
    }
    Ok(removed)
}
impl VideoMaker {
    pub(super) fn poll_retention(&mut self, ctx: &egui::Context) {
        if let Some(job) = &self.retention_job {
            match job.try_recv() {
                Ok(result) => {
                    if let Err(error) = result {
                        log::warn!("{error}");
                    }
                    self.retention_job = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.retention_job = None,
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(500));
                }
            }
        }
        if self.retention_job.is_none() && self.retention_at.is_none_or(|at| at.elapsed() >= Duration::from_secs(3600)) {
            self.store.data.prune_revisions(Utc::now());
            if let Err(error) = self.store.save() {
                self.message = error;
                return;
            }
            self.retention_at = Some(Instant::now());
            let root = self.store.root.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let _ = tx.send(collect_snapshots(&root));
            });
            self.retention_job = Some(rx);
        }
        if let Some(at) = self.retention_at {
            ctx.request_repaint_after(Duration::from_secs(3600).saturating_sub(at.elapsed()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::{Revision, Store};
    use super::*;
    #[test]
    fn retention_is_per_video_and_ages_from_minutes_to_hours_to_days() {
        let now = DateTime::parse_from_rfc3339("2026-10-30T12:02:00Z").unwrap().with_timezone(&Utc);
        let mut w = Workspace::default();
        let c = w.add_channel();
        let v = w.add_video(c).unwrap();
        let video = w.selected().unwrap().1.clone();
        for (days, seconds) in [(0, 0), (0, 30), (0, 600), (10, 0), (10, 30), (10, 7200), (20, 0), (20, 30), (20, 90000), (31, 0)] {
            w.revisions.push(Revision {
                at: (now - chrono::Duration::days(days) - chrono::Duration::seconds(seconds)).format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                label: "자동 저장".into(),
                video: video.clone(),
            });
        }
        let mut other = w.revisions.last().unwrap().clone();
        other.video.id = v + 100;
        other.at = now.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        w.revisions.push(other);
        w.prune_revisions(now);
        assert_eq!(w.revisions.iter().filter(|r| r.video.id == v).count(), 6);
        assert!(w.revisions.iter().all(|r| now.timestamp() - timestamp(&r.at).unwrap() <= 2_592_000));
        assert_eq!(w.revisions.len(), 7);
    }
    #[test]
    fn a_future_clock_record_does_not_displace_the_current_draft() {
        let now = Utc::now();
        let mut w = Workspace::default();
        let c = w.add_channel();
        w.add_video(c).unwrap();
        let video = w.selected().unwrap().1.clone();
        for time in [now, now + chrono::Duration::days(5), now + chrono::Duration::days(6)] {
            w.revisions.push(Revision { at: time.format("%Y-%m-%d %H:%M:%S UTC").to_string(), label: "자동 저장".into(), video: video.clone() });
        }
        w.prune_revisions(now);
        assert_eq!(w.revisions.len(), 2);
        assert!(w.revisions.iter().any(|r| timestamp(&r.at).unwrap() <= now.timestamp()));
    }
    #[cfg(unix)]
    #[test]
    fn collection_never_follows_a_linked_video_directory() {
        let root = std::env::temp_dir().join(format!("purpose-retention-link-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("workspace.json"), serde_json::to_vec(&Workspace::default()).unwrap()).unwrap();
        let outside = root.with_extension("external");
        let file = outside.join("2/edit-revisions/edit-1.fcproj");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, b"external").unwrap();
        fs::File::options().write(true).open(&file).unwrap().set_times(fs::FileTimes::new().set_modified(SystemTime::UNIX_EPOCH)).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("videos")).unwrap();
        assert_eq!(collect_snapshots(&root).unwrap(), 0);
        assert!(file.exists());
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }
    #[test]
    fn collection_preserves_current_trash_and_backup_refs_and_never_removes_media() {
        let root = std::env::temp_dir().join(format!("purpose-retention-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let mut s = Store::load(root.clone()).unwrap();
        let c = s.data.add_channel();
        let v = s.data.add_video(c).unwrap();
        let dir = s.video_dir(v).join("edit-revisions");
        fs::create_dir_all(&dir).unwrap();
        for name in ["edit-1.fcproj", "edit-2.fcproj", "edit-3.fcproj", "notes.fcproj"] {
            let p = dir.join(name);
            fs::write(&p, b"test").unwrap();
            fs::File::options().write(true).open(p).unwrap().set_times(fs::FileTimes::new().set_modified(SystemTime::UNIX_EPOCH)).unwrap();
        }
        s.data.selected_mut().unwrap().film_revision = Some("edit-1.fcproj".into());
        s.save().unwrap();
        s.data.selected_mut().unwrap().film_revision = Some("edit-2.fcproj".into());
        s.save().unwrap();
        assert_eq!(collect_snapshots(&root).unwrap(), 1);
        for name in ["edit-1.fcproj", "edit-2.fcproj", "notes.fcproj"] {
            assert!(dir.join(name).exists());
        }
        fs::remove_dir_all(root).unwrap();
    }
}
