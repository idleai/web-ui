//! Behavioral and accessibility contracts for reusable foundations.

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_core::{ElementId, Mutation, Mutations};
use dioxus_html::SerializedHtmlEventConverter;

use crate::controls::{
    Button, Checkbox, ControlState, Disclosure, FieldMessage, FieldState, IconButton, Select,
    SelectOption, TextField,
};
use crate::host::{
    FileTarget, HostActionButton, HostCapabilities, HostError, HostKind, HostRequest,
};
use crate::icons::{Icon, IconName};
use crate::status::{EmptyState, ErrorState, LoadingState};
use crate::theme::{Density, Theme, ThemeProvider};

fn mount<P: Clone + 'static>(app: fn(P) -> Element, props: P) -> (VirtualDom, Mutations) {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new_with_props(app, props);
    let mutations = dom.rebuild_to_vec();
    (dom, mutations)
}

fn listener(mutations: &Mutations, event: &str) -> ElementId {
    mutations
        .edits
        .iter()
        .find_map(|edit| {
            if let Mutation::NewEventListener { name, id } = edit {
                (name == event).then_some(*id)
            } else {
                None
            }
        })
        .expect("fixture must have the requested event listener")
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

fn markup(app: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[derive(Clone)]
struct ActionFixture {
    state: ControlState,
    calls: Rc<RefCell<Vec<()>>>,
    icon: bool,
}

fn action_fixture(props: ActionFixture) -> Element {
    rsx! {
        if props.icon {
            IconButton { label: "Refresh", icon: IconName::Refresh, state: props.state, onpress: move |()| props.calls.borrow_mut().push(()) }
        } else {
            Button { label: "Save", state: props.state, onpress: move |()| props.calls.borrow_mut().push(()) }
        }
    }
}

#[test]
fn disabled_and_busy_actions_block_even_synthetic_activation() {
    for icon in [false, true] {
        for state in [
            ControlState::Ready,
            ControlState::Disabled,
            ControlState::Busy,
        ] {
            let calls = Rc::default();
            let (dom, mutations) = mount(
                action_fixture,
                ActionFixture {
                    state,
                    calls: Rc::clone(&calls),
                    icon,
                },
            );
            click(&dom, listener(&mutations, "click"));
            assert_eq!(
                calls.borrow().len(),
                usize::from(state == ControlState::Ready),
                "only ready actions dispatch"
            );
            let html = dioxus_ssr::render(&dom);
            assert!(
                html.contains("type=\"button\""),
                "actions must not implicitly submit forms"
            );
            if state == ControlState::Busy {
                assert!(
                    html.contains("aria-busy=\"true\"") && html.contains("aria-disabled=\"true\""),
                    "busy buttons expose pending and unavailable semantics"
                );
                assert!(
                    !html.contains(" disabled="),
                    "busy buttons retain keyboard focus"
                );
            }
        }
    }
}

fn file(revision: &str) -> FileTarget {
    FileTarget {
        repository: "repo-1".into(),
        path: "src/lib.rs".into(),
        revision: Some(revision.into()),
        position: None,
    }
}

fn host_requests() -> Vec<HostRequest> {
    vec![
        HostRequest::OpenExternal {
            url: "https://example.com/evidence".into(),
        },
        HostRequest::CopyText {
            text: "record-1".into(),
        },
        HostRequest::OpenFile(file("after")),
        HostRequest::OpenDiff {
            before: file("before"),
            after: file("after"),
        },
        HostRequest::RevealFile(file("after")),
    ]
}

#[test]
fn host_kind_never_grants_capabilities_and_grants_are_independent() {
    for kind in [HostKind::Browser, HostKind::VsCode] {
        for request in host_requests() {
            let empty = HostCapabilities::new(kind);
            assert_eq!(empty.kind(), kind, "host identity is preserved");
            assert_eq!(
                empty.check(&request),
                Err(HostError::Unavailable(request.capability())),
                "all capabilities start unavailable"
            );
            let granted = empty.with(request.capability());
            assert_eq!(
                granted.check(&request),
                Ok(()),
                "the advertised action is enabled"
            );
            for other in host_requests() {
                assert_eq!(
                    granted.check(&other).is_ok(),
                    request.capability() == other.capability(),
                    "one capability cannot grant another"
                );
            }
        }
    }
}

#[derive(Clone)]
struct HostFixture {
    capabilities: Rc<RefCell<HostCapabilities>>,
    request: HostRequest,
    calls: Rc<RefCell<Vec<HostRequest>>>,
}

fn host_fixture(props: HostFixture) -> Element {
    rsx! {
        HostActionButton {
            id: "native-open", label: "Open evidence", unavailable_reason: "No editor adapter.",
            capabilities: props.capabilities.borrow().clone(), request: props.request,
            onrequest: move |request| props.calls.borrow_mut().push(request),
        }
    }
}

#[test]
fn host_dispatch_preserves_evidence_and_stops_after_capability_removal() {
    for kind in [HostKind::Browser, HostKind::VsCode] {
        for request in host_requests() {
            let capabilities = Rc::new(RefCell::new(
                HostCapabilities::new(kind).with(request.capability()),
            ));
            let calls = Rc::default();
            let (mut dom, mutations) = mount(
                host_fixture,
                HostFixture {
                    capabilities: Rc::clone(&capabilities),
                    request: request.clone(),
                    calls: Rc::clone(&calls),
                },
            );
            let target = listener(&mutations, "click");
            click(&dom, target);
            assert_eq!(
                *calls.borrow(),
                vec![request],
                "the host receives exact repository, revision and content values"
            );
            *capabilities.borrow_mut() = HostCapabilities::new(kind);
            dom.mark_dirty(ScopeId::APP);
            dom.render_immediate(&mut dioxus_core::NoOpMutations);
            click(&dom, target);
            assert_eq!(
                calls.borrow().len(),
                1,
                "a removed capability cannot dispatch again"
            );
            let html = dioxus_ssr::render(&dom);
            assert!(
                html.contains("aria-describedby=\"native-open-unavailable\"")
                    && html.contains("No editor adapter."),
                "unavailable actions have an associated explanation"
            );
        }
    }
}

#[derive(Clone)]
struct FieldFixture {
    state: FieldState,
    commits: Rc<RefCell<Vec<String>>>,
    inputs: Rc<RefCell<Vec<String>>>,
    escapes: Rc<RefCell<Vec<()>>>,
}

fn field_fixture(props: FieldFixture) -> Element {
    rsx! {
        TextField {
            id: "search", label: "Search", value: "supplied query", state: props.state,
            oninput: move |value| props.inputs.borrow_mut().push(value),
            oncommit: move |value| props.commits.borrow_mut().push(value),
            onescape: move |()| props.escapes.borrow_mut().push(()),
        }
    }
}

#[test]
fn keyboard_callbacks_preserve_composition_modifiers_and_unhandled_keys() {
    let commits = Rc::default();
    let escapes = Rc::default();
    let (dom, mutations) = mount(
        field_fixture,
        FieldFixture {
            state: FieldState::Editable,
            commits: Rc::clone(&commits),
            inputs: Rc::default(),
            escapes: Rc::clone(&escapes),
        },
    );
    let target = listener(&mutations, "keydown");
    for (key, code, modifiers, composing) in [
        (Key::Enter, Code::Enter, Modifiers::empty(), true),
        (Key::Escape, Code::Escape, Modifiers::empty(), true),
        (Key::Enter, Code::Enter, Modifiers::CONTROL, false),
        (Key::Escape, Code::Escape, Modifiers::SHIFT, false),
        (Key::Tab, Code::Tab, Modifiers::empty(), false),
    ] {
        let event = dispatch(
            &dom,
            target,
            "keydown",
            SerializedKeyboardData::new(key, code, Location::Standard, false, modifiers, composing),
        );
        assert!(
            event.default_action_enabled() && event.propagates(),
            "unhandled keys pass through untouched"
        );
    }
    assert!(
        commits.borrow().is_empty() && escapes.borrow().is_empty(),
        "composition and shortcuts cannot invoke plain-key actions"
    );
    for (key, code) in [(Key::Enter, Code::Enter), (Key::Escape, Code::Escape)] {
        let event = dispatch(
            &dom,
            target,
            "keydown",
            SerializedKeyboardData::new(
                key,
                code,
                Location::Standard,
                false,
                Modifiers::empty(),
                false,
            ),
        );
        assert!(
            !event.default_action_enabled() && !event.propagates(),
            "handled keys do not also trigger a parent action"
        );
    }
    assert_eq!(
        *commits.borrow(),
        vec!["supplied query"],
        "commit returns the supplied controlled value"
    );
    assert_eq!(
        escapes.borrow().len(),
        1,
        "escape requests cancellation exactly once"
    );
}

#[test]
fn readonly_and_disabled_fields_do_not_emit_edits_or_commits() {
    for state in [
        FieldState::Editable,
        FieldState::ReadOnly,
        FieldState::Disabled,
    ] {
        let inputs = Rc::default();
        let commits = Rc::default();
        let (dom, mutations) = mount(
            field_fixture,
            FieldFixture {
                state,
                commits: Rc::clone(&commits),
                inputs: Rc::clone(&inputs),
                escapes: Rc::default(),
            },
        );
        let _event = dispatch(
            &dom,
            listener(&mutations, "input"),
            "input",
            SerializedFormData::new("next".into(), Vec::new()),
        );
        let _event = dispatch(
            &dom,
            listener(&mutations, "keydown"),
            "keydown",
            SerializedKeyboardData::new(
                Key::Enter,
                Code::Enter,
                Location::Standard,
                false,
                Modifiers::empty(),
                false,
            ),
        );
        assert_eq!(
            inputs.borrow().len(),
            usize::from(state == FieldState::Editable),
            "only editable fields emit changes"
        );
        assert_eq!(
            commits.borrow().len(),
            usize::from(state == FieldState::Editable),
            "only editable fields commit"
        );
    }
}

#[test]
fn unconfigured_enter_and_escape_keep_native_behavior() {
    let (dom, mutations) = mount(
        |()| rsx! { TextField { id: "plain", label: "Plain", value: "", oninput: move |_| {} } },
        (),
    );
    for (key, code) in [(Key::Enter, Code::Enter), (Key::Escape, Code::Escape)] {
        let event = dispatch(
            &dom,
            listener(&mutations, "keydown"),
            "keydown",
            SerializedKeyboardData::new(
                key,
                code,
                Location::Standard,
                false,
                Modifiers::empty(),
                false,
            ),
        );
        assert!(
            event.default_action_enabled() && event.propagates(),
            "no callback means no interception"
        );
    }
}

#[test]
fn fields_associate_labels_feedback_and_invalid_state_and_escape_text() {
    let html = markup(|| {
        rsx! {
            TextField { id: "query", label: "Find <history>", value: "<script>alert(1)</script>", message: FieldMessage::Error("Select a <revision>".into()), oninput: move |_| {} }
            Select { id: "model", label: "Model", value: "b", options: vec![SelectOption { value: "b".into(), label: "Model B".into(), disabled: false }], message: FieldMessage::Hint("Choose one".into()), onchange: move |_| {} }
            Checkbox { id: "sharing", label: "Share session", checked: true, onchange: move |_| {} }
        }
    });
    for expected in [
        "for=\"query\"",
        "id=\"query\"",
        "aria-invalid=\"true\"",
        "aria-describedby=\"query-message\"",
        "id=\"query-message\"",
        "for=\"model\"",
        "aria-describedby=\"model-message\"",
        "for=\"sharing\"",
        "type=\"checkbox\"",
    ] {
        assert!(
            html.contains(expected),
            "missing accessible field association: {expected}"
        );
    }
    assert!(
        !html.contains("<script>") && html.contains("&#60;script&#62;"),
        "field values must be escaped, never injected as HTML: {html}"
    );
    assert!(
        html.contains("Select a &#60;revision&#62;"),
        "feedback is plain text"
    );
}

#[test]
fn disclosure_is_controlled_and_collapsed_content_is_hidden() {
    let changes = Rc::new(RefCell::new(Vec::new()));
    let (dom, mutations) = mount(
        |changes: Rc<RefCell<Vec<bool>>>| {
            rsx! {
                Disclosure { id: "evidence", label: "Evidence", expanded: false, onchange: move |value| changes.borrow_mut().push(value),
                    Button { label: "Inner action", onpress: move |()| {} }
                }
            }
        },
        Rc::clone(&changes),
    );
    click(&dom, listener(&mutations, "click"));
    assert_eq!(
        *changes.borrow(),
        vec![true],
        "disclosure requests expansion from its owner"
    );
    let html = dioxus_ssr::render(&dom);
    assert!(
        html.contains("aria-expanded=\"false\"")
            && html.contains("aria-controls=\"evidence-panel\"")
            && html.contains("hidden="),
        "the view remains collapsed until its owner supplies new state"
    );
    assert!(
        html.contains("Inner action"),
        "collapsed children remain mounted"
    );
}

#[test]
fn statuses_and_icons_have_explicit_accessibility_semantics() {
    let html = markup(|| {
        rsx! {
            LoadingState { label: "Loading history" }
            ErrorState { title: "History unavailable", message: "Try again" }
            EmptyState { title: "No history", message: "Nothing recorded" }
            Icon { name: IconName::File, label: "Recorded file" }
            Icon { name: IconName::Check }
            IconButton { label: "Refresh history", icon: IconName::Refresh, onpress: move |()| {} }
        }
    });
    assert_eq!(
        html.matches("role=\"status\"").count(),
        1,
        "only loading is a polite live region"
    );
    assert_eq!(
        html.matches("role=\"alert\"").count(),
        1,
        "the error is announced once"
    );
    assert!(
        !html.contains(">Retry<"),
        "no retry appears without a handler"
    );
    for expected in [
        "aria-live=\"polite\"",
        "aria-label=\"Recorded file\"",
        "role=\"img\"",
        "aria-hidden=\"true\"",
        "focusable=\"false\"",
        "aria-label=\"Refresh history\"",
    ] {
        assert!(
            html.contains(expected),
            "missing icon or status semantics: {expected}"
        );
    }
}

#[test]
fn retry_is_explicit_and_busy_retry_is_suppressed() {
    for state in [ControlState::Ready, ControlState::Busy] {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let (dom, mutations) = mount(
            |(state, calls): (ControlState, Rc<RefCell<Vec<()>>>)| {
                rsx! {
                    ErrorState { title: "Failed", message: "Retry when ready", retry_state: state, onretry: move |()| calls.borrow_mut().push(()) }
                }
            },
            (state, Rc::clone(&calls)),
        );
        assert!(
            calls.borrow().is_empty(),
            "rendering an error cannot retry automatically"
        );
        click(&dom, listener(&mutations, "click"));
        assert_eq!(
            calls.borrow().len(),
            usize::from(state == ControlState::Ready),
            "pending retries cannot repeat"
        );
    }
}

#[test]
fn themes_are_scoped_and_scaffold_consumers_remain_compatible() {
    let html = markup(|| {
        rsx! {
            ThemeProvider { theme: Theme::Dark, density: Density::Compact, p { "First" } }
            ThemeProvider { theme: Theme::Light, p { "Second" } }
            crate::Scaffold { view: app_core::ViewModel::default() }
        }
    });
    for expected in [
        "data-idle-theme=\"dark\"",
        "data-idle-theme=\"light\"",
        "data-idle-density=\"compact\"",
        "data-idle-density=\"comfortable\"",
        "Starting…",
    ] {
        assert!(
            html.contains(expected),
            "independent theme roots and bootstrap are preserved: {expected}"
        );
    }
    assert!(
        !html.contains("<style") && !html.contains("<script"),
        "hosts own style loading and platform integration"
    );
}
