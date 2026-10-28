//! Context-specific display names; saved parameter keys and Paper formulas stay unchanged.
use crate::i18n::tr_noop;
use figcraft_doc::PaperFilterKind;
pub(super) fn paper(kind: PaperFilterKind, name: &str, fallback: &'static str) -> &'static str {
    match (kind, name) {
        (PaperFilterKind::PaperTexture, "blending") => tr_noop("Blending|paperTexture"),
        (PaperFilterKind::PaperTexture, "distortion") => tr_noop("Distortion|paperTexture"),
        (PaperFilterKind::PaperTexture, "clip") => tr_noop("Clip|paperTexture"),
        (PaperFilterKind::PaperTexture, "angle") => tr_noop("Angle|paperTexture"),
        (PaperFilterKind::PaperTexture, "seed") => tr_noop("Seed|paperTexture"),
        (PaperFilterKind::PaperTexture, "roughness") => tr_noop("Roughness|paperTexture"),
        (PaperFilterKind::PaperTexture, "roughnessSize") => tr_noop("Roughness Size|paperTexture"),
        (PaperFilterKind::PaperTexture, "roughnessRows") => tr_noop("Roughness Rows|paperTexture"),
        (PaperFilterKind::PaperTexture, "fiber") => tr_noop("Fiber|paperTexture"),
        (PaperFilterKind::PaperTexture, "fiberSize") => tr_noop("Fiber Size|paperTexture"),
        (PaperFilterKind::PaperTexture, "folds") => tr_noop("Folds|paperTexture"),
        (PaperFilterKind::PaperTexture, "foldSizeX") => tr_noop("Fold Size X|paperTexture"),
        (PaperFilterKind::PaperTexture, "foldSizeY") => tr_noop("Fold Size Y|paperTexture"),
        (PaperFilterKind::PaperTexture, "foldOffsetX") => tr_noop("Fold Offset X|paperTexture"),
        (PaperFilterKind::PaperTexture, "foldOffsetY") => tr_noop("Fold Offset Y|paperTexture"),
        (PaperFilterKind::PaperTexture, "wrinkles") => tr_noop("Wrinkles|paperTexture"),
        (PaperFilterKind::PaperTexture, "wrinkleSize") => tr_noop("Wrinkle Size|paperTexture"),
        (PaperFilterKind::PaperTexture, "crumples") => tr_noop("Crumples|paperTexture"),
        (PaperFilterKind::PaperTexture, "crumpleCount") => tr_noop("Crumple Count|paperTexture"),
        (PaperFilterKind::PaperTexture, "drops") => tr_noop("Drops|paperTexture"),
        (_, "fit") => tr_noop("Fit|customMaterial"),
        (_, "scale") => tr_noop("Scale|customMaterial"),
        (_, "rotation") => tr_noop("Rotation|customMaterial"),
        (_, "originX") => tr_noop("Origin X|customMaterial"),
        (_, "originY") => tr_noop("Origin Y|customMaterial"),
        (_, "offsetX") => tr_noop("Offset X|customMaterial"),
        (_, "offsetY") => tr_noop("Offset Y|customMaterial"),
        (_, "worldWidth") => tr_noop("World Width|customMaterial"),
        (_, "worldHeight") => tr_noop("World Height|customMaterial"),
        (PaperFilterKind::FlutedGlass, "size") => tr_noop("Size|flutedGlass"),
        (PaperFilterKind::FlutedGlass, "shadows") => tr_noop("Shadows|flutedGlass"),
        (PaperFilterKind::FlutedGlass, "angle") => tr_noop("Angle|flutedGlass"),
        (PaperFilterKind::FlutedGlass, "stretch") => tr_noop("Stretch|flutedGlass"),
        (PaperFilterKind::FlutedGlass, "shape") => tr_noop("Shape|flutedGlass"),
        (PaperFilterKind::FlutedGlass, "distortion") => tr_noop("Distortion|flutedGlass"),
        (PaperFilterKind::FlutedGlass, "highlights") => tr_noop("Highlights|flutedGlass"),
        (PaperFilterKind::FlutedGlass, "distortionShape") => tr_noop("Distortion Shape|flutedGlass"),
        (PaperFilterKind::FlutedGlass, "shift") => tr_noop("Shift|flutedGlass"),
        (PaperFilterKind::FlutedGlass, "blur") => tr_noop("Blur|flutedGlass"),
        (PaperFilterKind::FlutedGlass, "edges") => tr_noop("Edges|flutedGlass"),
        (PaperFilterKind::FlutedGlass, "marginLeft") => tr_noop("Margin Left|flutedGlass"),
        (PaperFilterKind::FlutedGlass, "marginRight") => tr_noop("Margin Right|flutedGlass"),
        (PaperFilterKind::FlutedGlass, "marginTop") => tr_noop("Margin Top|flutedGlass"),
        (PaperFilterKind::FlutedGlass, "marginBottom") => tr_noop("Margin Bottom|flutedGlass"),
        (_, "grainMixer") => tr_noop("Grain Mixer|customMaterial"),
        (_, "grainOverlay") => tr_noop("Grain Overlay|customMaterial"),
        (PaperFilterKind::Water, "size") => tr_noop("Size|water"),
        (PaperFilterKind::Water, "highlights") => tr_noop("Highlights|water"),
        (PaperFilterKind::Water, "layering") => tr_noop("Layering|water"),
        (PaperFilterKind::Water, "edges") => tr_noop("Edges|water"),
        (PaperFilterKind::Water, "caustic") => tr_noop("Caustic|water"),
        (PaperFilterKind::Water, "waves") => tr_noop("Waves|water"),
        (PaperFilterKind::ImageDithering, "type") => tr_noop("Type|imageDithering"),
        (PaperFilterKind::ImageDithering, "pxSize") => tr_noop("Px Size|imageDithering"),
        (_, "originalColors") => tr_noop("Original Colors|customMaterial"),
        (_, "inverted") => tr_noop("Inverted|customMaterial"),
        (PaperFilterKind::ImageDithering, "colorSteps") => tr_noop("Color Steps|imageDithering"),
        (PaperFilterKind::HalftoneDots, "radius") => tr_noop("Radius|halftoneDots"),
        (_, "contrast") => tr_noop("Contrast|customMaterial"),
        (PaperFilterKind::HalftoneDots, "size") => tr_noop("Size|halftoneDots"),
        (_, "grainSize") => tr_noop("Grain Size|customMaterial"),
        (PaperFilterKind::HalftoneDots, "grid") => tr_noop("Grid|halftoneDots"),
        (PaperFilterKind::HalftoneDots, "type") => tr_noop("Type|halftoneDots"),
        (PaperFilterKind::HalftoneCmyk, "size") => tr_noop("Size|halftoneCmyk"),
        (PaperFilterKind::HalftoneCmyk, "gridNoise") => tr_noop("Grid Noise|halftoneCmyk"),
        (PaperFilterKind::HalftoneCmyk, "softness") => tr_noop("Softness|halftoneCmyk"),
        (PaperFilterKind::HalftoneCmyk, "floodC") => tr_noop("Flood C|halftoneCmyk"),
        (PaperFilterKind::HalftoneCmyk, "floodM") => tr_noop("Flood M|halftoneCmyk"),
        (PaperFilterKind::HalftoneCmyk, "floodY") => tr_noop("Flood Y|halftoneCmyk"),
        (PaperFilterKind::HalftoneCmyk, "floodK") => tr_noop("Flood K|halftoneCmyk"),
        (PaperFilterKind::HalftoneCmyk, "gainC") => tr_noop("Gain C|halftoneCmyk"),
        (PaperFilterKind::HalftoneCmyk, "gainM") => tr_noop("Gain M|halftoneCmyk"),
        (PaperFilterKind::HalftoneCmyk, "gainY") => tr_noop("Gain Y|halftoneCmyk"),
        (PaperFilterKind::HalftoneCmyk, "gainK") => tr_noop("Gain K|halftoneCmyk"),
        (PaperFilterKind::HalftoneCmyk, "type") => tr_noop("Type|halftoneCmyk"),
        (PaperFilterKind::LensDistortion, "spread") => tr_noop("Spread|lensDistortion"),
        (PaperFilterKind::LensDistortion, "bias") => tr_noop("Bias|lensDistortion"),
        (PaperFilterKind::LensDistortion, "angle") => tr_noop("Angle|lensDistortion"),
        (PaperFilterKind::LensDistortion, "perspective") => tr_noop("Perspective|lensDistortion"),
        (PaperFilterKind::LensDistortion, "count") => tr_noop("Count|lensDistortion"),
        (PaperFilterKind::LensDistortion, "dispersion") => tr_noop("Dispersion|lensDistortion"),
        (PaperFilterKind::LensDistortion, "dispersionShift") => tr_noop("Dispersion Shift|lensDistortion"),
        (PaperFilterKind::LensDistortion, "dispersionColor") => tr_noop("Dispersion Color|lensDistortion"),
        (PaperFilterKind::LensDistortion, "focusCenter") => tr_noop("Focus Center|lensDistortion"),
        (PaperFilterKind::LensDistortion, "focusEdges") => tr_noop("Focus Edges|lensDistortion"),
        (PaperFilterKind::LensDistortion, "swirl") => tr_noop("Swirl|lensDistortion"),
        (PaperFilterKind::LensDistortion, "noise") => tr_noop("Noise|lensDistortion"),
        (PaperFilterKind::LensDistortion, "noiseFrequency") => tr_noop("Noise Frequency|lensDistortion"),
        (PaperFilterKind::LensDistortion, "noiseOffset") => tr_noop("Noise Offset|lensDistortion"),
        (PaperFilterKind::LensDistortion, "lensBulge") => tr_noop("Lens Bulge|lensDistortion"),
        (PaperFilterKind::LensDistortion, "lensCircle") => tr_noop("Lens Circle|lensDistortion"),
        (PaperFilterKind::LensDistortion, "imageX") => tr_noop("Image X|lensDistortion"),
        (PaperFilterKind::LensDistortion, "imageY") => tr_noop("Image Y|lensDistortion"),
        _ => fallback,
    }
}
pub(super) fn metal(label: &'static str) -> &'static str {
    match label {
        "Speed" => tr_noop("Speed|liquidMetal"),
        "Repetition" => tr_noop("Repetition|liquidMetal"),
        "Softness" => tr_noop("Softness|liquidMetal"),
        "Distortion" => tr_noop("Distortion|liquidMetal"),
        "Contour" => tr_noop("Contour|liquidMetal"),
        "Angle" => tr_noop("Angle|liquidMetal"),
        "Red shift" => tr_noop("Red shift|liquidMetal"),
        "Blue shift" => tr_noop("Blue shift|liquidMetal"),
        "Scale" => tr_noop("Scale|liquidMetal"),
        "Rotation" => tr_noop("Rotation|liquidMetal"),
        "Frame (ms)" => tr_noop("Frame (ms)|liquidMetal"),
        _ => label,
    }
}
pub(super) fn option(label: &'static str) -> &'static str {
    match label {
        "None" => tr_noop("None|customOption"),
        "Contain" => tr_noop("Contain|customOption"),
        "Cover" => tr_noop("Cover|customOption"),
        "Lines" => tr_noop("Lines|customOption"),
        "Lines Irregular" => tr_noop("Lines Irregular|customOption"),
        "Wave" => tr_noop("Wave|customOption"),
        "Zigzag" => tr_noop("Zigzag|customOption"),
        "Pattern" => tr_noop("Pattern|customOption"),
        "Prism" => tr_noop("Prism|customOption"),
        "Lens" => tr_noop("Lens|customOption"),
        "Contour" => tr_noop("Contour|customOption"),
        "Cascade" => tr_noop("Cascade|customOption"),
        "Flat" => tr_noop("Flat|customOption"),
        "Random" => tr_noop("Random|customOption"),
        "2x2" => tr_noop("2x2|customOption"),
        "4x4" => tr_noop("4x4|customOption"),
        "8x8" => tr_noop("8x8|customOption"),
        "Square" => tr_noop("Square|customOption"),
        "Hex" => tr_noop("Hex|customOption"),
        "Classic" => tr_noop("Classic|customOption"),
        "Gooey" => tr_noop("Gooey|customOption"),
        "Holes" => tr_noop("Holes|customOption"),
        "Soft" => tr_noop("Soft|customOption"),
        "Dots" => tr_noop("Dots|customOption"),
        "Ink" => tr_noop("Ink|customOption"),
        "Sharp" => tr_noop("Sharp|customOption"),
        _ => label,
    }
}
pub(super) fn color(name: &str, fallback: &'static str) -> &'static str {
    match name {
        "colorBack" => tr_noop("Background|customColor"),
        "colorFront" => tr_noop("Foreground|customColor"),
        "colorPaper" => tr_noop("Paper|customColor"),
        "colorShadow" => tr_noop("Shadow|customColor"),
        "colorHighlight" => tr_noop("Highlight|customColor"),
        "colorC" => tr_noop("C|customColor"),
        "colorM" => tr_noop("M|customColor"),
        "colorY" => tr_noop("Y|customColor"),
        "colorK" => tr_noop("K|customColor"),
        _ => fallback,
    }
}
pub(super) fn time(name: &str) -> &'static str {
    match name {
        "speed" => tr_noop("Speed|customAnimation"),
        _ => tr_noop("Frame (ms)|customAnimation"),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn korean(key: &str) -> &str {
        let lang = crate::i18n::languages().iter().find(|lang| lang.code == "ko").unwrap();
        let (_, value) = lang.ui.iter().find(|(k, _)| *k == key).unwrap_or_else(|| panic!("untranslated custom label: {key}"));
        match value {
            crate::i18n::Text::Plain(text) => text,
            _ => panic!("a parameter label is not plural"),
        }
    }
    #[test]
    fn every_custom_parameter_option_and_color_has_a_contextual_korean_name() {
        for kind in PaperFilterKind::ALL {
            for spec in kind.params() {
                let key = paper(kind, spec.name, spec.label);
                assert!(key.contains('|'), "unmapped {kind:?}.{}", spec.name);
                assert!(korean(key).chars().any(|c| ('\u{ac00}'..='\u{d7a3}').contains(&c)), "{key}");
                for (label, _) in spec.options {
                    assert_ne!(option(label), *label);
                    korean(option(label));
                }
            }
            for spec in kind.colors() {
                korean(color(spec.name, spec.label));
            }
        }
        for label in
            ["Speed", "Repetition", "Softness", "Distortion", "Contour", "Angle", "Red shift", "Blue shift", "Scale", "Rotation", "Frame (ms)"]
        {
            korean(metal(label));
        }
        assert_eq!(korean(metal("Red shift")), "빨강 채널 이동");
        assert_eq!(korean(paper(PaperFilterKind::HalftoneCmyk, "gainC", "")), "시안 도트 게인");
        assert_eq!(korean(paper(PaperFilterKind::Water, "caustic", "")), "집광 무늬");
    }
}
