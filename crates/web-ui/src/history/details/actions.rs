//! Typed actions gated by explicit presentation-host adapters.

use app_core::history::{Event as HistoryEvent, OpenTarget, RecordRef, RequestState};
use dioxus::prelude::*;

use crate::controls::{Button, ControlState};
use crate::host::HostCapabilities;

#[component]
pub(super) fn OpenButton(
    id: String,
    record: RecordRef,
    target: OpenTarget,
    capabilities: HostCapabilities,
    state: RequestState,
    onaction: EventHandler<HistoryEvent>,
) -> Element {
    let label = match target {
        OpenTarget::Record => "Open stored record",
        OpenTarget::Original => "Open Original",
        OpenTarget::File => "Open recorded revision",
        OpenTarget::Diff => "Open recorded comparison",
    };
    let available = capabilities.supports_history(target);
    let pending = state == RequestState::Loading;
    rsx! {
        div { class: "idle-history-open",
            Button {
                label,
                state: if available { if pending { ControlState::Busy } else { ControlState::Ready } } else { ControlState::Disabled },
                described_by: (!available).then(|| format!("{id}-unavailable")),
                onpress: move |()| {
                    if !pending && capabilities.supports_history(target) {
                        onaction.call(HistoryEvent::Open { record: record.clone(), target });
                    }
                },
            }
            if !available {
                p { id: "{id}-unavailable", class: "idle-history-caption", "{label} is unavailable in this host." }
            }
        }
    }
}

#[component]
pub(super) fn LoadButton(
    operation: String,
    label: String,
    refresh: bool,
    #[props(default)] busy: bool,
    onaction: EventHandler<HistoryEvent>,
) -> Element {
    rsx! {
        Button {
            label, state: if busy { ControlState::Busy } else { ControlState::Ready },
            onpress: move |()| {
                if !busy { onaction.call(HistoryEvent::LoadOperationDetails { operation: operation.clone(), refresh }); }
            },
        }
    }
}
