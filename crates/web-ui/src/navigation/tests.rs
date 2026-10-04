//! Navigation rendering and app-core dispatch contracts.

use std::{any::Any, cell::RefCell, rc::Rc};

use app_core::{
    Event as AppEvent, ViewModel, resources, sessions,
    workspace::{NavigationSection, WorkspaceMode},
};
use dioxus::prelude::*;
use dioxus_core::{AttributeValue, ElementId, Mutation, Mutations};
use dioxus_html::SerializedHtmlEventConverter;

use super::{SessionCreation, WorkspaceNavigation};

#[path = "../../examples/navigation/fixture.rs"]
mod fixture;
use fixture::Fixture;

#[derive(Clone)]
struct Harness {
    fixture: Rc<RefCell<Fixture>>,
    view: ViewModel,
    creation: Option<SessionCreation>,
    now: Option<u64>,
    events: Rc<RefCell<Vec<AppEvent>>>,
}

impl Harness {
    fn new(mode: WorkspaceMode) -> Self {
        let fixture = Fixture::new(mode).expect("valid navigation fixture");
        Self {
            view: fixture.view(),
            creation: Some(fixture.creation()),
            fixture: Rc::new(RefCell::new(fixture)),
            now: Some(1000),
            events: Rc::default(),
        }
    }
}

fn app(props: Harness) -> Element {
    rsx! { WorkspaceNavigation { id: "navigation", view: props.view, creation: props.creation, now_ms: props.now,
        onaction: move |event: AppEvent| {
            props.events.borrow_mut().push(event.clone());
            props.fixture.borrow_mut().dispatch(event).expect("UI intent dispatch");
        },
    } }
}

fn mount(props: Harness) -> (VirtualDom, Mutations) {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new_with_props(app, props);
    let mutations = dom.rebuild_to_vec();
    (dom, mutations)
}

fn target(mutations: &Mutations, attr: &str, value: &str) -> ElementId {
    mutations
        .edits
        .iter()
        .find_map(|edit| {
            if let Mutation::SetAttribute {
                name,
                value: AttributeValue::Text(text),
                id,
                ..
            } = edit
            {
                (name == &attr && text == value).then_some(*id)
            } else {
                None
            }
        })
        .expect("control attribute")
}

fn dispatch(dom: &VirtualDom, target: ElementId, event: &str, data: impl Any) {
    let data: Rc<dyn Any> = Rc::new(PlatformEventData::new(Box::new(data)));
    dom.runtime()
        .handle_event(event, Event::new(data, true), target);
}

fn click(dom: &VirtualDom, mutations: &Mutations, attr: &str, value: &str) {
    dispatch(
        dom,
        target(mutations, attr, value),
        "click",
        SerializedMouseData::default(),
    );
}

fn markup(props: Harness) -> String {
    let (dom, _mutations) = mount(props);
    dioxus_ssr::render(&dom)
}

#[test]
fn reference_order_control_first_supplied_counts_and_distinct_identities() {
    for mode in [WorkspaceMode::Standalone, WorkspaceMode::Managed] {
        let props = Harness::new(mode);
        let html = markup(props.clone());
        let mut cursor = 0;
        for label in [
            "Workspace",
            "Users",
            "Sessions",
            "Projections",
            "Compute hosts",
            "Model providers",
            "Activity",
            "Settings",
            "Agent Rules",
        ] {
            let position = html
                .get(cursor..)
                .expect("section position")
                .find(label)
                .expect("section label");
            cursor = cursor.saturating_add(position).saturating_add(label.len());
        }
        assert!(
            html.find("Ambient control") < html.find("data-identity=\"session-shared\""),
            "control is pinned ahead of supplied runners"
        );
        for text in [
            "Alice",
            "Bob",
            "MacBook Pro",
            "prod-cluster",
            "Host: host-shared",
            "Online status unknown",
            "Ambient · Running",
            "24",
            "Partial results",
            "Sessions count\">7",
            "Users count\">4",
            "Compute hosts count\">2",
        ] {
            assert!(html.contains(text), "retains supplied {text} in {mode:?}");
        }
        assert_eq!(
            html.matches("data-contributor=").count(),
            4,
            "compute hosts never become user rows"
        );
        assert!(
            props.events.borrow().is_empty(),
            "mounting never initiates an action"
        );
        assert!(
            html.contains("height:176px"),
            "Activity has a bounded viewport"
        );
    }
}

