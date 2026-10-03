//! Controlled filters over literal provider fields.

use app_core::projections::{
    Event as ProjectionEvent, ProjectionFilter, ProjectionKind, ProjectionView,
};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{Element, EventHandler, Props, component, dioxus_core, dioxus_elements, rsx};

use crate::controls::{
    Button, ControlState, FieldMessage, Select, SelectOption, TextField, TextFieldKind,
};

/// Literal text, exact status and conjunctive label filters, owned by app-core.
/// Choices reflect shown rows plus active filters; clearing restores all loaded
/// choices. Labels/statuses are opaque, including commas, whitespace and empty text.
#[component]
pub fn ProjectionFilters(
    id: String,
    view: ProjectionView,
    onaction: EventHandler<ProjectionEvent>,
) -> Element {
    let kind = view.kind;
    let mut statuses = vec![None];
    if view.filter.status.is_some() {
        statuses.push(view.filter.status.clone());
    }
    let mut labels = view.filter.labels.clone();
    for row in &view.rows {
        if row.status.is_some() && !statuses.contains(&row.status) {
            statuses.push(row.status.clone());
        }
        for label in &row.labels {
            if !labels.contains(label) {
                labels.push(label.clone());
            }
        }
    }
    let options = statuses
        .iter()
        .map(|status| SelectOption {
            value: status_value(status.as_deref()),
            label: status
                .as_deref()
                .map_or("All statuses", |value| super::status_label(Some(value)))
                .into(),
            disabled: false,
        })
        .collect();
    let current_status = status_value(view.filter.status.as_deref());
    let text_filter = view.filter.clone();
    let status_filter = view.filter.clone();
    let clear_state = if view.filter == ProjectionFilter::default() {
        ControlState::Disabled
    } else {
        ControlState::Ready
    };
    rsx! {
        div { class: "idle-projection-filters", role: "group", aria_label: "Projection filters",
            div { class: "idle-projection-filter-fields",
                TextField {
                    id: "{id}-text", label: "Filter text", kind: TextFieldKind::Search,
                    value: view.filter.text.clone(),
                    message: FieldMessage::Hint("Case-sensitive text in title or summary.".into()),
                    oninput: move |text| onaction.call(ProjectionEvent::SetFilter {
                        kind, filter: ProjectionFilter { text, ..text_filter.clone() },
                    }),
                }
                Select {
                    id: "{id}-status", label: "Status in shown rows", value: current_status, options,
                    onchange: move |value: String| {
                        if let Some(status) = statuses.iter().find(|status| status_value(status.as_deref()) == value) {
                            onaction.call(ProjectionEvent::SetFilter { kind, filter: ProjectionFilter { status: status.clone(), ..status_filter.clone() } });
                        }
                    },
                }
            }
            if !labels.is_empty() {
                div { class: "idle-projection-label-filters", role: "group", aria_label: "Labels in shown rows and active filters",
                    span { class: "idle-label", "Labels (match all)" }
                    for label in labels {
                        LabelFilter { key: "{label}", label, kind, filter: view.filter.clone(), onaction }
                    }
                }
            }
            Button { label: "Clear filters", state: clear_state, onpress: move |()| onaction.call(ProjectionEvent::SetFilter { kind, filter: ProjectionFilter::default() }) }
        }
    }
}

fn status_value(status: Option<&str>) -> String {
    status.map_or_else(|| "all".into(), |status| format!("status:{status}"))
}

#[component]
fn LabelFilter(
    label: String,
    kind: ProjectionKind,
    filter: ProjectionFilter,
    onaction: EventHandler<ProjectionEvent>,
) -> Element {
    let active = filter.labels.contains(&label);
    let display = if label.is_empty() {
        "Empty label"
    } else {
        &label
    };
    rsx! {
        button {
            r#type: "button", class: "idle-button idle-projection-label-filter", aria_pressed: active.to_string(),
            onclick: move |_| {
                let mut filter = filter.clone();
                if active { filter.labels.retain(|value| *value != label); } else { filter.labels.push(label.clone()); }
                onaction.call(ProjectionEvent::SetFilter { kind, filter });
            },
            "{display}"
        }
    }
}
