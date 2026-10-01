//! Native events preserve exact drafts, scopes and safe recovery identities.

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use app_core::sessions::{SessionCapability, SessionLoadState, SessionPermission};
use dioxus::prelude::*;
use dioxus_core::{ElementId, Mutation, Mutations};
use dioxus_html::SerializedHtmlEventConverter;

use super::{
    SessionEvent, SessionMutationState, SessionRetryAdvice, ViewModel, WorkspaceMode, connected,
    failure, render, token,
};
use crate::controls::SelectOption;
use crate::sessions::feedback::{MutationFeedback, RecoveryAccess};
use crate::sessions::{
    InvitationAccess, InvitationDraft, PromptComposer, PromptDraft, SessionSharing,
};

mod revocation;

fn mount<P: Clone + 'static>(app: fn(P) -> Element, props: P) -> (VirtualDom, Mutations) {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new_with_props(app, props);
    let mutations = dom.rebuild_to_vec();
    (dom, mutations)
}

fn listener(mutations: &Mutations, event: &str, index: usize) -> ElementId {
    mutations
        .edits
        .iter()
        .filter_map(|mutation| {
            if let Mutation::NewEventListener { name, id } = mutation {
                (name == event).then_some(*id)
            } else {
                None
            }
        })
        .nth(index)
        .expect("fixture listener")
}

fn dispatch(dom: &VirtualDom, target: ElementId, name: &str, data: impl Any) -> Event<dyn Any> {
    let data: Rc<dyn Any> = Rc::new(PlatformEventData::new(Box::new(data)));
    let event = Event::new(data, true);
    dom.runtime().handle_event(name, event.clone(), target);
    event
}

fn click(dom: &VirtualDom, target: ElementId) {
    let _event = dispatch(dom, target, "click", SerializedMouseData::default());
}

#[derive(Clone)]
struct ComposerFixture {
    view: Rc<RefCell<ViewModel>>,
    draft: PromptDraft,
    actions: Rc<RefCell<Vec<SessionEvent>>>,
}

fn composer(props: ComposerFixture) -> Element {
    rsx! { PromptComposer { id: "composer", view: props.view.borrow().clone(), draft: props.draft,
        oninput: move |_| {}, onaction: move |event| props.actions.borrow_mut().push(event),
    } }
}

fn composer_fixture() -> ComposerFixture {
    let view = connected(WorkspaceMode::Standalone).view().sessions;
    ComposerFixture {
        draft: PromptDraft {
            token: token(&view),
            text: "  keep\n🙂  ".into(),
        },
        view: Rc::new(RefCell::new(view)),
        actions: Rc::default(),
    }
}

#[test]
fn composer_handles_only_send_shortcuts_and_never_clears_controlled_text() {
    let fixture = composer_fixture();
    let (dom, mutations) = mount(composer, fixture.clone());
    let target = listener(&mutations, "keydown", 0);
    for (modifiers, repeating, composing) in [
        (Modifiers::empty(), false, false),
        (Modifiers::SHIFT, false, false),
        (Modifiers::CONTROL, false, true),
        (Modifiers::META, false, true),
        (Modifiers::CONTROL, true, false),
        (Modifiers::CONTROL | Modifiers::SHIFT, false, false),
    ] {
        let event = dispatch(
            &dom,
            target,
            "keydown",
            SerializedKeyboardData::new(
                Key::Enter,
                Code::Enter,
                Location::Standard,
                repeating,
                modifiers,
                composing,
            ),
        );
        assert!(
            event.default_action_enabled() && event.propagates(),
            "composition, repeated and unrelated keys pass through"
        );
    }
    assert!(
        fixture.actions.borrow().is_empty(),
        "editing and composition never submit"
    );
    for modifiers in [Modifiers::CONTROL, Modifiers::META] {
        let event = dispatch(
            &dom,
            target,
            "keydown",
            SerializedKeyboardData::new(
                Key::Enter,
                Code::Enter,
                Location::Standard,
                false,
                modifiers,
                false,
            ),
        );
        assert!(
            !event.default_action_enabled() && !event.propagates(),
            "send shortcut is handled once"
        );
    }
    assert_eq!(
        fixture.actions.borrow().len(),
        2,
        "both platform shortcuts dispatch"
    );
    for event in fixture.actions.borrow().iter() {
        assert!(
            matches!(event, SessionEvent::Submit { id, text } if *id == fixture.draft.token.mutation && *text == fixture.draft.text),
            "submission preserves the reserved identity and exact text"
        );
    }
    assert!(
        dioxus_ssr::render(&dom).contains("  keep\n🙂  "),
        "the host retains the draft until app-core stores it"
    );
}

