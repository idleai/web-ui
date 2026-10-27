//! One physical activity, with task and item disclosures in their own columns.

use app_core::history::{
    Event as HistoryEvent,
    timeline::{Event, Row, Surface},
};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{
    Element, EventHandler, ModifiersInteraction, MouseData, Props, component, dioxus_core,
    dioxus_elements, rsx,
};

use super::{graph::GraphRow, icon, input};
use crate::{controls::ControlState, history::graph::browser};

#[derive(Clone, PartialEq)]
pub(super) struct Paint {
    pub id: String,
    pub top: f64,
    pub position: u64,
    pub width: f64,
    pub pitch: f64,
    pub pan: f64,
    pub selected: bool,
    pub focused: bool,
    pub matched: bool,
    pub highlight: Option<u32>,
    pub task: ControlState,
    pub details: ControlState,
}

#[component]
pub(super) fn ActivityRow(
    row: Row,
    paint: Paint,
    onaction: EventHandler<HistoryEvent>,
    onfocusrow: EventHandler<String>,
) -> Element {
    let occurrence = row.occurrence.clone();
    let group = row
        .group
        .as_ref()
        .filter(|group| group.header && group.count > 1)
        .cloned();
    let folded = row.presentation.task_summary
        || row.presentation.label.is_empty() && group.as_ref().is_some_and(|group| !group.expanded);
    let physical_label = if row.presentation.label.is_empty() {
        fallback_label(&row.title)
    } else {
        &row.presentation.label
    };
    let label = if folded { "task" } else { physical_label }.to_owned();
    let content_icon = icon::icon(physical_label);
    let preview = super::super::summary::plain(&row.preview);
    let date = row.timestamp.map_or_else(
        || "Time not recorded".into(),
        super::super::summary::timestamp,
    );
    let context = [row.author.as_str(), row.session.as_str()]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    let details = row.details.clone().filter(|_| !folded);
    let expanded = if folded {
        Some(false)
    } else {
        details.as_ref().map(|details| details.expanded)
    };
    let file = row.presentation.file && !folded;
    let click_id = occurrence.clone();
    let inspect_id = occurrence.clone();
    let detail_id = occurrence.clone();
    let key_row = row.clone();
    rsx! {
        div { id: paint.id, role: "row", class: "idle-timeline-row idle-timeline-columns",
            tabindex: if paint.focused { "0" } else { "-1" }, aria_expanded: expanded.map(|expanded| expanded.to_string()),
            aria_rowindex: paint.position.to_string(), aria_selected: paint.selected.to_string(),
            "data-occurrence": occurrence.clone(), "data-operation": row.address.record().map(|address| address.record.operation.clone()),
            "data-record-hash": row.address.record().map(|address| address.record.hash.clone()), "data-source": row.address.record().map(|address| format!("{:?}", address.source)),
            "data-match": paint.matched.to_string(), "data-file": file.to_string(), "data-child": row.presentation.parent.is_some().to_string(),
            style: "transform:translateY({paint.top}px)",
            onfocusin: move |_| onfocusrow.call(occurrence.clone()),
            onclick: move |event| {
                if browser::row_click(&event.data()) && single_click(&event.data()) {
                    input::focus_row(&event.data());
                    onaction.call(HistoryEvent::Timeline(Event::Click { surface: Surface::Editor, occurrence: click_id.clone() }));
                }
            },
            ondoubleclick: move |event| {
                if !file && browser::row_click(&event.data()) {
                    input::focus_row(&event.data());
                    onaction.call(HistoryEvent::Timeline(Event::Inspect { surface: Surface::Editor, occurrence: inspect_id.clone() }));
                }
            },
            onkeydown: move |event| {
                if event.is_composing() || !event.modifiers().is_empty() || !input::row_key(&event.data()) { return; }
                if let Some(action) = input::action(&key_row, &event.key()) {
                    event.prevent_default(); event.stop_propagation();
                    onaction.call(HistoryEvent::Timeline(action));
                }
            },
            GraphRow { graph: row.graph.clone(), width: paint.width, pitch: paint.pitch, pan: paint.pan, highlight: paint.highlight, selected: paint.selected, height: 34,
                bundle: folded, node: row.presentation.parent.is_none() }
            div { class: "idle-timeline-activity", role: "gridcell", title: format!("{label} · {context}"),
                span { class: "idle-timeline-title", "{label}" }
                if let Some(details) = details {
                    button { r#type: "button", class: "idle-timeline-detail-toggle",
                        aria_label: if details.expanded { "Collapse item details" } else { "Expand item details" },
                        aria_expanded: details.expanded.to_string(), aria_busy: (paint.details == ControlState::Busy).to_string(),
                        onclick: move |event| {
                            event.stop_propagation(); input::focus_row(&event.data());
                            onaction.call(HistoryEvent::Timeline(Event::Select { surface: Surface::Editor, occurrence: detail_id.clone(), open: false }));
                            onaction.call(HistoryEvent::Timeline(Event::Details { surface: Surface::Editor, occurrence: detail_id.clone() }));
                        },
                        if details.expanded { "▾" } else { "▸" }
                    }
                }
            }
            div { class: "idle-timeline-tags", role: "gridcell", title: row.tags.join(" · "),
                if let Some(group) = group {
                    button { r#type: "button", class: "idle-timeline-disclosure",
                        aria_label: format!("{} {} activities", if group.expanded { "Collapse" } else { "Expand" }, group.count),
                        title: format!("{} {} activities", if group.expanded { "Collapse" } else { "Expand" }, group.count),
                        aria_expanded: group.expanded.to_string(), aria_busy: (paint.task == ControlState::Busy).to_string(),
                        onclick: move |event| { event.stop_propagation(); input::focus_row(&event.data()); onaction.call(HistoryEvent::Timeline(Event::Toggle { surface: Surface::Editor, group: group.id.clone() })); },
                        if group.expanded { "▾ " } else { "▸ " } "{group.count} activities"
                    }
                }
                for value in &row.tags { {tag(value)} }
            }
            div { class: "idle-timeline-content", role: "gridcell", title: row.preview.clone(),
                if file {
                    {file_content(row.presentation.file_path.as_deref().unwrap_or(&row.preview))}
                } else {
                    {content_icon}
                    if !row.presentation.title.is_empty() { span { class: "idle-timeline-content-title", "{row.presentation.title}" } }
                    span { class: "idle-timeline-preview", "{preview}" }
                }
            }
            time { class: "idle-timeline-date", role: "gridcell", title: format!("{date} UTC · Recorded time"), "{date}" }
            if !row.presentation.session_label.is_empty() {
                div { class: "idle-timeline-session-label", "aria-hidden": "true", "{row.presentation.session_label}" }
            }
        }
    }
}

