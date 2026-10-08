//! Fixed-height viewport anchors over bounded native windows.

use app_core::history::timeline::{Event, Surface, SurfaceView};

use crate::history::graph::viewport::Viewport;

pub(super) const ROW_HEIGHT: f64 = 28.0;

pub(super) struct State {
    pub viewport: Viewport,
    offset: u64,
    focus: Option<String>,
    focus_revision: u64,
}

impl Default for State {
    fn default() -> Self {
        let mut viewport = Viewport::default();
        viewport.row_height = ROW_HEIGHT;
        Self {
            viewport,
            offset: 0,
            focus: None,
            focus_revision: 0,
        }
    }
}

impl State {
    pub(super) fn reconcile(&mut self, view: &SurfaceView) {
        let Some(window) = &view.window else {
            return;
        };
        let newest = self.offset == 0 && self.viewport.top < 1.0;
        self.viewport.update(
            &window
                .rows
                .iter()
                .map(|row| row.occurrence.clone())
                .collect::<Vec<_>>(),
        );
        self.offset = window.offset;
        if newest && window.offset == 0 {
            self.viewport.scroll(0.0);
        }
        if self.focus != view.focus || self.focus_revision != view.focus_revision {
            if let Some(index) = view.focus.as_ref().and_then(|id| self.viewport.index(id)) {
                self.viewport.focus(index);
                self.focus.clone_from(&view.focus);
                self.focus_revision = view.focus_revision;
            } else if view.focus.is_none() && window.offset == 0 {
                self.viewport.scroll(0.0);
                self.viewport.focused = None;
                self.focus = None;
                self.focus_revision = view.focus_revision;
            }
        }
    }

    pub(super) fn reading(&self, view: &SurfaceView) -> (Option<String>, Vec<String>, bool) {
        let first = self
            .viewport
            .rows
            .partition_point(|row| row.top + row.height <= self.viewport.top);
        let groups = view
            .window
            .as_ref()
            .into_iter()
            .flat_map(|window| {
                window
                    .rows
                    .iter()
                    .skip(first)
                    .take(self.visible_count())
                    .filter_map(|row| row.group.as_ref().map(|group| group.id.clone()))
            })
            .collect();
        (
            self.viewport.rows.get(first).map(|row| row.key.clone()),
            groups,
            self.offset == 0 && self.viewport.top < 1.0,
        )
    }

    pub(super) fn visible(&self, surface: Surface, view: &SurfaceView) -> Event {
        let (anchor, groups, at_newest) = self.reading(view);
        Event::Visible {
            surface,
            anchor,
            groups,
            at_newest,
        }
    }

    pub(super) fn visible_count(&self) -> usize {
        self.viewport
            .rows
            .iter()
            .filter(|row| {
                row.top < self.viewport.top + self.viewport.height
                    && row.top + row.height > self.viewport.top
            })
            .count()
    }
}

#[cfg(test)]
mod tests {
    use app_core::history::timeline::SurfaceView;

    use super::State;

    fn view() -> SurfaceView {
        SurfaceView {
            window: Some(super::super::super::tests::native_window()),
            ..SurfaceView::default()
        }
    }

    #[test]
    fn prepend_and_eviction_retain_the_same_occurrence_and_pixel_offset() {
        let mut view = view();
        let mut state = State::default();
        state.viewport.height = 56.0;
        state.reconcile(&view);
        state.viewport.scroll(285.0);
        let before = state.reading(&view).0;
        let Some(window) = &mut view.window else {
            return;
        };
        let mut added = window.rows.first().unwrap().clone();
        added.occurrence = "newest".into();
        window.rows.insert(0, added);
        state.reconcile(&view);
        assert_eq!(
            state.reading(&view).0,
            before,
            "prepend keeps the visible occurrence"
        );
        assert!(
            (state.viewport.top - 313.0).abs() < 0.1,
            "within-row offset survives a prepend"
        );
        let Some(window) = &mut view.window else {
            return;
        };
        drop(window.rows.drain(..5));
        window.offset = window.offset.saturating_add(5);
        state.reconcile(&view);
        assert_eq!(
            state.reading(&view).0,
            before,
            "cache eviction preserves the surviving anchor"
        );
        assert!(
            (state.viewport.top - 173.0).abs() < 0.1,
            "eviction preserves the five-pixel offset"
        );
    }

    #[test]
    fn deferred_and_repeated_seeks_apply_only_when_the_destination_arrives() {
        let mut view = view();
        let mut state = State::default();
        state.viewport.height = 56.0;
        state.reconcile(&view);
        view.focus = Some("later-window".into());
        view.focus_revision = 1;
        state.reconcile(&view);
        assert_ne!(
            state.focus, view.focus,
            "an unavailable destination stays pending"
        );
        let Some(window) = &mut view.window else {
            return;
        };
        window.rows.get_mut(30).unwrap().occurrence = "later-window".into();
        state.reconcile(&view);
        assert_eq!(
            state.focus, view.focus,
            "the arriving page completes the pending seek"
        );
        assert!(state.viewport.top > 700.0, "the destination is revealed");
        state.viewport.scroll(0.0);
        view.focus_revision = 2;
        state.reconcile(&view);
        assert!(
            state.viewport.top > 700.0,
            "seeking the same item again is honored"
        );
        view.focus = None;
        view.focus_revision = 3;
        let Some(window) = &mut view.window else {
            return;
        };
        window.offset = 0;
        state.reconcile(&view);
        assert!(
            state.viewport.top < 0.1,
            "Latest returns to the newest position"
        );
    }

    #[test]
    fn latest_discards_the_old_resize_focus_before_filtered_rows_arrive() {
        let mut view = view();
        let mut state = State::default();
        state.viewport.height = 280.0;
        state.reconcile(&view);
        view.focus = Some(
            view.window
                .as_ref()
                .unwrap()
                .rows
                .get(30)
                .unwrap()
                .occurrence
                .clone(),
        );
        view.focus_revision = 1;
        state.reconcile(&view);
        view.focus = None;
        view.focus_revision = 2;
        state.reconcile(&view);
        let window = view.window.as_mut().unwrap();
        window.offset = 0;
        window.rows.truncate(12);
        state.reconcile(&view);
        assert!(state.viewport.resize(1000.0, 281.0));
        assert!(
            state.viewport.top.abs() < 0.1,
            "filtering keeps the newest row fully visible"
        );
    }
}
