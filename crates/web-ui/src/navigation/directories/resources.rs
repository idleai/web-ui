//! Resource navigation never grants access or adopts a runtime model.

use app_core::{
    Event,
    resources::{self, ModelProviderKind, ResourceAvailability, ResourceLoadState},
    workspace::NavigationSection,
};
use dioxus::prelude::*;

use super::super::chrome::{NavRow, Notice, RowStatus, Section, navigate};
use crate::controls::{ControlState, IconButton};
use crate::icons::IconName;

#[component]
pub(in crate::navigation) fn ComputeHosts(
    view: resources::ViewModel,
    section: NavigationSection,
    enabled: bool,
    onaction: EventHandler<Event>,
) -> Element {
    let count = (view.load == ResourceLoadState::Ready || !view.hosts.is_empty())
        .then(|| view.hosts.len().to_string());
    rsx! {
        Section { section: NavigationSection::ComputeHosts, current: section, enabled, count, onaction,
            action: rsx! { IconButton { label: "Add compute host — registration unavailable", icon: IconName::Add, state: ControlState::Disabled, onpress: move |()| {} } },
            for row in &view.hosts {
                NavRow { key: "{row.host.id}", name: row.host.name.clone(), identity: row.host.id.clone(), icon: IconName::Host,
                    status: availability(row.availability), detail: availability(row.availability).label, disabled: !enabled,
                    selected: section == NavigationSection::ComputeHosts && view.selected_host.as_ref() == Some(&row.host.id),
                    onpress: {
                        let id = row.host.id.clone();
                        move |()| {
                            onaction.call(Event::Resources(resources::Event::SelectHost(Some(id.clone()))));
                            navigate(onaction, NavigationSection::ComputeHosts);
                        }
                    },
                }
            }
            ResourceFeedback { load: view.load.clone(), empty: view.hosts.is_empty() }
            if let Some(error) = view.action_error { Notice { text: error.message, error: true } }
        }
    }
}

#[component]
pub(in crate::navigation) fn ModelProviders(
    view: resources::ViewModel,
    section: NavigationSection,
    enabled: bool,
    onaction: EventHandler<Event>,
) -> Element {
    let count = (view.load == ResourceLoadState::Ready || !view.providers.is_empty())
        .then(|| view.providers.len().to_string());
    rsx! {
        Section { section: NavigationSection::ModelProviders, current: section, enabled, count, onaction,
            action: rsx! { IconButton { label: "Add model provider — registration unavailable", icon: IconName::Add, state: ControlState::Disabled, onpress: move |()| {} } },
            for row in &view.providers {
                NavRow { key: "{row.provider.id}", name: row.provider.name.clone(), identity: row.provider.id.clone(), icon: IconName::Provider,
                    status: availability(row.availability), detail: match row.provider.kind { ModelProviderKind::External => "External", ModelProviderKind::Local { .. } => "Local" }, disabled: !enabled,
                    selected: section == NavigationSection::ModelProviders && view.selected_provider.as_ref() == Some(&row.provider.id),
                    onpress: {
                        let id = row.provider.id.clone();
                        move |()| {
                            onaction.call(Event::Resources(resources::Event::SelectProvider(Some(id.clone()))));
                            navigate(onaction, NavigationSection::ModelProviders);
                        }
                    },
                }
            }
            ResourceFeedback { load: view.load, empty: view.providers.is_empty() }
        }
    }
}

fn availability(value: ResourceAvailability) -> RowStatus {
    let (label, tone) = match value {
        ResourceAvailability::Unknown => ("Availability unknown", "neutral"),
        ResourceAvailability::Available => ("Available", "success"),
        ResourceAvailability::Unavailable => ("Unavailable", "danger"),
    };
    RowStatus {
        label: label.into(),
        tone,
    }
}

#[component]
fn ResourceFeedback(load: ResourceLoadState, empty: bool) -> Element {
    match load {
        ResourceLoadState::Idle => rsx! { Notice { text: "Resources not connected" } },
        ResourceLoadState::Loading => rsx! { Notice { text: "Loading resources…" } },
        ResourceLoadState::Suspended => {
            rsx! { Notice { text: "Disconnected · waiting to reconnect" } }
        }
        ResourceLoadState::Failed(error) => rsx! { Notice { text: error.message, error: true } },
        ResourceLoadState::Ready if empty => {
            rsx! { Notice { text: "No publications in this workspace" } }
        }
        ResourceLoadState::Ready => rsx! {},
    }
}
