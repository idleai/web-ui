//! Controlled row disclosure for message blocks, attempts and file observations.

use app_core::history::{
    BlockState, Event as HistoryEvent, ItemView, Paging, RequestState, Selected,
};
use dioxus::prelude::*;

use super::content::{HistoryBlock, PreviewText};
use super::observation::{ObservationCard, kind_label};
use crate::controls::{Button, ControlState, Disclosure};

/// Rich content for a graph row or a standalone history list.
///
/// Supply a document-unique `id`. The app-core item controls expansion even after
/// virtualization/remounting. Dispatch `onaction` into the owning Crux runtime.
/// Previews always label omitted content; expansion exposes all cached blocks
/// and observations, with explicit paging for records that are not loaded yet.
#[component]
pub fn HistoryRow(id: String, item: ItemView, onaction: EventHandler<HistoryEvent>) -> Element {
    let kind = item
        .observations
        .first()
        .map_or("History item", |record| kind_label(record.kind));
    let count = item.observations.len();
    let key = item.key.clone();
    let selection = Selected {
        item: Some(key.clone()),
        observation: None,
    };
    let mut blocks = item.blocks.clone();
    if blocks.iter().all(|block| block.attempt.is_none()) {
        blocks.sort_by_key(|block| (block.position.is_none(), block.position));
    }
    let hidden = blocks.len().saturating_sub(1);
    let problems = blocks
        .iter()
        .filter(|block| {
            matches!(
                block.state,
                BlockState::Unavailable(_) | BlockState::Conflicted(_)
            )
        })
        .count();
    rsx! {
        div { class: "idle-history-row", "data-item": item.key.clone(),
            div { class: "idle-history-row-heading",
                strong { "{kind}" }
                span { class: "idle-history-caption", "{count} loaded observations" }
                Button { label: "Inspect records", onpress: move |()| onaction.call(HistoryEvent::Select(selection.clone())) }
            }
            Disclosure {
                id, label: if item.expanded { "Collapse content" } else { "Expand content" }, expanded: item.expanded,
                onchange: move |_| onaction.call(HistoryEvent::ToggleDisclosure(key.clone())),
                if item.expanded {
                    p { class: "idle-history-identity", "Item: {item.key}" }
                    for block in &blocks {
                        HistoryBlock { key: "{block.attempt:?}:{block.channel:?}:{block.block}", block: block.clone(), expanded: true }
                    }
                    if item.blocks.is_empty() {
                        for observation in &item.observations {
                            if let Some(preview) = &observation.preview { PreviewText { preview: preview.clone(), expanded: true } }
                        }
                    }
                    for observation in &item.observations {
                        ObservationCard { key: "{observation.record.operation}:{observation.record.hash}", observation: observation.clone(), onaction }
                    }
                    ItemPaging { item: item.key.clone(), paging: item.paging.clone(), onaction }
                }
            }
            if !item.expanded {
                if let Some(block) = blocks.first() { HistoryBlock { block: block.clone(), expanded: false } }
                else if let Some(preview) = item.observations.iter().find_map(|record| record.preview.clone()) {
                    PreviewText { preview, expanded: false }
                } else { p { class: "idle-history-caption", "No content preview loaded. Expand or inspect the records." } }
                if hidden > 0 { p { class: "idle-history-caption", "{hidden} more blocks. Expand this item to read them." } }
                if problems > 0 { p { class: "idle-history-warning", "{problems} unavailable or conflicted blocks" } }
                if item.observations.iter().any(|record| record.problem.is_some()) {
                    p { class: "idle-history-warning", "Recorded observations need attention. Expand for details." }
                }
                if !item.paging.exhausted { p { class: "idle-history-caption", "Item scan incomplete; more observations may exist." } }
            }
        }
    }
}

#[component]
pub(super) fn ItemPaging(
    item: String,
    paging: Paging,
    onaction: EventHandler<HistoryEvent>,
) -> Element {
    let busy = paging.state == RequestState::Loading;
    rsx! {
        div { class: "idle-history-item-paging",
            if let RequestState::Failed(error) = &paging.state { p { role: "alert", "{error.message}" } }
            if busy { p { role: "status", "Loading item observations…" } }
            if paging.exhausted {
                p { class: "idle-history-caption", "Item scan complete. Content availability is shown separately." }
            } else {
                p { class: "idle-history-caption", "Item scan incomplete; more observations may exist." }
                Button {
                    label: if matches!(paging.state, RequestState::Failed(_)) { "Retry item scan" } else { "Load more observations" },
                    state: if busy { ControlState::Busy } else { ControlState::Ready },
                    onpress: move |()| { if !busy { onaction.call(HistoryEvent::LoadItem(item.clone())); } },
                }
            }
        }
    }
}
