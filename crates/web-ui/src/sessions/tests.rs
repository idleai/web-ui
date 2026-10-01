//! Rendering and interaction contracts at the app-core boundary.

use app_core::sessions::{
    Event as SessionEvent, SessionAcknowledgement, SessionCompletion, SessionDelivery,
    SessionError, SessionErrorCode, SessionInputRef, SessionInputState, SessionInputUpdate,
    SessionMutationId, SessionMutationState, SessionPromptView, SessionReceipt, SessionRequestKey,
    SessionResult, SessionRetryAdvice, ViewModel, scripted::demo_snapshot,
};
use app_core::workspace::WorkspaceMode;
use app_core::{Core, Effect};
use dioxus::prelude::*;

use super::{SessionActionToken, SessionConversation};
use crate::host::{HostCapabilities, HostKind};

mod actions;
#[path = "../../examples/sessions/output.rs"]
mod output;

fn connected(mode: WorkspaceMode) -> Core {
    let snapshot = demo_snapshot(mode, "contributor-alice").expect("f24 session fixture");
    let core = Core::new();
    for effect in core.process_event(app_core::Event::Sessions(SessionEvent::Connect(
        snapshot.context.clone(),
    ))) {
        if let Effect::Session(mut request) = effect {
            let _effects = core
                .resolve(
                    &mut *request,
                    Ok(SessionResult::Snapshot(Box::new(snapshot.clone()))),
                )
                .expect("fixture snapshot");
        }
    }
    let _effects = core.process_event(app_core::Event::Sessions(SessionEvent::Select(Some(
        "session-shared".into(),
    ))));
    core
}

fn token(view: &ViewModel) -> SessionActionToken {
    SessionActionToken {
        context: view.context.clone().expect("fixture context"),
        session_id: view.selected.clone().expect("fixture selection"),
        mutation: SessionMutationId {
            request_id: "local-request".into(),
            expires_at_ms: 2000,
        },
    }
}

fn failure() -> SessionError {
    SessionError {
        code: SessionErrorCode::Unavailable,
        message: "Runtime unavailable".into(),
        retry: SessionRetryAdvice::QueryStatus,
    }
}

fn delivery(order: u64) -> SessionDelivery {
    SessionDelivery {
        accepted_at_ms: 1000,
        order,
        ordered_at_ms: 1001,
    }
}

fn remote(view: &ViewModel, state: Option<SessionInputState>) -> SessionPromptView {
    let contributor = demo_snapshot(WorkspaceMode::Standalone, "contributor-bob")
        .expect("second contributor")
        .contributor;
    let input = SessionInputRef {
        session_id: "session-shared".into(),
        request: SessionRequestKey {
            workspace_id: view.context.as_ref().expect("context").workspace_id.clone(),
            contributor_id: contributor.contributor_id.clone(),
            request_id: "bob-request".into(),
        },
    };
    SessionPromptView {
        input: input.clone(),
        contributor: contributor.clone(),
        text: None,
        submission: None,
        runtime: state.map(|state| SessionInputUpdate {
            input,
            contributor,
            runtime_id: "runtime-evo".into(),
            revision: 7,
            state,
        }),
    }
}

#[derive(Clone)]
struct ConversationFixture {
    view: ViewModel,
    kind: HostKind,
    history: Option<app_core::history::ViewModel>,
}

fn conversation(props: ConversationFixture) -> Element {
    rsx! { SessionConversation {
        id: "session", view: props.view, history: props.history, capabilities: HostCapabilities::new(props.kind),
        onaction: move |_| {}, onhistoryaction: move |_| {},
    } }
}

fn render<P: Clone + 'static>(app: fn(P) -> Element, props: P) -> String {
    let mut dom = VirtualDom::new_with_props(app, props);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn all_runtime_states_render_only_supplied_order_and_actual_contributor() {
    let cases = [
        (None, "Awaiting runtime confirmation", false),
        (
            Some(SessionInputState::Accepted { at_ms: 1000 }),
            "Accepted · awaiting order",
            false,
        ),
        (
            Some(SessionInputState::Rejected {
                at_ms: 1000,
                error: failure(),
            }),
            "Rejected by runtime",
            false,
        ),
        (
            Some(SessionInputState::Ordered(delivery(83))),
            "Ordered · awaiting execution",
            true,
        ),
        (
            Some(SessionInputState::Running {
                delivery: delivery(83),
                started_at_ms: 1002,
            }),
            "Running",
            true,
        ),
        (
            Some(SessionInputState::Completed {
                delivery: delivery(83),
                completed_at_ms: 1003,
                outcome: SessionCompletion::Succeeded,
            }),
            "Completed",
            true,
        ),
        (
            Some(SessionInputState::Completed {
                delivery: delivery(83),
                completed_at_ms: 1003,
                outcome: SessionCompletion::Failed(failure()),
            }),
            "Execution failed",
            true,
        ),
        (
            Some(SessionInputState::Completed {
                delivery: delivery(83),
                completed_at_ms: 1003,
                outcome: SessionCompletion::Cancelled,
            }),
            "Cancelled",
            true,
        ),
    ];
    for mode in [WorkspaceMode::Standalone, WorkspaceMode::Managed] {
        for kind in [HostKind::Browser, HostKind::VsCode] {
            for (state, label, ordered) in &cases {
                let mut view = connected(mode).view().sessions;
                view.prompts.push(remote(&view, state.clone()));
                let html = render(
                    conversation,
                    ConversationFixture {
                        view,
                        kind,
                        history: None,
                    },
                );
                assert!(html.contains(label), "supplied runtime state: {label}");
                assert_eq!(
                    html.contains("Runtime order #83"),
                    *ordered,
                    "only runtime delivery carries an order"
                );
                assert!(
                    html.contains("data-contributor=\"contributor-bob\""),
                    "the contributor stays distinct from the owner"
                );
                assert!(
                    !html.contains(">Session owner<"),
                    "Bob's input does not acquire Alice's ownership label"
                );
                assert!(
                    html.contains("Prompt text is unavailable"),
                    "recovery without text stays explicit"
                );
            }
        }
    }
}

