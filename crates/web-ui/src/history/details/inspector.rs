//! Graph composition and selection-bound record inspector.

use app_core::history::{
    ActivityKind, Event as HistoryEvent, ItemView, OperationDetailsState, RequestState, ViewModel,
};
use dioxus::prelude::*;

use super::observation::ObservationCard;
use super::records::OperationPanel;
use super::row::{HistoryRow, ItemPaging};
use crate::history::graph::HistoryGraph;
use crate::host::HostCapabilities;

/// History graph with rich rows and an inspector that survives virtualization.
///
/// Bundle the theme, graph and details stylesheets. Supply only installed host
/// capabilities and route `onaction` to app-core. Browser and extension hosts use
/// the same component and can also compose [`HistoryRow`] and [`HistoryDetails`]
/// independently.
#[component]
pub fn HistoryTimeline(
    id: String,
    view: ViewModel,
    capabilities: HostCapabilities,
    onaction: EventHandler<HistoryEvent>,
    #[props(default = 480)] height: u32,
) -> Element {
    let row_prefix = id.clone();
    let render_item = Callback::new(move |item: ItemView| {
        rsx! { HistoryRow { id: "{row_prefix}-content-{item.key}", item, onaction } }
    });
    rsx! {
        div { class: "idle-history-timeline",
            HistoryGraph { id: id.clone(), view: view.clone(), onaction, render_item, height }
            HistoryDetails { id: "{id}-details", view, capabilities, onaction }
        }
    }
}

/// Exact record/Original inspector driven entirely by app-core selection and
/// lookup state. A selected item remains inspectable outside the filtered graph.
/// Cached field bytes are shown in full, never reconstructed from display text.
#[component]
pub fn HistoryDetails(
    id: String,
    view: ViewModel,
    capabilities: HostCapabilities,
    onaction: EventHandler<HistoryEvent>,
) -> Element {
    let item = view
        .selected_item
        .as_ref()
        .filter(|item| Some(&item.key) == view.selected.item.as_ref())
        .or_else(|| {
            view.items
                .iter()
                .find(|item| Some(&item.key) == view.selected.item.as_ref())
        });
    let observation = item.and_then(|item| {
        item.observations.iter().find(|observation| {
            Some(&observation.record.operation) == view.selected.observation.as_ref()
        })
    });
    let original = observation.and_then(|observation| observation.original.clone());
    rsx! {
        section { id: id.clone(), class: "idle-history-details", aria_label: "History record details",
            h2 { "History details" }
            if view.selected.item.is_none() && view.selected.observation.is_none() {
                p { "Inspect an item to read its observations and exact recorded content." }
            }
            if let Some(item) = item {
                p { class: "idle-history-identity", "Item: {item.key}" }
                if view.selected.observation.is_none() { p { "Choose an observation to load its exact fields and stored bytes." } }
                for observation in &item.observations {
                    ObservationCard { key: "{observation.record.operation}:{observation.record.hash}", observation: observation.clone(), onaction }
                }
                ItemPaging { item: item.key.clone(), paging: item.paging.clone(), onaction }
            } else if let Some(key) = &view.selected.item {
                p { class: "idle-history-warning", "Selected item {key} has no loaded observations." }
                crate::controls::Button { label: "Load selected item", onpress: {
                    let key = key.clone();
                    move |()| onaction.call(HistoryEvent::LoadItem(key.clone()))
                } }
            }
            if let Some(operation) = &view.selected.observation {
                OperationPanel {
                    id: "{id}-selected", operation: operation.clone(), state: lookup(&view, operation),
                    original: observation.is_some_and(|record| record.kind == ActivityKind::Original),
                    capabilities: capabilities.clone(), open: view.open.clone(), onaction,
                }
                if let Some(original) = original.filter(|original| original != operation) {
                    OperationPanel {
                        id: "{id}-original", state: lookup(&view, &original), operation: original, original: true,
                        capabilities: capabilities.clone(), open: view.open.clone(), onaction,
                    }
                }
            }
            if view.open == RequestState::Loading { p { role: "status", "Opening recorded content…" } }
            if let RequestState::Failed(error) = &view.open { p { role: "alert", "{error.message}" } }
        }
    }
}

fn lookup(view: &ViewModel, operation: &str) -> OperationDetailsState {
    view.operation_details
        .iter()
        .find(|details| details.operation == operation)
        .map_or(OperationDetailsState::Idle, |details| details.state.clone())
}
