//! Full editor table over host-indexed rows and routing coverage.

mod graph;
mod mini;
mod row;
mod state;

use std::{cell::RefCell, rc::Rc};

use app_core::history::{
    ActivityKind, Event as HistoryEvent, Filter, RequestState,
    timeline::{Event, Surface, ViewModel},
};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{
    Element, EventHandler, InteractionLocation, Key, ModifiersInteraction, MountedData,
    PointerData, Props, ReadableExt, Signal, WritableExt, component, dioxus_core, dioxus_elements,
    rsx, use_effect, use_hook, use_reactive, use_signal,
};

use crate::{
    controls::{Button, ControlState},
    history::graph::browser,
};
pub(super) use mini::MiniTimeline;
use row::{ActivityRow, Paint};
use state::State;

#[component]
pub(super) fn TimelineTable(
    id: String,
    view: ViewModel,
    onaction: EventHandler<HistoryEvent>,
) -> Element {
    let state = use_hook(|| {
        let mut state = State::default();
        state.viewport.row_height = 34.0;
        Rc::new(RefCell::new(state))
    });
    let mut mounted = use_signal(|| None::<Rc<MountedData>>);
    let tick = use_signal(|| 0_u64);
    let _revision = *tick.read();
    let mut width = use_signal(|| 0.0_f64);
    let mut pitch = use_signal(|| 15.0_f64);
    let mut pan = use_signal(|| 0.0_f64);
    let mut drag = use_signal(|| None::<(f64, f64)>);
    let editor = &view.editor;
    let lane_count = editor.window.as_ref().map_or(1, |window| {
        let maximum = if editor.filter == Filter::default() {
            window.max_lane
        } else {
            window
                .rows
                .iter()
                .map(|row| graph::maximum(&row.graph))
                .max()
                .unwrap_or(0)
        };
        maximum.saturating_add(1)
    });
    let column_width = if width() == 0.0 {
        graph::x(lane_count, pitch()).clamp(44.0, 240.0)
    } else {
        width()
    };
    let graph_width = graph::x(lane_count, pitch()).max(column_width);
    let maximum_pan = (graph_width - column_width).max(0.0);
    let horizontal = pan().min(maximum_pan);
    let selected = view
        .selected
        .as_ref()
        .map(|selection| selection.occurrence.clone());
    let highlight = editor
        .window
        .as_ref()
        .into_iter()
        .flat_map(|window| &window.rows)
        .find(|row| Some(&row.occurrence) == selected.as_ref())
        .map(|row| row.graph.lane);
    let (rows, top, total, active, anchor, groups, at_newest) = {
        let mut state = state.borrow_mut();
        state.reconcile(editor);
        let rows = state
            .viewport
            .mounted_indices()
            .into_iter()
            .filter_map(|index| {
                let window = editor.window.as_ref()?;
                let row = window.rows.get(index)?.clone();
                let position = state.viewport.rows.get(index)?;
                let paint = Paint {
                    id: row_id(&id, &row.occurrence),
                    top: position.top,
                    position: window
                        .offset
                        .saturating_add(u64::try_from(index).unwrap_or(u64::MAX))
                        .saturating_add(2),
                    width: column_width,
                    pitch: pitch(),
                    pan: horizontal,
                    selected: selected.as_ref() == Some(&row.occurrence),
                    matched: editor
                        .search
                        .matches
                        .iter()
                        .any(|hit| hit.occurrence == row.occurrence),
                    highlight,
                };
                Some((row, paint))
            })
            .collect::<Vec<_>>();
        let (anchor, groups, at_newest) = state.reading(editor);
        let active = selected
            .as_ref()
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
    let _scroll_effect = use_effect(use_reactive((&top,), move |(top,)| {
        if let Some(mounted) = mounted.read().as_ref() {
            browser::scroll(mounted, top);
        }
    }));
    let _visible_effect = use_effect(use_reactive(
        (&anchor, &groups, &at_newest),
        move |(anchor, groups, at_newest)| {
            onaction.call(HistoryEvent::Timeline(Event::Visible {
                surface: Surface::Editor,
                anchor,
                groups,
                at_newest,
            }));
        },
    ));
    let scroll_state = Rc::clone(&state);
    let resize_state = Rc::clone(&state);
    let key_state = Rc::clone(&state);
    let key_view = view.clone();
    let scroll_view = editor.clone();
    let busy = editor.state == RequestState::Loading;
    let (count, shown, offset) = editor.window.as_ref().map_or((0, 0, 0), |window| {
        (window.activities, window.visible_rows, window.offset)
    });
    let count_label = if editor.window.is_none() && (busy || editor.progress.is_some()) {
        "Counting activities…".into()
    } else if editor.progress.is_some() {
        format!("{count} activities in the previous snapshot")
    } else {
        format!("{count} activities")
    };
    let newer = editor
        .window
        .as_ref()
        .is_some_and(|window| window.newer.is_some());
    let older = editor
        .window
        .as_ref()
        .is_some_and(|window| window.older.is_some());
    let filter = editor.filter.clone();
    let kind = filter_kind(&filter);
    let mounted_rows = rows.len();
    rsx! {
        div { class: "idle-timeline", style: "--timeline-graph-width:{column_width}px", "data-row-count": mounted_rows.to_string(),
            details { class: "idle-timeline-settings",
                summary { "View options" }
                div { class: "idle-timeline-tools",
                label { "Activity "
                    select { aria_label: "Filter activity kind", value: kind, onchange: move |event| {
                        let mut filter = filter.clone();
                        filter.kinds = kinds(&event.value());
                        onaction.call(HistoryEvent::Timeline(Event::Filter { surface: Surface::Editor, filter }));
                    },
                        option { value: "all", "All activities" } option { value: "file", "Files" }
                        option { value: "message", "Messages" } option { value: "tool", "Tools" }
                        option { value: "execution", "Executions" } option { value: "git", "Git commits" }
                    }
                }
                label { "Graph width " input { r#type: "range", min: "44", max: "600", step: "1", value: column_width.to_string(), aria_label: "Graph column width",
                    oninput: move |event| { if let Ok(value) = event.value().parse::<f64>() { width.set(value.clamp(44.0, 600.0)); } } } }
                label { "Zoom " input { r#type: "range", min: "10", max: "40", step: "1", value: pitch().to_string(), aria_label: "Graph lane spacing",
                    oninput: move |event| { if let Ok(value) = event.value().parse::<f64>() { pitch.set(value.clamp(10.0, 40.0)); } } } }
                if maximum_pan > 0.0 {
                    label { "Pan " input { r#type: "range", min: "0", max: maximum_pan.to_string(), value: horizontal.to_string(), aria_label: "Pan graph horizontally",
                        oninput: move |event| { if let Ok(value) = event.value().parse::<f64>() { pan.set(value.clamp(0.0, maximum_pan)); } } } }
                    span { class: "idle-timeline-continuation-label", "{lane_count} lanes" }
                }

            }
            }
            div { class: "idle-timeline-horizontal",
                div { id: id.clone(), class: "idle-history-viewport idle-timeline-viewport", role: "grid", tabindex: "0",
                    aria_label: "Activity history", aria_activedescendant: active, aria_rowcount: shown.saturating_add(1).to_string(), aria_colcount: "5", aria_busy: busy.to_string(),
                    onmounted: move |event| mounted.set(Some(event.data())),
                    onscroll: move |event| {
                        let mut state = scroll_state.borrow_mut();
                        state.viewport.scroll(event.scroll_top());
                        let before = state.viewport.top < 140.0 && newer;
                        let after = state.viewport.top + state.viewport.height > state.viewport.total - 280.0 && older;
                        onaction.call(HistoryEvent::Timeline(state.visible(Surface::Editor, &scroll_view)));
                        invalidate(tick);
                        if !busy && (before || after) { onaction.call(HistoryEvent::Timeline(Event::Page { surface: Surface::Editor, newer: before })); }
                    },
                    onresize: move |event| { event.stop_propagation(); if let Ok(size) = event.get_content_box_size()
                        && resize_state.borrow_mut().viewport.resize(size.width, (size.height - 28.0).max(1.0)) { invalidate(tick); } },
                    onkeydown: move |event| {
                        if event.is_composing() || !event.modifiers().is_empty() || !browser::tree_key(&event.data()) { return; }
                        if let Some(action) = key_action(&key_view, Surface::Editor, &event.key(), key_state.borrow().visible_count()) {
                            event.prevent_default(); event.stop_propagation();
                            onaction.call(HistoryEvent::Timeline(action));
                        }
                    },
                div { id: "{id}-columns", class: "idle-timeline-header idle-timeline-columns", role: "row", aria_rowindex: "1",
                    div { role: "columnheader", "Graph"
                        div { class: "idle-timeline-resizer", role: "separator", aria_orientation: "vertical", tabindex: "0", aria_label: "Resize graph column",
                            aria_valuemin: "44", aria_valuemax: "600", aria_valuenow: column_width.to_string(),
                            onpointerdown: move |event| { capture(&event.data()); drag.set(Some((event.client_coordinates().x, column_width))); event.prevent_default(); },
                            onpointermove: move |event| { if let Some((start, initial)) = drag() { width.set((initial + event.client_coordinates().x - start).clamp(44.0,600.0)); } },
                            onpointerup: move |_| drag.set(None), onpointercancel: move |_| drag.set(None),
                            onkeydown: move |event| { let amount = if event.key() == Key::ArrowLeft { -10.0 } else if event.key() == Key::ArrowRight { 10.0 } else { return }; event.prevent_default(); event.stop_propagation(); width.set((column_width + amount).clamp(44.0,600.0)); },
                        }
                    }
                    div { role: "columnheader", "Activity" } div { role: "columnheader", "Tags" }
                    div { role: "columnheader", "Content" } div { role: "columnheader", "Date" }
                }
                    div { class: "idle-timeline-canvas", style: "height:{total}px",
                        for (row, paint) in rows { ActivityRow { key: "{row.occurrence}", row, paint, onaction } }
                    }
                    if shown == 0 {
                        p { class: "idle-timeline-empty", if busy { "Preparing recorded activity…" } else { "No recorded activities in this scope." } }
                    }
                }
            }
            div { class: "idle-timeline-footer",
                if editor.new_activity { Button { label: "New activity", onpress: move |()| onaction.call(HistoryEvent::Timeline(Event::Latest(Surface::Editor))) } }
                span { "{count_label}" }
                if newer { Button { label: "Newer", state: if busy { ControlState::Busy } else { ControlState::Ready }, onpress: move |()| onaction.call(HistoryEvent::Timeline(Event::Page { surface: Surface::Editor, newer: true })) } }
                if older { Button { label: "Older", state: if busy { ControlState::Busy } else { ControlState::Ready }, onpress: move |()| onaction.call(HistoryEvent::Timeline(Event::Page { surface: Surface::Editor, newer: false })) } }
                if offset > 0 { Button { label: "Latest", onpress: move |()| onaction.call(HistoryEvent::Timeline(Event::Latest(Surface::Editor))) } }
                if let Some(progress) = &editor.progress { span { role: "status", "{progress.stage} · {progress.processed}" }
                    Button { label: "Cancel", onpress: move |()| onaction.call(HistoryEvent::Timeline(Event::Cancel(Surface::Editor))) }
                } else if busy { span { role: "status", "Loading…" } }
                if let RequestState::Failed(error) = &editor.state { span { role: "alert", "{error.message}" }
                    Button { label: "Retry", onpress: move |()| onaction.call(HistoryEvent::Timeline(Event::Refresh(Surface::Editor))) } }
                for gap in editor.window.as_ref().into_iter().flat_map(|window| &window.gaps) { span { role: "status", "{gap}" } }
                if let Some(notice) = &view.notice { span { role: "status", "{notice}" } }
                if let RequestState::Failed(error) = &view.open { span { role: "alert", "{error.message}" } }
            }
        }
    }
}

fn invalidate(mut tick: Signal<u64>) {
    let next = tick.peek().saturating_add(1);
    tick.set(next);
}

pub(super) fn row_id(id: &str, occurrence: &str) -> String {
    format!("{id}-row-{occurrence}")
}

fn key_action(view: &ViewModel, surface: Surface, key: &Key, visible: usize) -> Option<Event> {
    let state = match surface {
        Surface::Editor => &view.editor,
        Surface::Mini => &view.mini,
    };
    let current = view.selected.as_ref().and_then(|selected| {
        state
            .window
            .as_ref()?
            .rows
            .iter()
            .find(|row| row.occurrence == selected.occurrence)
    });
    if *key == Key::ArrowUp {
        Some(Event::Move { surface, delta: -1 })
    } else if *key == Key::ArrowDown {
        Some(Event::Move { surface, delta: 1 })
    } else if matches!(key, Key::PageUp | Key::PageDown) {
        Some(Event::Move {
            surface,
            delta: i32::try_from(visible)
                .unwrap_or(20)
                .max(1)
                .saturating_mul(if *key == Key::PageUp { -1 } else { 1 }),
        })
    } else if *key == Key::Enter {
        current.map(|row| Event::Select {
            surface,
            occurrence: row.occurrence.clone(),
            open: surface == Surface::Editor,
        })
    } else if matches!(key, Key::ArrowLeft | Key::ArrowRight) {
        current
            .and_then(|row| row.group.as_ref())
            .filter(|group| group.expanded == (*key == Key::ArrowLeft))
            .map(|group| Event::Toggle {
                surface,
                group: group.id.clone(),
            })
    } else {
        None
    }
}

fn filter_kind(filter: &Filter) -> &'static str {
    match filter.kinds.as_slice() {
        [ActivityKind::File] => "file",
        [ActivityKind::Message] => "message",
        [ActivityKind::Tool] => "tool",
        [ActivityKind::Session, ActivityKind::Turn] => "execution",
        [ActivityKind::Commit] => "git",
        _ => "all",
    }
}

fn kinds(value: &str) -> Vec<ActivityKind> {
    match value {
        "file" => vec![ActivityKind::File],
        "message" => vec![ActivityKind::Message],
        "tool" => vec![ActivityKind::Tool],
        "execution" => vec![ActivityKind::Session, ActivityKind::Turn],
        "git" => vec![ActivityKind::Commit],
        _ => Vec::new(),
    }
}

fn capture(data: &PointerData) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast as _;
        if let Some(event) = data.downcast::<web_sys::PointerEvent>()
            && let Some(element) = event
                .target()
                .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
        {
            let _captured = element.set_pointer_capture(event.pointer_id());
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _: &PointerData = data;
}
