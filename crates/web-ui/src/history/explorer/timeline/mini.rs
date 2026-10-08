//! Recent sidebar activities use the editor's native rows and graph fragments.

use std::{cell::RefCell, rc::Rc};

use app_core::history::{
    Event as HistoryEvent, RequestState,
    timeline::{Event, Row, Surface, ViewModel},
};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{
    Element, EventHandler, ModifiersInteraction, MountedData, Props, ReadableExt, WritableExt,
    component, dioxus_core, dioxus_elements, rsx, use_effect, use_hook, use_reactive, use_signal,
};

use super::{State, graph::GraphRow, invalidate, key_action, row::single_click, row_id};
use crate::{
    controls::Button,
    history::graph::browser,
    icons::{Icon, IconName},
};

#[component]
pub(in crate::history::explorer) fn MiniTimeline(
    id: String,
    chain: Option<String>,
    view: ViewModel,
    height: u32,
    onaction: EventHandler<HistoryEvent>,
    onopen: EventHandler<()>,
) -> Element {
    let _load = use_effect(use_reactive((&chain,), move |(chain,)| {
        if chain.is_some() {
            onaction.call(HistoryEvent::Timeline(Event::Load(Surface::Mini)));
        }
    }));
    let state = use_hook(move || {
        let mut state = State::default();
        state.viewport.height = f64::from(height);
        Rc::new(RefCell::new(state))
    });
    let mut mounted = use_signal(|| None::<Rc<MountedData>>);
    let tick = use_signal(|| 0_u64);
    let _revision = *tick.read();
    let mini = &view.mini;
    let selected = view
        .selected
        .as_ref()
        .map(|selection| &selection.occurrence);
    let highlight = mini
        .window
        .as_ref()
        .into_iter()
        .flat_map(|window| &window.rows)
        .find(|row| Some(&row.occurrence) == selected)
        .map(|row| row.graph.lane);
    let (rows, top, total, active, anchor, groups, at_newest) = {
        let mut state = state.borrow_mut();
        state.reconcile(mini);
        let rows: Vec<_> = state
            .viewport
            .mounted_indices()
            .into_iter()
            .filter_map(|index| {
                let row = mini.window.as_ref()?.rows.get(index)?.clone();
                let position = state.viewport.rows.get(index)?;
                let selected = selected == Some(&row.occurrence);
                Some((row, position.top, selected))
            })
            .collect();
        let (anchor, groups, at_newest) = state.reading(mini);
        let active = selected
            .filter(|id| state.viewport.index(id).is_some())
            .map(|key| row_id(&id, key));
        (
            rows,
            state.viewport.top,
            state.viewport.total,
            active,
            anchor,
            groups,
            at_newest,
        )
    };
    let _scroll = use_effect(use_reactive((&top,), move |(top,)| {
        if let Some(mounted) = mounted.read().as_ref() {
            browser::scroll(mounted, top);
        }
    }));
    let _visible = use_effect(use_reactive(
        (&anchor, &groups, &at_newest),
        move |(anchor, groups, at_newest)| {
            onaction.call(HistoryEvent::Timeline(Event::Visible {
                surface: Surface::Mini,
                anchor,
                groups,
                at_newest,
            }));
        },
    ));
    let scroll_state = Rc::clone(&state);
    let resize_state = Rc::clone(&state);
    let scroll_view = mini.clone();
    let key_view = view.clone();
    let key_state = Rc::clone(&state);
    let busy = mini.state == RequestState::Loading;
    let continuation = mini
        .window
        .as_ref()
        .is_some_and(|window| window.older.is_some() || window.max_lane > 2);
    rsx! {
        div { class: "idle-history-mini",
            div { id: id.clone(), class: "idle-history-viewport idle-mini-viewport", role: "grid", tabindex: "0",
                aria_label: "Recent workspace activity", aria_activedescendant: active, aria_colcount: "2", aria_busy: busy.to_string(),
                style: "height:{height}px", onmounted: move |event| mounted.set(Some(event.data())),
                onresize: move |event| { event.stop_propagation(); if let Ok(size) = event.get_content_box_size()
                    && resize_state.borrow_mut().viewport.resize(size.width, size.height) { invalidate(tick); } },
                onscroll: move |event| { let mut state = scroll_state.borrow_mut(); state.viewport.scroll(event.scroll_top());
                    onaction.call(HistoryEvent::Timeline(state.visible(Surface::Mini, &scroll_view))); invalidate(tick); },
                onkeydown: move |event| {
                    if event.is_composing() || !event.modifiers().is_empty() || !browser::tree_key(&event.data()) { return; }
                    if let Some(action) = key_action(&key_view, Surface::Mini, &event.key(), key_state.borrow().visible_count()) {
                        event.prevent_default(); event.stop_propagation(); onaction.call(HistoryEvent::Timeline(action));
                    }
                },
                div { class: "idle-mini-rows", style: "height:{total}px",
                    for (row, top, selected) in rows {
                        MiniRow { key: "{row.occurrence}", id: row_id(&id, &row.occurrence), row, top, selected, highlight, onaction }
                    }
                }
                if mini.window.is_none() && !busy { span { class: "idle-mini-empty", "No recorded activity" } }
            }
            div { class: "idle-mini-footer",
                if busy { span { role: "status", "Updating…" } }
                else if continuation { span { "History continues" } }
                if mini.new_activity { Button { label: "New activity", onpress: move |()| onaction.call(HistoryEvent::Timeline(Event::Latest(Surface::Mini))) } }
                Button { label: "Open Activity", onpress: onopen }
            }
            if let RequestState::Failed(error) = &mini.state {
                div { class: "idle-mini-error", role: "alert", "{error.message}"
                    Button { label: "Retry", onpress: move |()| onaction.call(HistoryEvent::Timeline(Event::Refresh(Surface::Mini))) }
                }
            }
        }
    }
}

