//! Compute, provider, model and controller views with explicit runtime actions.

use app_core::resources::{Event, ResourceLoadState, ResourceMutation, ResourceViewModel};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{Element, EventHandler, Props, component, dioxus_core, dioxus_elements, rsx};

use crate::controls::Button;

mod cards;
mod feedback;

/// A resource directory. Mutation intents go to the host, which must retain an
/// original request identity before forwarding them to an authorized runtime.
#[component]
pub fn ResourceDirectory(
    view: ResourceViewModel,
    hosts: bool,
    onaction: EventHandler<Event>,
    onexecute: EventHandler<ResourceMutation>,
) -> Element {
    let title = if hosts {
        "Compute hosts"
    } else {
        "Model providers"
    };
    rsx! {
        section { class: "idle-resources idle-stack", aria_label: title,
            div { class: "idle-resource-heading",
                h2 { "{title}" }
                Button { label: "Refresh resources", onpress: move |()| onaction.call(Event::Refresh) }
            }
            feedback::LoadFeedback { view: view.clone() }
            if hosts {
                if view.selected_host.is_some() {
                    Button { label: "Show all hosts", onpress: move |()| onaction.call(Event::SelectHost(None)) }
                }
                for host in view.hosts.iter().filter(|host| view.selected_host.as_ref().is_none_or(|id| id == &host.host.id)) {
                    cards::HostCard { key: "{host.host.id}", host: host.clone(), onaction, onexecute }
                }
                if view.hosts.is_empty() && view.load == ResourceLoadState::Ready {
                    p { "No compute hosts are published in this repository." }
                }
                if !view.packages.is_empty() {
                    h3 { "Local models" }
                    p { "Installation options come from the connected runtime." }
                    for package in &view.packages {
                        cards::PackageCard { key: "{package.package_info.host_id}:{package.package_info.package_id}", package: package.clone(), onexecute }
                    }
                }
            } else {
                if view.selected_provider.is_some() {
                    Button { label: "Show all providers", onpress: move |()| onaction.call(Event::SelectProvider(None)) }
                }
                for provider in view.providers.iter().filter(|provider| view.selected_provider.as_ref().is_none_or(|id| id == &provider.provider.id)) {
                    cards::ProviderCard { key: "{provider.provider.id}", provider: provider.clone(), onaction }
                    for model in view.models.iter().filter(|model| model.model.key.provider_id == provider.provider.id) {
                        cards::ModelCard { key: "{model.model.key.provider_id}:{model.model.key.model_id}", model: model.clone(), onexecute }
                    }
                }
                if view.providers.is_empty() && view.load == ResourceLoadState::Ready {
                    p { "No model providers are published in this repository." }
                }
                for selection in &view.selections {
                    p { key: "{selection.target.runtime_id}:{selection.target.session_id}",
                        "Session {selection.target.session_id}: "
                        if let Some(model) = &selection.selected { "{model.provider_id} / {model.model_id}" }
                        else { "no model confirmed by the runtime" }
                    }
                }
            }
            feedback::ControllerStatus { view: view.controller }
            for mutation in view.mutations {
                feedback::MutationStatus { key: "{mutation.request.request_id}", mutation, onaction }
            }
        }
    }
}

#[cfg(test)]
mod tests;
