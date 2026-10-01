//! Dioxus owns keyed DOM/SVG nodes; app-core owns every semantic action.

use std::cell::RefCell;
use std::fmt::Write as _;
use std::rc::Rc;

use app_core::history::{ActivityKind, Event as HistoryEvent, ItemView, RequestState, ViewModel};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{
    Callback, Element, EventHandler, ModifiersInteraction, MountedData, Props, ReadableExt, Signal,
    WritableExt, component, dioxus_core, dioxus_elements, rsx, use_effect, use_hook, use_memo,
    use_reactive, use_signal,
};

use super::browser;
use super::paths::{self, Path};
use super::state::State;
use super::viewport::Row;
use crate::controls::{Button, ControlState};

/// Virtualized history graph shared by browser and extension hosts.
///
/// `id` must be unique in the document. Load [`super::STYLESHEET`] with the
/// foundation stylesheet. `render_item` receives the complete app-core item for
/// f31 content; row measurements, focus and scrolling remain graph-owned.
/// Hosts dispatch `onaction` into their Crux runtime and supply its next view.
#[component]
pub fn HistoryGraph(
    id: String,
    view: ViewModel,
    onaction: EventHandler<HistoryEvent>,
    #[props(default = "Activity history".into())] label: String,
    #[props(default = 480)] height: u32,
    render_item: Option<Callback<ItemView, Element>>,
) -> Element {
    let state = use_hook(|| Rc::new(RefCell::new(State::default())));
    let tick = use_signal(|| 0_u64);
    let _revision = *tick.read();
    let mut mounted = use_signal(|| None::<Rc<MountedData>>);
    let now = browser::now();
    let (rows, paths, top, total, graph_width, active, warning, unresolved) = {
        let mut state = state.borrow_mut();
        state.reconcile(&view, now);
        let rows: Vec<_> = state
            .viewport
            .mounted_indices()
            .into_iter()
            .filter_map(|index| {
                let row = state.viewport.rows.get(index)?;
                let item = state
                    .snapshot
                    .items
                    .iter()
                    .find(|item| item.key == row.key)?;
                Some(RenderRow {
                    row: row.clone(),
                    item: item.clone(),
                    index,
                    total: state.viewport.rows.len(),
                    lane: *state.layout.lanes.get(&row.key).unwrap_or(&0),
                    focused: state.viewport.focused.as_ref() == Some(&row.key),
                    selected: view.selected.item.as_ref() == Some(&row.key),
                    born: state.node_births.get(&row.key).copied(),
                })
            })
            .collect();
        let paths = paths::visible(
            &state.snapshot.connections,
            &state.layout,
            &state.viewport,
            &state.births,
        );
        (
            rows,
            paths,
            state.viewport.top,
            state.viewport.total,
            paths::width(&state.layout),
            state.viewport.focused.as_ref().map(|key| row_id(&id, key)),
            state.layout.warning.clone(),
            state
                .snapshot
                .connections
                .iter()
                .filter(|connection| connection.unresolved())
                .count(),
        )
    };
    let _scroll_effect = use_effect(use_reactive((&top,), move |(top,)| {
        if let Some(mounted) = mounted.read().as_ref() {
            browser::scroll(mounted, top);
        }
    }));
    let scroll_state = Rc::clone(&state);
    let resize_state = Rc::clone(&state);
    let key_state = Rc::clone(&state);
    let select_state = Rc::clone(&state);
    let measure_state = Rc::clone(&state);
    let select = move |key: String| {
        let action = select_state.borrow_mut().click(key);
        if let Some(mounted) = mounted.read().as_ref() {
            browser::focus(mounted);
        }
        invalidate(tick);
        onaction.call(action);
    };
    let measure = move |(key, height): (String, f64)| {
        if measure_state.borrow_mut().viewport.measure(&key, height) {
            invalidate(tick);
        }
    };
    let busy = view.paging.state == RequestState::Loading;
    let can_page = !view.paging.exhausted;
    rsx! {
        section { class: "idle-history", aria_label: label.clone(),
            div { class: "idle-history-legend", aria_label: "Connection types",
                span { "Causal parent" }
                span { "· · Logical cause" }
                span { "— — Link" }
                if unresolved > 0 { span { "{unresolved} connections have endpoints outside this view" } }
            }
            if let Some(warning) = warning { p { class: "idle-history-notice", "{warning}" } }
            div {
                id: id.clone(), class: "idle-history-viewport", role: "tree", tabindex: "0",
                aria_label: label, aria_activedescendant: active, aria_busy: busy.to_string(),
                style: "height:{height}px",
                onmounted: move |event| mounted.set(Some(event.data())),
                onscroll: move |event| { scroll_state.borrow_mut().viewport.scroll(event.scroll_top()); invalidate(tick); },
                onresize: move |event| {
                    event.stop_propagation();
                    if let Ok(size) = event.get_content_box_size()
                        && resize_state.borrow_mut().viewport.resize(size.width, size.height) { invalidate(tick); }
                },
                onkeydown: move |event| {
                    if event.is_composing() || !event.modifiers().is_empty() || !browser::tree_key(&event.data()) { return; }
                    let (handled, action) = key_state.borrow_mut().key(&event.key());
                    if handled {
                        event.prevent_default(); event.stop_propagation(); invalidate(tick);
                        if let Some(action) = action { onaction.call(action); }
                    }
                },
                div { class: "idle-history-canvas", style: "height:{total}px;min-width:calc({graph_width}px + 160px)",
                    svg {
                        class: "idle-history-edges", width: "{graph_width}", height: "{total}",
                        view_box: "0 0 {graph_width} {total}", "aria-hidden": "true", "focusable": "false",
                        for path in paths {
                            ConnectionPath { key: "{path.connection.key}", path }
                        }
                    }
                    for row in rows {
                        HistoryItem { key: "{row.item.key}", id: row_id(&id, &row.item.key), row, graph_width, onselect: select.clone(), onmeasure: measure.clone(), render_item }
                    }
                }
                if view.items.is_empty() && !busy { p { class: "idle-history-empty", "No loaded history items." } }
            }
            div { class: "idle-history-footer",
                if let RequestState::Failed(error) = &view.paging.state { p { role: "alert", "{error.message}" } }
                if let RequestState::Failed(error) = &view.reconciliation { p { role: "alert", "{error.message}" } }
                if view.reconciliation == RequestState::Loading { span { role: "status", "Refreshing history…" } }
                if can_page {
                    Button { label: if busy { "Loading history…" } else { "Load more history" }, state: if busy { ControlState::Busy } else { ControlState::Ready }, onpress: move |()| onaction.call(HistoryEvent::LoadMore) }
                } else { span { "End of loaded scan" } }
            }
        }
    }
}

