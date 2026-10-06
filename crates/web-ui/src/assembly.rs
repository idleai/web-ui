//! Host composition of workspace navigation, history and session surfaces.

use app_core::{Event, ViewModel, history, projections, sessions, workspace};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{
    Element, EventHandler, Props, WritableExt, component, dioxus_core, dioxus_elements, rsx,
    use_signal,
};

use crate::controls::{Button, ControlState, Select, SelectOption, TextField, TextFieldKind};
use crate::history::details::HistoryTimeline;
use crate::host::HostCapabilities;
use crate::navigation::{SessionCreation, WorkspaceNavigation, WorkspaceNavigationPane};
use crate::projections::ProjectionPanel;
use crate::provenance::ActivitySnapshot;
use crate::sessions::{PromptComposer, SessionConversation, SessionFeedback, SessionSharing};
use crate::theme::{Density, Theme, ThemeProvider};

/// Host presentation size; application state and events are identical in both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Surface {
    /// Narrow workspace sidebar.
    #[default]
    Sidebar,
    /// Content of one native host view; its header and divider belong to the host.
    SidebarPane(workspace::NavigationSection),
    /// Wider editor tab.
    Detail,
}

/// Assemble available shared components over a persistent host-owned app-core.
/// Mount one surface per document and load the theme, navigation, graph, details
/// and session/projection stylesheets. Sidebar selections retain the existing
/// history and session details. Supply `destination` for host-composed member,
/// resource and configuration screens selected through app-core.
#[component]
pub fn WorkspaceSurface(
    view: ViewModel,
    capabilities: HostCapabilities,
    onaction: EventHandler<Event>,
    #[props(default)] surface: Surface,
    #[props(default = Theme::VsCode)] theme: Theme,
    error: Option<String>,
    creation: Option<SessionCreation>,
    now_ms: Option<u64>,
    destination: Option<Element>,
    onopen: Option<EventHandler<String>>,
    activity: Option<ActivitySnapshot>,
) -> Element {
    if let Surface::SidebarPane(section) = surface {
        return rsx! {
            ThemeProvider { theme, density: Density::Compact,
                main { class: "idle-workspace-sidebar idle-native-pane",
                    WorkspaceNavigationPane { id: "idle-navigation", section, view, onaction, creation, now_ms }
                    if let Some(error) = error { p { role: "alert", "{error}" } }
                }
            }
        };
    }
    let history_action = EventHandler::new(move |event| onaction.call(Event::History(event)));
    let session_action = EventHandler::new(move |event| onaction.call(Event::Sessions(event)));
    let directory = view.workspace.clone();
    let selected = directory.selected_workspace.clone();
    let session_selected = directory.section == workspace::NavigationSection::Sessions;
    let options = std::iter::once(SelectOption {
        value: String::new(),
        label: "Select a workspace".to_owned(),
        disabled: true,
    })
    .chain(directory.workspaces.iter().map(|workspace| SelectOption {
        value: workspace.id.clone(),
        label: workspace.name.clone(),
        disabled: false,
    }))
    .collect();
    let density = if surface == Surface::Sidebar {
        Density::Compact
    } else {
        Density::Comfortable
    };
    rsx! {
        ThemeProvider { theme, density,
            main { class: if surface == Surface::Sidebar { "idle-workspace-sidebar" } else { "idle-shell idle-stack" },
                if surface == Surface::Sidebar {
                    WorkspaceNavigation { id: "idle-navigation", view: view.clone(), onaction, creation, now_ms }
                } else {
                h1 { class: "idle-heading", "Idle" }
                Select { id: "idle-workspace", label: "Workspace", value: selected.clone().unwrap_or_default(), options,
                    onchange: move |value: String| {
                        if !value.is_empty() { onaction.call(Event::Workspace(workspace::Event::SelectWorkspace(value))); }
                    },
                }
                if directory.workspaces.is_empty() { p { "No local history workspaces are available. Open a trusted workspace folder or check its history settings." } }
                for state in [directory.directory_state, directory.snapshot_state] {
                    if let workspace::WorkspaceRequestState::Failed(error) = state {
                        p { role: "alert", "{error.message}" }
                    }
                }
                Button { label: "Reload workspaces", onpress: move |()| onaction.call(Event::Workspace(workspace::Event::Load)) }
                }
                if let Some(error) = error { p { role: "alert", "{error}" } }
                if selected.is_some() {
                    if surface == Surface::Detail { DetailNavigation { current: directory.section, onaction } }
                    if let Some(destination) = destination { {destination} }
                    else if surface == Surface::Detail && directory.section == workspace::NavigationSection::Workspace && view.repository.context.is_some() {
                        crate::repository::RepositoryOverview { view: view.repository.clone(), onaction: move |event| onaction.call(Event::Repository(event)), onopen, now_ms }
                    } else if directory.section == workspace::NavigationSection::Members {
                        crate::repository::RepositoryUsers { view: view.repository.clone(), workspace: view.workspace.clone(), onaction: move |event| onaction.call(Event::Repository(event)), onopen, now_ms }
                    } else if session_selected && view.repository.context.is_some() {
                        crate::repository::RecordedSessions { view: view.repository.clone(), onaction: move |event| onaction.call(Event::Repository(event)), now_ms }
                        {rsx! { HistorySearch { key: "recorded:{view.repository.selected_session:?}", view: view.history.search.clone(), onaction: history_action } }}
                        Button { label: "Refresh recorded history", onpress: move |()| history_action.call(history::Event::Refresh) }
                        HistoryTimeline { id: "idle-recorded-history", view: view.history.clone(), capabilities: capabilities.clone(), onaction: history_action, activity: activity.clone(), height: if surface == Surface::Sidebar { 400 } else { 640 } }
                    }
                    else if session_selected {
                        if view.sessions.context.is_none() { p { "A session connection is not available for this workspace." } }
                        SessionFeedback { view: view.sessions.clone(), onaction: session_action }
                        for session in &view.sessions.sessions {
                            Button { key: "{session.session.id}", label: session.session.title.clone(), onpress: {
                                let id = session.session.id.clone();
                                move |()| session_action.call(sessions::Event::Select(Some(id.clone())))
                            } }
                        }
                        SessionConversation { id: "idle-session", view: view.sessions.clone(), history: Some(view.history.clone()), capabilities: capabilities.clone(), onaction: session_action, onhistoryaction: history_action, now_ms: None }
                        PromptComposer { id: "idle-composer", view: view.sessions.clone(), draft: None, oninput: move |_text| {}, onaction: session_action }
                        SessionSharing { id: "idle-sharing", view: view.sessions, draft: None, recipients: Vec::new(), onchange: move |_draft| {}, onaction: session_action, now_ms: None }
                    } else if directory.section == workspace::NavigationSection::Activity || (surface == Surface::Detail && directory.section == workspace::NavigationSection::Workspace) {
                        HistorySearch { key: "{view.history.chain:?}", view: view.history.search.clone(), onaction: history_action }
                        Button { label: "Refresh history", onpress: move |()| history_action.call(history::Event::Refresh) }
                        HistoryTimeline { id: "idle-history", view: view.history, capabilities, onaction: history_action, activity, height: if surface == Surface::Sidebar { 400 } else { 640 } }
                    } else if directory.section == workspace::NavigationSection::Projections {
                        if view.repository.context.is_some() {
                            p { "Tasks show open GitHub issues and pull requests. Errors show failed checks and workflow runs for the recorded checkout HEAD. Triage and Human input use explicit labels and review requests." }
                            crate::repository::SourceReports { view: view.repository.clone(), prefix: "github", now_ms }
                        }
                        for kind in [projections::ProjectionKind::Task, projections::ProjectionKind::Error, projections::ProjectionKind::Triage, projections::ProjectionKind::NeedInput] {
                            ProjectionPanel { id: format!("idle-projection-{kind:?}"), view: view.projections.clone(), kind, onaction: move |event| onaction.call(Event::Projections(event)), onopen }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn DetailNavigation(
    current: workspace::NavigationSection,
    onaction: EventHandler<Event>,
) -> Element {
    use workspace::NavigationSection;
    rsx! {
        nav { class: "idle-detail-navigation", aria_label: "Workspace views",
            for (section, label) in [
                (NavigationSection::Workspace, "Workspace"),
                (NavigationSection::Members, "Users"),
                (NavigationSection::Sessions, "Sessions"),
                (NavigationSection::Projections, "Projections"),
                (NavigationSection::ComputeHosts, "Compute hosts"),
                (NavigationSection::ModelProviders, "Model providers"),
                (NavigationSection::Activity, "Activity"),
                (NavigationSection::Settings, "Settings"),
                (NavigationSection::AgentRules, "Agent Rules"),
            ] {
                button { key: "{section:?}", r#type: "button", class: "idle-button", aria_current: if current == section { "page" } else { "false" },
                    onclick: move |_| onaction.call(Event::Workspace(workspace::Event::Navigate(section))), "{label}"
                }
            }
        }
    }
}

#[component]
fn HistorySearch(view: history::SearchView, onaction: EventHandler<history::Event>) -> Element {
    let mut search = use_signal(|| view.text.clone());
    let loading = view.paging.state == history::RequestState::Loading;
    let search_state = if loading {
        ControlState::Busy
    } else {
        ControlState::Ready
    };
    let more_state = if loading {
        ControlState::Busy
    } else if view.paging.exhausted {
        ControlState::Disabled
    } else {
        ControlState::Ready
    };
    let next_state = if view.matches.is_empty() {
        ControlState::Disabled
    } else {
        ControlState::Ready
    };
    let more_label = if matches!(view.paging.state, history::RequestState::Failed(_)) {
        "Retry search"
    } else {
        "Search more"
    };
    rsx! {
        TextField { id: "idle-search", label: "Search history", kind: TextFieldKind::Search, value: search(),
            oninput: move |value| search.set(value), oncommit: move |value| onaction.call(history::Event::Search(value)) }
        Button { label: "Search", state: search_state, onpress: move |()| onaction.call(history::Event::Search(search())) }
        if !view.text.is_empty() {
            p { "{view.matches.len()} loaded matches" }
            if loading { p { role: "status", "Searching history…" } }
            if let history::RequestState::Failed(error) = view.paging.state { p { role: "alert", "{error.message}" } }
            if !view.unavailable.is_empty() {
                p { role: "status", "{view.unavailable.len()} fields could not be searched. Results may be incomplete." }
            }
            if view.paging.exhausted { p { "Search reached the end of the available history." } }
            Button { label: "Next match", state: next_state, onpress: move |()| onaction.call(history::Event::NavigateMatch(1)) }
            Button { label: more_label, state: more_state, onpress: move |()| onaction.call(history::Event::SearchMore) }
        }
    }
}
