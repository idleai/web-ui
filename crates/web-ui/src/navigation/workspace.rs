//! Workspace and repository selectors over authoritative directory bindings.

use app_core::{
    Event,
    subscriptions::{ConnectionStatus, SubscriptionViewModel},
    workspace::{self, NavigationSection, WorkspaceMode, WorkspaceRequestState},
};
use dioxus::prelude::*;

use super::chrome::{Notice, RowStatus, Section, Status};
use crate::controls::{ControlState, IconButton, Select, SelectOption};
use crate::icons::IconName;

#[component]
pub(super) fn WorkspacePicker(
    id: String,
    view: workspace::ViewModel,
    connection: SubscriptionViewModel,
    onaction: EventHandler<Event>,
) -> Element {
    let enabled = view.selected_workspace.is_some();
    let options = std::iter::once(SelectOption {
        value: String::new(),
        label: "Select a workspace".into(),
        disabled: true,
    })
    .chain(view.workspaces.iter().map(|workspace| SelectOption {
        value: workspace.id.clone(),
        label: workspace.name.clone(),
        disabled: false,
    }))
    .collect();
    let selected = view
        .snapshot
        .as_ref()
        .map(|snapshot| &snapshot.workspace)
        .filter(|workspace| Some(&workspace.id) == view.selected_workspace.as_ref());
    let repositories = selected.map(|workspace| {
        let mut options = Vec::new();
        if workspace.mode == WorkspaceMode::Managed {
            options.push(SelectOption {
                value: String::new(),
                label: "All repositories".into(),
                disabled: false,
            });
        }
        options.extend(workspace.repositories.iter().map(|repo| SelectOption {
            value: repo.id.clone(),
            label: repo.name.clone(),
            disabled: false,
        }));
        options
    });
    let loading = view.directory_state == WorkspaceRequestState::Loading;
    let empty = view.directory_state == WorkspaceRequestState::Ready && view.workspaces.is_empty();
    let connected = connection
        .context
        .as_ref()
        .is_some_and(|context| Some(&context.workspace) == view.selected_workspace.as_ref());
    let connection_label = if connected {
        connection.status.label()
    } else {
        "Connection unknown"
    };
    let connection_tone = match connection.status {
        ConnectionStatus::Live if connected => "success",
        ConnectionStatus::Expired | ConnectionStatus::Failed if connected => "danger",
        ConnectionStatus::MissingContent | ConnectionStatus::Waiting if connected => "warning",
        ConnectionStatus::Stopped
        | ConnectionStatus::Connecting
        | ConnectionStatus::Authenticating
        | ConnectionStatus::Reconciling
        | ConnectionStatus::Live
        | ConnectionStatus::Expired
        | ConnectionStatus::Failed
        | ConnectionStatus::MissingContent
        | ConnectionStatus::Waiting => "neutral",
    };
    rsx! {
        Section { section: NavigationSection::Workspace, current: view.section, enabled, onaction,
            action: rsx! { IconButton { label: "Reload workspaces", icon: IconName::Refresh,
                state: if loading { ControlState::Busy } else { ControlState::Ready },
                onpress: move |()| onaction.call(Event::Workspace(workspace::Event::Load)),
            } },
            div { class: "idle-navigation-selectors",
                Select { id: "{id}-workspace", label: "Workspace", value: view.selected_workspace.clone().unwrap_or_default(), options,
                    onchange: move |value: String| { if !value.is_empty() { onaction.call(Event::Workspace(workspace::Event::SelectWorkspace(value))); } },
                }
                if let Some(options) = repositories {
                    Select { id: "{id}-repository", label: "Repository", value: view.selected_repository.clone().unwrap_or_default(), options,
                        onchange: move |value: String| onaction.call(Event::Workspace(workspace::Event::SelectRepository((!value.is_empty()).then_some(value)))),
                    }
                }
            }
            if enabled {
                div { class: "idle-navigation-connection",
                    Status { status: RowStatus { label: connection_label.into(), tone: connection_tone } }
                    span { "{connection_label}" }
                }
                if connected && let Some(error) = connection.error { Notice { text: error.message, error: true } }
            }
            for (label, state) in [("Workspaces", view.directory_state), ("Workspace", view.snapshot_state)] {
                match state {
                    WorkspaceRequestState::Idle => rsx! { Notice { text: "{label} not loaded" } },
                    WorkspaceRequestState::Loading => rsx! { Notice { text: "Loading {label}…" } },
                    WorkspaceRequestState::Ready => rsx! {},
                    WorkspaceRequestState::Failed(error) => rsx! { Notice { text: error.message, error: true } },
                }
            }
            if empty { Notice { text: "No workspaces available" } }
            if let Some(error) = view.selection_error { Notice { text: error.message, error: true } }
        }
    }
}
