//! Exact source/related addresses and supplied limitations.

use app_core::projections::{
    Event as ProjectionEvent, ProjectionGap, ProjectionKind, ProjectionReference, ProjectionRow,
};
use dioxus::prelude::*;

use crate::controls::Button;

#[component]
pub(super) fn RecordLinks(
    kind: ProjectionKind,
    row: ProjectionRow,
    onaction: EventHandler<ProjectionEvent>,
) -> Element {
    rsx! {
        details { class: "idle-projection-records",
            summary { "Records ({row.sources.len()} sources, {row.related.len()} related)" }
            ul {
                for (index, reference) in row.sources.iter().enumerate() {
                    RecordLink { key: "source-{index}", label: format!("Inspect source {}", index.saturating_add(1)),
                        selection: super::selection(kind, &row.key), reference: reference.clone(), onaction,
                    }
                }
                for (index, reference) in row.related.iter().enumerate() {
                    RecordLink { key: "related-{index}", label: format!("Inspect related {}", index.saturating_add(1)),
                        selection: super::selection(kind, &row.key), reference: reference.clone(), onaction,
                    }
                }
            }
        }
    }
}

#[component]
fn RecordLink(
    label: String,
    selection: app_core::projections::ProjectionSelection,
    reference: ProjectionReference,
    onaction: EventHandler<ProjectionEvent>,
) -> Element {
    let address = reference.clone();
    rsx! {
        li {
            Button { label, onpress: move |()| onaction.call(ProjectionEvent::Inspect { selection: selection.clone(), reference: reference.clone() }) }
            RecordAddress { reference: address }
        }
    }
}

#[component]
fn RecordAddress(reference: ProjectionReference) -> Element {
    rsx! {
        dl { class: "idle-projection-facts idle-projection-address",
            if let Some(observation) = reference.observation { dt { "Observation" } dd { "{observation}" } }
            if let Some(item) = reference.item { dt { "Item" } dd { "{item}" } }
            if let Some(hash) = reference.record_hash { dt { "Record digest" } dd { "{hash}" } }
        }
    }
}

#[component]
pub(super) fn Gaps(gaps: Vec<ProjectionGap>) -> Element {
    rsx! {
        if !gaps.is_empty() {
            section { class: "idle-projection-gaps", aria_label: "Result limitations",
                h3 { "Result limitations" }
                ul {
                    for gap in gaps {
                        li {
                            p { "{gap.message}" }
                            if let Some(reference) = gap.reference { RecordAddress { reference } }
                        }
                    }
                }
            }
        }
    }
}
