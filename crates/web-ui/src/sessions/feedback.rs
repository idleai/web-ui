//! Delivery, connection and safe recovery feedback from app-core.

use app_core::sessions::{
    Event as SessionEvent, SessionAcknowledgement, SessionLoadState, SessionMutationState,
    SessionRetryAdvice, ViewModel,
};
use dioxus::prelude::*;

use crate::controls::{Button, ControlState};
use crate::status::{LoadingState, StatusBadge, StatusTone};

/// Connection and validation feedback, separate from any runtime execution state.
#[component]
pub fn SessionFeedback(view: ViewModel, onaction: EventHandler<SessionEvent>) -> Element {
    let loading = view.load == SessionLoadState::Loading;
    rsx! {
        div { class: "idle-session-feedback",
            {match &view.load {
                SessionLoadState::Idle => rsx! { p { "Connect to load sessions." } },
                SessionLoadState::Loading => rsx! { LoadingState { label: "Loading sessions…" } },
                SessionLoadState::Ready => rsx! {},
                SessionLoadState::Failed(error) => rsx! { p { role: "alert", "Sessions could not refresh: {error.message}. Retained details may be out of date." } },
            }}
            if let SessionLoadState::Failed(error) = &view.updates {
                p { role: "alert", "Session updates disconnected: {error.message}. Retained details may be out of date." }
            }
            if let Some(error) = &view.action_error { p { role: "alert", "{error.message}" } }
            if view.context.is_some() {
                Button {
                    label: "Refresh sessions", state: if loading { ControlState::Busy } else { ControlState::Ready },
                    onpress: move |()| onaction.call(SessionEvent::Refresh),
                }
            }
        }
    }
}

pub(super) fn mutation_label(state: &SessionMutationState) -> (&'static str, StatusTone) {
    match state {
        SessionMutationState::Pending => ("Pending delivery", StatusTone::Info),
        SessionMutationState::Acknowledged(SessionAcknowledgement::Received(_)) => {
            ("Received by coordination", StatusTone::Info)
        }
        SessionMutationState::Acknowledged(SessionAcknowledgement::Committed { .. }) => {
            ("Sharing change committed", StatusTone::Success)
        }
        SessionMutationState::Created(_) => ("Session created", StatusTone::Success),
        SessionMutationState::RuntimeReported => ("Runtime reported", StatusTone::Info),
        SessionMutationState::Failed(_) => ("Delivery failed", StatusTone::Danger),
        SessionMutationState::Unknown => ("Delivery outcome unknown", StatusTone::Warning),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RecoveryAccess {
    pub retry: bool,
    pub lookup: bool,
}

#[component]
pub(super) fn MutationFeedback(
    request_id: String,
    state: SessionMutationState,
    access: RecoveryAccess,
    now_ms: Option<u64>,
    onaction: EventHandler<SessionEvent>,
) -> Element {
    let (label, tone) = mutation_label(&state);
    let recovery = match &state {
        SessionMutationState::Unknown => Some((
            "Check delivery",
            SessionEvent::Recover(request_id),
            access.lookup,
        )),
        SessionMutationState::Failed(error) => match error.retry {
            SessionRetryAdvice::Never => None,
            SessionRetryAdvice::QueryStatus => Some((
                "Check delivery",
                SessionEvent::Recover(request_id),
                access.lookup,
            )),
            SessionRetryAdvice::SameRequest { not_before_ms } => not_before_ms
                .is_none_or(|deadline| now_ms.is_some_and(|now| now >= deadline))
                .then_some((
                    "Retry delivery",
                    SessionEvent::Retry(request_id),
                    access.retry,
                )),
        },
        SessionMutationState::Pending
        | SessionMutationState::Acknowledged(_)
        | SessionMutationState::Created(_)
        | SessionMutationState::RuntimeReported => None,
    };
    rsx! {
        div { class: "idle-session-delivery",
            StatusBadge { label, tone }
            if let SessionMutationState::Failed(error) = &state {
                p { "{error.message}" }
                if let SessionRetryAdvice::SameRequest { not_before_ms: Some(deadline) } = error.retry {
                    if now_ms.is_none_or(|now| now < deadline) {
                        p { class: "idle-session-caption", "Retry is waiting for the provider's deadline." }
                    }
                }
            }
            if let Some((label, action, enabled)) = recovery {
                Button {
                    label, state: if enabled { ControlState::Ready } else { ControlState::Disabled },
                    onpress: move |()| { if enabled { onaction.call(action.clone()); } },
                }
            }
        }
    }
}
