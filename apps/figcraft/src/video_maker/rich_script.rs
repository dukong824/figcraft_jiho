//! Persisted character-based formatting and image anchors for the freeform script.
use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InlineStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub size: Option<f32>,
    pub line_height: Option<f32>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub style: InlineStyle,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextKind {
    #[default]
    Text,
    H1,
    H2,
    H3,
    H4,
    Bullet,
}
impl TextKind {
    pub const ALL: [Self; 6] = [Self::Text, Self::H1, Self::H2, Self::H3, Self::H4, Self::Bullet];
    pub fn label(self) -> &'static str {
        match self {
            Self::Text => "일반 텍스트",
            Self::H1 => "제목 1",
            Self::H2 => "제목 2",
            Self::H3 => "제목 3",
            Self::H4 => "제목 4",
            Self::Bullet => "글머리 기호",
        }
    }
    pub fn size(self, base: f32) -> f32 {
        match self {
            Self::H1 => 30.0,
            Self::H2 => 25.0,
            Self::H3 => 21.0,
            Self::H4 => 18.0,
            _ => base,
        }
    }
    pub fn heading(self) -> bool {
        matches!(self, Self::H1 | Self::H2 | Self::H3 | Self::H4)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Paragraph {
    pub at: usize,
    pub kind: TextKind,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScriptImage {
    pub id: u64,
    pub asset_id: u64,
    pub at: usize,
    #[serde(default)]
    pub width: Option<f32>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RichScript {
    #[serde(default)]
    pub spans: Vec<Span>,
    #[serde(default)]
    pub paragraphs: Vec<Paragraph>,
    #[serde(default)]
    pub images: Vec<ScriptImage>,
    #[serde(default)]
    pub images_migrated: bool,
}
pub fn byte(text: &str, at: usize) -> usize {
    text.char_indices().nth(at).map_or(text.len(), |(i, _)| i)
}
pub fn line_start(text: &str, at: usize) -> usize {
    text[..byte(text, at)].rfind('\n').map_or(0, |i| text[..i + 1].chars().count())
}
pub fn line_end(text: &str, at: usize) -> usize {
    let b = byte(text, at);
    text[b..].find('\n').map_or_else(|| text.chars().count(), |i| text[..b + i + 1].chars().count())
}
impl RichScript {
    pub fn all(&self, range: Range<usize>, test: impl Fn(&InlineStyle) -> bool) -> bool {
        if range.is_empty() {
            return false;
        }
        let mut boundaries = vec![range.start];
        for span in &self.spans {
            for at in [span.start, span.end] {
                if range.contains(&at) {
                    boundaries.push(at);
                }
            }
        }
        boundaries.into_iter().all(|at| test(&self.style(at)))
    }
    pub fn style(&self, at: usize) -> InlineStyle {
        self.spans.iter().find(|s| s.start <= at && at < s.end).map(|s| s.style.clone()).unwrap_or_default()
    }
    pub fn kind(&self, text: &str, at: usize) -> TextKind {
        let start = line_start(text, at);
        self.paragraphs.iter().find(|p| p.at == start).map(|p| p.kind).unwrap_or_default()
    }
    pub fn apply(&mut self, range: Range<usize>, edit: impl Fn(&mut InlineStyle)) {
        if range.is_empty() {
            return;
        }
        let mut cuts = vec![range.start, range.end];
        for s in &self.spans {
            cuts.extend([s.start, s.end]);
        }
        cuts.sort_unstable();
        cuts.dedup();
        let mut spans: Vec<Span> = Vec::new();
        for pair in cuts.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let mut style = self.style(a);
            if a >= range.start && a < range.end {
                edit(&mut style);
            }
            if style == InlineStyle::default() {
                continue;
            }
            if let Some(last) = spans.last_mut()
                && last.end == a
                && last.style == style
            {
                last.end = b;
            } else {
                spans.push(Span { start: a, end: b, style });
            }
        }
        self.spans = spans;
    }
    pub fn set_kind(&mut self, text: &str, range: Range<usize>, kind: TextKind) {
        let start = line_start(text, range.start);
        let end = if range.end > range.start { line_start(text, range.end.saturating_sub(1)) } else { start };
        self.paragraphs.retain(|p| p.at < start || p.at > end);
        let mut at = start;
        loop {
            if kind != TextKind::Text {
                self.paragraphs.push(Paragraph { at, kind });
            }
            let next = line_end(text, at);
            if next <= at || next > end {
                break;
            }
            at = next;
        }
        self.paragraphs.sort_by_key(|p| p.at);
    }
    pub fn insert_image(&mut self, asset_id: u64, at: usize) {
        let id = self.images.iter().map(|p| p.id).max().unwrap_or(0).saturating_add(1);
        self.images.push(ScriptImage { id, asset_id, at, width: None });
        self.images.sort_by_key(|p| p.at);
    }
    /// Rebase metadata around the actual changed characters, including CJK edits and paste.
    pub fn text_changed(&mut self, old: &str, new: &str) {
        let old_lines: Vec<_> = old.split_inclusive('\n').collect();
        let new_lines: Vec<_> = new.split_inclusive('\n').collect();
        if old_lines.len() == new_lines.len()
            && old_lines.iter().zip(&new_lines).all(|(a, b)| a.trim_start_matches([' ', '\t']) == b.trim_start_matches([' ', '\t']))
        {
            let mut offset = 0;
            let mut edits = Vec::new();
            for (a, b) in old_lines.iter().zip(&new_lines) {
                if a != b {
                    let old_prefix = a.chars().take_while(|c| matches!(c, ' ' | '\t')).count();
                    let prefix: String = b.chars().take_while(|c| matches!(c, ' ' | '\t')).collect();
                    edits.push((offset, old_prefix, prefix));
                }
                offset += a.chars().count();
            }
            if edits.len() > 1 {
                let mut working = old.to_owned();
                for (at, count, prefix) in edits.into_iter().rev() {
                    let previous = working.clone();
                    working.replace_range(byte(&previous, at)..byte(&previous, at + count), &prefix);
                    self.text_changed(&previous, &working);
                }
                return;
            }
        }
        let a: Vec<char> = old.chars().collect();
        let b: Vec<char> = new.chars().collect();
        let start = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
        let suffix = a[start..].iter().rev().zip(b[start..].iter().rev()).take_while(|(x, y)| x == y).count();
        let end = a.len() - suffix;
        let inserted = b.len() - start - suffix;
        let delta = inserted as isize - (end - start) as isize;
        let shift = |at: usize| {
            if at < start {
                at
            } else if at >= end {
                at.saturating_add_signed(delta)
            } else {
                start + inserted
            }
        };
        let inherited = self.style(if end > start { start } else { start.saturating_sub(1) });
        let kind = self.kind(old, start);
        let mut spans = Vec::new();
        for s in &self.spans {
            if s.start < start {
                spans.push(Span { start: s.start, end: s.end.min(start), style: s.style.clone() });
            }
            if s.end > end {
                spans.push(Span { start: shift(s.start.max(end)), end: shift(s.end), style: s.style.clone() });
            }
        }
        if inserted > 0 && inherited != InlineStyle::default() {
            spans.push(Span { start, end: start + inserted, style: inherited });
        }
        spans.retain(|s| s.start < s.end);
        spans.sort_by_key(|s| s.start);
        self.spans = spans;
        for p in &mut self.paragraphs {
            p.at = line_start(new, shift(p.at).min(b.len()));
        }
        for (i, c) in b.iter().enumerate().skip(start).take(inserted) {
            if *c == '\n' && kind != TextKind::Text {
                self.paragraphs.push(Paragraph { at: i + 1, kind });
            }
        }
        self.paragraphs.sort_by_key(|p| p.at);
        self.paragraphs.dedup_by_key(|p| p.at);
        for image in &mut self.images {
            image.at = shift(image.at).min(b.len());
        }
    }
    pub fn normalize(&mut self, text: &str) {
        let len = text.chars().count();
        for span in &mut self.spans {
            span.start = span.start.min(len);
            span.end = span.end.min(len);
        }
        self.spans.retain(|s| s.start < s.end);
        for image in &mut self.images {
            image.at = image.at.min(len);
            if line_start(text, image.at) != image.at {
                image.at = line_end(text, image.at);
            }
        }
        self.images.sort_by_key(|i| i.at);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_formatting_preserves_other_characters_and_overlapping_marks() {
        let mut r = RichScript::default();
        r.apply(1..4, |s| s.bold = true);
        assert!(!r.all(0..4, |s| s.bold));
        assert!(r.all(1..4, |s| s.bold));
        r.apply(2..5, |s| s.italic = true);
        assert!(!r.style(0).bold);
        assert!(r.style(1).bold);
        assert!(r.style(2).bold && r.style(2).italic);
        assert!(!r.style(4).bold && r.style(4).italic);
        r.apply(2..3, |s| s.bold = false);
        assert!(!r.style(2).bold);
        assert!(r.style(3).bold);
    }
    #[test]
    fn unicode_edits_rebase_images_and_formatting_without_byte_offsets() {
        let mut r = RichScript::default();
        r.apply(3..5, |s| s.underline = true);
        r.insert_image(9, 6);
        r.text_changed("가나\n다라\n끝", "새가나\n다라\n끝");
        assert!(r.style(4).underline);
        assert_eq!(r.images[0].at, 7);
        r.set_kind("새가나\n다라\n끝", 4..6, TextKind::H2);
        assert_eq!(r.kind("새가나\n다라\n끝", 5), TextKind::H2);
    }
    #[test]
    fn rich_content_roundtrips_and_reorders_images_at_real_line_boundaries() {
        let mut r = RichScript::default();
        r.apply(0..2, |s| {
            s.bold = true;
            s.size = Some(22.0)
        });
        r.insert_image(7, 1);
        r.insert_image(8, 6);
        r.normalize("첫 줄\n둘째 줄");
        assert_eq!(r.images[0].at, 4);
        r.images[1].at = 0;
        r.images.sort_by_key(|i| i.at);
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(r, serde_json::from_str(&json).unwrap());
        assert_eq!(r.images[0].asset_id, 8);
    }

    #[test]
    fn repeated_layout_does_not_move_an_image_to_the_end_of_the_document() {
        let mut rich = RichScript::default();
        rich.insert_image(1, 4);
        for _ in 0..20 {
            rich.normalize("첫 줄\n둘째 줄\n마지막");
        }
        assert_eq!(rich.images[0].at, 4);
    }

    #[test]
    fn indenting_several_lines_preserves_their_independent_styles_and_images() {
        let old = "가나\n다라\n마바";
        let new = "\t가나\n\t다라\n\t마바";
        let mut rich = RichScript::default();
        rich.apply(3..5, |s| s.underline = true);
        rich.insert_image(1, 6);
        rich.text_changed(old, new);
        assert!(rich.style(5).underline);
        assert!(!rich.style(1).underline);
        assert_eq!(rich.images[0].at, 9);
        rich.text_changed(new, old);
        assert!(rich.style(3).underline);
        assert_eq!(rich.images[0].at, 6);
    }
}