#[component]
fn MiniRow(
    id: String,
    row: Row,
    top: f64,
    selected: bool,
    highlight: Option<u32>,
    onaction: EventHandler<HistoryEvent>,
) -> Element {
    let occurrence = row.occurrence.clone();
    let title = if row.preview.is_empty() {
        row.title.clone()
    } else {
        format!("{} · {}", row.title, row.preview)
    };
    let date = row.timestamp.map_or_else(
        || "Time not recorded".into(),
        super::super::summary::timestamp,
    );
    rsx! {
        div { id, role: "row", class: "idle-history-mini-row", aria_selected: selected.to_string(),
            "data-occurrence": row.occurrence, "data-operation": row.address.record().map(|address| address.record.operation.clone()), "data-record-hash": row.address.record().map(|address| address.record.hash.clone()),
            title: format!("{title}\n{} · {}\n{date}", row.author, row.session), style: "transform:translateY({top}px)",
            onclick: move |event| { if browser::row_click(&event.data()) && single_click(&event.data()) {
                onaction.call(HistoryEvent::Timeline(Event::Select { surface: Surface::Mini, occurrence: occurrence.clone(), open: false }));
            } },
            GraphRow { graph: row.graph, width: 80.0, pitch: 22.0, pan: 0.0, highlight, selected }
            div { class: "idle-mini-summary", role: "gridcell",
                if let Some(group) = row.group.filter(|group| group.header) {
                    button { r#type: "button", class: "idle-timeline-disclosure", tabindex: "-1", aria_expanded: group.expanded.to_string(),
                        aria_label: format!("{} {} activities", if group.expanded { "Collapse" } else { "Expand" }, group.count),
                        onclick: move |event| { event.stop_propagation(); onaction.call(HistoryEvent::Timeline(Event::Toggle { surface: Surface::Mini, group: group.id.clone() })); },
                        Icon { name: if group.expanded { IconName::ChevronDown } else { IconName::ChevronRight } }
                    }
                    span { class: "idle-mini-count", "{group.count}" }
                }
                span { class: "idle-mini-title", "{title}" }
            }
        }
    }
}
