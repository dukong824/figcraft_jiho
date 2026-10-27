//! Custom sliders driven through the real panel: live values, one undo, numeric entry and steps.
use egui::{Event, Id, Key, Rect, pos2, vec2};
use figcraft_doc::{Effect, EffectKind, NodeId, PaperFilterKind};
use figcraft_engine::Session;
use serde_json::json;

use crate::layers_drag_tests::Ui;

fn setup(effect: Effect, interleaved: bool) -> (Ui, NodeId, usize) {
    let mut s = Session::new();
    let r = NodeId(s.execute("shape.rectangle", &json!({"width":60,"height":60})).unwrap()["id"].as_u64().unwrap());
    if interleaved {
        s.execute("effect.add", &json!({"ids":[r],"type":"dropShadow"})).unwrap();
    }
    s.execute("effect.add", &json!({"ids":[r],"effect":effect})).unwrap();
    s.execute("select.set", &json!({"ids":[r]})).unwrap();
    let index = usize::from(interleaved);
    let mut u = Ui::new(s);
    u.ctx.enable_accesskit();
    u.ctx.data_mut(|d| d.insert_temp(Id::unique(("fx_open", index)), true));
    u.frame(vec![]);
    (u, r, index)
}

fn slider(u: &mut Ui, label: &str, field: Id) -> (Rect, f64) {
    for _ in 0..40 {
        let out = u.frame_output(vec![]);
        if let Some(node) = out
            .platform_output
            .accesskit_update
            .unwrap()
            .nodes
            .into_iter()
            .map(|(_, n)| n)
            .find(|n| n.role() == egui::accesskit::Role::Slider && n.label() == Some(label))
            && let Some(bounds) = node.bounds()
        {
            let rect = Rect::from_min_max(pos2(bounds.x0 as f32, bounds.y0 as f32), pos2(bounds.x1 as f32, bounds.y1 as f32));
            if rect.min.y > 160.0 && rect.max.y < 865.0 {
                return (rect, node.numeric_value().unwrap());
            }
        }
        let y = u.ctx.read_response(field.with("field")).unwrap().rect.center().y;
        u.frame(vec![
            Event::PointerMoved(pos2(1320.0, 500.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                source: egui::MouseWheelSource::Unknown,
                delta: vec2(0.0, if y > 500.0 { -120.0 } else { 120.0 }),
                phase: egui::TouchPhase::Move,
                modifiers: Default::default(),
            },
        ]);
        for _ in 0..15 {
            u.frame(vec![]);
        }
    }
    panic!("no visible slider {label}")
}

#[test]
fn custom_slider_live_drag_clamps_and_finishes_one_undo_without_a_release_change() {
    let mut effect = Effect::of_type("paperLiquidMetal").unwrap();
    if let EffectKind::PaperLiquidMetal(p) = &mut effect.kind {
        p.speed = 0.0;
        p.distortion = 0.075;
    }
    let (mut u, r, index) = setup(effect.clone(), true);
    let (track, _) = slider(&mut u, "Distortion", Id::unique(("paper_metal", index, 3usize)));
    assert_eq!(u.app.session.doc().unwrap().node(r).unwrap().effects[index], effect, "opening never rounds a saved value");
    let shadow = u.app.session.doc().unwrap().node(r).unwrap().effects[0].clone();
    let before = u.app.session.undo_depth().0;
    u.button(track.lerp_inside(vec2(0.2, 0.5)), true);
    u.frame(vec![Event::PointerMoved(track.lerp_inside(vec2(0.55, 0.5)))]);
    let EffectKind::PaperLiquidMetal(p) = &u.app.session.doc().unwrap().node(r).unwrap().effects[index].kind else { panic!("metal missing") };
    assert!(p.distortion > 0.4, "the effect updates while the pointer is held");
    assert_eq!(u.app.session.undo_depth().0, before);
    let outside = pos2(track.max.x + 80.0, track.center().y);
    u.frame(vec![Event::PointerMoved(outside)]);
    u.button(outside, false);
    u.frame(vec![]);
    let EffectKind::PaperLiquidMetal(p) = &u.app.session.doc().unwrap().node(r).unwrap().effects[index].kind else { panic!("metal missing") };
    assert_eq!(p.distortion, 1.0);
    assert_eq!(u.app.session.undo_depth().0, before + 1);
    assert!(u.app.session.ds().unwrap().interaction.is_none());
    assert_eq!(u.app.session.doc().unwrap().node(r).unwrap().effects[0], shadow);
    u.app.session.undo().unwrap();
    assert_eq!(u.app.session.doc().unwrap().node(r).unwrap().effects[index], effect);
}

#[test]
fn custom_slider_numeric_input_stays_synced_and_sample_counts_use_integer_steps() {
    let (mut u, r, index) = setup(Effect::of_type("paperLensDistortion").unwrap(), false);
    let field = Id::unique(("paper_filter", index, "count"));
    let _ = slider(&mut u, "Count", field);
    let at = u.ctx.read_response(field.with("field")).unwrap().rect.center();
    u.button(at, true);
    u.button(at, false);
    u.frame(vec![Event::Text("17".into())]);
    let key = |pressed| Event::Key { key: Key::Enter, physical_key: None, pressed, repeat: false, modifiers: Default::default() };
    u.frame(vec![key(true), key(false)]);
    let (track, value) = slider(&mut u, "Count", field);
    assert_eq!(value, 17.0);
    u.button(track.lerp_inside(vec2(0.25, 0.5)), true);
    u.frame(vec![Event::PointerMoved(track.lerp_inside(vec2(0.63, 0.5)))]);
    u.button(track.lerp_inside(vec2(0.63, 0.5)), false);
    let EffectKind::PaperImageFilter(p) = &u.app.session.doc().unwrap().node(r).unwrap().effects[index].kind else { panic!("filter missing") };
    let count = p.params["count"];
    assert_eq!(count.fract(), 0.0);
    assert!((2.0..=50.0).contains(&count));
    assert_ne!(count, 17.0);
    assert!(u.app.session.ds().unwrap().interaction.is_none());
}

#[test]
fn custom_sliders_cover_all_materials_and_keep_long_frame_values_when_opened() {
    let kinds = std::iter::once("paperLiquidMetal").chain(PaperFilterKind::ALL.iter().map(|kind| kind.type_name()));
    for ty in kinds {
        let mut effect = Effect::of_type(ty).unwrap();
        let (label, field) = match &mut effect.kind {
            EffectKind::PaperLiquidMetal(p) => {
                p.speed = 0.0;
                p.frame = 123_456.25;
                ("Frame (ms)", Id::unique(("paper_metal", 0usize, 10usize)))
            }
            EffectKind::PaperImageFilter(p) => {
                p.speed = 0.0;
                if p.filter == PaperFilterKind::Water {
                    p.frame = -25_000.25;
                    ("Frame (ms)", Id::unique(("paper_filter_time", 0usize, "frame")))
                } else {
                    let spec = p.filter.params().iter().find(|s| !s.boolean && s.options.is_empty()).unwrap();
                    (spec.label, Id::unique(("paper_filter", 0usize, spec.name)))
                }
            }
            _ => panic!("unexpected standard effect"),
        };
        let (mut u, r, _) = setup(effect.clone(), false);
        let before = u.app.session.undo_depth();
        let (_, value) = slider(&mut u, label, field);
        assert!(value.is_finite());
        assert_eq!(u.app.session.doc().unwrap().node(r).unwrap().effects[0], effect, "{ty} changed on opening");
        assert_eq!(u.app.session.undo_depth(), before);
    }
}
