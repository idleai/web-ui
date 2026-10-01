//! Explicit session participation grants; compute access remains independent.

use app_core::sessions::{
    Event as SessionEvent, SessionCapability, SessionGrant, SessionGrantStatus, SessionMutation,
    SessionMutationState, SessionPermission, SessionRetryAdvice, ViewModel,
};
use dioxus::prelude::*;

use super::SessionActionToken;
use super::feedback::{MutationFeedback, RecoveryAccess};
use crate::controls::{Button, ButtonVariant, ControlState, Select, SelectOption};
use crate::status::{StatusBadge, StatusTone};

/// Explicit permission set offered by the invitation form.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InvitationAccess {
    /// Read session content.
    #[default]
    Observe,
    /// Read content and submit attributed prompts.
    Contribute,
    /// Read, submit prompts and manage session invitations.
    Manage,
}

impl InvitationAccess {
    fn value(self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Contribute => "contribute",
            Self::Manage => "manage",
        }
    }

    fn permissions(self) -> Vec<SessionPermission> {
        match self {
            Self::Observe => vec![SessionPermission::Observe],
            Self::Contribute => vec![SessionPermission::Observe, SessionPermission::SubmitInput],
            Self::Manage => vec![
                SessionPermission::Observe,
                SessionPermission::SubmitInput,
                SessionPermission::Invite,
            ],
        }
    }
}

/// Controlled invitation, with IDs reserved by the host rather than the component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvitationDraft {
    /// Current session and a fresh mutation identity, also usable for revocation.
    pub token: SessionActionToken,
    /// New non-reusable grant identity reserved by the host.
    pub grant_id: String,
    /// Selected contributor identity from the host's authorized member choices.
    pub grantee: String,
    /// Explicit permission set, defaulting to observation only.
    pub access: InvitationAccess,
    /// Optional provider-clock expiry, set by the host's invitation policy/form.
    pub expires_at_ms: Option<u64>,
}

fn unavailable(view: &ViewModel, draft: Option<&InvitationDraft>) -> Option<&'static str> {
    if super::selected(view).is_none() {
        Some("Select a session to manage sharing.")
    } else if !super::ready(view) {
        Some("Reconnect to manage session sharing.")
    } else if view.capabilities.share != SessionCapability::Available {
        Some("Sharing is unavailable on this connection.")
    } else if !super::permitted(view, SessionPermission::Invite) {
        Some("You do not have permission to manage sharing in this session.")
    } else if draft.is_none_or(|draft| !draft.token.unused(view)) {
        Some("Waiting for a new sharing request for this session.")
    } else {
        None
    }
}

