//! UI fixtures use exported native windows; no fixture code calculates topology.

use std::collections::{BTreeMap, BTreeSet};

use app_core::{
    ViewModel,
    history::{
        self,
        timeline::{Cursor, Event, Match, Row, Selection, Surface, SurfaceView, Window},
    },
    module::EffectError,
    workspace,
};

pub(super) const RECORDS: &str = include_str!("timeline.json");

pub(super) struct Fixture {
    pub view: ViewModel,
    rows: Vec<Row>,
    headers: BTreeMap<String, Row>,
    local: Local,
    mini: Local,
    compact: bool,
    pub opens: u64,
}

#[derive(Default)]
struct Local {
    choices: BTreeMap<String, bool>,
    temporary: BTreeSet<String>,
    anchor: Option<String>,
    at_newest: bool,
}

impl Fixture {
    pub(super) fn load() -> Result<Self, String> {
        let value: serde_json::Value =
            serde_json::from_str(RECORDS).map_err(|error| error.to_string())?;
        Self::from_recording(&value)
    }

    pub(super) fn load_case(name: &str) -> Result<Self, String> {
        if name == "dense" {
            return Self::load();
        }
        let cases: serde_json::Value = serde_json::from_str(include_str!("simple-cases.json"))
            .map_err(|error| error.to_string())?;
        let window = cases.get(name).ok_or("Unknown graph case")?;
        let focus = window
            .pointer("/rows/0/occurrence")
            .ok_or("Missing case row")?;
        Self::from_recording(&serde_json::json!({
            "topology": window, "windows": [window], "focus": focus,
        }))
    }

    fn from_recording(value: &serde_json::Value) -> Result<Self, String> {
        let topology: Window = serde_json::from_value(
            value
                .get("topology")
                .cloned()
                .ok_or("Missing native topology")?,
        )
        .map_err(|error| error.to_string())?;
        let windows: Vec<Window> = serde_json::from_value(
            value
                .get("windows")
                .cloned()
                .ok_or("Missing native windows")?,
        )
        .map_err(|error| error.to_string())?;
        let rows = windows.into_iter().flat_map(|window| window.rows).collect();
        let headers = topology
            .rows
            .iter()
            .filter_map(|row| {
                row.group
                    .as_ref()
                    .filter(|group| group.header)
                    .map(|group| (group.id.clone(), row.clone()))
            })
            .collect();
        let focus = value
            .get("focus")
            .and_then(serde_json::Value::as_str)
            .ok_or("Missing native fixture focus")?;
        let start = topology
            .rows
            .iter()
            .position(|row| row.occurrence == focus)
            .unwrap_or(0)
            .saturating_sub(3);
        let mut initial = topology;
        initial.offset = initial
            .offset
            .saturating_add(u64::try_from(start).unwrap_or(0));
        initial.rows = initial.rows.into_iter().skip(start).collect();
        initial.newer = (initial.offset > 0).then(|| cursor(initial.offset.saturating_sub(200)));
        let mut recent = initial.clone();
        recent.rows.truncate(40);
        let view = ViewModel {
            workspace: workspace::ViewModel {
                selected_workspace: Some("fixture-workspace".into()),
                section: workspace::NavigationSection::Activity,
                workspaces: vec![workspace::WorkspaceInfo {
                    id: "fixture-workspace".into(),
                    name: "memos · native timeline fixture".into(),
                    chain: "fixture-chain".into(),
                    revision: 1,
                    mode: workspace::WorkspaceMode::Standalone,
                    repositories: Vec::new(),
                }],
                ..workspace::ViewModel::default()
            },
            history: history::ViewModel {
                chain: Some("fixture-chain".into()),
                timeline: history::timeline::ViewModel {
                    editor: SurfaceView {
                        window: Some(initial),
                        state: history::RequestState::Ready,
                        ..SurfaceView::default()
                    },
                    mini: SurfaceView {
                        window: Some(recent),
                        state: history::RequestState::Ready,
                        ..SurfaceView::default()
                    },
                    ..history::timeline::ViewModel::default()
                },
                ..history::ViewModel::default()
            },
            ..ViewModel::default()
        };
        Ok(Self {
            view,
            rows,
            headers,
            local: Local::default(),
            mini: Local::default(),
            compact: false,
            opens: 0,
        })
    }