#[test]
fn backend_receipt_and_local_vector_position_cannot_supply_runtime_order() {
    let core = connected(WorkspaceMode::Standalone);
    let view = core.view().sessions;
    let _effects = core.process_event(app_core::Event::Sessions(SessionEvent::Submit {
        id: token(&view).mutation,
        text: "  exact\n<script>alert(1)</script>  ".into(),
    }));
    let mut view = core.view().sessions;
    let prompt = view.prompts.first_mut().expect("pending prompt");
    prompt.submission = Some(SessionMutationState::Acknowledged(
        SessionAcknowledgement::Received(SessionReceipt {
            request: prompt.input.request.clone(),
            received_at_ms: 1000,
            retry_until_ms: 2000,
        }),
    ));
    let html = render(
        conversation,
        ConversationFixture {
            view: view.clone(),
            kind: HostKind::Browser,
            history: None,
        },
    );
    assert!(
        html.contains("Received by coordination") && html.contains("Awaiting runtime confirmation"),
        "coordination receipt remains separate"
    );
    assert!(
        !html.contains("Runtime order #") && !html.contains("<script>"),
        "neither receipt nor prompt markup executes"
    );
    assert!(
        html.contains("  exact\n&#60;script&#62;alert(1)&#60;/script&#62;  "),
        "exact prompt whitespace is retained"
    );
    let mut first = remote(&view, Some(SessionInputState::Ordered(delivery(83))));
    first.input.request.request_id = "first-visible".into();
    let mut second = remote(&view, Some(SessionInputState::Ordered(delivery(9))));
    second.input.request.request_id = "second-visible".into();
    view.prompts = vec![first, second];
    let html = render(
        conversation,
        ConversationFixture {
            view,
            kind: HostKind::VsCode,
            history: None,
        },
    );
    assert!(
        html.find("first-visible") < html.find("second-visible"),
        "rendering preserves app-core vector position"
    );
    assert!(
        html.contains("Runtime order #83") && html.contains("Runtime order #9"),
        "orders stay exactly as supplied"
    );
}

#[test]
fn history_uses_explicit_session_binding_and_retains_tool_attempts_and_authors() {
    let view = connected(WorkspaceMode::Standalone).view().sessions;
    let mut history = output::view(view.selected_history.as_ref().expect("history binding"));
    for item in &mut history.items {
        item.expanded = true;
    }
    let html = render(
        conversation,
        ConversationFixture {
            view: view.clone(),
            kind: HostKind::Browser,
            history: Some(history.clone()),
        },
    );
    for text in [
        "MESSAGE_END",
        "Author: agent:runner",
        "runtime-recorder",
        "Stdout",
        "Stderr",
        "Partial content",
        "Content unavailable",
    ] {
        assert!(
            html.contains(text),
            "recorded content remains available: {text}"
        );
    }
    assert_eq!(
        html.matches("data-attempt=").count(),
        3,
        "attempts/channels keep distinct blocks"
    );
    let mut no_observation = view.clone();
    for session in &mut no_observation.sessions {
        session
            .actions
            .retain(|permission| *permission != app_core::sessions::SessionPermission::Observe);
    }
    let html = render(
        conversation,
        ConversationFixture {
            view: no_observation,
            kind: HostKind::Browser,
            history: Some(history.clone()),
        },
    );
    assert!(
        !html.contains("MESSAGE_END"),
        "session participation without observation cannot reveal cached output"
    );
    history.chain = Some("another-chain".into());
    let html = render(
        conversation,
        ConversationFixture {
            view: view.clone(),
            kind: HostKind::Browser,
            history: Some(history.clone()),
        },
    );
    assert!(!html.contains("MESSAGE_END"), "old chain content is hidden");
    history.chain = view
        .selected_history
        .as_ref()
        .map(|binding| binding.chain.clone());
    history.filter.session = Some("another-session".into());
    let html = render(
        conversation,
        ConversationFixture {
            view: view.clone(),
            kind: HostKind::VsCode,
            history: Some(history.clone()),
        },
    );
    assert!(
        !html.contains("MESSAGE_END"),
        "old session content is hidden"
    );
    history.filter.session = view
        .selected_history
        .as_ref()
        .map(|binding| binding.item.clone());
    for item in &mut history.items {
        for observation in &mut item.observations {
            observation.session = Some("another-session".into());
        }
    }
    history.selected_item = history.items.first().cloned();
    history.selected.item = history.selected_item.as_ref().map(|item| item.key.clone());
    let html = render(
        conversation,
        ConversationFixture {
            view,
            kind: HostKind::VsCode,
            history: Some(history),
        },
    );
    assert!(
        !html.contains("MESSAGE_END") && !html.contains("History record details"),
        "cached items and inspector must also match the session"
    );
}
