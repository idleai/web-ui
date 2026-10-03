//! Shared sessions over app-core views and events, for browser and extension hosts.
//!
//! Hosts retain drafts, reserve and persist mutation identities, and execute effects.
//! Prompt position, coordination receipt and recorded output never establish runtime
//! order or completion. Load [`STYLESHEET`] with the theme and history details styles.

mod composer;
mod conversation;
mod feedback;
mod sharing;

pub use composer::{PromptComposer, PromptComposerProps, PromptDraft, SessionActionToken};
pub use conversation::{
    ParticipantAttribution, ParticipantAttributionProps, SessionConversation,
    SessionConversationProps,
};
pub use feedback::{SessionFeedback, SessionFeedbackProps};
pub use sharing::{InvitationAccess, InvitationDraft, SessionSharing, SessionSharingProps};

use app_core::sessions::{SessionLoadState, SessionPermission, SessionView, ViewModel};

/// Shared session styles; hosts load this alongside the theme and details styles.
pub const STYLESHEET: &str = include_str!("../assets/sessions.css");

fn selected(view: &ViewModel) -> Option<&SessionView> {
    view.sessions
        .iter()
        .find(|session| Some(&session.session.id) == view.selected.as_ref())
}

pub(crate) fn ready(view: &ViewModel) -> bool {
    view.context.is_some()
        && view.load == SessionLoadState::Ready
        && !matches!(view.updates, SessionLoadState::Failed(_))
}

fn permitted(view: &ViewModel, permission: SessionPermission) -> bool {
    ready(view) && selected(view).is_some_and(|session| session.actions.contains(&permission))
}

#[cfg(test)]
mod tests;
