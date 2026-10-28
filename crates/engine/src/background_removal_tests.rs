use crate::{Session, params::id_of};
use base64::Engine as _;
use figcraft_color::{ImageScaleMode, Paint, PaintKind};
use serde_json::json;

fn png(w: u32, h: u32, alpha: u8) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(w, h, image::Rgba([32, 100, 240, alpha]));
    let mut out = std::io::Cursor::new(Vec::new());
    image.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

fn setup() -> (Session, figcraft_doc::NodeId, String) {
    let mut s = Session::new();
    let id = id_of(&s.execute("shape.rectangle", &json!({"width":100,"height":80})).unwrap()["id"]).unwrap();
    let (hash, _, _) = figcraft_format::add_image(s.doc_mut().unwrap(), png(7, 5, 128)).unwrap();
    let mut paint = Paint::image(&hash, ImageScaleMode::Crop);
    if let PaintKind::Image(img) = &mut paint.kind {
        img.crop = Some([0.8, 0.0, 0.0, 0.9, 0.1, 0.05]);
        img.filters.contrast = 0.2;
    }
    s.execute("node.set", &json!({"ids":[id],"props":{"fills":[paint]}})).unwrap();
    (s, id, hash)
}

fn params(id: figcraft_doc::NodeId, source: &str, size: (u32, u32)) -> serde_json::Value {
    let png = base64::engine::general_purpose::STANDARD.encode(png(size.0, size.1, 64));
    json!({"id":id,"index":0,"source":source,"image":png,"mask":png})
}

#[test]
fn background_removal_roundtrips_original_mask_crop_and_one_undo() {
    let (mut s, id, source) = setup();
    let before = s.doc().unwrap().get(id).unwrap().fills.clone();
    let original_bytes = s.doc().unwrap().images[&source].bytes.clone();
    s.execute("image.background.apply", &params(id, &source, (7, 5))).unwrap();
    let doc = s.doc().unwrap();
    let PaintKind::Image(img) = &doc.get(id).unwrap().fills[0].kind else { panic!() };
    let bg = img.background_removal.as_ref().unwrap();
    assert_eq!(bg.original, source);
    assert!(figcraft_doc::used_images(doc).contains(&bg.mask));
    assert!(figcraft_doc::used_images(doc).contains(&source));
    assert_eq!(doc.images[&source].bytes, original_bytes);
    assert_eq!((doc.images[&img.image].width, doc.images[&img.image].height), (7, 5));
    let bytes = figcraft_format::save_bytes(doc, None).unwrap();
    let (loaded, warnings) = figcraft_format::load_bytes(&bytes).unwrap();
    assert!(warnings.is_empty());
    assert_eq!(loaded.get(id).unwrap().fills, doc.get(id).unwrap().fills);
    assert_eq!(loaded.images[&source].bytes, original_bytes);
    s.undo().unwrap();
    assert_eq!(s.doc().unwrap().get(id).unwrap().fills, before);
    s.redo().unwrap();
    s.execute("image.background.restore", &json!({"id":id,"index":0})).unwrap();
    assert_eq!(s.doc().unwrap().get(id).unwrap().fills, before);
    s.undo().unwrap();
    let PaintKind::Image(img) = &s.doc().unwrap().get(id).unwrap().fills[0].kind else { panic!() };
    assert!(img.background_removal.is_some());
}

#[test]
fn background_removal_rejects_stale_paints_wrong_dimensions_and_corrupt_data_atomically() {
    let (mut s, id, source) = setup();
    let before = s.doc().unwrap().clone();
    for p in [
        params(id, "stale-image-hash", (7, 5)),
        params(id, &source, (4, 3)),
        json!({"id":id,"index":0,"source":source,"image":"broken","mask":"broken"}),
    ] {
        assert!(s.execute("image.background.apply", &p).is_err());
        assert_eq!(s.doc().unwrap(), &before);
    }
}
