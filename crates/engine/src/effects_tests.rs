//! Effects through the session: adding every kind by name, editing noise/texture/glass, scaling,
//! inspecting and generated CSS.

use serde_json::json;

use crate::Session;
use crate::codegen::{CodeLang, code_for};
use crate::params::id_of;
use figcraft_doc::{Effect, EffectKind, Glass, NoiseMode};

fn rect(s: &mut Session) -> figcraft_doc::NodeId {
    id_of(&s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 60})).unwrap()["id"]).unwrap()
}

#[test]
fn torn_paper_command_roundtrip_scale_and_undo() {
    let mut s = Session::new();
    let id = rect(&mut s);
    s.execute("effect.add", &json!({"ids":[id],"type":"tornPaper"})).unwrap();
    s.execute("effect.set",&json!({"ids":[id],"index":0,"effect":{"type":"tornPaper","depth":8,"size":22,"edgeWidth":1.5,"seed":73,"color":{"r":1,"g":0.9,"b":0.8,"a":1}}})).unwrap();
    let before = s.doc().unwrap().get(id).unwrap().effects[0].clone();
    let bytes = figcraft_format::save_bytes(s.doc().unwrap(), None).unwrap();
    let (loaded, warnings) = figcraft_format::load_bytes(&bytes).unwrap();
    assert!(warnings.is_empty());
    assert_eq!(loaded.get(id).unwrap().effects[0], before);
    s.execute("transform.scale", &json!({"ids":[id],"factor":2})).unwrap();
    let EffectKind::TornPaper(p) = &s.doc().unwrap().get(id).unwrap().effects[0].kind else { panic!() };
    assert_eq!((p.depth, p.size, p.edge_width, p.seed), (16.0, 44.0, 3.0, 73));
    s.undo().unwrap();
    assert_eq!(s.doc().unwrap().get(id).unwrap().effects[0], before);
    s.execute("effect.remove", &json!({"ids":[id],"index":0})).unwrap();
    s.undo().unwrap();
    assert_eq!(s.doc().unwrap().get(id).unwrap().effects[0], before);
    assert!(s.execute("effect.set", &json!({"ids":[id],"index":0,"effect":{"type":"tornPaper","depht":9}})).is_err());
}

#[test]
fn paper_metal_commands_roundtrip_and_undo() {
    let mut s = Session::new();
    let r = rect(&mut s);
    s.execute("effect.add", &json!({"ids":[r], "effect":{"type":"paperLiquidMetal", "speed":0.75, "shiftRed":0.2}})).unwrap();
    let reported = crate::inspect::node_json(s.doc().unwrap(), r, 0)["effects"].clone();
    s.execute("node.set", &json!({"ids":[r], "props":{"effects":reported}})).unwrap();
    let EffectKind::PaperLiquidMetal(p) = &s.doc().unwrap().node(r).unwrap().effects[0].kind else { panic!("missing Paper effect") };
    assert_eq!((p.speed, p.shift_red), (0.75, 0.2));
    s.execute("effect.set", &json!({"ids":[r], "index":0, "effect":{"type":"paperLiquidMetal","speed":0}})).unwrap();
    s.undo().unwrap();
    let EffectKind::PaperLiquidMetal(p) = &s.doc().unwrap().node(r).unwrap().effects[0].kind else { panic!("missing Paper effect") };
    assert_eq!(p.speed, 0.75);
    assert!(s.execute("effect.set", &json!({"ids":[r], "index":0, "effect":{"type":"paperLiquidMetal","speeed":1}})).is_err());
}

#[test]
fn effect_add_takes_every_type() {
    let mut s = Session::new();
    let r = rect(&mut s);
    for (_, ty) in Effect::TYPES {
        s.execute("effect.add", &json!({"ids": [r], "type": ty})).unwrap();
    }
    s.execute("effect.add", &json!({"ids": [r], "type": "GLASS"})).unwrap();
    let n = s.doc().unwrap().node(r).unwrap();
    let labels: Vec<&str> = n.effects.iter().map(Effect::label).collect();
    assert_eq!(labels, Effect::TYPES.iter().map(|(label, _)| *label).chain(["Glass"]).collect::<Vec<_>>());
    let err = s.execute("effect.add", &json!({"ids": [r], "type": "sparkle"})).unwrap_err();
    assert!(err.to_string().contains("sparkle"), "{err}");
    // Undo removes one at a time.
    s.undo().unwrap();
    assert_eq!(s.doc().unwrap().node(r).unwrap().effects.len(), Effect::TYPES.len());
}

