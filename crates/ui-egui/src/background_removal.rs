//! Local inference is a platform service. Only the completed command edits the captured image.
use crate::*;
use services::{BackgroundEvent, BackgroundJob};

pub(crate) struct Pending {
    pub job: BackgroundJob,
    pub status: String,
    uid: u64,
    id: NodeId,
    index: usize,
    source: String,
}

impl FigApp {
    pub(crate) fn start_background_removal(&mut self) {
        if self.background_removal.is_some() {
            return;
        }
        let Ok(ds) = self.session.ds() else { return };
        let ids = &ds.selection;
        if ids.len() != 1 {
            self.toast("Select one image layer.");
            return;
        }
        let Some(&id) = ids.iter().next() else { return };
        let Ok(doc) = self.session.doc() else { return };
        let Some((index, img)) = doc.node(id).and_then(|n| {
            n.fills.iter().enumerate().find_map(|(i, p)| match &p.kind {
                figcraft_doc::color::PaintKind::Image(img) if p.visible => Some((i, img)),
                _ => None,
            })
        }) else {
            self.toast("Select a layer with an image fill.");
            return;
        };
        if img.background_removal.is_some() {
            self.toast("Restore the original before processing again.");
            return;
        }
        let Some(data) = doc.images.get(&img.image) else { return };
        let (uid, source, bytes) = (ds.uid, img.image.clone(), data.bytes.clone());
        match self.services.remove_background(bytes) {
            Ok(job) => self.background_removal = Some(Pending { job, uid, id, index, source, status: "Preparing BiRefNet…".into() }),
            Err(why) => self.show_error(why),
        }
    }
}

pub(crate) fn tick(app: &mut FigApp) {
    let Some(pending) = app.background_removal.as_mut() else { return };
    // Changing/closing documents cancels the captured job: a late result can never touch another file.
    if !app.session.ds().is_ok_and(|ds| ds.uid == pending.uid) {
        app.background_removal = None;
        return;
    }
    let mut finished = None;
    loop {
        match pending.job.events.try_recv() {
            Ok(BackgroundEvent::Progress(s)) => pending.status = s,
            Ok(BackgroundEvent::Finished(result)) => {
                finished = Some(result);
                break;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => break,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                finished = Some(Err("BiRefNet worker stopped.".into()));
                break;
            }
        }
    }
    let Some(result) = finished else { return };
    let Some(pending) = app.background_removal.take() else { return };
    match result {
        Ok(result) => {
            use base64::Engine as _;
            let b64 = base64::engine::general_purpose::STANDARD;
            if app.run("image.background.apply", json!({"id":pending.id,"index":pending.index,"source":pending.source,"image":b64.encode(result.png),"mask":b64.encode(result.mask)})).is_some() {
                app.toast(format!("Background removed · {} · Original resolution", result.backend));
            }
        }
        Err(why) => app.show_error(why),
    }
}