fn invalidate(mut tick: Signal<u64>) {
    let next = tick.peek().saturating_add(1);
    tick.set(next);
}

#[derive(Debug, Clone, PartialEq)]
struct RenderRow {
    row: Row,
    item: ItemView,
    index: usize,
    total: usize,
    lane: usize,
    focused: bool,
    selected: bool,
    born: Option<f64>,
}

#[component]
fn HistoryItem(
    id: String,
    row: RenderRow,
    graph_width: f64,
    onselect: EventHandler<String>,
    onmeasure: EventHandler<(String, f64)>,
    render_item: Option<Callback<ItemView, Element>>,
) -> Element {
    let key = row.item.key.clone();
    let measure_key = key.clone();
    let lane_x = paths::x(row.lane);
    let position = row.index.saturating_add(1);
    let count = row.item.observations.len();
    let caption = row
        .item
        .observations
        .first()
        .map_or("Item", |record| kind_label(record.kind));
    let motion = use_memo(use_reactive((&row.born,), |(born,)| {
        motion_style(born, 0.0, 160.0)
    }));
    let selected = row.selected.to_string();
    rsx! {
        div {
            id, class: "idle-history-item", role: "treeitem", aria_level: "1",
            aria_selected: selected, aria_expanded: row.item.expanded.to_string(),
            aria_setsize: "{row.total}", aria_posinset: "{position}",
            "data-item-key": key.clone(), "data-focused": row.focused.to_string(),
            style: "transform:translateY({row.row.top}px);padding-left:{graph_width}px;{motion}",
            onclick: move |event| { if browser::row_click(&event.data()) { onselect.call(key.clone()); } },
            onresize: move |event| {
                    event.stop_propagation();
                if let Ok(size) = event.get_border_box_size() { onmeasure.call((measure_key.clone(), size.height)); }
            },
            span { class: "idle-history-dot", style: "left:{lane_x}px", aria_hidden: "true" }
            div { class: "idle-history-item-content",
                if let Some(render_item) = render_item { {render_item.call(row.item.clone())} }
                else {
                    span { class: "idle-history-kind", "{caption}" }
                    span { class: "idle-history-key", title: row.item.key.clone(), "{row.item.key}" }
                    span { class: "idle-history-count", "{count} records" }
                }
                for observation in &row.item.observations {
                    span { key: "{observation.record.operation}:{observation.record.hash}", class: "idle-history-record", "data-operation": observation.record.operation.clone(), "data-record-hash": observation.record.hash.clone(),
                        "Observation {observation.record.operation}. Record {observation.record.hash}."
                        if let Some(problem) = &observation.problem { " {problem}" }
                    }
                }
                if row.item.observations.iter().any(|record| record.problem.is_some()) { span { class: "idle-history-problem", "Record needs attention" } }
            }
        }
    }
}

