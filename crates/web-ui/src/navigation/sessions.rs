//! Control-first session rows and a context-bound creation action.

use app_core::{
    Event,
    resources::{ControllerAssignment, ControllerPhase, ControllerView},
    sessions::{
        self, SessionCapability, SessionContext, SessionDraft, SessionKind, SessionLoadState,
        SessionMutationId, SessionRelationship,
    },
    workspace::NavigationSection,
};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{
    Element, EventHandler, Props, ReadableExt, WritableExt, component, dioxus_core,
    dioxus_elements, rsx, use_signal,
};

use super::chrome::{NavRow, Notice, RowStatus, Section, navigate};
use crate::icons::{Icon, IconName};

/// Host-prepared runner creation, persisted before executing the emitted effect.
///
/// Reserve a fresh mutation identity for each new creation. Retain the exact draft
/// for recovery and let app-core own retries; the sidebar never allocates IDs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionCreation {
    /// Authenticated connection for which the creation was prepared.
    pub context: SessionContext,
    /// Caller-reserved request identity and first-receipt deadline.
    pub mutation: SessionMutationId,
    /// Supplied title, compute host and optional parent.
    pub draft: SessionDraft,
}

fn unavailable(
    view: &sessions::ViewModel,
    creation: Option<&SessionCreation>,
    now_ms: Option<u64>,
) -> Option<&'static str> {
    if !crate::sessions::ready(view) {
        return Some("Connect sessions to add a runner");
    }
    if view.capabilities.create != SessionCapability::Available {
        return Some("Session creation is unavailable on this connection");
    }
    let Some(creation) = creation else {
        return Some("Prepare a session title and compute host to add a runner");
    };
    if view.context.as_ref() != Some(&creation.context) {
        return Some("The prepared session belongs to another connection");
    }
    if now_ms.is_none_or(|now| now >= creation.mutation.expires_at_ms) {
        return Some("Waiting for a current session setup");
    }
    if creation.mutation.request_id.trim().is_empty()
        || creation.draft.title.trim().is_empty()
        || creation.draft.host_id.trim().is_empty()
    {
        return Some("Session setup is not ready");
    }
    if view
        .mutations
        .iter()
        .any(|entry| entry.request.mutation.request_id == creation.mutation.request_id)
    {
        return Some("Session creation already submitted");
    }
    None
}

#[component]
pub(super) fn Sessions(
    view: sessions::ViewModel,
    controller: ControllerView,
    section: NavigationSection,
    enabled: bool,
    creation: Option<SessionCreation>,
    now_ms: Option<u64>,
    onaction: EventHandler<Event>,
) -> Element {
    let count = (view.load == SessionLoadState::Ready || !view.sessions.is_empty())
        .then(|| view.sessions.len().to_string());
    let controls = view
        .sessions
        .iter()
        .filter(|row| row.session.kind == SessionKind::Control);
    let runners = view
        .sessions
        .iter()
        .filter(|row| row.session.kind == SessionKind::Runner);
    let control_status = control_status(&controller);
    rsx! {
        Section { section: NavigationSection::Sessions, current: section, enabled, count, onaction,
            action: rsx! { AddSession { view: view.clone(), creation, now_ms, enabled, onaction } },
            div { class: "idle-navigation-list",
                if !view.sessions.iter().any(|row| row.session.kind == SessionKind::Control) {
                    NavRow { name: "Control model", icon: IconName::Session, detail: "Ambient · {control_status.label}", status: control_status.clone(), disabled: !enabled,
                        onpress: move |()| navigate(onaction, NavigationSection::Sessions),
                    }
                }
                for row in controls.chain(runners) {
                    NavRow { key: "{row.session.id}", name: row.session.title.clone(), icon: IconName::Session,
                        identity: row.session.id.clone(), disabled: !enabled,
                        selected: section == NavigationSection::Sessions && view.selected.as_ref() == Some(&row.session.id),
                        status: if row.session.kind == SessionKind::Control { control_status.clone() } else { RowStatus { label: "Runtime status unknown".into(), tone: "neutral" } },
                        detail: if row.session.kind == SessionKind::Control { format!("Ambient · {}", control_status.label) } else { match row.relationship { SessionRelationship::Owned => "Owned".into(), SessionRelationship::Invited => "Invited".into() } },
                        onpress: {
                            let id = row.session.id.clone();
                            move |()| {
                                onaction.call(Event::Sessions(sessions::Event::Select(Some(id.clone()))));
                                navigate(onaction, NavigationSection::Sessions);
                            }
                        },
                    }
                }
            }
            match view.load {
                SessionLoadState::Idle => rsx! { Notice { text: "Sessions not connected" } },
                SessionLoadState::Loading => rsx! { Notice { text: "Loading sessions…" } },
                SessionLoadState::Ready => rsx! {},
                SessionLoadState::Failed(error) => rsx! { Notice { text: error.message, error: true } },
            }
            if let SessionLoadState::Failed(error) = view.updates { Notice { text: error.message, error: true } }
            if let Some(error) = view.action_error { Notice { text: error.message, error: true } }
        }
    }
}

fn control_status(view: &ControllerView) -> RowStatus {
    let (label, tone) = match &view.phase {
        ControllerPhase::Unknown => (
            match view.assignment {
                ControllerAssignment::Unknown => "Status unknown".into(),
                ControllerAssignment::Unassigned => "Unassigned".into(),
                ControllerAssignment::Assigned => "Assigned · runtime unknown".into(),
                ControllerAssignment::Expired => "Lease expired".into(),
            },
            "neutral",
        ),
        ControllerPhase::Starting => ("Starting".into(), "neutral"),
        ControllerPhase::Running => ("Running".into(), "success"),
        ControllerPhase::Paused => ("Paused".into(), "warning"),
        ControllerPhase::Stopped => ("Stopped".into(), "neutral"),
        ControllerPhase::Failed(message) => (format!("Error: {message}"), "danger"),
    };
    RowStatus { label, tone }
}

#[component]
fn AddSession(
    view: sessions::ViewModel,
    creation: Option<SessionCreation>,
    now_ms: Option<u64>,
    enabled: bool,
    onaction: EventHandler<Event>,
) -> Element {
    let reason = unavailable(&view, creation.as_ref(), now_ms);
    let mut submitted = use_signal(|| None::<SessionCreation>);
    let disabled = !enabled
        || reason.is_some()
        || creation
            .as_ref()
            .is_some_and(|creation| submitted.read().as_ref() == Some(creation));
    rsx! {
        button { r#type: "button", class: "idle-button idle-icon-button", "data-variant": "quiet", aria_label: String::from("Add session"),
            title: reason.unwrap_or("Add session"), disabled,
            onclick: move |_| {
                if !disabled && let Some(creation) = &creation && submitted.peek().as_ref() != Some(creation) {
                    submitted.set(Some(creation.clone()));
                    onaction.call(Event::Sessions(sessions::Event::Create { id: creation.mutation.clone(), draft: creation.draft.clone() }));
                    navigate(onaction, NavigationSection::Sessions);
                }
            },
            Icon { name: IconName::Add }
        }
    }
}
