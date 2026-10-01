//! Controlled prompt editing with host-reserved, context-bound request identities.

use app_core::sessions::{
    Event as SessionEvent, SessionCapability, SessionContext, SessionMutationId, SessionPermission,
    ViewModel,
};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{
    Callback, Element, EventHandler, Key, KeyboardEvent, Modifiers, ModifiersInteraction, Props,
    WritableExt, component, dioxus_core, dioxus_elements, rsx, use_signal,
};

use crate::controls::{Button, ButtonVariant, ControlState};

/// Host-reserved identity for one new action in one authenticated session context.
///
/// Reserve a fresh ID for each new mutation and persist its exact payload before
/// executing the emitted event's effect. Retries use app-core's retained request.
/// This token has no runtime delivery order. Replace it after an action is stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionActionToken {
    /// Exact connection and contributor for which the draft was prepared.
    pub context: SessionContext,
    /// Directory session for which the draft was prepared.
    pub session_id: String,
    /// Unique mutation ID and provider-aligned first-receipt deadline.
    pub mutation: SessionMutationId,
}

impl SessionActionToken {
    pub(super) fn matches(&self, view: &ViewModel) -> bool {
        view.context.as_ref() == Some(&self.context)
            && view.selected.as_ref() == Some(&self.session_id)
            && super::selected(view).is_some()
    }

    pub(super) fn unused(&self, view: &ViewModel) -> bool {
        self.matches(view)
            && !self.mutation.request_id.trim().is_empty()
            && !view
                .mutations
                .iter()
                .any(|mutation| mutation.request.mutation.request_id == self.mutation.request_id)
    }
}

/// Host-controlled prompt text and its original session/request scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptDraft {
    /// Target and new mutation identity, retained through local edits.
    pub token: SessionActionToken,
    /// Exact text; submission preserves whitespace and line breaks.
    pub text: String,
}

fn unavailable(view: &ViewModel, draft: Option<&PromptDraft>) -> Option<&'static str> {
    if super::selected(view).is_none() {
        return Some("Select a session to write a prompt.");
    }
    if !super::ready(view) {
        return Some("Reconnect to send prompts. Your draft is retained.");
    }
    if view.capabilities.input != SessionCapability::Available {
        return Some("Prompt delivery is unavailable on this connection.");
    }
    if !super::permitted(view, SessionPermission::SubmitInput) {
        return Some("You do not have permission to send prompts in this session.");
    }
    if draft.is_none_or(|draft| !draft.token.matches(view)) {
        return Some("Waiting for a draft for this session.");
    }
    if draft.is_some_and(|draft| !draft.token.unused(view)) {
        return Some("Prompt submitted. Waiting for a new draft.");
    }
    None
}

/// Multiline prompt composer. Plain Enter inserts a newline; Ctrl/Cmd+Enter sends.
///
/// IME composition and key repeats never send. This component neither clears the
/// controlled draft nor creates optimistic prompts. Feed the resulting app-core
/// view back before rotating the token/clearing text, so validation failures retain
/// the draft. A stale session or contributor token cannot submit in a new context.
#[component]
pub fn PromptComposer(
    id: String,
    view: ViewModel,
    draft: Option<PromptDraft>,
    oninput: EventHandler<String>,
    onaction: EventHandler<SessionEvent>,
) -> Element {
    let reason = unavailable(&view, draft.as_ref());
    let scoped = draft.as_ref().filter(|draft| draft.token.matches(&view));
    let text = scoped.map_or_else(String::new, |draft| draft.text.clone());
    let can_send = reason.is_none() && !text.trim().is_empty();
    let editable = scoped.is_some();
    let mut composing = use_signal(|| false);
    let send = Callback::new(move |()| {
        if can_send
            && !composing()
            && let Some(draft) = &draft
        {
            onaction.call(SessionEvent::Submit {
                id: draft.token.mutation.clone(),
                text: draft.text.clone(),
            });
        }
    });
    rsx! {
        section { id: id.clone(), class: "idle-session-composer", aria_label: "Prompt composer",
            label { class: "idle-label", r#for: "{id}-text", "Prompt" }
            textarea {
                id: "{id}-text", class: "idle-input idle-session-draft", rows: "4",
                value: text, disabled: !editable,
                aria_describedby: "{id}-hint", placeholder: "Add a prompt to this session…",
                oninput: move |event| { if editable { oninput.call(event.value()); } },
                oncompositionstart: move |_| composing.set(true),
                oncompositionend: move |_| composing.set(false),
                onkeydown: move |event: KeyboardEvent| {
                    if event.key() == Key::Enter
                        && matches!(event.modifiers(), Modifiers::CONTROL | Modifiers::META)
                        && !composing() && !event.is_composing() && !event.is_auto_repeating() && can_send
                    {
                        event.prevent_default();
                        event.stop_propagation();
                        send.call(());
                    }
                },
            }
            div { class: "idle-session-actions",
                p { id: "{id}-hint", class: "idle-session-caption",
                    if let Some(reason) = reason { "{reason}" }
                    else { "Ctrl or Cmd + Enter to send. Enter adds a line." }
                }
                Button {
                    label: "Send prompt", variant: ButtonVariant::Primary,
                    state: if can_send { ControlState::Ready } else { ControlState::Disabled },
                    described_by: "{id}-hint", onpress: move |()| send.call(()),
                }
            }
        }
    }
}
