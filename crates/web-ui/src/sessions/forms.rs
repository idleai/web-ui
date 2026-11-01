//! Host-owned drafts and edits for the live session screen.

use super::{InvitationDraft, PromptDraft};
use crate::{controls::SelectOption, navigation::SessionCreation};

/// Saved forms bound to the current workspace and authenticated contributor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionForms {
    /// Keep the recorded history view open after clearing its session filter.
    pub show_recorded_history: bool,
    /// Prepared creation on the connected compute host.
    pub creation: Option<SessionCreation>,
    /// Exact prompt and request identity for the selected session.
    pub prompt: Option<PromptDraft>,
    /// Prepared sharing choices for the selected session.
    pub invitation: Option<InvitationDraft>,
    /// Active contributors eligible to receive a session invitation.
    pub recipients: Vec<SelectOption>,
    /// Compute setup problem that prevents new work.
    pub readiness: Option<String>,
}

/// A local edit to persist before submitting a session action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionFormEdit {
    /// Title for the next session.
    Title(String),
    /// Text for the selected session's prompt.
    Prompt(String),
    /// Recipient and permissions for the selected session.
    Invitation(Box<InvitationDraft>),
}