#[test]
fn session_host_provider_and_configuration_selections_reach_app_core() {
    let props = Harness::new(WorkspaceMode::Managed);
    let (dom, mutations) = mount(props.clone());
    click(&dom, &mutations, "data-identity", "session-shared");
    let view = props.fixture.borrow().view();
    assert_eq!(
        view.sessions.selected.as_deref(),
        Some("session-shared"),
        "exact directory identity selected"
    );
    assert_eq!(
        view.workspace.section,
        NavigationSection::Sessions,
        "session destination selected"
    );
    click(&dom, &mutations, "data-identity", "host-cluster");
    assert_eq!(
        props
            .fixture
            .borrow()
            .view()
            .resources
            .selected_host
            .as_deref(),
        Some("host-cluster"),
        "host selection reaches resource reducer"
    );
    click(&dom, &mutations, "data-identity", "provider-external");
    assert_eq!(
        props
            .fixture
            .borrow()
            .view()
            .resources
            .selected_provider
            .as_deref(),
        Some("provider-external"),
        "provider selection reaches resource reducer"
    );
    for (title, section) in [
        ("Settings", NavigationSection::Settings),
        ("Agent Rules", NavigationSection::AgentRules),
    ] {
        click(&dom, &mutations, "title", title);
        assert_eq!(
            props.fixture.borrow().view().workspace.section,
            section,
            "separate configuration route"
        );
    }
    assert!(
        !props
            .events
            .borrow()
            .iter()
            .any(|event| matches!(event, AppEvent::Resources(resources::Event::Execute { .. }))),
        "navigation does not execute on a host or change a runtime model"
    );
}

#[test]
fn repository_and_workspace_changes_preserve_scopes_and_retire_old_rows() {
    let props = Harness::new(WorkspaceMode::Managed);
    let (dom, mutations) = mount(props.clone());
    dispatch(
        &dom,
        target(&mutations, "id", "navigation-repository"),
        "change",
        SerializedFormData::new("repository-two".into(), Vec::new()),
    );
    assert_eq!(
        props
            .fixture
            .borrow()
            .view()
            .workspace
            .selected_repository
            .as_deref(),
        Some("repository-two"),
        "repository intent uses its stable ID"
    );
    dispatch(
        &dom,
        target(&mutations, "id", "navigation-repository"),
        "change",
        SerializedFormData::new(String::new(), Vec::new()),
    );
    assert!(
        props
            .fixture
            .borrow()
            .view()
            .workspace
            .selected_repository
            .is_none(),
        "managed workspace-wide selection is explicit"
    );
    dispatch(
        &dom,
        target(&mutations, "id", "navigation-workspace"),
        "change",
        SerializedFormData::new("workspace-two".into(), Vec::new()),
    );
    let view = props.fixture.borrow().view();
    assert_eq!(
        view.workspace.selected_workspace.as_deref(),
        Some("workspace-two"),
        "workspace selected in core"
    );
    assert!(
        view.sessions.sessions.is_empty() && view.resources.hosts.is_empty(),
        "old workspace data is retired"
    );
    let html = markup(Harness { view, ..props });
    assert!(
        !html.contains("MacBook Pro") && !html.contains("Ambient control"),
        "new workspace cannot show old private rows"
    );
}

#[test]
fn session_creation_uses_original_scope_and_payload_and_blocks_duplicate_clicks() {
    let props = Harness::new(WorkspaceMode::Managed);
    let creation = props.creation.clone().expect("prepared creation");
    let (dom, mutations) = mount(props.clone());
    click(&dom, &mutations, "aria-label", "Add session");
    click(&dom, &mutations, "aria-label", "Add session");
    assert_eq!(
        props.fixture.borrow().pending_creations(),
        1,
        "one runtime creation effect even before a rerender"
    );
    let view = props.fixture.borrow().view();
    let mutation = view.sessions.mutations.first().expect("pending creation");
    assert_eq!(
        mutation.request.mutation, creation.mutation,
        "original request ID and deadline retained"
    );
    assert_eq!(
        mutation.mutation,
        sessions::SessionMutation::Create(creation.draft),
        "exact host-prepared title and host retained"
    );
    assert_eq!(
        view.sessions.sessions.len(),
        7,
        "no optimistic session or execution claim"
    );
}

