use std::collections::{BTreeMap, BTreeSet};

use app_core::history::{
    Event as HistoryEvent, OpenTarget, OperationDetailsState, RecordRef, ViewModel,
};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{
    Element, EventHandler, Props, ReadableExt, WritableExt, component, dioxus_core,
    dioxus_elements, rsx, use_signal,
};

use super::ActivitySource;
use crate::history::details::actions::OpenButton;
use crate::history::details::records::OperationPanel;
use crate::host::HostCapabilities;

#[component]
pub(super) fn Sources(
    id: String,
    sources: Vec<ActivitySource>,
    view: ViewModel,
    capabilities: HostCapabilities,
    onaction: EventHandler<HistoryEvent>,
) -> Element {
    let mut expanded = use_signal(BTreeSet::<RecordRef>::new);
    let mut grouped: BTreeMap<RecordRef, (BTreeSet<String>, BTreeSet<String>)> = BTreeMap::new();
    for source in sources {
        let (items, originals) = grouped.entry(source.record).or_default();
        if let Some(item) = source.item {
            let _inserted = items.insert(item);
        }
        if let Some(original) = source.original {
            let _inserted = originals.insert(original);
        }
    }
    rsx! {
        div { class: "idle-activity-sources",
            for (index, (record, (items, originals))) in grouped.iter().enumerate() {
                details { key: "{record.operation}:{record.hash}", class: "idle-activity-source",
                    open: expanded.read().contains(record),
                    "data-operation": record.operation.clone(), "data-record-hash": record.hash.clone(),
                    summary {
                        id: "{id}-{index}-summary",
                        onclick: {
                            let record = record.clone();
                            move |event| {
                                event.prevent_default();
                                let mut expanded = expanded.write();
                                if !expanded.remove(&record) {
                                    let _inserted = expanded.insert(record.clone());
                                }
                            }
                        },
                        "Source record · {record.operation}"
                    }
                    if expanded.read().contains(record) {
                        p { class: "idle-history-identity", "Record digest: {record.hash}" }
                        for item in items { p { class: "idle-history-identity", "Item: {item}" } }
                        if items.len() > 1 || originals.len() > 1 {
                            p { class: "idle-history-warning", "Different item or Original links were supplied for this source. All links are shown for inspection." }
                        }
                        OpenButton { id: "{id}-{index}-exact", record: record.clone(), target: OpenTarget::Record, capabilities: capabilities.clone(), state: view.open.clone(), onaction }
                        OperationPanel { id: "{id}-{index}-record", operation: record.operation.clone(), state: lookup(&view, &record.operation), original: false, capabilities: capabilities.clone(), open: view.open.clone(), onaction }
                        for original in originals {
                            OperationPanel { key: "{original}", id: "{id}-{index}-{original}-original", operation: original.clone(), state: lookup(&view, original), original: true, capabilities: capabilities.clone(), open: view.open.clone(), onaction }
                        }
                    }
                }
            }
        }
    }
}

fn lookup(view: &ViewModel, operation: &str) -> OperationDetailsState {
    view.operation_details
        .iter()
        .find(|details| details.operation == operation)
        .map_or(OperationDetailsState::Idle, |details| details.state.clone())
}