/// Invitation form, ownership and recorded participation grants for the selection.
///
/// `recipients` comes from the host's authorized member view. Grants are rendered
/// as recorded; expiry labels use only the optional provider-aligned `now_ms`.
/// app-core's supplied actions govern controls, including for the session owner.
/// Committed mutations never add/remove grants locally; the next view does that.
#[component]
pub fn SessionSharing(
    id: String,
    view: ViewModel,
    draft: Option<InvitationDraft>,
    recipients: Vec<SelectOption>,
    onchange: EventHandler<InvitationDraft>,
    onaction: EventHandler<SessionEvent>,
    now_ms: Option<u64>,
) -> Element {
    let selected = super::selected(&view);
    let reason = unavailable(&view, draft.as_ref());
    let scoped = draft.as_ref().filter(|draft| draft.token.matches(&view));
    let editable = reason.is_none();
    let can_invite = editable
        && scoped.is_some_and(|draft| {
            !draft.grant_id.trim().is_empty()
                && recipients
                    .iter()
                    .any(|recipient| !recipient.disabled && recipient.value == draft.grantee)
                && selected.is_some_and(|session| {
                    !session
                        .grants
                        .iter()
                        .any(|grant| grant.id == draft.grant_id)
                })
                && draft
                    .expires_at_ms
                    .is_none_or(|expiry| now_ms.is_some_and(|now| expiry > now))
        });
    let mut choices = vec![SelectOption {
        value: String::new(),
        label: "Choose a participant".into(),
        disabled: true,
    }];
    choices.extend(recipients);
    let recipient_draft = scoped.cloned();
    let access_draft = scoped.cloned();
    let invite_draft = scoped.cloned();
    let access_options = [
        (InvitationAccess::Observe, "Read session"),
        (InvitationAccess::Contribute, "Read and send prompts"),
        (
            InvitationAccess::Manage,
            "Read, send prompts and manage sharing",
        ),
    ]
    .into_iter()
    .map(|(access, label)| SelectOption {
        value: access.value().into(),
        label: label.into(),
        disabled: false,
    })
    .collect();
    rsx! {
        section { id: id.clone(), class: "idle-session-sharing", aria_label: "Session sharing",
            h2 { "Participants and sharing" }
            p { class: "idle-session-caption", "Session invitations grant participation. Compute access is managed separately." }
            if let Some(session) = selected {
                p { class: "idle-session-owner", "Session owner: " strong { "{session.session.owner}" } }
                ul { class: "idle-session-grants",
                    for grant in &session.grants {
                        GrantRow {
                            key: "{grant.id}", grant: grant.clone(), now_ms,
                            token: scoped.map(|draft| draft.token.clone()),
                            state: revocation_state(&view, grant, editable),
                            onaction,
                        }
                    }
                }
                if session.grants.is_empty() { p { "No participation grants recorded." } }
            }
            div { class: "idle-stack",
                Select {
                    id: "{id}-recipient", label: "Participant", options: choices,
                    value: scoped.map_or_else(String::new, |draft| draft.grantee.clone()),
                    state: if editable { ControlState::Ready } else { ControlState::Disabled },
                    onchange: move |value| {
                        if editable && let Some(mut draft) = recipient_draft.clone() {
                            draft.grantee = value;
                            onchange.call(draft);
                        }
                    },
                }
                Select {
                    id: "{id}-access", label: "Session access", options: access_options,
                    value: scoped.map_or(InvitationAccess::Observe, |draft| draft.access).value(),
                    state: if editable { ControlState::Ready } else { ControlState::Disabled },
                    onchange: move |value: String| {
                        let access = match value.as_str() {
                            "observe" => Some(InvitationAccess::Observe),
                            "contribute" => Some(InvitationAccess::Contribute),
                            "manage" => Some(InvitationAccess::Manage),
                            _ => None,
                        };
                        if editable && let (Some(mut draft), Some(access)) = (access_draft.clone(), access) {
                            draft.access = access;
                            onchange.call(draft);
                        }
                    },
                }
                p { id: "{id}-hint", class: "idle-session-caption",
                    if let Some(reason) = reason { "{reason}" }
                    else { "Choose a participant and review their access before inviting them." }
                }
                if let Some(expiry) = scoped.and_then(|draft| draft.expires_at_ms) {
                    p { class: "idle-session-caption", "Grant expiry (Unix ms): {expiry}" }
                }
                Button {
                    label: "Invite participant", variant: ButtonVariant::Primary,
                    described_by: "{id}-hint",
                    state: if can_invite { ControlState::Ready } else { ControlState::Disabled },
                    onpress: move |()| {
                        if can_invite && let Some(draft) = &invite_draft {
                            onaction.call(SessionEvent::Invite {
                                id: draft.token.mutation.clone(), grant_id: draft.grant_id.clone(),
                                grantee: draft.grantee.clone(), permissions: draft.access.permissions(),
                                expires_at_ms: draft.expires_at_ms,
                            });
                        }
                    },
                }
            }
            for mutation in view.mutations.iter().filter(|mutation| match &mutation.mutation {
                SessionMutation::Invite { session_id, .. } | SessionMutation::Revoke { session_id, .. } => Some(session_id) == view.selected.as_ref(),
                SessionMutation::Create(_) | SessionMutation::Submit { .. } => false,
            }) {
                div { key: "{mutation.request.mutation.request_id}", class: "idle-session-sharing-result",
                    p { {match &mutation.mutation {
                        SessionMutation::Invite { grantee, .. } => format!("Invitation for {grantee}"),
                        SessionMutation::Revoke { grant_id, .. } => format!("Revoke grant {grant_id}"),
                        SessionMutation::Create(_) | SessionMutation::Submit { .. } => String::new(),
                    }} }
                    MutationFeedback {
                        request_id: mutation.request.mutation.request_id.clone(), state: mutation.state.clone(),
                        access: RecoveryAccess {
                            retry: super::permitted(&view, SessionPermission::Invite),
                            lookup: super::ready(&view) && selected.is_some(),
                        }, now_ms, onaction,
                    }
                }
            }
        }
    }
}