    pub(super) fn dispatch(&mut self, event: history::Event) {
        if let history::Event::Timeline(event) = event {
            let compact = matches!(
                &event,
                Event::Load(Surface::Mini)
                    | Event::Latest(Surface::Mini)
                    | Event::Refresh(Surface::Mini)
                    | Event::Cancel(Surface::Mini)
                    | Event::Seek {
                        surface: Surface::Mini,
                        ..
                    }
                    | Event::Reveal {
                        surface: Surface::Mini,
                        ..
                    }
                    | Event::Page {
                        surface: Surface::Mini,
                        ..
                    }
                    | Event::Filter {
                        surface: Surface::Mini,
                        ..
                    }
                    | Event::Select {
                        surface: Surface::Mini,
                        ..
                    }
                    | Event::Move {
                        surface: Surface::Mini,
                        ..
                    }
                    | Event::Toggle {
                        surface: Surface::Mini,
                        ..
                    }
                    | Event::Find {
                        surface: Surface::Mini,
                        ..
                    }
                    | Event::Match {
                        surface: Surface::Mini,
                        ..
                    }
                    | Event::Visible {
                        surface: Surface::Mini,
                        ..
                    }
            );
            self.compact = compact;
            if compact {
                self.swap_surface();
            }
            self.timeline(event);
            if compact {
                self.swap_surface();
            }
            self.compact = false;
        }
    }

    fn swap_surface(&mut self) {
        std::mem::swap(&mut self.local, &mut self.mini);
        let timeline = &mut self.view.history.timeline;
        std::mem::swap(&mut timeline.editor, &mut timeline.mini);
    }

    fn page_size(&self) -> usize {
        if self.compact { 40 } else { 200 }
    }

    fn timeline(&mut self, event: Event) {
        match event {
            Event::Select {
                occurrence, open, ..
            } => {
                if let Some(row) = self.rows.iter().find(|row| row.occurrence == occurrence) {
                    self.view.history.timeline.selected = Some(Selection {
                        occurrence,
                        address: row.address.clone(),
                    });
                    if open {
                        self.opens = self.opens.saturating_add(1);
                        self.view.history.timeline.open = history::RequestState::Ready;
                    }
                }
            }
            Event::Reveal { selection, .. } => {
                self.view.history.timeline.selected = Some(selection.clone());
                self.seek(&selection.occurrence);
            }
            Event::Move { delta, .. } => self.navigate(delta),
            Event::Seek { occurrence, .. } => self.seek(&occurrence),
            Event::Toggle { group, .. } => {
                let expanded = self
                    .view
                    .history
                    .timeline
                    .editor
                    .window
                    .as_ref()
                    .into_iter()
                    .flat_map(|window| &window.rows)
                    .filter_map(|row| row.group.as_ref())
                    .find(|value| value.id == group)
                    .is_some_and(|group| group.expanded);
                let _previous = self.local.choices.insert(group.clone(), !expanded);
                let _removed = self.local.temporary.remove(&group);
                self.refresh();
            }
            Event::Find { text, .. } => {
                self.local.temporary.clear();
                let matches = self
                    .rows
                    .iter()
                    .filter(|row| {
                        !text.is_empty()
                            && (row.preview.to_lowercase().contains(&text.to_lowercase())
                                || row.title.to_lowercase().contains(&text.to_lowercase()))
                    })
                    .map(|row| Match {
                        occurrence: row.occurrence.clone(),
                        group: row.group.as_ref().map(|group| group.id.clone()),
                        address: row.address.clone(),
                        preview: row.preview.clone(),
                    })
                    .collect::<Vec<_>>();
                let total = u64::try_from(matches.len()).unwrap_or(0);
                self.view.history.timeline.editor.search = history::timeline::Search {
                    text,
                    matches,
                    total,
                    state: history::RequestState::Ready,
                    ..history::timeline::Search::default()
                };
                if total > 0 {
                    self.find_match(0);
                } else {
                    self.refresh();
                }
            }
            Event::Match { delta, .. } => self.find_match(delta),
            Event::Page { newer, .. } => {
                let current = self
                    .view
                    .history
                    .timeline
                    .editor
                    .window
                    .as_ref()
                    .map_or(0, |window| window.offset);
                let limit = u64::try_from(self.page_size()).unwrap_or(0);
                self.show(if newer {
                    current.saturating_sub(limit)
                } else {
                    current.saturating_add(limit)
                });
            }
            Event::Latest(_) => {
                self.local.at_newest = true;
                self.view.history.timeline.editor.focus = None;
                self.view.history.timeline.editor.new_activity = false;
                self.show(0);
            }
            Event::Refresh(_) => self.refresh(),
            Event::Visible {
                surface,
                anchor,
                at_newest,
                ..
            } => {
                if surface == Surface::Editor || self.compact {
                    self.local.anchor = anchor;
                    self.local.at_newest = at_newest;
                }
            }
            Event::Filter { filter, .. } => {
                self.view.history.timeline.editor.filter = filter;
                self.view.history.timeline.editor.focus = None;
                self.show(0);
            }
            Event::Open(_) => self.opens = self.opens.saturating_add(1),
            Event::Load(_) | Event::Cancel(_) | Event::Suspend | Event::Completed { .. } => {}
        }
    }

    fn visible_rows(&self) -> Vec<Row> {
        let kinds = &self.view.history.timeline.editor.filter.kinds;
        self.rows
            .iter()
            .filter(|row| kinds.is_empty() || kinds.contains(&row.kind))
            .filter_map(|row| {
                let mut row = row.clone();
                if let Some(group) = &mut row.group {
                    group.expanded = self.local.temporary.contains(&group.id)
                        || self
                            .local
                            .choices
                            .get(&group.id)
                            .copied()
                            .unwrap_or(group.live);
                    if !group.expanded {
                        return if group.header {
                            self.headers.get(&group.id).cloned()
                        } else {
                            None
                        };
                    }
                }
                Some(row)
            })
            .collect()
    }

