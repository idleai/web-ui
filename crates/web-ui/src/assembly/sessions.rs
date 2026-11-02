//! Live runner selection and saved creation, prompt and sharing forms.

use app_core::{Event, ViewModel, sessions};
use dioxus::prelude::*;

use crate::controls::{Button, ControlState, TextField};
use crate::host::HostCapabilities;
use crate::sessions::{
    PromptComposer, SessionConversation, SessionFeedback, SessionFormEdit, SessionForms,
    SessionSharing,
};

#[component]
pub(super) fn LiveSessions(
    view: ViewModel,
    capabilities: HostCapabilities,
    forms: Option<SessionForms>,
    onedit: Option<EventHandler<SessionFormEdit>>,
    onaction: EventHandler<Event>,
    now_ms: Option<u64>,
) -> Element {
    let forms = forms.unwrap_or_default();
    let session_action = EventHandler::new(move |event| onaction.call(Event::Sessions(event)));
    let edit = EventHandler::new(move |event| {
        if let Some(onedit) = onedit {
            onedit.call(event);
        }
    });
    rsx! {
        section { class: "idle-stack", aria_label: "Live sessions",
            h2 { "Live sessions" }
            SessionFeedback { view: view.sessions.clone(), onaction: session_action }
            if let Some(message) = forms.readiness { p { role: "alert", "{message}" } }
            Creation { view: view.sessions.clone(), creation: forms.creation, onedit: edit, onaction: session_action, now_ms }
            nav { class: "idle-session-actions", aria_label: "Live runner sessions",
                for session in &view.sessions.sessions {
                    Button { key: "{session.session.id}", label: session.session.title.clone(), onpress: {
                        let id = session.session.id.clone();
                        move |()| session_action.call(sessions::Event::Select(Some(id.clone())))
                    } }
                }
            }
            div { class: "idle-stack",
                SessionConversation { id: "idle-session", view: view.sessions.clone(), history: Some(view.history.clone()), capabilities,
                    onaction: session_action, onhistoryaction: move |event| onaction.call(Event::History(event)), now_ms }
                PromptComposer { id: "idle-composer", view: view.sessions.clone(), draft: forms.prompt,
                    oninput: move |text| edit.call(SessionFormEdit::Prompt(text)), onaction: session_action }
                SessionSharing { id: "idle-sharing", view: view.sessions, draft: forms.invitation, recipients: forms.recipients,
                    onchange: move |draft| edit.call(SessionFormEdit::Invitation(Box::new(draft))), onaction: session_action, now_ms }
            }
            if view.repository.context.is_some() {
                details { summary { "Recorded sessions" }
                    crate::repository::RecordedSessions { view: view.repository, onaction: move |event| onaction.call(Event::Repository(event)), now_ms }
                }
            }
        }
    }
}

#[component]
fn Creation(
    view: sessions::ViewModel,
    creation: Option<crate::navigation::SessionCreation>,
    onedit: EventHandler<SessionFormEdit>,
    onaction: EventHandler<sessions::Event>,
    now_ms: Option<u64>,
) -> Element {
    let reason = crate::navigation::sessions::unavailable(&view, creation.as_ref(), now_ms);
    let title = creation
        .as_ref()
        .map_or_else(String::new, |value| value.draft.title.clone());
    let host = creation.as_ref().map(|value| value.draft.host_id.clone());
    rsx! {
        section { class: "idle-stack", aria_label: "Create a session",
            TextField { id: "idle-session-title", label: "Session title", value: title,
                oninput: move |text| onedit.call(SessionFormEdit::Title(text)) }
            if let Some(host) = host { p { class: "idle-session-caption", "Compute host: {host}" } }
            if let Some(reason) = reason { p { role: "status", "{reason}" } }
            Button { label: "Create session", state: if reason.is_none() { ControlState::Ready } else { ControlState::Disabled },
                onpress: move |()| {
                    if reason.is_none() && let Some(creation) = &creation {
                        onaction.call(sessions::Event::Create { id: creation.mutation.clone(), draft: creation.draft.clone() });
                    }
                } }
        }
    }
}
