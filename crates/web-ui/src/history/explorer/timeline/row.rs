//! One compact row, with disclosure separated from native activation.

use app_core::history::{
    ActivityKind, Event as HistoryEvent,
    timeline::{Event, Row, Surface},
};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{
    Element, EventHandler, MouseData, Props, component, dioxus_core, dioxus_elements, rsx,
};

use super::graph::GraphRow;
use crate::{
    history::graph::browser,
    icons::{Icon, IconName},
};

#[derive(Clone, PartialEq)]
pub(super) struct Paint {
    pub id: String,
    pub top: f64,
    pub position: u64,
    pub width: f64,
    pub pitch: f64,
    pub pan: f64,
    pub selected: bool,
    pub matched: bool,
    pub highlight: Option<u32>,
}

#[component]
pub(super) fn ActivityRow(row: Row, paint: Paint, onaction: EventHandler<HistoryEvent>) -> Element {
    let occurrence = row.occurrence.clone();
    let group = row.group.as_ref().filter(|group| group.header).cloned();
    let label = if group.as_ref().is_some_and(|group| !group.expanded) {
        "task"
    } else {
        match row.title.as_str() {
            "read" | "open" | "close" => "explore",
            "edit" => "change",
            "run" => "tooluse",
            "system" | "note" => "meta",
            title => title,
        }
    }
    .to_owned();
    let (preview, extra_lines) = super::super::summary::preview(&row.preview);
    let date = row.timestamp.map_or_else(
        || "Time not recorded".into(),
        super::super::summary::timestamp,
    );
    let context = [row.author.as_str(), row.session.as_str()]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    let unresolved = row
        .relationships
        .iter()
        .filter(|link| link.unresolved.is_some())
        .count();
    rsx! {
        div { id: paint.id, role: "row", class: "idle-timeline-row idle-timeline-columns",
            aria_rowindex: paint.position.to_string(), aria_selected: paint.selected.to_string(),
            "data-occurrence": occurrence.clone(), "data-operation": row.address.record().map(|address| address.record.operation.clone()),
            "data-record-hash": row.address.record().map(|address| address.record.hash.clone()), "data-source": row.address.record().map(|address| format!("{:?}", address.source)),
            "data-match": paint.matched.to_string(), "data-failed": row.tags.iter().any(|tag| tag == "failed").to_string(), style: "transform:translateY({paint.top}px)",
            onclick: move |event| {
                if browser::row_click(&event.data()) && single_click(&event.data()) {
                    onaction.call(HistoryEvent::Timeline(Event::Select { surface: Surface::Editor, occurrence: occurrence.clone(), open: true }));
                }
            },
            GraphRow { graph: row.graph.clone(), width: paint.width, pitch: paint.pitch, pan: paint.pan, highlight: paint.highlight, selected: paint.selected, height: 34 }
            div { class: "idle-timeline-activity", role: "gridcell", title: format!("{label} · {context}"),
                if let Some(group) = group {
                    button { r#type: "button", class: "idle-timeline-disclosure", tabindex: "-1",
                        aria_label: format!("{} {} activities", if group.expanded { "Collapse" } else { "Expand" }, group.count),
                        aria_expanded: group.expanded.to_string(),
                        onclick: move |event| { event.stop_propagation(); onaction.call(HistoryEvent::Timeline(Event::Toggle { surface: Surface::Editor, group: group.id.clone() })); },
                        Icon { name: if group.expanded { IconName::ChevronDown } else { IconName::ChevronRight } }
                    }
                }
                span { class: "idle-timeline-title", "{label}" }
            }
            div { class: "idle-timeline-tags", role: "gridcell", title: row.tags.join(" · "),
                if let Some(group) = &row.group {
                    if group.header { span { class: "idle-timeline-count", "{group.count} activities" } }
                    if group.live { span { class: "idle-timeline-live", "Live" } }
                }
                for tag in &row.tags { span { class: "idle-timeline-tag", title: tag.clone(), "{tag}" } }
                if unresolved > 0 { span { class: "idle-timeline-warning", title: format!("{unresolved} recorded relationship endpoints are unavailable or ambiguous"),
                    Icon { name: IconName::Alert, label: format!("{unresolved} unavailable relationships") }
                } }
                if row.unavailable.is_some() { Icon { name: IconName::Alert, label: "Recorded content unavailable; opens operation JSON" } }
            }
            div { class: "idle-timeline-content", role: "gridcell", title: format!("{}\n{context}", row.preview),
                div { class: "idle-timeline-description",
                    if row.kind == ActivityKind::Commit { span { class: "idle-timeline-caption", "Git" } }
                    span { class: "idle-timeline-preview", {preview} }
                }
                if extra_lines > 0 { span { class: "idle-timeline-more", "+{extra_lines} lines" } }
            }
            time { class: "idle-timeline-date", role: "gridcell", title: format!("{date} UTC · Recorded time"), "{date}" }
        }
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