    fn show(&mut self, offset: u64) {
        let rows = self.visible_rows();
        let total = u64::try_from(rows.len()).unwrap_or(0);
        let offset = offset.min(total.saturating_sub(1));
        let limit = self.page_size();
        let rows = rows
            .into_iter()
            .skip(usize::try_from(offset).unwrap_or(0))
            .take(limit)
            .collect::<Vec<_>>();
        let end = offset.saturating_add(u64::try_from(rows.len()).unwrap_or(0));
        if let Some(window) = &mut self.view.history.timeline.editor.window {
            window.rows = rows;
            window.offset = offset;
            window.visible_rows = total;
            window.newer = (offset > 0)
                .then(|| cursor(offset.saturating_sub(u64::try_from(limit).unwrap_or(0))));
            window.older = (end < total).then(|| cursor(end));
        }
        self.view.history.timeline.editor.state = history::RequestState::Ready;
    }

    fn refresh(&mut self) {
        let rows = self.visible_rows();
        let offset = self
            .local
            .anchor
            .as_ref()
            .and_then(|anchor| rows.iter().position(|row| &row.occurrence == anchor))
            .map_or(0, |index| {
                u64::try_from(index.saturating_sub(10)).unwrap_or(0)
            });
        self.show(if self.local.at_newest { 0 } else { offset });
    }

    fn seek(&mut self, occurrence: &str) {
        if let Some(group) = self
            .rows
            .iter()
            .find(|row| row.occurrence == occurrence)
            .and_then(|row| row.group.as_ref())
        {
            let _inserted = self.local.temporary.insert(group.id.clone());
        }
        let rows = self.visible_rows();
        let offset = rows
            .iter()
            .position(|row| row.occurrence == occurrence)
            .map_or(0, |index| {
                u64::try_from(index.saturating_sub(30)).unwrap_or(0)
            });
        self.show(offset);
        self.view.history.timeline.editor.focus = Some(occurrence.into());
    }

    fn navigate(&mut self, delta: i32) {
        let rows = self.visible_rows();
        let current = self
            .view
            .history
            .timeline
            .selected
            .as_ref()
            .and_then(|selected| {
                rows.iter()
                    .position(|row| row.occurrence == selected.occurrence)
            });
        let next = current.map_or(
            self.view
                .history
                .timeline
                .editor
                .window
                .as_ref()
                .map_or(0, |window| usize::try_from(window.offset).unwrap_or(0)),
            |index| {
                if delta < 0 {
                    index.saturating_sub(usize::try_from(delta.unsigned_abs()).unwrap_or(0))
                } else {
                    index
                        .saturating_add(usize::try_from(delta).unwrap_or(0))
                        .min(rows.len().saturating_sub(1))
                }
            },
        );
        if let Some(row) = rows.get(next) {
            let occurrence = row.occurrence.clone();
            self.timeline(Event::Select {
                surface: Surface::Editor,
                occurrence: occurrence.clone(),
                open: false,
            });
            if !self
                .view
                .history
                .timeline
                .editor
                .window
                .as_ref()
                .is_some_and(|window| window.rows.iter().any(|row| row.occurrence == occurrence))
            {
                self.show(u64::try_from(next.saturating_sub(30)).unwrap_or(0));
            }
            self.view.history.timeline.editor.focus = Some(occurrence);
        }
    }

    fn find_match(&mut self, delta: i32) {
        let search = &mut self.view.history.timeline.editor.search;
        if search.total == 0 {
            return;
        }
        let current = search.current.unwrap_or(0);
        let next = if delta < 0 {
            current
                .saturating_add(search.total)
                .saturating_sub(u64::from(delta.unsigned_abs()))
                .checked_rem(search.total)
                .unwrap_or(0)
        } else {
            current
                .saturating_add(u64::try_from(delta).unwrap_or(0))
                .checked_rem(search.total)
                .unwrap_or(0)
        };
        search.current = Some(next);
        if let Some(hit) = usize::try_from(next)
            .ok()
            .and_then(|index| search.matches.get(index))
            .cloned()
        {
            self.local.temporary.clear();
            self.view.history.timeline.selected = Some(Selection {
                occurrence: hit.occurrence.clone(),
                address: hit.address,
            });
            self.seek(&hit.occurrence);
        }
    }

    pub(super) fn fail(&mut self) {
        self.view.history.timeline.editor.state = history::RequestState::Failed(EffectError {
            message: "Fixture history is offline".into(),
        });
    }
}

fn cursor(offset: u64) -> Cursor {
    Cursor {
        revision: "fixture".into(),
        view: "fixture".into(),
        offset,
    }
}
