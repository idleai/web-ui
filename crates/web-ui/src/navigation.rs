//! Workspace sidebar over shared app-core state and typed events.
//!
//! Mount under a theme provider and load [`STYLESHEET`] and the history graph
//! stylesheet. Hosts own connection adapters and persisted creation drafts.

mod activity;
mod chrome;
mod directories;
mod pane;
mod sessions;
mod workspace;

pub use pane::WorkspaceNavigationPane;
pub use sessions::SessionCreation;

use app_core::{Event, ViewModel, workspace::NavigationSection};
use dioxus::prelude::*;

use chrome::{ConfigurationLinks, Section};

/// Navigation styles scoped to the shared theme, for browser and VS Code hosts.
pub const STYLESHEET: &str = include_str!("../assets/navigation.css");

/// Ordered workspace overview, including an independently scrollable Activity graph.
///
/// Supply a document-unique `id`, a persistent root view and its event dispatcher.
/// Selection is controlled entirely by app-core; rendering never starts requests.
/// A prepared creation draft enables the session add action when its scope,
/// deadline and supplied runtime capability are current.
#[component]
pub fn WorkspaceNavigation(
    id: String,
    view: ViewModel,
    onaction: EventHandler<Event>,
    creation: Option<SessionCreation>,
    now_ms: Option<u64>,
) -> Element {
    let directory = view.workspace.clone();
    let enabled = directory.selected_workspace.is_some();
    let section = directory.section;
    rsx! {
        nav { id: id.clone(), class: "idle-navigation", aria_label: "Workspace navigation",
            workspace::WorkspacePicker { id: id.clone(), view: directory.clone(), repository: view.repository, connection: view.subscriptions, onaction }
            directories::Users { view: directory.clone(), onaction }
            sessions::Sessions { view: view.sessions, controller: view.resources.controller.clone(), section, enabled, creation, now_ms, onaction }
            directories::Projections { view: view.projections, section, enabled, onaction }
            directories::ComputeHosts { view: view.resources.clone(), section, enabled, onaction }
            directories::ModelProviders { view: view.resources, section, enabled, onaction }
            Section { section: NavigationSection::Activity, current: section, enabled, onaction,
                activity::Activity { id: "{id}-activity", view: view.history, onaction }
            }
            ConfigurationLinks { current: section, enabled, onaction }
        }
    }
}

#[cfg(test)]
mod tests;
