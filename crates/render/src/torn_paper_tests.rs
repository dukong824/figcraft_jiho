use super::*;
use figcraft_color::{Color, Paint};
use figcraft_doc::{Document, Effect, EffectKind, NodeId, NodeKind, TornPaper};

fn fixture(kind: NodeKind, p: TornPaper) -> (Document, NodeId) {
    let mut doc = Document::new("torn paper");
    let page = doc.first_page().unwrap();
    let id = doc.create(page, kind, "Paper", 40.0, 30.0, 180.0, 130.0).unwrap();
    let n = doc.get_mut(id).unwrap();
    n.fills = vec![Paint::solid(Color::from_hex("CC6040").unwrap())];
    n.effects = vec![Effect { kind: EffectKind::TornPaper(p), visible: true }];
    (doc, id)
}

fn paint(doc: &Document, id: NodeId, origin: (f64, f64), width: u32, height: u32) -> Pixmap {
    let mut pix = Pixmap::new(width, height).unwrap();
    render_nodes_into(doc, &[id], &View { origin, scale: 1.0, width, height }, &mut pix);
    pix
}

fn pixel(p: &Pixmap, x: u32, y: u32) -> &[u8] {
    let i = ((y * p.width() + x) * 4) as usize;
    &p.data()[i..i + 4]
}

#[test]
fn torn_paper_cuts_a_varied_edge_preserves_interior_and_export_size() {
    let (doc, id) = fixture(NodeKind::Rectangle, TornPaper::default());
    let out = render_node(&doc, id, 1.0).unwrap();
    assert_eq!((out.width(), out.height()), (180, 130));
    assert_eq!(pixel(&out, 90, 65), [204, 96, 64, 255]);
    assert!(out.data().chunks_exact(4).any(|p| p[3] == 0));
    assert!(out.data().chunks_exact(4).any(|p| p[3] > 0 && p[3] < 255));
    assert!(out.data().chunks_exact(4).any(|p| p[3] == 255 && p[1] > 150), "paper cross-section is visible");
    let edge: Vec<_> = (20..160).map(|x| (0..30).find(|&y| pixel(&out, x, y)[3] > 127).unwrap()).collect();
    assert!(edge.iter().max().unwrap() - edge.iter().min().unwrap() >= 2, "torn edge is irregular, not a straight crop");
    for p in out.data().chunks_exact(4) {
        assert!(p[..3].iter().all(|&c| c <= p[3]));
    }
}

#[test]
fn torn_paper_seed_zero_depth_and_visibility_are_reversible() {
    let (mut doc, id) = fixture(NodeKind::Rectangle, TornPaper::default());
    let first = render_node(&doc, id, 1.0).unwrap();
    assert_eq!(first.data(), render_node(&doc, id, 1.0).unwrap().data());
    let EffectKind::TornPaper(p) = &mut doc.get_mut(id).unwrap().effects[0].kind else { panic!() };
    p.seed = 2;
    assert_ne!(first.data(), render_node(&doc, id, 1.0).unwrap().data());
    doc.get_mut(id).unwrap().effects[0].visible = false;
    let clean = render_node(&doc, id, 1.0).unwrap();
    let effect = &mut doc.get_mut(id).unwrap().effects[0];
    effect.visible = true;
    let EffectKind::TornPaper(p) = &mut effect.kind else { panic!() };
    p.depth = 0.0;
    assert_eq!(clean.data(), render_node(&doc, id, 1.0).unwrap().data());
}

#[test]
fn torn_paper_partial_views_and_rotated_shapes_keep_the_same_pattern() {
    for transform in [Affine::IDENTITY, Affine::rotate(0.19), Affine::scale(2.0)] {
        let (mut doc, id) = fixture(NodeKind::Rectangle, TornPaper::default());
        doc.get_mut(id).unwrap().transform *= transform;
        let full = paint(&doc, id, (-30.0, -20.0), 340, 260);
        let part = paint(&doc, id, (75.0, 35.0), 95, 95);
        for y in 0..95 {
            for x in 0..95 {
                for (a, b) in pixel(&full, x + 105, y + 55).iter().zip(pixel(&part, x, y)) {
                    assert!(a.abs_diff(*b) <= 1, "seam at {x},{y}, transform={transform:?}: {a}/{b}");
                }
            }
        }
    }
}

#[test]
fn torn_paper_shadows_follow_the_cut_pixels() {
    let (mut doc, id) = fixture(NodeKind::Rectangle, TornPaper::default());
    let mut shadow = Effect::drop_shadow();
    if let EffectKind::DropShadow(s) = &mut shadow.kind {
        s.spread = 3.0;
    }
    doc.get_mut(id).unwrap().effects.push(shadow);
    let n = doc.get(id).unwrap();
    assert!(crate::shadow::hiding_cover(&doc, n).is_none(), "an old rectangular outline must not hide the ripped notches");
    let layer = paint(&doc, id, (0.0, 0.0), 300, 220);
    let caster = crate::shadow::Caster::new(&doc, n, &layer, Affine::IDENTITY, 1.0);
    assert!(caster.silhouette.is_none(), "spread uses the new pixels, not the old vector outline");
}

#[test]
fn torn_paper_keeps_the_interior_of_translucent_paints() {
    let (mut doc, id) = fixture(NodeKind::Rectangle, TornPaper::default());
    doc.get_mut(id).unwrap().fills[0].opacity = 0.3;
    let ripped = render_node(&doc, id, 1.0).unwrap();
    doc.get_mut(id).unwrap().effects[0].visible = false;
    let clean = render_node(&doc, id, 1.0).unwrap();
    assert_eq!(pixel(&ripped, 90, 65), pixel(&clean, 90, 65));
    assert!(ripped.data().chunks_exact(4).filter(|p| p[3] > 0).count() > 10000);
}