#[test]
fn paper_image_filters_roundtrip_edits_undo_and_reject_unknown_parameters() {
    use figcraft_doc::{PaperFilterKind, PaperImageFilter};
    let mut s = Session::new();
    let r = rect(&mut s);
    for filter in PaperFilterKind::ALL {
        for color in filter.colors() {
            assert!(color.default.iter().all(|v| (0.0..=1.0).contains(v)), "{} palette", filter.label());
        }
        s.execute("effect.add", &json!({"ids":[r], "type":filter.type_name()})).unwrap();
    }
    let water: PaperImageFilter = serde_json::from_value(json!({"filter":"water"})).unwrap();
    assert!(water.animated(), "water's omitted speed uses its own preset default");
    let mut params = PaperImageFilter::new(PaperFilterKind::HalftoneCmyk);
    params.set_value("grainSize", 0.8);
    s.execute("effect.set", &json!({"ids":[r], "index":5, "effect":{"type":"paperImageFilter","filter":params.filter,"params":params.params}}))
        .unwrap();
    let reported = crate::inspect::node_json(s.doc().unwrap(), r, 0)["effects"].clone();
    s.execute("node.set", &json!({"ids":[r], "props":{"effects":reported}})).unwrap();
    let EffectKind::PaperImageFilter(p) = &s.doc().unwrap().node(r).unwrap().effects[5].kind else { panic!("filter missing") };
    assert_eq!(p.filter, PaperFilterKind::HalftoneCmyk);
    assert_eq!(p.value(p.filter.params().iter().find(|p| p.name == "grainSize").unwrap()), 0.8);
    let effect = s.doc().unwrap().node(r).unwrap().effects[5].clone();
    let serialized = serde_json::to_vec(s.doc().unwrap()).unwrap();
    let restored: figcraft_doc::Document = serde_json::from_slice(&serialized).unwrap();
    assert_eq!(restored.node(r).unwrap().effects[5], effect);
    s.execute("effect.remove", &json!({"ids":[r], "index":5})).unwrap();
    s.undo().unwrap();
    assert_eq!(s.doc().unwrap().node(r).unwrap().effects[5], effect);
    assert!(
        s.execute("effect.set", &json!({"ids":[r], "index":5, "effect":{"type":"paperImageFilter","filter":"water","params":{"causticc":0.5}}}))
            .is_err()
    );
    assert!(s.execute("effect.add", &json!({"ids":[r], "type":"paperHeatmap"})).is_err());
    assert!(s.execute("effect.add", &json!({"ids":[r], "type":"paperMeshGradient"})).is_err());
}

#[test]
fn effect_set_edits_noise_texture_and_glass() {
    let mut s = Session::new();
    let r = rect(&mut s);
    s.execute(
        "effect.add",
        &json!({"ids": [r], "effect": {"type": "noise", "mode": "duo", "size": 2, "density": 0.4, "secondary": {"r": 1, "g": 1, "b": 1, "a": 1}}}),
    )
    .unwrap();
    s.execute("effect.add", &json!({"ids": [r], "effect": {"type": "texture", "radius": 6, "clipToShape": true}})).unwrap();
    s.execute("effect.set", &json!({"ids": [r], "index": 1, "effect": {"type": "glass", "refraction": 0.3, "frost": 10, "splay": 0.5}})).unwrap();
    let n = s.doc().unwrap().node(r).unwrap();
    let EffectKind::Noise(nz) = &n.effects[0].kind else { panic!("{:?}", n.effects) };
    assert_eq!((nz.mode, nz.size, nz.density), (NoiseMode::Duo, 2.0, 0.4));
    let EffectKind::Glass(g) = &n.effects[1].kind else { panic!("{:?}", n.effects) };
    assert_eq!((g.refraction, g.frost, g.splay, g.depth), (0.3, 10.0, 0.5, Glass::default().depth));
    // Junk is refused, not stored.
    assert!(s.execute("effect.set", &json!({"ids": [r], "index": 0, "effect": {"type": "noise", "mode": "rainbow"}})).is_err());
    assert!(s.execute("effect.set", &json!({"ids": [r], "index": 0, "effect": {"type": "glass", "depth": "deep"}})).is_err());
}