fn revocation_state(view: &ViewModel, grant: &SessionGrant, enabled: bool) -> ControlState {
    if grant.status != SessionGrantStatus::Active {
        return ControlState::Disabled;
    }
    let unresolved = view.mutations.iter().any(|mutation| {
        matches!(
            &mutation.mutation,
            SessionMutation::Revoke { session_id, grant_id, expected_revision }
                if *session_id == grant.session_id && *grant_id == grant.id
                    && *expected_revision == grant.revision
        ) && match &mutation.state {
            SessionMutationState::Pending
            | SessionMutationState::Acknowledged(_)
            | SessionMutationState::Unknown => true,
            SessionMutationState::Failed(error) => error.retry != SessionRetryAdvice::Never,
            SessionMutationState::Created(_) | SessionMutationState::RuntimeReported => false,
        }
    });
    if unresolved {
        ControlState::Busy
    } else if enabled {
        ControlState::Ready
    } else {
        ControlState::Disabled
    }
}

#[component]
fn GrantRow(
    grant: SessionGrant,
    token: Option<SessionActionToken>,
    state: ControlState,
    now_ms: Option<u64>,
    onaction: EventHandler<SessionEvent>,
) -> Element {
    let (label, tone) = match &grant.status {
        SessionGrantStatus::Revoked { .. } => ("Revoked", StatusTone::Warning),
        SessionGrantStatus::Active
            if grant
                .expires_at_ms
                .is_some_and(|expiry| now_ms.is_some_and(|now| now >= expiry)) =>
        {
            ("Expired", StatusTone::Neutral)
        }
        SessionGrantStatus::Active => ("Issued", StatusTone::Info),
    };
    let can_revoke = state == ControlState::Ready && token.is_some();
    let grant_id = grant.id.clone();
    let permissions = grant
        .permissions
        .iter()
        .map(|permission| match permission {
            SessionPermission::Observe => "Read",
            SessionPermission::SubmitInput => "Send prompts",
            SessionPermission::Invite => "Manage sharing",
        })
        .collect::<Vec<_>>()
        .join(" · ");
    rsx! {
        li { class: "idle-session-grant", "data-grant-id": grant.id.clone(),
            div { class: "idle-session-actions", strong { "{grant.grantee}" } StatusBadge { label, tone } }
            p { "{permissions}" }
            details {
                summary { "Grant details" }
                p { "Grant: {grant.id}" }
                p { "Invited by: {grant.granted_by}" }
                p { "Revision: {grant.revision}" }
                if let Some(expiry) = grant.expires_at_ms { p { "Expires (Unix ms): {expiry}" } }
                else { p { "No expiration recorded" } }
                if let SessionGrantStatus::Revoked { at_ms, by } = &grant.status { p { "Revoked by {by} at {at_ms} (Unix ms)" } }
            }
            Button {
                label: "Revoke invitation", variant: ButtonVariant::Danger,
                state,
                onpress: move |()| {
                    if can_revoke && let Some(token) = &token {
                        onaction.call(SessionEvent::Revoke { id: token.mutation.clone(), grant_id: grant_id.clone() });
                    }
                },
            }
        }
    }
}
