use crate::*;
use figcraft_color::{ImageScaleMode, Paint, PaintKind};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

struct Worker(Option<BackgroundJob>);
impl Services for Worker {
    fn remove_background(&mut self, _: Arc<Vec<u8>>) -> Result<BackgroundJob, String> {
        self.0.take().ok_or("No job".into())
    }
}

fn fixture() -> (FigApp, NodeId, String, mpsc::Sender<BackgroundEvent>, Arc<AtomicBool>, Vec<u8>) {
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(3, 2, image::Rgba([90, 120, 200, 128])).write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    let png = bytes.into_inner();
    let mut s = Session::new();
    let id = figcraft_engine::params::id_of(&s.execute("shape.rectangle", &json!({"width":100,"height":100})).unwrap()["id"]).unwrap();
    let (hash, _, _) = figcraft_format::add_image(s.doc_mut().unwrap(), png.clone()).unwrap();
    s.execute("node.set", &json!({"ids":[id],"props":{"fills":[Paint::image(&hash,ImageScaleMode::Fill)]}})).unwrap();
    s.ds_mut().unwrap().selection = vec![id];
    let (tx, events) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let app = FigApp::new(s, Box::new(Worker(Some(BackgroundJob { events, cancel: cancel.clone() }))));
    (app, id, hash, tx, cancel, png)
}

#[test]
fn background_removal_completion_edits_the_captured_image_after_selection_moves() {
    let (mut app, id, original, tx, _, png) = fixture();
    app.run("image.removeBackground", json!({}));
    assert!(app.background_removal.is_some());
    app.session.ds_mut().unwrap().selection.clear();
    tx.send(BackgroundEvent::Finished(Ok(BackgroundResult { png: png.clone(), mask: png, backend: "test".into() }))).unwrap();
    background_removal::tick(&mut app);
    assert!(app.background_removal.is_none());
    let PaintKind::Image(img) = &app.session.doc().unwrap().get(id).unwrap().fills[0].kind else { panic!() };
    assert_eq!(img.background_removal.as_ref().unwrap().original, original);
    app.session.undo().unwrap();
    let PaintKind::Image(img) = &app.session.doc().unwrap().get(id).unwrap().fills[0].kind else { panic!() };
    assert!(img.background_removal.is_none());
}

#[test]
fn background_removal_document_switch_cancels_and_ignores_the_late_result() {
    let (mut app, _, _, tx, cancel, png) = fixture();
    app.run("image.removeBackground", json!({}));
    app.session.execute("file.new", &json!({})).unwrap();
    let before = app.session.doc().unwrap().clone();
    tx.send(BackgroundEvent::Finished(Ok(BackgroundResult { png: png.clone(), mask: png, backend: "test".into() }))).unwrap();
    background_removal::tick(&mut app);
    assert!(app.background_removal.is_none());
    assert!(cancel.load(Ordering::Relaxed));
    assert_eq!(app.session.doc().unwrap(), &before);
}
