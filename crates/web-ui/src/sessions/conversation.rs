//! Attributed prompts and session-bound recorded conversation/tool output.

use app_core::history::{self, ActivityKind, Event as HistoryEvent, ItemView, RequestState};
use app_core::sessions::{
    Event as SessionEvent, SessionCompletion, SessionContributor, SessionInputState, SessionKind,
    SessionPermission, SessionPromptView, SessionRelationship, ViewModel,
};
use dioxus::prelude::*;

use super::feedback::{MutationFeedback, RecoveryAccess};
use crate::controls::{Button, ControlState};
use crate::history::details::{HistoryDetails, HistoryRow};
use crate::host::HostCapabilities;
use crate::status::{EmptyState, LoadingState, StatusBadge, StatusTone};

/// Actual submitting identity, with ownership shown only when it matches.
///
/// Verified authentication details remain available without substituting an owner,
/// host label or recording integration for the contributor.
#[component]
pub fn ParticipantAttribution(
    contributor: SessionContributor,
    owner: Option<String>,
    current: Option<String>,
) -> Element {
    rsx! {
        div { class: "idle-session-attribution", "data-contributor": contributor.contributor_id.clone(),
            strong { "{contributor.contributor_id}" }
            if current.as_ref() == Some(&contributor.contributor_id) { span { "You" } }
            if owner.as_ref() == Some(&contributor.contributor_id) { span { "Session owner" } }
            details {
                summary { "Contributor details" }
                p { "Identity source: {contributor.issuer}" }
                p { "Subject: {contributor.subject}" }
            }
        }
    }
}