#[test]
fn changed_context_permissions_and_adapter_availability_block_synthetic_sends() {
    for case in 0..9 {
        let mut fixture = composer_fixture();
        match case {
            0 => fixture.view.borrow_mut().selected = None,
            1 => fixture.draft.token.context.contributor_id = "other-user".into(),
            2 => fixture.draft.token.context.provider = "other-provider".into(),
            3 => fixture.draft.token.session_id = "other-session".into(),
            4 => fixture.view.borrow_mut().capabilities.input = SessionCapability::Unavailable,
            5 => fixture
                .view
                .borrow_mut()
                .sessions
                .iter_mut()
                .for_each(|session| session.actions.clear()),
            6 => fixture.view.borrow_mut().load = SessionLoadState::Loading,
            7 => fixture.view.borrow_mut().updates = SessionLoadState::Failed(failure()),
            _ => fixture.draft.text = " \n\t ".into(),
        }
        let (dom, mutations) = mount(composer, fixture.clone());
        click(&dom, listener(&mutations, "click", 0));
        assert!(
            fixture.actions.borrow().is_empty(),
            "case {case} cannot send even a synthetic click"
        );
        if case <= 3 {
            assert!(
                !dioxus_ssr::render(&dom).contains("  keep\n🙂  "),
                "a stale draft is hidden in another context"
            );
        }
    }
}

#[test]
fn capability_revocation_takes_effect_on_the_same_mounted_composer() {
    let fixture = composer_fixture();
    let (mut dom, mutations) = mount(composer, fixture.clone());
    let button = listener(&mutations, "click", 0);
    click(&dom, button);
    fixture.view.borrow_mut().capabilities.input = SessionCapability::Unavailable;
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    click(&dom, button);
    assert_eq!(
        fixture.actions.borrow().len(),
        1,
        "revoking the adapter stops dispatch without remounting"
    );
    let html = dioxus_ssr::render(&dom);
    assert!(
        html.contains("Prompt delivery is unavailable")
            && html.contains("aria-describedby=\"composer-hint\""),
        "disabled send has an associated reason"
    );
}

#[test]
fn mutation_already_stored_in_app_core_cannot_be_submitted_again() {
    let core = connected(WorkspaceMode::Standalone);
    let view = core.view().sessions;
    let draft = PromptDraft {
        token: token(&view),
        text: "Original".into(),
    };
    let _effects = core.process_event(app_core::Event::Sessions(SessionEvent::Submit {
        id: draft.token.mutation.clone(),
        text: draft.text.clone(),
    }));
    let fixture = ComposerFixture {
        view: Rc::new(RefCell::new(core.view().sessions)),
        draft,
        actions: Rc::default(),
    };
    let (dom, mutations) = mount(composer, fixture.clone());
    click(&dom, listener(&mutations, "click", 0));
    assert!(
        fixture.actions.borrow().is_empty(),
        "retained mutation blocks another dispatch until host reserves a fresh ID"
    );
}

#[derive(Clone)]
struct SharingFixture {
    view: ViewModel,
    draft: InvitationDraft,
    actions: Rc<RefCell<Vec<SessionEvent>>>,
}

fn sharing(props: SharingFixture) -> Element {
    rsx! { SessionSharing { id: "sharing", view: props.view, draft: props.draft, now_ms: 1000,
        recipients: vec![SelectOption { value: "contributor-bob".into(), label: "Bob".into(), disabled: false }],
        onchange: move |_| {}, onaction: move |event| props.actions.borrow_mut().push(event),
    } }
}

