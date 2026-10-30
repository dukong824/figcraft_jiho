use super::model::Section;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Video { video_id: u64, section: Section },
    History,
    Guide { channel_id: Option<u64> },
}

impl Route {
    pub fn video(self) -> Option<(u64, Section)> {
        match self {
            Self::Video { video_id, section } => Some((video_id, section)),
            Self::History | Self::Guide { .. } => None,
        }
    }
}

#[derive(Clone, Debug)]
struct Tab {
    id: u64,
    history: Vec<Option<Route>>,
    position: usize,
}

impl Tab {
    fn new(id: u64, route: Option<Route>) -> Self {
        Self { id, history: vec![route], position: 0 }
    }

    fn route(&self) -> Option<Route> {
        self.history.get(self.position).copied().flatten()
    }

    fn navigate(&mut self, route: Route) {
        if self.route() == Some(route) {
            return;
        }
        self.history.truncate(self.position + 1);
        self.history.push(Some(route));
        self.position = self.history.len() - 1;
    }
}

pub struct Tabs {
    tabs: Vec<Tab>,
    active: usize,
    next_id: u64,
}

impl Tabs {
    pub fn new(initial: Option<Route>) -> Self {
        Self { tabs: vec![Tab::new(1, initial)], active: 0, next_id: 2 }
    }

    pub fn route(&self) -> Option<Route> {
        self.tabs.get(self.active).and_then(Tab::route)
    }

    pub fn items(&self) -> impl Iterator<Item = (usize, u64, Option<Route>, bool)> + '_ {
        self.tabs.iter().enumerate().map(|(index, tab)| (index, tab.id, tab.route(), index == self.active))
    }

    pub fn navigate(&mut self, route: Route) {
        if let Some(tab) = self.tabs.get_mut(self.active) {
            tab.navigate(route);
        }
    }

    pub fn new_tab(&mut self) {
        self.tabs.push(Tab::new(self.next_id, None));
        self.next_id += 1;
        self.active = self.tabs.len() - 1;
    }

    pub fn select(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active = index;
        }
    }

    pub fn close(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.new_tab();
        } else if index < self.active || self.active == self.tabs.len() {
            self.active = self.active.saturating_sub(1);
        }
    }

    pub fn remove_video(&mut self, id: u64) {
        for tab in &mut self.tabs {
            let current = tab.route();
            tab.history.retain(|r| r.is_none_or(|r| r.video().is_none_or(|(video_id, _)| video_id != id)));
            if tab.history.is_empty() {
                tab.history.push(None);
            }
            tab.position = tab.history.iter().rposition(|r| *r == current).unwrap_or(tab.history.len() - 1);
        }
    }

    pub fn can_back(&self) -> bool {
        self.tabs.get(self.active).is_some_and(|tab| tab.position > 0)
    }

    pub fn can_forward(&self) -> bool {
        self.tabs.get(self.active).is_some_and(|tab| tab.position + 1 < tab.history.len())
    }

    pub fn back(&mut self) {
        if self.can_back()
            && let Some(tab) = self.tabs.get_mut(self.active)
        {
            tab.position -= 1;
        }
    }

    pub fn forward(&mut self) {
        if self.can_forward()
            && let Some(tab) = self.tabs.get_mut(self.active)
        {
            tab.position += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_is_a_browser_page_and_survives_video_removal() {
        let video = route(1, Section::Script);
        let mut tabs = Tabs::new(Some(video));
        tabs.navigate(Route::History);
        tabs.back();
        assert_eq!(tabs.route(), Some(video));
        tabs.forward();
        assert_eq!(tabs.route(), Some(Route::History));
        tabs.new_tab();
        tabs.navigate(route(2, Section::Edit));
        tabs.select(0);
        tabs.remove_video(1);
        assert_eq!(tabs.route(), Some(Route::History));
        assert!(!tabs.can_back());
    }

    #[test]
    fn trashing_a_video_removes_it_from_all_tab_histories() {
        let mut tabs = Tabs::new(Some(Route::Video { video_id: 1, section: Section::Script }));
        tabs.navigate(Route::Video { video_id: 2, section: Section::Design });
        tabs.new_tab();
        tabs.navigate(Route::Video { video_id: 2, section: Section::Library });
        tabs.remove_video(2);
        assert_eq!(tabs.route(), None);
        tabs.select(0);
        assert_eq!(tabs.route().unwrap().video().unwrap().0, 1);
        assert!(!tabs.can_forward());
    }

    fn route(video_id: u64, section: Section) -> Route {
        Route::Video { video_id, section }
    }

    #[test]
    fn each_tab_keeps_its_own_history_and_forward_branch_is_replaced() {
        let first = route(1, Section::Script);
        let second = route(1, Section::Design);
        let third = route(2, Section::Edit);
        let mut tabs = Tabs::new(Some(first));
        tabs.navigate(second);
        tabs.new_tab();
        assert_eq!(tabs.route(), None);
        tabs.navigate(third);
        tabs.back();
        assert_eq!(tabs.route(), None);
        tabs.forward();
        assert_eq!(tabs.route(), Some(third));
        tabs.select(0);
        assert_eq!(tabs.route(), Some(second));
        tabs.back();
        assert_eq!(tabs.route(), Some(first));
        tabs.navigate(third);
        assert!(!tabs.can_forward());
        tabs.select(1);
        assert_eq!(tabs.route(), Some(third));
    }

    #[test]
    fn closing_tabs_selects_a_neighbor_and_keeps_one_blank_tab() {
        let first = route(1, Section::Library);
        let mut tabs = Tabs::new(Some(first));
        tabs.new_tab();
        tabs.navigate(route(2, Section::Script));
        tabs.new_tab();
        tabs.close(1);
        assert_eq!(tabs.route(), None);
        tabs.close(1);
        assert_eq!(tabs.route(), Some(first));
        tabs.close(0);
        assert_eq!(tabs.route(), None);
        assert_eq!(tabs.items().count(), 1);
    }
}