/// A key an effect doesn't have is refused, naming what it takes: serde dropped it without a word,
/// so a shadow given `offsetX`/`offsetY` (the names variable bindings use) kept the default offset.
#[test]
fn effects_refuse_keys_they_dont_have() {
    let mut s = Session::new();
    let r = rect(&mut s);
    let shadow = json!({"type": "dropShadow", "color": "#00000040", "offsetX": 0, "offsetY": 1, "radius": 3});
    let err = s.execute("node.set", &json!({"ids": [r], "props": {"effects": [shadow]}})).unwrap_err().to_string();
    assert!(err.contains("effects[0]") && err.contains("offsetX, offsetY") && err.contains("offset"), "{err}");
    assert!(s.doc().unwrap().node(r).unwrap().effects.is_empty(), "nothing stored");
    let err = s.execute("effect.add", &json!({"ids": [r], "effect": {"type": "layerBlur", "radious": 8}})).unwrap_err().to_string();
    assert!(err.contains("radious"), "{err}");
    s.execute("effect.add", &json!({"ids": [r], "type": "dropShadow"})).unwrap();
    let err = s.execute("effect.set", &json!({"ids": [r], "index": 0, "effect": {"type": "dropShadow", "spred": 2}})).unwrap_err().to_string();
    assert!(err.contains("spred"), "{err}");
    // Every kind's own keys still go in, camelCase or snake_case, and so does what inspect reports.
    for ty in ["innerShadow", "layerBlur", "backgroundBlur", "noise", "texture", "glass"] {
        s.execute("effect.add", &json!({"ids": [r], "type": ty})).unwrap();
    }
    let reported = crate::inspect::node_json(s.doc().unwrap(), r, 0)["effects"].clone();
    s.execute("node.set", &json!({"ids": [r], "props": {"effects": reported}})).unwrap();
    let camel = json!({"type": "dropShadow", "offset": [0, 1], "showBehind": true, "visible": true});
    let snake = json!({"type": "innerShadow", "offset": [0, 1], "show_behind": false});
    s.execute("node.set", &json!({"ids": [r], "props": {"effects": [camel, snake]}})).unwrap();
}

#[test]
fn scaling_scales_grain_and_glass() {
    let mut s = Session::new();
    let r = rect(&mut s);
    for ty in ["noise", "texture", "glass"] {
        s.execute("effect.add", &json!({"ids": [r], "type": ty})).unwrap();
    }
    s.execute("transform.scale", &json!({"ids": [r], "factor": 2})).unwrap();
    let n = s.doc().unwrap().node(r).unwrap();
    let EffectKind::Noise(nz) = &n.effects[0].kind else { panic!() };
    let EffectKind::Texture(t) = &n.effects[1].kind else { panic!() };
    let EffectKind::Glass(g) = &n.effects[2].kind else { panic!() };
    let d = Glass::default();
    assert_eq!((nz.size, t.size, t.radius, g.depth, g.frost), (1.0, 1.0, 8.0, d.depth * 2.0, d.frost * 2.0));
    // Unitless values stay.
    assert_eq!((g.refraction, nz.density), (d.refraction, 0.5));
}

#[test]
fn inspect_and_css_show_the_new_effects() {
    let mut s = Session::new();
    let r = rect(&mut s);
    for ty in ["noise", "texture", "glass"] {
        s.execute("effect.add", &json!({"ids": [r], "type": ty})).unwrap();
    }
    let doc = s.doc().unwrap();
    let sections = crate::devinspect::properties(doc, r);
    let fx = sections.iter().find(|s| s.title == "Effects").unwrap();
    let labels: Vec<&str> = fx.rows.iter().map(|p| p.label.as_str()).collect();
    assert_eq!(labels, vec!["Noise", "Texture", "Glass"]);
    assert!(fx.rows[2].value.contains("refraction 80%"), "{}", fx.rows[2].value);
    let css = code_for(doc, r, CodeLang::Css);
    assert!(css.contains("backdrop-filter: blur(2px);"), "{css}");
}

#[test]
fn negative_or_fractional_index_is_refused_not_item_zero() {
    let mut s = Session::new();
    let r = rect(&mut s);
    s.execute("effect.add", &json!({"ids": [r], "type": "layerBlur"})).unwrap();
    s.execute("export.add", &json!({"ids": [r]})).unwrap();
    s.execute("export.add", &json!({"ids": [r]})).unwrap();
    let before = s.doc().unwrap().node(r).unwrap().clone();
    assert_eq!(before.exports.len(), 2);
    for index in [json!(-1), json!(-0.5), json!(1.5)] {
        let set = json!({"ids": [r], "index": index, "effect": {"type": "dropShadow"}});
        assert!(s.execute("effect.set", &set).is_err(), "effect.set index {index}");
        let upd = json!({"ids": [r], "index": index, "format": "svg"});
        assert!(s.execute("export.update", &upd).is_err(), "export.update index {index}");
        assert!(s.execute("export.remove", &json!({"ids": [r], "index": index})).is_err(), "export.remove index {index}");
        let n = s.doc().unwrap().node(r).unwrap();
        assert_eq!(n.effects, before.effects, "index {index}");
        assert_eq!(n.exports, before.exports, "index {index}");
    }
}
