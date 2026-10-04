//! List, table, card and status-column presentations of the same supplied rows.

use app_core::projections::{
    Event as ProjectionEvent, ProjectionKind, ProjectionRow, ProjectionSelection, ProjectionView,
};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{Element, EventHandler, Props, component, dioxus_core, dioxus_elements, rsx};

use super::records::RecordLinks;

/// A compact list in supplied row order, with native selection buttons.
/// Compose with [`super::ProjectionSummary`] and [`super::ProjectionFeedback`]
/// when using it outside [`super::ProjectionPanel`].
#[component]
pub fn ProjectionList(
    view: ProjectionView,
    selected: Option<ProjectionSelection>,
    onaction: EventHandler<ProjectionEvent>,
    onopen: Option<EventHandler<String>>,
) -> Element {
    rsx! {
        ul { class: "idle-projection-list", aria_label: super::title(view.kind),
            for row in view.rows {
                li { key: "{row.key}",
                    ProjectionCard { kind: view.kind, row, selected: selected.clone(), onaction, onopen }
                }
            }
        }
    }
}

/// Responsive cards preserving provider order and full source/related references.
#[component]
pub fn ProjectionCards(
    view: ProjectionView,
    selected: Option<ProjectionSelection>,
    onaction: EventHandler<ProjectionEvent>,
    onopen: Option<EventHandler<String>>,
) -> Element {
    rsx! {
        ul { class: "idle-projection-cards", aria_label: super::title(view.kind),
            for row in view.rows {
                li { key: "{row.key}",
                    ProjectionCard { kind: view.kind, row, selected: selected.clone(), onaction, onopen }
                }
            }
        }
    }
}

/// One selectable supplied row; selecting never changes its status or content.
#[component]
pub fn ProjectionCard(
    kind: ProjectionKind,
    row: ProjectionRow,
    selected: Option<ProjectionSelection>,
    onaction: EventHandler<ProjectionEvent>,
    onopen: Option<EventHandler<String>>,
) -> Element {
    let active = selected.as_ref() == Some(&super::selection(kind, &row.key));
    rsx! {
        article { class: "idle-projection-card", "data-row-key": row.key.clone(), "data-selected": active.to_string(),
            h3 { RowTitle { kind, row: row.clone(), selected, onaction } }
            if let Some(summary) = &row.summary { p { class: "idle-projection-row-summary", "{summary}" } }
            RowMetadata { row: row.clone() }
            RecordLinks { kind, row: row.clone(), onaction, onopen }
        }
    }
}

#[component]
fn RowTitle(
    kind: ProjectionKind,
    row: ProjectionRow,
    selected: Option<ProjectionSelection>,
    onaction: EventHandler<ProjectionEvent>,
) -> Element {
    let selection = super::selection(kind, &row.key);
    let active = selected.as_ref() == Some(&selection);
    rsx! {
        button { r#type: "button", class: "idle-button idle-projection-title", "data-variant": "quiet",
            aria_pressed: active.to_string(),
            onclick: move |_| onaction.call(ProjectionEvent::Select(Some(selection.clone()))),
            "{row.title}"
        }
    }
}

#[component]
fn RowMetadata(row: ProjectionRow) -> Element {
    rsx! {
        div { class: "idle-projection-row-metadata",
            span { class: "idle-projection-status", "{super::status_label(row.status.as_deref())}" }
            Labels { labels: row.labels }
        }
    }
}

#[component]
fn Labels(labels: Vec<String>) -> Element {
    rsx! {
        ul { class: "idle-projection-row-labels", aria_label: "Labels",
            for (index, label) in labels.iter().enumerate() {
                li { key: "{index}", if label.is_empty() { "Empty label" } else { "{label}" } }
            }
        }
    }
}

/// A semantic table with a keyboard-scrollable region for narrow containers.
/// Columns never discard summaries or exact record links at compact density.
#[component]
pub fn ProjectionTable(
    view: ProjectionView,
    selected: Option<ProjectionSelection>,
    onaction: EventHandler<ProjectionEvent>,
    onopen: Option<EventHandler<String>>,
) -> Element {
    rsx! {
        div { class: "idle-projection-table-scroll", tabindex: "0", role: "region", aria_label: format!("{} table", super::title(view.kind)),
            table { class: "idle-projection-table",
                caption { class: "idle-visually-hidden", "{super::title(view.kind)}" }
                thead { tr {
                    th { scope: "col", "Item" }
                    th { scope: "col", "Status" }
                    th { scope: "col", "Labels" }
                    th { scope: "col", "Records" }
                } }
                tbody {
                    for row in view.rows {
                        tr { key: "{row.key}", "data-row-key": row.key.clone(),
                            "data-selected": (selected.as_ref() == Some(&super::selection(view.kind, &row.key))).to_string(),
                            th { scope: "row",
                                RowTitle { kind: view.kind, row: row.clone(), selected: selected.clone(), onaction }
                                if let Some(summary) = &row.summary { p { class: "idle-projection-row-summary", "{summary}" } }
                            }
                            td { "{super::status_label(row.status.as_deref())}" }
                            td { Labels { labels: row.labels.clone() } }
                            td { RecordLinks { kind: view.kind, row: row.clone(), onaction, onopen } }
                        }
                    }
                }
            }
        }
    }
}

/// Columns group exact statuses in first-seen order, retaining row order within
/// each column. Missing and empty statuses remain distinct. Columns wrap/stack
/// with the container; no task transitions or totals are invented by this view.
#[component]
pub fn ProjectionBoard(
    view: ProjectionView,
    selected: Option<ProjectionSelection>,
    onaction: EventHandler<ProjectionEvent>,
    onopen: Option<EventHandler<String>>,
) -> Element {
    #[cfg(target_arch = "wasm32")]
    let preserve = super::board_state::use_board_state(&view);
    let mut statuses = Vec::new();
    for row in &view.rows {
        if !statuses.contains(&row.status) {
            statuses.push(row.status.clone());
        }
    }
    rsx! {
        div { class: "idle-projection-board", "data-projection-kind": format!("{:?}", view.kind),
            onmounted: move |event| {
                #[cfg(target_arch = "wasm32")]
                preserve.call(event);
                #[cfg(not(target_arch = "wasm32"))]
                let _event = event;
            },
            for status in statuses {
                section { key: "{status:?}", class: "idle-projection-column", aria_label: super::status_label(status.as_deref()),
                    h3 { class: "idle-projection-column-heading", "{super::status_label(status.as_deref())}" }
                    ul {
                        for row in view.rows.iter().filter(|row| row.status == status) {
                            li { key: "{row.key}",
                                ProjectionCard { kind: view.kind, row: row.clone(), selected: selected.clone(), onaction, onopen }
                            }
                        }
                    }
                }
            }
        }
    }
}
