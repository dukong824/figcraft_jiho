//! Codex operates the same live commands as the editors; the script remains suggestion-only.
use super::{VideoMaker, agent_transport::Bridge, model::Section, navigation::Route};
use base64::Engine as _;
use figcraft_ui_egui::FigApp;
use serde_json::{Value, json};
use std::{sync::atomic::Ordering, time::Instant};

fn allowed(target: &str, command: &str) -> bool {
    match target {
        "design" => {
            let blocked = [
                "file.",
                "doc.",
                "plugin.",
                "plugins.",
                "collab.",
                "live.",
                "online.",
                "app.",
                "publish.",
                "settings.",
                "preferences.",
                "command.",
                "library.",
            ];
            command != "edit.repeat" && !blocked.iter().any(|prefix| command.starts_with(prefix)) && figcraft_engine::find_command(command).is_some()
        }
        "edit" => {
            let prefixes = [
                "timeline.",
                "clip.",
                "captions.",
                "sequence.",
                "effects.",
                "graphics.",
                "color.",
                "edit.",
                "select.",
                "markers.",
                "marker.",
                "track.",
                "audio.",
                "mixer.",
                "multicam.",
                "motion.",
                "transition.",
            ];
            (prefixes.iter().any(|prefix| command.starts_with(prefix)) || command == "file.newSequence")
                && filmcraft_engine::find_command(command).is_some()
        }
        _ => false,
    }
}
fn ids(arguments: &Value) -> Result<Vec<u64>, String> {
    let values = arguments["ids"].as_array().ok_or("자료 id 목록이 필요합니다")?;
    if values.is_empty() || values.len() > 128 {
        return Err("한 번에 1~128개를 선택하세요".into());
    }
    values.iter().map(|v| v.as_u64().ok_or("잘못된 자료 id입니다".into())).collect()
}
impl VideoMaker {
    pub fn agent_logic(&mut self, fig: &mut FigApp, ctx: &egui::Context) {
        if self.chat_job.as_ref().is_some_and(|job| job.cancelled()) {
            self.agent = None;
            return;
        }
        let pending = self.agent.as_ref().map(Bridge::take_pending).unwrap_or_default();
        for request in pending {
            if request.cancelled.load(Ordering::Relaxed) || Instant::now() >= request.deadline {
                continue;
            }
            let result = self.agent_tool(fig, &request.name, &request.arguments);
            let _ = request.reply.send(result);
            ctx.request_repaint();
        }
    }
    fn agent_focus(&mut self, fig: &mut FigApp, video: u64, section: Section) -> Result<(), String> {
        if !self.store.data.channels.iter().flat_map(|c| &c.videos).any(|v| v.id == video) {
            return Err("이 작업의 영상이 휴지통으로 이동되었습니다".into());
        }
        let route = Route::Video { video_id: video, section };
        if self.tabs.route() != Some(route) {
            self.navigate(route);
        }
        match section {
            Section::Design => {
                if fig.session.ds().ok().and_then(|ds| ds.path.as_deref()).map(std::path::Path::new) != Some(self.store.design_path(video).as_path())
                {
                    self.design_loaded = None;
                }
                self.ensure_design(fig);
                if self.design_loaded != Some(video) {
                    return Err("대상 영상의 디자인을 열 수 없습니다".into());
                }
            }
            Section::Edit => self.ensure_film()?,
            _ => {}
        }
        Ok(())
    }
    pub(super) fn agent_tool(&mut self, fig: &mut FigApp, name: &str, arguments: &Value) -> Result<Value, String> {
        let video = self.agent.as_ref().ok_or("Codex 작업 연결이 종료되었습니다")?.video;
        let guide = self.guide_text(video)?;
        if name == "read_guide" {
            if let Some(agent) = &mut self.agent {
                agent.guide = Some(guide.clone());
            }
            let (channel, target) = self
                .store
                .data
                .channels
                .iter()
                .find_map(|c| c.videos.iter().find(|v| v.id == video).map(|v| (c, v)))
                .ok_or("영상을 찾을 수 없습니다")?;
            return Ok(
                json!({"video_id":video,"video":target.name,"channel":channel.name,"guide":guide,"policy":"대본과 가이드는 읽기 전용입니다. 대본 수정은 답변으로 제안하세요. 디자인·편집·라이브러리는 앱 도구로 자율적으로 수정할 수 있습니다. 모든 변경은 이 영상에만 적용됩니다."}),
            );
        }
        if self.agent.as_ref().and_then(|a| a.guide.as_deref()) != Some(guide.as_str()) {
            return Err("먼저 read_guide로 최신 공통·채널 가이드를 읽으세요".into());
        }
        match name {
            "inspect_project" => {
                let target = arguments["target"].as_str().unwrap_or("all");
                let mut result = json!({"video_id":video,"imports":{"busy":self.imports.busy(),"completed":self.imports.completed,"total":self.imports.total,"errors":self.imports.errors}});
                if target == "all" || target == "design" {
                    self.agent_focus(fig, video, Section::Design)?;
                    result["design"] = figcraft_engine::inspect::document(&fig.session, 12, true);
                }
                if target == "all" || target == "edit" {
                    self.agent_focus(fig, video, Section::Edit)?;
                    if let Some(editor) = &self.film {
                        result["edit"] = serde_json::from_str(&editor.app.session.project.to_json()).map_err(|e| e.to_string())?;
                        result["edit_state"] = json!(editor.app.session.state);
                    }
                }
                let v = self.store.data.channels.iter().flat_map(|c| &c.videos).find(|v| v.id == video).ok_or("영상을 찾을 수 없습니다")?;
                if target == "all" || target == "library" {
                    result["selected_assets"] = if self.video_id() == Some(video) { json!(self.library_selection) } else { json!([]) };
                    result["library"] = json!(
                        v.assets
                            .iter()
                            .map(
                                |a| json!({"id":a.id,"name":a.name,"path":self.store.asset_path(video,a),"kind":a.kind.label(),"duration":a.duration})
                            )
                            .collect::<Vec<_>>()
                    );
                    result["trash"] = json!(v.removed_assets);
                }
                if target == "all" || target == "script" {
                    result["script"] = json!({"title":v.name,"body":v.script_body,"read_only":true});
                }
                Ok(result)
            }
            "list_commands" => {
                let target = arguments["target"].as_str().ok_or("design 또는 edit 대상이 필요합니다")?;
                let query = arguments["query"].as_str().unwrap_or_default().to_lowercase();
                let commands = match target {
                    "design" => figcraft_engine::all_commands()
                        .iter()
                        .filter(|c| allowed(target, c.id) && (c.id.to_lowercase().contains(&query) || c.label.to_lowercase().contains(&query)))
                        .map(|c| json!({"id":c.id,"label":c.label,"params":c.params,"undoable":c.history}))
                        .collect::<Vec<_>>(),
                    "edit" => filmcraft_engine::command_specs()
                        .iter()
                        .filter(|c| allowed(target, c.id) && (c.id.to_lowercase().contains(&query) || c.label.to_lowercase().contains(&query)))
                        .map(|c| json!({"id":c.id,"label":c.label,"params":c.params}))
                        .collect(),
                    _ => return Err("대본은 제안만 할 수 있습니다. 대상은 design 또는 edit이어야 합니다".into()),
                };
                Ok(json!({"target":target,"commands":commands}))
            }
            "run_command" => {
                let target = arguments["target"].as_str().unwrap_or_default();
                let command = arguments["command"].as_str().ok_or("명령이 필요합니다")?;
                if !allowed(target, command) {
                    return Err("대상 편집기의 편집 명령만 실행할 수 있습니다. list_commands를 확인하세요. 대본은 제안 전용입니다".into());
                }
                let params = arguments.get("params").cloned().unwrap_or(json!({}));
                if target == "design" {
                    self.agent_focus(fig, video, Section::Design)?;
                    let result = fig.session.execute(command, &params).map_err(|e| e.to_string())?;
                    fig.session.execute("file.save", &json!({})).map_err(|e| e.to_string())?;
                    Ok(result)
                } else {
                    self.agent_focus(fig, video, Section::Edit)?;
                    let result = self.film.as_mut().ok_or("편집기가 없습니다")?.app.session.execute(command, params).map_err(|e| e.to_string())?;
                    self.save_film()?;
                    Ok(result)
                }
            }
            "library_import" => {
                let paths = arguments["paths"].as_array().ok_or("파일 경로 목록이 필요합니다")?;
                if paths.is_empty() || paths.len() > 64 {
                    return Err("한 번에 1~64개의 파일을 가져오세요".into());
                }
                let mut queued = Vec::new();
                for path in paths {
                    queued.push(self.queue_import(video, std::path::PathBuf::from(path.as_str().ok_or("잘못된 파일 경로입니다")?), None)?);
                }
                Ok(json!({"queued":queued}))
            }
            "library_rename" => {
                self.rename_asset(
                    video,
                    arguments["id"].as_u64().ok_or("자료 id가 필요합니다")?,
                    arguments["name"].as_str().ok_or("이름이 필요합니다")?,
                )?;
                Ok(json!({"renamed":true}))
            }
            "library_remove" => {
                let ids = ids(arguments)?;
                self.remove_assets(video, &ids)?;
                Ok(json!({"removed":ids}))
            }
            "library_copy" => {
                let ids = ids(arguments)?;
                let sources = ids
                    .iter()
                    .map(|id| {
                        let asset = self
                            .store
                            .data
                            .channels
                            .iter()
                            .flat_map(|c| &c.videos)
                            .find(|v| v.id == video)
                            .and_then(|v| v.asset(*id))
                            .ok_or("자료를 찾을 수 없습니다")?;
                        Ok((self.store.asset_path(video, asset), asset.name.clone()))
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                let mut queued = Vec::new();
                for (path, name) in sources {
                    queued.push(self.queue_import(video, path, Some(name))?);
                }
                Ok(json!({"queued":queued}))
            }
            "library_restore" => {
                let ids = ids(arguments)?;
                let v = self.store.data.channels.iter_mut().flat_map(|c| &mut c.videos).find(|v| v.id == video).ok_or("영상을 찾을 수 없습니다")?;
                for id in &ids {
                    if let Some(index) = v.removed_assets.iter().position(|a| a.id == *id) {
                        v.assets.push(v.removed_assets.remove(index));
                    }
                }
                self.save();
                Ok(json!({"restored":ids}))
            }
            "library_place" => {
                let id = arguments["id"].as_u64().ok_or("자료 id가 필요합니다")?;
                match arguments["target"].as_str() {
                    Some("design") => {
                        let asset = self
                            .store
                            .data
                            .channels
                            .iter()
                            .flat_map(|c| &c.videos)
                            .find(|v| v.id == video)
                            .and_then(|v| v.asset(id))
                            .cloned()
                            .ok_or("자료가 없습니다")?;
                        if !matches!(asset.kind, super::model::AssetKind::Image | super::model::AssetKind::Design) {
                            return Err("디자인에는 이미지 또는 디자인 파일을 배치하세요".into());
                        }
                        self.agent_focus(fig, video, Section::Design)?;
                        let result =
                            fig.session.execute("file.import", &json!({"path":self.store.asset_path(video,&asset)})).map_err(|e| e.to_string())?;
                        fig.session.execute("file.save", &json!({})).map_err(|e| e.to_string())?;
                        Ok(result)
                    }
                    Some("edit") => {
                        self.agent_focus(fig, video, Section::Edit)?;
                        self.append_film_asset(id)?;
                        Ok(json!({"placed":id}))
                    }
                    _ => Err("배치 대상은 design 또는 edit이어야 합니다. 대본은 제안 전용입니다".into()),
                }
            }
            "export_design" => {
                self.agent_focus(fig, video, Section::Design)?;
                let before = self.video().map_or(0, |v| v.assets.len());
                self.export_design(fig);
                let assets = self.video().map(|v| v.assets.iter().skip(before).collect::<Vec<_>>()).unwrap_or_default();
                if assets.is_empty() {
                    return Err(self.message.clone());
                }
                Ok(json!({"assets":assets}))
            }
            "design_preview" => {
                self.agent_focus(fig, video, Section::Design)?;
                let path = self.store.video_dir(video).join(".agent-preview.png");
                let params = json!({"format":"png","path":path,"ids":arguments.get("ids").cloned().unwrap_or(json!([])),"scale":1.0});
                fig.session.execute("file.export", &params).map_err(|e| e.to_string())?;
                let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
                let _ = std::fs::remove_file(path);
                if bytes.len() > 10 * 1024 * 1024 {
                    return Err("미리보기가 너무 큽니다. 특정 레이어를 지정하세요".into());
                }
                Ok(json!({"_image":base64::engine::general_purpose::STANDARD.encode(bytes)}))
            }
            _ => Err("알 수 없는 앱 도구입니다".into()),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guided_agent_edits_design_library_and_timeline_but_keeps_script_read_only() {
        let root = std::env::temp_dir().join(format!("purpose-agent-live-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut maker = VideoMaker::at(root.clone()).unwrap();
        let video = maker.video_id().unwrap();
        maker.store.data.codex_guide = "공통 제작 원칙".into();
        maker.store.data.channels[0].codex_guide = "친근한 말투, 빠른 편집".into();
        maker.store.data.selected_mut().unwrap().script_body = "사용자가 쓴 대본".into();
        maker.save();
        maker.agent = Some(Bridge::start(video, egui::Context::default()).unwrap());
        let mut fig = FigApp::new(figcraft_engine::Session::new(), Box::new(figcraft_ui_egui::NoServices));
        assert!(maker.agent_tool(&mut fig, "run_command", &json!({"target":"design","command":"shape.rectangle"})).is_err());
        let guide = maker.agent_tool(&mut fig, "read_guide", &json!({})).unwrap();
        assert!(guide["guide"].as_str().unwrap().contains("친근한 말투"));
        let channel = maker.store.data.channels[0].id;
        let other = maker.store.data.add_video(channel).unwrap();
        maker.navigate(Route::Video { video_id: other, section: Section::Library });
        maker
            .agent_tool(
                &mut fig,
                "run_command",
                &json!({"target":"design","command":"shape.rectangle","params":{"x":0,"y":0,"width":96,"height":64}}),
            )
            .unwrap();
        let layer = fig.session.selection().to_vec();
        maker
            .agent_tool(
                &mut fig,
                "run_command",
                &json!({"target":"design","command":"node.set","params":{"ids":layer,"props":{"name":"에이전트 타이틀"}}}),
            )
            .unwrap();
        let exported = maker.agent_tool(&mut fig, "export_design", &json!({})).unwrap();
        let id = exported["assets"][0]["id"].as_u64().unwrap();
        assert_eq!(exported["assets"][0]["name"], "에이전트 타이틀.png");
        maker.agent_tool(&mut fig, "library_rename", &json!({"id":id,"name":"오프닝 타이틀.png"})).unwrap();
        maker.agent_tool(&mut fig, "library_place", &json!({"id":id,"target":"edit"})).unwrap();
        let editor = maker.film.as_ref().unwrap();
        assert_eq!(editor.app.session.active_sequence().unwrap().video_tracks[0].items.len(), 1);
        assert!(editor.app.session.project.items.values().any(|item| item.name == "오프닝 타이틀.png"));
        maker.agent_tool(&mut fig, "library_remove", &json!({"ids":[id]})).unwrap();
        assert!(maker.video().unwrap().assets.is_empty());
        assert!(maker.store.asset_path(video, maker.video().unwrap().asset(id).unwrap()).exists());
        maker.agent_tool(&mut fig, "library_restore", &json!({"ids":[id]})).unwrap();
        assert!(maker.agent_tool(&mut fig, "run_command", &json!({"target":"script","command":"node.set"})).is_err());
        assert_eq!(maker.video().unwrap().script_body, "사용자가 쓴 대본");
        assert_eq!(maker.video_id(), Some(video));
        assert!(!maker.store.design_path(other).exists());
        assert!(maker.store.data.channels[0].videos.iter().find(|v| v.id == other).unwrap().assets.is_empty());
        maker.store.data.channels[0].codex_guide.push_str(" 새 지침");
        assert!(maker.agent_tool(&mut fig, "run_command", &json!({"target":"design","command":"shape.rectangle"})).is_err());
        maker.shutdown_film();
        let _ = std::fs::remove_dir_all(root);
    }
    #[test]
    fn command_boundary_exposes_editing_but_not_file_replacement_or_script_writes() {
        assert!(allowed("design", "shape.rectangle"));
        assert!(allowed("design", "node.set"));
        assert!(allowed("edit", "clip.rename"));
        assert!(allowed("edit", "captions.add"));
        for (target, id) in
            [("design", "file.open"), ("design", "command.batch"), ("edit", "file.open"), ("script", "node.set"), ("edit", "file.save")]
        {
            assert!(!allowed(target, id));
        }
    }
}