#[test]
fn unavailable_expired_used_and_foreign_creation_drafts_block_synthetic_actions() {
    for scenario in 0..7 {
        let mut props = Harness::new(WorkspaceMode::Managed);
        match scenario {
            0 => props.creation = None,
            1 => props.now = None,
            2 => props.now = Some(3000),
            3 => {
                props
                    .creation
                    .as_mut()
                    .expect("creation")
                    .context
                    .contributor_id = "another-user".into();
            }
            4 => props.view.sessions.capabilities.create = sessions::SessionCapability::Unavailable,
            5 => props.view.sessions.load = sessions::SessionLoadState::Loading,
            6 => {
                props
                    .fixture
                    .borrow_mut()
                    .dispatch(AppEvent::Sessions(sessions::Event::Create {
                        id: props.creation.as_ref().expect("creation").mutation.clone(),
                        draft: props.creation.as_ref().expect("creation").draft.clone(),
                    }))
                    .expect("initial creation");
                props.view = props.fixture.borrow().view();
            }
            _ => continue,
        }
        let (dom, mutations) = mount(props.clone());
        click(&dom, &mutations, "aria-label", "Add session");
        assert!(
            props.events.borrow().is_empty(),
            "scenario {scenario} cannot emit creation"
        );
        for label in [
            "Add compute host — registration unavailable",
            "Add model provider — registration unavailable",
        ] {
            click(&dom, &mutations, "aria-label", label);
        }
        assert!(
            props.events.borrow().is_empty(),
            "unsupported registration never emits a substitute action"
        );
    }
}

#[test]
fn unavailable_loading_failure_and_expiry_do_not_become_success_or_zero_counts() {
    let mut props = Harness::new(WorkspaceMode::Managed);
    props
        .fixture
        .borrow_mut()
        .fail_resources()
        .expect("failure");
    props.view = props.fixture.borrow().view();
    let html = markup(props.clone());
    assert!(
        html.contains("Resource provider is offline")
            && html.contains("MacBook Pro")
            && html.contains("Availability unknown"),
        "failed refresh retains rows with unknown health and failure text"
    );
    props.fixture.borrow_mut().expire().expect("expiry");
    props.view = props.fixture.borrow().view();
    assert!(
        markup(props.clone()).contains("Online status unknown"),
        "expired member activity has unknown online status"
    );
    props.view.projections.tasks.total = Some(u64::MAX);
    props.view.projections.tasks.loaded_count = 9;
    let html = markup(props.clone());
    assert!(
        html.contains("18446744073709551615") && html.contains("9 loaded"),
        "supplied scope total is not a loaded-row count"
    );
    props.fixture.borrow_mut().disconnect().expect("disconnect");
    props.view = props.fixture.borrow().view();
    let html = markup(props);
    assert!(
        html.contains("Sessions not connected") && html.contains("Resources not connected"),
        "disconnected domains are explicit"
    );
    assert!(
        !html.contains("Sessions count\">0"),
        "unknown directory is not an empty snapshot"
    );
}

fn assembled(view: ViewModel) -> Element {
    rsx! { crate::assembly::WorkspaceSurface { view, capabilities: crate::host::HostCapabilities::new(crate::host::HostKind::Browser), onaction: move |_event| {} } }
}

#[test]
fn sidebar_assembly_keeps_existing_details_and_separate_configuration_routes() {
    let props = Harness::new(WorkspaceMode::Managed);
    for section in [
        NavigationSection::Workspace,
        NavigationSection::Activity,
        NavigationSection::Sessions,
        NavigationSection::Projections,
        NavigationSection::Settings,
        NavigationSection::AgentRules,
    ] {
        let mut view = props.view.clone();
        view.workspace.section = section;
        let mut dom = VirtualDom::new_with_props(assembled, view);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(
            html.contains("Workspace navigation"),
            "navigation stays mounted across destination changes"
        );
        assert_eq!(
            html.contains("id=\"idle-history\""),
            section == NavigationSection::Activity,
            "history details appear only on Activity"
        );
        assert_eq!(
            html.contains("Prompt composer"),
            section == NavigationSection::Sessions,
            "session prompting remains reachable"
        );
        assert_eq!(
            html.contains("id=\"idle-projection-Task\""),
            section == NavigationSection::Projections,
            "projection summaries open the shared panels"
        );
    }
}

#[test]
fn workspace_empty_state_requires_a_successful_directory_response() {
    let mut props = Harness::new(WorkspaceMode::Managed);
    props.view = ViewModel::default();
    assert!(
        !markup(props.clone()).contains("No workspaces available"),
        "an unloaded directory is not a known empty result"
    );
    props.view.workspace.directory_state = app_core::workspace::WorkspaceRequestState::Ready;
    assert!(
        markup(props).contains("No workspaces available"),
        "a supplied empty directory has clear feedback"
    );
}