#[component]
fn ConnectionPath(path: Path) -> Element {
    let connection = &path.connection;
    let motion = use_memo(use_reactive(
        (&path.born, &path.delay, &path.duration),
        |(born, delay, duration)| motion_style(born, delay, duration),
    ));
    let class = connection.kind.class();
    rsx! {
        path {
            class: "idle-history-path", d: path.d, path_length: (class == "causal").then_some("1"),
            "data-kind": class, "data-unresolved": connection.unresolved().to_string(),
            "data-connection-key": connection.key.clone(),
            "data-operation": connection.record.operation.clone(), "data-record-hash": connection.record.hash.clone(),
            style: "{motion}",
            title { "{connection.caption()}" }
        }
    }
}

fn motion_style(born: Option<f64>, delay: f64, duration: f64) -> String {
    let Some(born) = born else {
        return String::new();
    };
    let elapsed = (browser::now() - born).max(0.0);
    if elapsed >= delay + duration {
        return String::new();
    }
    let delay = delay - elapsed;
    format!(
        "--history-enter:idle-history-arrive;--history-edge-enter:idle-history-grow;--history-delay:{delay}ms;--history-duration:{duration}ms"
    )
}

pub(super) fn row_id(id: &str, key: &str) -> String {
    let mut encoded = String::new();
    for byte in key.as_bytes() {
        let _written = write!(encoded, "{byte:02x}");
    }
    format!("{id}-item-{encoded}")
}

const fn kind_label(kind: ActivityKind) -> &'static str {
    match kind {
        ActivityKind::Session => "Session",
        ActivityKind::Turn => "Turn",
        ActivityKind::Message => "Message",
        ActivityKind::Tool => "Tool",
        ActivityKind::File => "File",
        ActivityKind::Commit => "Commit",
        ActivityKind::Note => "Note",
        ActivityKind::Author => "Author",
        ActivityKind::Link => "Link",
        ActivityKind::Original => "Original",
        ActivityKind::Initialization => "Chain",
        ActivityKind::Unknown => "Unknown record",
    }
}
