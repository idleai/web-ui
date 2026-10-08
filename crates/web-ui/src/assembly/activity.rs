//! Editor-sized activity destination without the general workspace dashboard.

use app_core::{Event, ViewModel, workspace};
use dioxus::prelude::*;

use crate::controls::{IconButton, Select, SelectOption};
use crate::history::explorer::HistoryExplorer;
use crate::host::HostCapabilities;
use crate::icons::IconName;
use crate::provenance::ActivitySnapshot;
use crate::theme::{Density, Theme, ThemeProvider};

#[component]
pub(super) fn ActivityEditor(
    view: ViewModel,
    capabilities: HostCapabilities,
    theme: Theme,
    error: Option<String>,
    activity: Option<ActivitySnapshot>,
    onaction: EventHandler<Event>,
) -> Element {
    let selected = view
        .workspace
        .selected_workspace
        .clone()
        .unwrap_or_default();
    let options = std::iter::once(SelectOption {
        value: String::new(),
        label: "Select a workspace".into(),
        disabled: true,
    })
    .chain(
        view.workspace
            .workspaces
            .iter()
            .map(|workspace| SelectOption {
                value: workspace.id.clone(),
                label: workspace.name.clone(),
                disabled: false,
            }),
    )
    .collect();
    rsx! {
        ThemeProvider { theme, density: Density::Compact,
            main { class: "idle-activity-editor",
                header { class: "idle-activity-editor-context",
                    strong { "Activity" }
                    Select { id: "idle-activity-workspace", label: "Workspace", value: selected, options,
                        onchange: move |value: String| { if !value.is_empty() { onaction.call(Event::Workspace(workspace::Event::SelectWorkspace(value))); } } }
                    IconButton { label: "Open workspace overview", icon: IconName::Workspace,
                        onpress: move |()| onaction.call(Event::Workspace(workspace::Event::Navigate(workspace::NavigationSection::Workspace))) }
                }
                if let Some(error) = error { p { role: "alert", "{error}" } }
                for state in [view.workspace.directory_state, view.workspace.snapshot_state] {
                    if let workspace::WorkspaceRequestState::Failed(error) = state { p { role: "alert", "{error.message}" } }
                }
                if view.workspace.selected_workspace.is_none() {
                    p { "Select a workspace to browse its recorded activity." }
                } else {
                    HistoryExplorer { id: "idle-history", view: view.history, capabilities, activity,
                        onaction: move |event| onaction.call(Event::History(event)) }
                }
            }
        }
    }
}
