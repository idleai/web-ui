//! Ephemeral graph state. Selection, disclosure and loading stay in app-core.

use std::collections::BTreeMap;

use app_core::history::{Event, Selected, ViewModel};
use dioxus::prelude::Key;

use super::model::{GraphSnapshot, Layout};
use super::viewport::Viewport;

#[derive(Debug, Default)]
pub(super) struct State {
    chain: Option<String>,
    initialized: bool,
    pub(super) snapshot: GraphSnapshot,
    pub(super) layout: Layout,
    pub(super) viewport: Viewport,
    pub(super) births: BTreeMap<String, f64>,
    pub(super) node_births: BTreeMap<String, f64>,
    selected: Selected,
    pending_reveal: Option<String>,
    pointer_selection: Option<Selected>,
}

impl State {
    pub(super) fn reconcile(&mut self, view: &ViewModel, now: f64) {
        if self.chain != view.chain {
            let (width, height) = (self.viewport.width, self.viewport.height);
            *self = Self {
                chain: view.chain.clone(),
                ..Self::default()
            };
            let _resized = self.viewport.resize(width, height);
        }
        if self.snapshot.items != view.items {
            let next = GraphSnapshot::from_view(view);
            if self.initialized {
                let old: BTreeMap<_, _> = self
                    .snapshot
                    .connections
                    .iter()
                    .map(|connection| (&connection.key, connection))
                    .collect();
                for connection in &next.connections {
                    if old.get(&connection.key).is_none_or(|old| {
                        old.source != connection.source || old.target != connection.target
                    }) {
                        let _: Option<f64> = self.births.insert(connection.key.clone(), now);
                    }
                }
                for item in &next.items {
                    if self.viewport.row(&item.key).is_none() {
                        let _: Option<f64> = self.node_births.insert(item.key.clone(), now);
                    }
                }
            }
            self.births.retain(|key, _| {
                next.connections
                    .iter()
                    .any(|connection| &connection.key == key)
            });
            self.node_births
                .retain(|key, _| next.items.iter().any(|item| &item.key == key));
            self.layout.reconcile(&next);
            self.viewport.update(&self.layout.order);
            self.snapshot = next;
        }
        if self.pointer_selection.as_ref() == Some(&view.selected) {
            self.pointer_selection = None;
            self.pending_reveal = None;
        } else if self.selected != view.selected {
            self.pointer_selection = None;
            self.pending_reveal.clone_from(&view.selected.item);
        }
        self.selected.clone_from(&view.selected);
        if let Some(index) = self
            .pending_reveal
            .as_ref()
            .and_then(|key| self.viewport.index(key))
        {
            self.viewport.focus(index);
            self.pending_reveal = None;
        }
        self.initialized |= !view.items.is_empty();
        self.births.retain(|_, born| now - *born < 240.0);
        self.node_births.retain(|_, born| now - *born < 160.0);
    }

    pub(super) fn click(&mut self, key: String) -> Event {
        if self.viewport.index(&key).is_some() {
            self.viewport.focused = Some(key.clone());
        }
        let selected = Selected {
            item: Some(key),
            observation: None,
        };
        // The host's selection update acknowledges this click without revealing it again.
        self.pointer_selection = Some(selected.clone());
        self.pending_reveal = None;
        Event::Select(selected)
    }

    pub(super) fn key(&mut self, key: &Key) -> (bool, Option<Event>) {
        let Some(current) = self
            .viewport
            .focused
            .as_ref()
            .and_then(|key| self.viewport.index(key))
        else {
            return (false, None);
        };
        if *key == Key::Enter || matches!(key, Key::Character(character) if character == " ") {
            return (true, self.selection());
        }
        if matches!(key, Key::ArrowLeft | Key::ArrowRight) {
            let item = self
                .snapshot
                .items
                .iter()
                .find(|item| Some(&item.key) == self.viewport.focused.as_ref());
            let event = item
                .filter(|item| item.expanded == (*key == Key::ArrowLeft))
                .map(|item| Event::ToggleDisclosure(item.key.clone()));
            return (true, event);
        }
        let index = if *key == Key::ArrowDown {
            current
                .saturating_add(1)
                .min(self.viewport.rows.len().saturating_sub(1))
        } else if *key == Key::ArrowUp {
            current.saturating_sub(1)
        } else if *key == Key::Home {
            0
        } else if *key == Key::End {
            self.viewport.rows.len().saturating_sub(1)
        } else if matches!(key, Key::PageDown | Key::PageUp) {
            let top = self.viewport.rows.get(current).map_or(0.0, |row| row.top);
            if *key == Key::PageDown {
                let target = top + self.viewport.height;
                self.viewport
                    .rows
                    .partition_point(|row| row.top < target)
                    .min(self.viewport.rows.len().saturating_sub(1))
            } else {
                let target = (top - self.viewport.height).max(0.0);
                self.viewport
                    .rows
                    .partition_point(|row| row.top <= target)
                    .saturating_sub(1)
            }
        } else {
            return (false, None);
        };
        self.viewport.focus(index);
        (true, None)
    }

    fn selection(&self) -> Option<Event> {
        self.viewport.focused.as_ref().map(|key| {
            Event::Select(Selected {
                item: Some(key.clone()),
                observation: None,
            })
        })
    }
}
