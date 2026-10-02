//! Initial host composition of the completed history and session surfaces.

use app_core::{Event, ViewModel, history, sessions, workspace};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{
    Element, EventHandler, Props, WritableExt, component, dioxus_core, dioxus_elements, rsx,
    use_signal,
};

use crate::controls::{Button, Select, SelectOption, TextField, TextFieldKind};
use crate::history::details::HistoryTimeline;
use crate::host::HostCapabilities;
use crate::sessions::{PromptComposer, SessionConversation, SessionFeedback, SessionSharing};
use crate::theme::{Density, Theme, ThemeProvider};

/// Host presentation size; application state and events are identical in both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Surface {
    /// Narrow workspace sidebar.
    #[default]
    Sidebar,
    /// Wider editor tab.
    Detail,
}

/// Assemble available shared components over a persistent host-owned app-core.
/// Mount one surface per document and load the theme, graph, details and session stylesheets.
#[component]
pub fn WorkspaceSurface(
    view: ViewModel,
    capabilities: HostCapabilities,
    onaction: EventHandler<Event>,
    #[props(default)] surface: Surface,
    #[props(default = Theme::VsCode)] theme: Theme,
    error: Option<String>,
) -> Element {
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
            main { class: "idle-shell idle-stack",
                h1 { class: "idle-heading", "Idle" }
                if let Some(error) = error { p { role: "alert", "{error}" } }
                Select { id: "idle-workspace", label: "Workspace", value: selected.clone().unwrap_or_default(), options,
                    onchange: move |value: String| {
                        if !value.is_empty() { onaction.call(Event::Workspace(workspace::Event::SelectWorkspace(value))); }
                    },
                }
                if directory.workspaces.is_empty() { p { "Open a trusted workspace folder to browse its history." } }
                for state in [directory.directory_state, directory.snapshot_state] {
                    if let workspace::WorkspaceRequestState::Failed(error) = state {
                        p { role: "alert", "{error.message}" }
                    }
                }
                Button { label: "Reload workspaces", onpress: move |()| onaction.call(Event::Workspace(workspace::Event::Load)) }
                if selected.is_some() {
                    nav { aria_label: "Workspace views",
                        Button { label: "Activity", onpress: move |()| onaction.call(Event::Workspace(workspace::Event::Navigate(workspace::NavigationSection::Activity))) }
                        Button { label: "Sessions", onpress: move |()| onaction.call(Event::Workspace(workspace::Event::Navigate(workspace::NavigationSection::Sessions))) }
                    }
                    if session_selected {
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
                    } else {
                        HistorySearch { key: "{view.history.chain:?}", text: view.history.search.text.clone(), onaction: history_action }
                        if !view.history.search.text.is_empty() {
                            p { "{view.history.search.matches.len()} loaded matches" }
                            Button { label: "Next match", onpress: move |()| history_action.call(history::Event::NavigateMatch(1)) }
                            Button { label: "Search more", onpress: move |()| history_action.call(history::Event::SearchMore) }
                        }
                        Button { label: "Refresh history", onpress: move |()| history_action.call(history::Event::Refresh) }
                        HistoryTimeline { id: "idle-history", view: view.history, capabilities, onaction: history_action, height: if surface == Surface::Sidebar { 400 } else { 640 } }
                    }
                }
            }
        }
    }
}

#[component]
fn HistorySearch(text: String, onaction: EventHandler<history::Event>) -> Element {
    let mut search = use_signal(|| text);
    rsx! {
        TextField { id: "idle-search", label: "Search history", kind: TextFieldKind::Search, value: search(),
            oninput: move |value| search.set(value), oncommit: move |value| onaction.call(history::Event::Search(value)) }
        Button { label: "Search", onpress: move |()| onaction.call(history::Event::Search(search())) }
    }
}
