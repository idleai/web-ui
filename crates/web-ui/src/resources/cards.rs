use app_core::resources::{
    ComputeHostView, Event, ModelPackageView, ModelProviderKind, ModelProviderView,
    ResourceAvailability, ResourceMutation, ResourcePermission, ServedModelView,
};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{Element, EventHandler, Props, component, dioxus_core, dioxus_elements, rsx};

use crate::controls::{Button, ControlState};

pub(super) const fn availability(value: ResourceAvailability) -> &'static str {
    match value {
        ResourceAvailability::Available => "Available",
        ResourceAvailability::Unavailable => "Unavailable",
        ResourceAvailability::Unknown => "Availability unknown",
    }
}

const fn enabled(value: bool) -> ControlState {
    if value {
        ControlState::Ready
    } else {
        ControlState::Disabled
    }
}

#[component]
pub(super) fn HostCard(
    host: ComputeHostView,
    onaction: EventHandler<Event>,
    onexecute: EventHandler<ResourceMutation>,
) -> Element {
    let connect = enabled(host.actions.contains(&ResourcePermission::ConnectHost));
    let id = host.host.id.clone();
    rsx! {
        article { class: "idle-resource-card", aria_label: host.host.name.clone(),
            h3 { "{host.host.name}" }
            p { "{availability(host.availability)}" }
            p { "Owner: {host.host.owner} · Revision {host.host.revision}" }
            p { "Published features: {host.host.features:?}" }
            div { class: "idle-resource-actions",
                Button { label: "View host", onpress: move |()| onaction.call(Event::SelectHost(Some(id.clone()))) }
                Button { label: "Connect", state: connect, onpress: move |()| onexecute.call(ResourceMutation::ConnectHost { host_id: host.host.id.clone() }) }
            }
        }
    }
}

#[component]
pub(super) fn ProviderCard(provider: ModelProviderView, onaction: EventHandler<Event>) -> Element {
    rsx! {
        article { class: "idle-resource-card", aria_label: provider.provider.name.clone(),
            h3 { "{provider.provider.name}" }
            p { "{availability(provider.availability)}" }
            p { "Owner: {provider.provider.owner} · Revision {provider.provider.revision}" }
            match &provider.provider.kind {
                ModelProviderKind::External => rsx! { p { "External provider" } },
                ModelProviderKind::Local { host_id, runtime_id } => rsx! { p { "Local provider · Host {host_id} · Runtime {runtime_id}" } },
            }
            Button { label: "View provider", onpress: move |()| onaction.call(Event::SelectProvider(Some(provider.provider.id.clone()))) }
        }
    }
}

#[component]
pub(super) fn ModelCard(
    model: ServedModelView,
    onexecute: EventHandler<ResourceMutation>,
) -> Element {
    rsx! {
        article { class: "idle-resource-card idle-resource-model", aria_label: model.model.name.clone(),
            h4 { "{model.model.name}" }
            p { "{model.model.key.provider_id} / {model.model.key.model_id}" }
            p { "{availability(model.availability)} · {model.model.features:?}" }
            if model.selectable_for.is_empty() {
                p { "No connected session can select this model." }
            }
            for target in model.selectable_for {
                Button { key: "{target.runtime_id}:{target.session_id}", label: format!("Use for {}", target.session_id),
                    onpress: { let key = model.model.key.clone(); move |()| onexecute.call(ResourceMutation::SelectModel { target: target.clone(), model: key.clone() }) },
                }
            }
        }
    }
}

#[component]
pub(super) fn PackageCard(
    package: ModelPackageView,
    onexecute: EventHandler<ResourceMutation>,
) -> Element {
    let intent = package.package_info.clone();
    rsx! {
        article { class: "idle-resource-card", aria_label: package.package_info.name.clone(),
            h4 { "{package.package_info.name}" }
            p { "Host: {package.package_info.host_id}" }
            Button { label: "Install and serve", state: enabled(package.can_install),
                onpress: move |()| onexecute.call(ResourceMutation::InstallModel(intent.clone())),
            }
        }
    }
}
