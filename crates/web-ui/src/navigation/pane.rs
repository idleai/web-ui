//! Content for one host-owned sidebar view, without an HTML pane header.

use app_core::{Event, ViewModel, workspace::NavigationSection};
use dioxus::prelude::*;

use super::{SessionCreation, activity, directories, sessions, workspace};

#[derive(Clone, Copy)]
pub(super) struct NativePane;

/// Render one navigation section inside a native collapsible, resizable view.
/// The host supplies its header, actions and layout; Rust owns the content.
#[component]
pub fn WorkspaceNavigationPane(
    id: String,
    section: NavigationSection,
    view: ViewModel,
    onaction: EventHandler<Event>,
    creation: Option<SessionCreation>,
    now_ms: Option<u64>,
) -> Element {
    let _pane = use_context_provider(|| NativePane);
    let enabled = view.workspace.selected_workspace.is_some();
    let current = view.workspace.section;
    let workspace_id = view.workspace.selected_workspace.clone();
    rsx! {
        nav { id: id.clone(), class: "idle-navigation idle-navigation-pane",
            "data-section": "{section:?}",
            "data-workspace": workspace_id,
            aria_label: super::chrome::label(section),
            match section {
                NavigationSection::Workspace => rsx! {
                    workspace::WorkspacePicker { id: id.clone(), view: view.workspace, repository: view.repository, connection: view.subscriptions, onaction }
                },
                NavigationSection::Members => rsx! { directories::Users { view: view.workspace, onaction } },
                NavigationSection::Sessions => rsx! { sessions::Sessions { view: view.sessions, controller: view.resources.controller, section: current, enabled, creation, now_ms, onaction } },
                NavigationSection::Projections => rsx! { directories::Projections { view: view.projections, section: current, enabled, onaction } },
                NavigationSection::ComputeHosts => rsx! { directories::ComputeHosts { view: view.resources, section: current, enabled, onaction } },
                NavigationSection::ModelProviders => rsx! { directories::ModelProviders { view: view.resources, section: current, enabled, onaction } },
                NavigationSection::Activity => rsx! {
                    activity::Activity { id: "{id}-activity", view: view.history, onaction }
                    super::chrome::ConfigurationLinks { current, enabled, onaction }
                },
                NavigationSection::Settings | NavigationSection::AgentRules => rsx! {},
            }
        }
    }
}