fn sharing_fixture() -> SharingFixture {
    let mut view = connected(WorkspaceMode::Managed).view().sessions;
    for session in &mut view.sessions {
        session.grants.clear();
    }
    SharingFixture {
        draft: InvitationDraft {
            token: token(&view),
            grant_id: "new-grant".into(),
            grantee: "contributor-bob".into(),
            access: InvitationAccess::default(),
            expires_at_ms: Some(1800),
        },
        view,
        actions: Rc::default(),
    }
}

#[test]
fn invitations_are_explicit_session_permissions_and_wait_for_the_returned_grant() {
    let fixture = sharing_fixture();
    let (dom, mutations) = mount(sharing, fixture.clone());
    click(&dom, listener(&mutations, "click", 0));
    assert!(
        matches!(fixture.actions.borrow().as_slice(), [SessionEvent::Invite { id, grant_id, grantee, permissions, expires_at_ms }] if *id == fixture.draft.token.mutation && grant_id == "new-grant" && grantee == "contributor-bob" && *permissions == vec![SessionPermission::Observe] && *expires_at_ms == Some(1800)),
        "default invitation grants observation only and preserves caller identities"
    );
    assert!(
        dioxus_ssr::render(&dom).contains("No participation grants recorded"),
        "clicking invite cannot create an optimistic grant"
    );
    for case in 0..3 {
        let mut fixture = sharing_fixture();
        match case {
            0 => fixture.view.capabilities.share = SessionCapability::Unavailable,
            1 => fixture.draft.token.context.workspace_id = "other-workspace".into(),
            _ => fixture.draft.grantee = "not-in-member-list".into(),
        }
        let (dom, mutations) = mount(sharing, fixture.clone());
        click(&dom, listener(&mutations, "click", 0));
        assert!(
            fixture.actions.borrow().is_empty(),
            "unavailable/stale/unknown recipient case {case} cannot invite"
        );
    }
}

#[derive(Clone)]
struct RecoveryFixture {
    state: SessionMutationState,
    now: Option<u64>,
    actions: Rc<RefCell<Vec<SessionEvent>>>,
}

fn recovery(props: RecoveryFixture) -> Element {
    rsx! { MutationFeedback { request_id: "original-request", state: props.state, access: RecoveryAccess { retry: true, lookup: true }, now_ms: props.now, onaction: move |event| props.actions.borrow_mut().push(event) } }
}

#[test]
fn uncertain_delivery_requires_lookup_and_delayed_retries_use_provider_time() {
    let fixture = RecoveryFixture {
        state: SessionMutationState::Unknown,
        now: None,
        actions: Rc::default(),
    };
    let (dom, mutations) = mount(recovery, fixture.clone());
    click(&dom, listener(&mutations, "click", 0));
    assert!(
        matches!(fixture.actions.borrow().as_slice(), [SessionEvent::Recover(id)] if id == "original-request"),
        "unknown outcome queries the original identity"
    );
    let mut error = failure();
    error.retry = SessionRetryAdvice::SameRequest {
        not_before_ms: Some(1500),
    };
    for now in [None, Some(1499)] {
        let html = render(
            recovery,
            RecoveryFixture {
                state: SessionMutationState::Failed(error.clone()),
                now,
                actions: Rc::default(),
            },
        );
        assert!(
            !html.contains(">Retry delivery<") && html.contains("deadline"),
            "retry waits for a known provider deadline"
        );
    }
    let fixture = RecoveryFixture {
        state: SessionMutationState::Failed(error),
        now: Some(1500),
        actions: Rc::default(),
    };
    let (dom, mutations) = mount(recovery, fixture.clone());
    click(&dom, listener(&mutations, "click", 0));
    assert!(
        matches!(fixture.actions.borrow().as_slice(), [SessionEvent::Retry(id)] if id == "original-request"),
        "retry reuses the original ID and payload"
    );
}
