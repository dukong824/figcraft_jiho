//! Selection is keyed by identity, so filtering and scrolling never select the wrong row.
use egui::{Modifiers, Pos2, Rect};
pub(super) struct Selection<T> {
    pub items: Vec<T>,
    anchor: Option<T>,
    pub marquee: Option<(Pos2, Vec<T>)>,
}
impl<T> Default for Selection<T> {
    fn default() -> Self {
        Self { items: Vec::new(), anchor: None, marquee: None }
    }
}
impl<T: Copy + PartialEq> Selection<T> {
    pub fn click(&mut self, id: T, order: &[T], modifiers: Modifiers) {
        if modifiers.shift
            && let (Some(a), Some(b)) = (self.anchor.and_then(|a| order.iter().position(|v| *v == a)), order.iter().position(|v| *v == id))
        {
            self.items = order[a.min(b)..=a.max(b)].to_vec();
            return;
        }
        if modifiers.command {
            if self.items.contains(&id) {
                self.items.retain(|v| *v != id);
            } else {
                self.items.push(id);
            }
        } else {
            self.items = vec![id];
        }
        self.anchor = Some(id);
    }
    pub fn drag(&mut self, pointer: Pos2, rects: &[(T, Rect)]) {
        if let Some((start, base)) = &self.marquee {
            self.items = base.clone();
            let bounds = Rect::from_two_pos(*start, pointer);
            for (id, rect) in rects {
                if bounds.intersects(*rect) && !self.items.contains(id) {
                    self.items.push(*id);
                }
            }
        }
    }
    pub fn retain(&mut self, order: &[T]) {
        self.items.retain(|id| order.contains(id));
        if self.anchor.is_some_and(|id| !order.contains(&id)) {
            self.anchor = None;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shift_range_toggle_and_marquee_use_visible_order_and_identity() {
        let mut s = Selection::default();
        s.click(3, &[3, 8, 2], Modifiers::NONE);
        s.click(2, &[3, 8, 2], Modifiers { shift: true, ..Default::default() });
        assert_eq!(s.items, vec![3, 8, 2]);
        s.click(8, &[3, 8, 2], Modifiers::COMMAND);
        assert_eq!(s.items, vec![3, 2]);
        s.retain(&[8, 2]);
        assert_eq!(s.items, vec![2]);
        s.marquee = Some((Pos2::ZERO, vec![]));
        s.drag(
            egui::pos2(50.0, 80.0),
            &[
                (8, Rect::from_min_size(egui::pos2(0.0, 10.0), egui::vec2(100.0, 30.0))),
                (2, Rect::from_min_size(egui::pos2(0.0, 100.0), egui::vec2(100.0, 30.0))),
            ],
        );
        assert_eq!(s.items, vec![8]);
    }
}
