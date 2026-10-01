//! An unresolved revocation retains its request identity after token rotation.

use std::rc::Rc;

use app_core::sessions::{SessionAcknowledgement, SessionGrantStatus, SessionReceipt};
use dioxus::prelude::dioxus_core::AttributeValue;

use super::{
    ElementId, InvitationAccess, InvitationDraft, Mutation, Mutations, SessionEvent,
    SessionMutationState, SessionRetryAdvice, SharingFixture, WorkspaceMode, click, connected,
    failure, mount, sharing, token,
};

fn buttons(mutations: &Mutations, variant: &str) -> Vec<ElementId> {
    mutations
        .edits
        .iter()
        .filter_map(|mutation| {
            if let Mutation::SetAttribute {
                name: "data-variant",
                value: AttributeValue::Text(value),
                id,
                ..
            } = mutation
            {
                (value == variant).then_some(*id)
            } else {
                None
            }
        })
        .collect()
}

fn pending(mode: WorkspaceMode) -> SharingFixture {
    let core = connected(mode);
    let id = token(&core.view().sessions).mutation;
    let _effects = core.process_event(app_core::Event::Sessions(SessionEvent::Revoke {
        id,
        grant_id: "grant-session-alice".into(),
    }));
    let mut view = core.view().sessions;
    for session in &mut view.sessions {
        session
            .grants
            .retain(|grant| grant.id == "grant-session-alice");
    }
    let mut next = token(&view);
    next.mutation.request_id = "next-sharing-request".into();
    SharingFixture {
        draft: InvitationDraft {
            token: next,
            grant_id: "new-grant".into(),
            grantee: "contributor-bob".into(),
            access: InvitationAccess::Observe,
            expires_at_ms: None,
        },
        view,
        actions: Rc::default(),
    }
}

#[test]
fn pending_revoke_blocks_rotated_tokens_without_blocking_other_grants() {
    for mode in [WorkspaceMode::Standalone, WorkspaceMode::Managed] {
        let mut fixture = pending(mode);
        let selected = fixture.view.selected.clone();
        let session = fixture
            .view
            .sessions
            .iter_mut()
            .find(|session| Some(&session.session.id) == selected.as_ref())
            .expect("selected session");
        let mut other = session.grants.first().expect("grant").clone();
        other.id = "other-grant".into();
        session.grants.push(other);
        let (dom, mutations) = mount(sharing, fixture.clone());
        assert!(
            dioxus_ssr::render(&dom).contains("aria-busy=\"true\""),
            "pending revocation has a busy control"
        );
        let revokes = buttons(&mutations, "danger");
        assert_eq!(revokes.len(), 2, "both grant controls remain mounted");
        for target in revokes {
            click(&dom, target);
        }
        assert!(
            matches!(fixture.actions.borrow().as_slice(), [SessionEvent::Revoke { id, grant_id }] if id.request_id == "next-sharing-request" && grant_id == "other-grant"),
            "only the other grant remains revocable after rotating the token"
        );
    }
}

#[test]
fn uncertain_retryable_and_committed_revocations_keep_the_original_request() {
    let initial = pending(WorkspaceMode::Standalone);
    let request = initial
        .view
        .mutations
        .first()
        .expect("revoke")
        .request
        .clone();
    let mut retryable = failure();
    retryable.retry = SessionRetryAdvice::SameRequest {
        not_before_ms: None,
    };
    let cursor =
        app_core::sessions::scripted::demo_snapshot(WorkspaceMode::Standalone, "contributor-alice")
            .expect("snapshot")
            .cursor;
    let cases = [
        (SessionMutationState::Unknown, Some("recover")),
        (SessionMutationState::Failed(failure()), Some("recover")),
        (SessionMutationState::Failed(retryable), Some("retry")),
        (
            SessionMutationState::Acknowledged(SessionAcknowledgement::Committed {
                receipt: SessionReceipt {
                    request: request.key(),
                    received_at_ms: 1000,
                    retry_until_ms: 3000,
                },
                committed_at_ms: 1000,
                through: cursor,
            }),
            None,
        ),
    ];
    for (state, recovery) in cases {
        let mut fixture = pending(WorkspaceMode::Standalone);
        fixture.view.mutations.first_mut().expect("revoke").state = state;
        let (dom, mutations) = mount(sharing, fixture.clone());
        click(
            &dom,
            *buttons(&mutations, "danger").first().expect("revoke"),
        );
        assert!(
            fixture.actions.borrow().is_empty(),
            "unsettled revoke stays blocked"
        );
        if let Some(recovery) = recovery {
            click(
                &dom,
                *buttons(&mutations, "secondary").first().expect("recovery"),
            );
            let actions = fixture.actions.borrow();
            let event = actions.first().expect("recovery action");
            assert!(
                matches!(event, SessionEvent::Recover(id) if recovery == "recover" && id == "local-request")
                    || matches!(event, SessionEvent::Retry(id) if recovery == "retry" && id == "local-request"),
                "recovery keeps the original identity"
            );
        }
    }
}

#[test]
fn terminal_failure_or_new_grant_revision_allows_a_new_revoke() {
    for case in 0..3 {
        let mut fixture = pending(WorkspaceMode::Standalone);
        if case == 0 {
            let mut terminal = failure();
            terminal.retry = SessionRetryAdvice::Never;
            fixture.view.mutations.first_mut().expect("revoke").state =
                SessionMutationState::Failed(terminal);
        } else {
            for session in &mut fixture.view.sessions {
                for grant in &mut session.grants {
                    grant.revision = grant.revision.saturating_add(1);
                    if case == 2 {
                        grant.status = SessionGrantStatus::Revoked {
                            at_ms: 1000,
                            by: "contributor-alice".into(),
                        };
                    }
                }
            }
        }
        let (dom, mutations) = mount(sharing, fixture.clone());
        click(
            &dom,
            *buttons(&mutations, "danger").first().expect("revoke"),
        );
        if case == 2 {
            assert!(
                fixture.actions.borrow().is_empty(),
                "revoked grant stays disabled"
            );
        } else {
            assert!(
                matches!(fixture.actions.borrow().as_slice(), [SessionEvent::Revoke { id, .. }] if id.request_id == "next-sharing-request"),
                "settled failure or newer active revision permits a new intent"
            );
        }
    }
}