/// Prompts and recorded output for app-core's selected session.
///
/// Prompts preserve app-core vector order; only a supplied runtime delivery fact
/// produces an order label. Output uses the existing history rows and inspector,
/// with exact bytes, disclosure and actions. Supply a history view filtered by
/// `selected_history.chain` and `selected_history.item`; stale bindings are hidden.
/// Prompt requests and history items stay separate because app-core does not supply
/// a prompt-to-turn mapping. No chronology is invented to interleave them.
#[component]
pub fn SessionConversation(
    id: String,
    view: ViewModel,
    history: Option<history::ViewModel>,
    capabilities: HostCapabilities,
    onaction: EventHandler<SessionEvent>,
    onhistoryaction: EventHandler<HistoryEvent>,
    now_ms: Option<u64>,
) -> Element {
    let selected = super::selected(&view);
    let current = view
        .context
        .as_ref()
        .map(|context| context.contributor_id.clone());
    let prompts: Vec<_> = view
        .prompts
        .iter()
        .filter(|prompt| Some(&prompt.input.session_id) == view.selected.as_ref())
        .collect();
    let scoped_history = selected.and_then(|session| {
        let binding = view.selected_history.as_ref()?;
        history.as_ref().filter(|history| {
            super::permitted(&view, SessionPermission::Observe)
                && *binding == session.session.history
                && history.chain.as_ref() == Some(&binding.chain)
                && history.filter.session.as_ref() == Some(&binding.item)
        })
    });
    rsx! {
        section { id: id.clone(), class: "idle-session-conversation", aria_label: "Session conversation",
            if let Some(session) = selected {
                header { class: "idle-session-heading",
                    h2 { "{session.session.title}" }
                    div { class: "idle-session-actions",
                        StatusBadge { label: match session.relationship { SessionRelationship::Owned => "Owned session", SessionRelationship::Invited => "Shared with you" } }
                        if session.session.kind == SessionKind::Control { StatusBadge { label: "Control session" } }
                    }
                    p { class: "idle-session-caption", "Owner: {session.session.owner}" }
                }
                section { aria_label: "Prompt delivery and execution",
                    h3 { "Prompts" }
                    if prompts.is_empty() { p { "No prompts available for this session." } }
                    ul { class: "idle-session-prompts",
                        for prompt in prompts {
                            PromptCard {
                                key: "{prompt.input:?}", prompt: prompt.clone(), owner: session.session.owner.clone(),
                                current: current.clone(),
                                access: PromptAccess {
                                    observe: super::permitted(&view, SessionPermission::Observe),
                                    submit: super::permitted(&view, SessionPermission::SubmitInput),
                                },
                                now_ms, onaction,
                            }
                        }
                    }
                }
                section { aria_label: "Conversation and tool output",
                    h3 { "Conversation and tool output" }
                    if let Some(history) = scoped_history {
                        RecordedConversation {
                            id: "{id}-history", view: history.clone(), capabilities, onaction: onhistoryaction,
                        }
                    } else {
                        p { class: "idle-session-caption", "Conversation history is unavailable for the selected session." }
                    }
                }
            } else {
                EmptyState { title: "No session selected", message: "Select an owned or invited session to follow its conversation." }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PromptAccess {
    observe: bool,
    submit: bool,
}

#[component]
fn PromptCard(
    prompt: SessionPromptView,
    owner: String,
    current: Option<String>,
    access: PromptAccess,
    now_ms: Option<u64>,
    onaction: EventHandler<SessionEvent>,
) -> Element {
    let own = current.as_ref() == Some(&prompt.contributor.contributor_id);
    let runtime = prompt.runtime.as_ref();
    let order = runtime
        .and_then(|update| update.state.delivery())
        .map(|delivery| delivery.order);
    let refresh_input = prompt.input.clone();
    rsx! {
        li {
            class: "idle-session-prompt", "data-request-id": prompt.input.request.request_id.clone(),
            "data-contributor-id": prompt.contributor.contributor_id.clone(),
            "data-runtime-order": order.map(|order| order.to_string()),
            ParticipantAttribution { contributor: prompt.contributor.clone(), owner, current }
            if let Some(text) = &prompt.text {
                pre { class: "idle-session-prompt-text", tabindex: "0", role: "region", aria_label: "Prompt text", "{text}" }
            } else {
                p { class: "idle-session-caption", "Prompt text is unavailable in the recovered session data." }
            }
            div { class: "idle-session-prompt-status",
                if let Some(submission) = &prompt.submission {
                    MutationFeedback {
                        request_id: prompt.input.request.request_id.clone(), state: submission.clone(),
                        access: RecoveryAccess {
                            retry: own && access.submit && runtime.is_none(),
                            lookup: own && (access.observe || access.submit),
                        }, now_ms, onaction,
                    }
                }
                if let Some(update) = runtime {
                    RuntimeStatus { state: update.state.clone() }
                } else {
                    StatusBadge { label: "Awaiting runtime confirmation" }
                }
                if let Some(order) = order {
                    span { class: "idle-session-order", "Runtime order #{order}" }
                }
            }
            details { class: "idle-session-caption",
                summary { "Delivery details" }
                p { "Request: {prompt.input.request.request_id}" }
                if let Some(update) = runtime {
                    p { "Runtime: {update.runtime_id}" }
                    p { "Runtime revision: {update.revision}" }
                }
            }
            Button {
                label: "Refresh execution status",
                state: if access.observe { ControlState::Ready } else { ControlState::Disabled },
                onpress: move |()| { if access.observe { onaction.call(SessionEvent::RefreshInput(refresh_input.clone())); } },
            }
        }
    }
}

fn runtime_label(state: &SessionInputState) -> (&'static str, StatusTone) {
    match state {
        SessionInputState::Rejected { .. } => ("Rejected by runtime", StatusTone::Danger),
        SessionInputState::Accepted { .. } => ("Accepted · awaiting order", StatusTone::Info),
        SessionInputState::Ordered(_) => ("Ordered · awaiting execution", StatusTone::Info),
        SessionInputState::Running { .. } => ("Running", StatusTone::Info),
        SessionInputState::Completed { outcome, .. } => match outcome {
            SessionCompletion::Succeeded => ("Completed", StatusTone::Success),
            SessionCompletion::Failed(_) => ("Execution failed", StatusTone::Danger),
            SessionCompletion::Cancelled => ("Cancelled", StatusTone::Warning),
        },
    }
}

#[component]
fn RuntimeStatus(state: SessionInputState) -> Element {
    let (label, tone) = runtime_label(&state);
    let error = match &state {
        SessionInputState::Rejected { error, .. }
        | SessionInputState::Completed {
            outcome: SessionCompletion::Failed(error),
            ..
        } => Some(error),
        SessionInputState::Accepted { .. }
        | SessionInputState::Ordered(_)
        | SessionInputState::Running { .. }
        | SessionInputState::Completed {
            outcome: SessionCompletion::Succeeded | SessionCompletion::Cancelled,
            ..
        } => None,
    };
    rsx! {
        div { class: "idle-session-runtime",
            StatusBadge { label, tone }
            if let Some(error) = error { p { "{error.message}" } }
        }
    }
}

fn belongs_to(item: &ItemView, session: Option<&String>) -> bool {
    let Some(session) = session else {
        return false;
    };
    // Full-item scans can add observations without repeated session context.
    // Require a recorded binding and reject explicit item/session conflicts.
    item.observations.iter().all(|record| {
        record.item == item.key
            && record
                .session
                .as_ref()
                .is_none_or(|recorded| recorded == session)
    }) && item.observations.iter().any(|record| {
        record.session.as_ref() == Some(session)
            && matches!(record.kind, ActivityKind::Message | ActivityKind::Tool)
    })
}

#[component]
fn RecordedConversation(
    id: String,
    view: history::ViewModel,
    capabilities: HostCapabilities,
    onaction: EventHandler<HistoryEvent>,
) -> Element {
    let items: Vec<_> = view
        .items
        .iter()
        .filter(|item| belongs_to(item, view.filter.session.as_ref()))
        .collect();
    let inspecting = view
        .selected_item
        .as_ref()
        .or_else(|| {
            view.items
                .iter()
                .find(|item| Some(&item.key) == view.selected.item.as_ref())
        })
        .is_some_and(|item| {
            Some(&item.key) == view.selected.item.as_ref()
                && belongs_to(item, view.filter.session.as_ref())
        });
    let busy = view.paging.state == RequestState::Loading;
    rsx! {
        div { class: "idle-session-records",
            if view.reconciliation == RequestState::Loading { LoadingState { label: "Refreshing conversation…" } }
            if let RequestState::Failed(error) = &view.reconciliation {
                p { role: "alert", "Conversation refresh failed: {error.message}. Retained output may be out of date." }
                Button { label: "Refresh conversation", onpress: move |()| onaction.call(HistoryEvent::Refresh) }
            }
            if items.is_empty() { p { "No conversation output loaded." } }
            for item in items {
                article { key: "{item.key}", class: "idle-session-output", "data-output-item": item.key.clone(),
                    div { class: "idle-session-caption",
                        for author in authors(item) { span { "Author: {author} " } }
                    }
                    HistoryRow { id: "{id}-item-{item.key}", item: item.clone(), onaction }
                }
            }
            if let RequestState::Failed(error) = &view.paging.state { p { role: "alert", "{error.message}" } }
            if !view.paging.exhausted {
                p { class: "idle-session-caption", "Conversation scan incomplete; more recorded output may be available." }
                Button {
                    label: "Load more conversation", state: if busy { ControlState::Busy } else { ControlState::Ready },
                    onpress: move |()| onaction.call(HistoryEvent::LoadMore),
                }
            }
            if inspecting {
                HistoryDetails { id: "{id}-details", view, capabilities, onaction }
            }
        }
    }
}

fn authors(item: &ItemView) -> Vec<&str> {
    let mut authors = Vec::new();
    for observation in &item.observations {
        let author = observation.author.as_deref().unwrap_or("Unknown");
        if !authors.contains(&author) {
            authors.push(author);
        }
    }
    authors
}
