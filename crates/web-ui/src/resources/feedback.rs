use app_core::resources::{
    ControllerView, Event, ResourceActionStage, ResourceCapabilities, ResourceLoadState,
    ResourceMutationView, ResourceRecoveryAction, ResourceViewModel,
};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{Element, EventHandler, Props, component, dioxus_core, dioxus_elements, rsx};

use crate::controls::{Button, ControlState};

#[component]
pub(super) fn LoadFeedback(view: ResourceViewModel) -> Element {
    rsx! {
        match view.load {
            ResourceLoadState::Idle => rsx! { p { "Select a repository to load resources." } },
            ResourceLoadState::Loading => rsx! { p { role: "status", "Loading resources…" } },
            ResourceLoadState::Suspended => rsx! { p { role: "status", "Resource connection interrupted. Refresh to reconnect." } },
            ResourceLoadState::Failed(error) => rsx! { p { role: "alert", "{error.message}" } },
            ResourceLoadState::Ready => rsx! {},
        }
        if let Some(error) = view.action_error { p { role: "alert", "{error.message}" } }
        if view.capabilities == ResourceCapabilities::default() {
            p { "Runtime actions are unavailable until a runtime is connected." }
        }
    }
}

#[component]
pub(super) fn ControllerStatus(view: ControllerView) -> Element {
    rsx! {
        section { class: "idle-resource-card", aria_label: "Controller status",
            h3 { "Controller" }
            p { "Assignment: {view.assignment:?} · Last epoch {view.ownership.last_epoch}" }
            p { "Runtime: {view.phase:?} · {super::cards::availability(view.availability)}" }
            if let Some(lease) = view.ownership.lease {
                p { "Host: {lease.target.host_id} · Session: {lease.target.session_id}" }
                p { "Lease expires at {lease.expires_at_ms} ms UTC" }
            }
        }
    }
}

#[component]
pub(super) fn MutationStatus(
    mutation: ResourceMutationView,
    onaction: EventHandler<Event>,
) -> Element {
    let id = mutation.request.request_id.clone();
    let state = if mutation.in_flight {
        ControlState::Busy
    } else {
        ControlState::Ready
    };
    rsx! {
        section { class: "idle-resource-card", aria_label: "Resource action status", aria_live: "polite",
            h3 { "Resource action" }
            p { "{mutation.mutation:?}" }
            if let Some(progress) = mutation.progress {
                match progress.stage {
                    ResourceActionStage::Received => rsx! { p { "Request received; waiting for the runtime." } },
                    ResourceActionStage::Running(message) => rsx! { p { "{message}" } },
                    ResourceActionStage::Succeeded => rsx! { p { "Runtime confirmed completion." } },
                    ResourceActionStage::Failed(error) => rsx! { p { role: "alert", "{error.message}" } },
                }
            } else if mutation.pending { p { "Waiting for an action result…" } }
            if let Some(error) = mutation.error { p { role: "alert", "{error.message}" } }
            for action in mutation.recovery {
                Button { label: match action { ResourceRecoveryAction::CheckStatus => "Check status", ResourceRecoveryAction::Retry => "Retry original request" }, state,
                    onpress: { let id = id.clone(); move |()| onaction.call(match action {
                        ResourceRecoveryAction::CheckStatus => Event::CheckStatus(id.clone()),
                        ResourceRecoveryAction::Retry => Event::Retry(id.clone()),
                    }) },
                }
            }
        }
    }
}
