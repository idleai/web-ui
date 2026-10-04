//! Exact source/related addresses and supplied limitations.

use std::collections::BTreeMap;

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
    onopen: Option<EventHandler<String>>,
) -> Element {
    rsx! {
        if let Some(url) = &row.url { crate::repository::SourceLink { label: "Open source page", url: url.clone(), onopen } }
        details { class: "idle-projection-records",
            summary { "Records ({row.sources.len()} sources, {row.related.len()} related)" }
            ul {
                for (index, (key, reference)) in keyed_references("source", &row.sources).into_iter().enumerate() {
                    RecordLink { key: "{key}", identity: key, label: format!("Inspect source {}", index.saturating_add(1)),
                        selection: super::selection(kind, &row.key), reference, onaction,
                    }
                }
                for (index, (key, reference)) in keyed_references("related", &row.related).into_iter().enumerate() {
                    RecordLink { key: "{key}", identity: key, label: format!("Inspect related {}", index.saturating_add(1)),
                        selection: super::selection(kind, &row.key), reference, onaction,
                    }
                }
            }
        }
    }
}

// Occurrences distinguish duplicate addresses without tying retained links to
// their position among unrelated records. Debug encoding preserves Option fields.
fn keyed_references(
    group: &str,
    references: &[ProjectionReference],
) -> Vec<(String, ProjectionReference)> {
    let mut occurrences = BTreeMap::new();
    references
        .iter()
        .map(|reference| {
            let address = format!("{reference:?}");
            let occurrence = occurrences.entry(address.clone()).or_insert(0_usize);
            let key = format!("{group}:{address}:{occurrence}");
            *occurrence = occurrence.saturating_add(1);
            (key, reference.clone())
        })
        .collect()
}

#[component]
fn RecordLink(
    identity: String,
    label: String,
    selection: app_core::projections::ProjectionSelection,
    reference: ProjectionReference,
    onaction: EventHandler<ProjectionEvent>,
) -> Element {
    let address = reference.clone();
    rsx! {
        li { "data-projection-reference": identity,
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