fn file_content(path: &str) -> Element {
    let (directory, name) = path.rsplit_once(['/', '\\']).unwrap_or(("", path));
    rsx! {
        span { class: "idle-timeline-file-icon", "aria-hidden": "true" }
        span { class: "idle-timeline-file-name", "{name}" }
        if !directory.is_empty() { span { class: "idle-timeline-file-directory", "{directory}" } }
    }
}

fn tag(value: &str) -> Element {
    let text = match value {
        "failed" => "✕",
        "subagent" => "↳ subagent",
        "return" => "↩ return",
        "fork" => "⇉ fork",
        other => other,
    };
    rsx! { span { class: "idle-timeline-tag", "data-tag": value, title: value, aria_label: value, "{text}" } }
}

fn fallback_label(title: &str) -> &str {
    match title {
        "read" | "open" | "close" => "explore",
        "edit" => "change",
        "run" | "result" => "tooluse",
        "message" | "summary" => "agent",
        "system" | "note" | "session" => "meta",
        title => title,
    }
}

pub(super) fn single_click(data: &MouseData) -> bool {
    #[cfg(target_arch = "wasm32")]
    if let Some(event) = data.downcast::<web_sys::MouseEvent>() {
        return event.detail() <= 1;
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _: &MouseData = data;
    true
}
