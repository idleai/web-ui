//! Rendering and interaction contracts over the real app-core reducer.

use std::{any::Any, cell::RefCell, rc::Rc};

use app_core::projections::{
    Event as ProjectionEvent, FreshnessStatus, ProjectionAvailability, ProjectionFilter,
    ProjectionKind, ProjectionLoadState, ProjectionSelection, ViewModel,
};
use dioxus::prelude::*;
use dioxus_core::{ElementId, Mutation, Mutations};
use dioxus_html::SerializedHtmlEventConverter;

use super::{
    ProjectionCard, ProjectionFeedback, ProjectionFilters, ProjectionLayout, ProjectionPanel,
    ProjectionSummary,
};

#[path = "../../examples/projections/fixture.rs"]
mod fixture;
use fixture::Fixture;

fn mount<P: Clone + 'static>(app: fn(P) -> Element, props: P) -> (VirtualDom, Mutations) {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new_with_props(app, props);
    let mutations = dom.rebuild_to_vec();
    (dom, mutations)
}

fn listener(mutations: &Mutations, name: &str, index: usize) -> ElementId {
    mutations
        .edits
        .iter()
        .filter_map(|edit| {
            if let Mutation::NewEventListener { name: event, id } = edit {
                (name == event).then_some(*id)
            } else {
                None
            }
        })
        .nth(index)
        .expect("fixture listener")
}

fn dispatch(dom: &VirtualDom, target: ElementId, name: &str, data: impl Any) {
    let data: Rc<dyn Any> = Rc::new(PlatformEventData::new(Box::new(data)));
    dom.runtime()
        .handle_event(name, Event::new(data, true), target);
}

fn click(dom: &VirtualDom, mutations: &Mutations, index: usize) {
    dispatch(
        dom,
        listener(mutations, "click", index),
        "click",
        SerializedMouseData::default(),
    );
}

fn click_control(dom: &VirtualDom, mutations: &Mutations, attribute: &str, index: usize) {
    let target = mutations.edits.iter().filter_map(|edit| {
        if let Mutation::SetAttribute { name, id, .. } = edit {
            (*name == attribute && mutations.edits.iter().any(|edit| matches!(edit, Mutation::NewEventListener { name, id: target } if name == "click" && target == id))).then_some(*id)
        } else { None }
    }).nth(index).expect("labelled fixture control");
    dispatch(dom, target, "click", SerializedMouseData::default());
}

#[derive(Clone)]
struct PanelFixture {
    view: ViewModel,
    layout: ProjectionLayout,
    actions: Rc<RefCell<Vec<ProjectionEvent>>>,
}

fn panel(props: PanelFixture) -> Element {
    rsx! { ProjectionPanel { id: "tasks", view: props.view, kind: ProjectionKind::Task, layout: props.layout, onaction: move |event| props.actions.borrow_mut().push(event) } }
}

fn markup(view: ViewModel) -> String {
    let (dom, _mutations) = mount(
        panel,
        PanelFixture {
            view,
            layout: ProjectionLayout::List,
            actions: Rc::default(),
        },
    );
    dioxus_ssr::render(&dom)
}

#[test]
fn all_layouts_keep_supplied_rows_counts_statuses_and_full_records_without_mount_actions() {
    for layout in [
        ProjectionLayout::List,
        ProjectionLayout::Table,
        ProjectionLayout::Cards,
        ProjectionLayout::Board,
    ] {
        let fixture = Fixture::new().expect("valid fixture");
        let actions = Rc::default();
        let (dom, _mutations) = mount(
            panel,
            PanelFixture {
                view: fixture.view(),
                layout,
                actions: Rc::clone(&actions),
            },
        );
        let html = dioxus_ssr::render(&dom);
        for text in [
            "Run workspace checks",
            "4 shown",
            "4 loaded",
            "4 total",
            "Current",
            "Complete",
            "No status",
            "Empty status",
            "snapshot/alpha",
            "1791072000123",
            "owner,infra",
        ] {
            assert!(html.contains(text), "{layout:?} retains {text}");
        }
        for full in [
            "30".repeat(32),
            "a0".repeat(32),
            "b0".repeat(32),
            "c0".repeat(32),
            "d0".repeat(32),
        ] {
            assert!(
                html.contains(&full),
                "{layout:?} retains full record addresses"
            );
        }
        assert!(
            !html.contains("<literal text>"),
            "provider text is escaped in {layout:?}"
        );
        assert!(
            html.contains("aria-pressed=\"false\""),
            "selection is announced by native controls"
        );
        assert!(
            actions.borrow().is_empty(),
            "mounting never requests data or changes state"
        );
        if layout == ProjectionLayout::Table {
            assert!(
                html.contains("scope=\"col\"") && html.contains("scope=\"row\""),
                "table headers retain native relationships"
            );
        }
        if layout == ProjectionLayout::Board {
            assert_eq!(
                html.matches("class=\"idle-projection-column\"").count(),
                4,
                "opaque, missing and empty statuses form separate lanes"
            );
        }
    }
}

#[test]
fn empty_loading_partial_unavailable_and_failed_results_are_distinct() {
    let mut fixture = Fixture::new().expect("valid fixture");
    fixture.dispatch(ProjectionEvent::Refresh).expect("refresh");
    let html = markup(fixture.view());
    assert!(
        html.contains("Loading projections")
            && html.contains("Stale")
            && html.contains("Run workspace checks"),
        "refresh keeps stale rows visible"
    );
    fixture.fail().expect("failure");
    let html = markup(fixture.view());
    assert!(
        html.contains("Provider is offline") && html.contains("Run workspace checks"),
        "failed refresh retains results with an alert"
    );
    fixture.dispatch(ProjectionEvent::Suspend).expect("suspend");
    assert!(
        markup(fixture.view()).contains("waiting to reconnect"),
        "continuity loss is separate from a read failure"
    );
    fixture
        .dispatch(ProjectionEvent::Reconnect)
        .expect("reconnect");
    fixture.complete().expect("complete");
    assert_eq!(
        fixture.view().load,
        ProjectionLoadState::Ready,
        "host reconnect replaces the snapshot"
    );
    for (availability, title) in [
        (ProjectionAvailability::Complete, "No items"),
        (ProjectionAvailability::Partial, "No items loaded"),
        (
            ProjectionAvailability::Unavailable,
            "Projection unavailable",
        ),
    ] {
        fixture.empty(availability).expect("replace");
        let html = markup(fixture.view());
        assert!(
            html.contains(title) && html.contains("Freshness unknown"),
            "{availability:?} never becomes current just because it has a generation time"
        );
        assert_eq!(
            html.contains("Total unknown"),
            availability != ProjectionAvailability::Complete,
            "only a supplied zero means no items"
        );
    }
    fixture
        .dispatch(ProjectionEvent::Disconnect)
        .expect("disconnect");
    let html = markup(fixture.view());
    assert!(
        html.contains("Connect a workspace"),
        "disconnected state has guidance"
    );
    assert!(
        !html.contains("No items") && !html.contains("Filter text"),
        "unloaded scope is not an empty result"
    );
}

fn summary(view: ViewModel) -> Element {
    rsx! { ProjectionSummary { view: view.tasks } }
}

#[test]
fn summary_preserves_large_supplied_counts_and_independent_coverage_and_freshness() {
    let mut view = Fixture::new().expect("valid fixture").view();
    view.tasks.total = Some(u64::MAX);
    view.tasks.loaded_count = 99;
    view.tasks.visible_count = 7;
    view.tasks.availability = ProjectionAvailability::Partial;
    view.tasks.freshness.status = FreshnessStatus::Current;
    let (dom, _mutations) = mount(summary, view);
    let html = dioxus_ssr::render(&dom);
    for text in [
        "18446744073709551615 total",
        "99 loaded",
        "7 shown",
        "Partial results",
        "Current",
    ] {
        assert!(
            html.contains(text),
            "metadata uses supplied {text}, not a client count or rounded number"
        );
    }
}

#[derive(Clone)]
struct Interactive {
    fixture: Rc<RefCell<Fixture>>,
    actions: Rc<RefCell<Vec<ProjectionEvent>>>,
}

impl Interactive {
    fn new() -> Self {
        Self {
            fixture: Rc::new(RefCell::new(Fixture::new().expect("valid fixture"))),
            actions: Rc::default(),
        }
    }
    fn apply(&self, event: ProjectionEvent) {
        self.actions.borrow_mut().push(event.clone());
        self.fixture
            .borrow_mut()
            .dispatch(event)
            .expect("valid UI intent");
    }
}

fn card(props: Interactive) -> Element {
    let view = props.fixture.borrow().view();
    rsx! { ProjectionCard { kind: ProjectionKind::Task, row: view.tasks.rows.first().expect("row").clone(), selected: view.selected, onaction: move |event| props.apply(event) } }
}

fn filters(props: Interactive) -> Element {
    let view = props.fixture.borrow().view();
    rsx! { ProjectionFilters { id: "filter", view: view.tasks, onaction: move |event| props.apply(event) } }
}

#[test]
fn selection_and_inspection_emit_the_original_key_and_complete_source_or_related_address() {
    let props = Interactive::new();
    let row = props
        .fixture
        .borrow()
        .view()
        .tasks
        .rows
        .first()
        .expect("row")
        .clone();
    let (dom, mutations) = mount(card, props.clone());
    click_control(&dom, &mutations, "aria-pressed", 0);
    assert_eq!(
        props.fixture.borrow().view().selected,
        Some(ProjectionSelection {
            kind: ProjectionKind::Task,
            key: row.key.clone()
        }),
        "native selection reaches app-core"
    );
    let references: Vec<_> = row.sources.iter().chain(&row.related).collect();
    let mut inspected = Vec::new();
    for index in 0..references.len() {
        click_control(&dom, &mutations, "aria-busy", index);
        let actions = props.actions.borrow();
        let (selected, reference) = actions
            .last()
            .and_then(|action| {
                if let ProjectionEvent::Inspect {
                    selection,
                    reference,
                } = action
                {
                    Some((selection, reference))
                } else {
                    None
                }
            })
            .expect("record control emits inspection");
        assert!(
            selected.key == row.key
                && selected.kind == ProjectionKind::Task
                && references.contains(&reference),
            "inspection preserves all addresses, including digest and optional item"
        );
        inspected.push(reference.clone());
        let inspection = props.fixture.borrow().inspection();
        if let Some(observation) = &reference.observation {
            assert!(
                inspection.contains(observation),
                "root history selects the supplied observation"
            );
        }
        if let Some(item) = &reference.item {
            assert!(
                inspection.contains(item),
                "root history selects the supplied item"
            );
        }
    }
    assert!(
        references
            .iter()
            .all(|reference| inspected.contains(reference)),
        "every supplied source and related address has its own action"
    );
}

#[test]
fn filters_are_conjunctive_and_keep_selection_and_scope_total_in_app_core() {
    let props = Interactive::new();
    props.apply(ProjectionEvent::Select(Some(ProjectionSelection {
        kind: ProjectionKind::Task,
        key: "task/checks".into(),
    })));
    let (dom, mutations) = mount(filters, props.clone());
    dispatch(
        &dom,
        listener(&mutations, "input", 0),
        "input",
        SerializedFormData::new("Review".into(), Vec::new()),
    );
    let view = props.fixture.borrow().view();
    assert_eq!(
        view.tasks.filter.text, "Review",
        "text passes through literally"
    );
    assert_eq!(
        (
            view.tasks.visible_count,
            view.tasks.loaded_count,
            view.tasks.total
        ),
        (1, 4, Some(4)),
        "core alone filters while retaining supplied totals"
    );
    assert!(
        markup(view).contains("Selected item is outside the current filters"),
        "filtered selection remains visible as feedback"
    );
    let (dom, mutations) = mount(filters, props.clone());
    dispatch(
        &dom,
        listener(&mutations, "change", 0),
        "change",
        SerializedFormData::new("status:queued".into(), Vec::new()),
    );
    let view = props.fixture.borrow().view();
    assert_eq!(
        view.tasks.filter.status.as_deref(),
        Some("queued"),
        "status is an exact provider string"
    );
    assert_eq!(
        view.tasks.filter.text, "Review",
        "changing status retains text"
    );
    let (dom, mutations) = mount(filters, props.clone());
    click_control(&dom, &mutations, "aria-pressed", 0);
    assert_eq!(
        props.fixture.borrow().view().tasks.filter.labels,
        ["needs review"],
        "label is one exact value"
    );
    let (dom, mutations) = mount(filters, props.clone());
    click_control(&dom, &mutations, "aria-pressed", 1);
    assert_eq!(
        props.fixture.borrow().view().tasks.filter.labels,
        ["needs review", ""],
        "empty labels are distinct and conjunctive"
    );
    let (dom, mutations) = mount(filters, props.clone());
    click_control(&dom, &mutations, "aria-busy", 0);
    let view = props.fixture.borrow().view();
    assert_eq!(
        view.tasks.filter,
        ProjectionFilter::default(),
        "clear is an app-core intent"
    );
    assert_eq!(view.tasks.visible_count, 4, "clear restores loaded choices");
    assert!(view.selected.is_some(), "clear filters retains selection");
}

#[test]
fn row_replacement_retains_keys_until_retracted_and_empty_status_is_filterable() {
    let props = Interactive::new();
    props.apply(ProjectionEvent::Select(Some(ProjectionSelection {
        kind: ProjectionKind::Task,
        key: "task/checks".into(),
    })));
    props.fixture.borrow_mut().prepend().expect("prepend");
    assert_eq!(
        props
            .fixture
            .borrow()
            .view()
            .selected
            .as_ref()
            .map(|value| value.key.as_str()),
        Some("task/checks"),
        "a lower observation ID cannot change selection"
    );
    props.fixture.borrow_mut().retract().expect("retract");
    assert!(
        props.fixture.borrow().view().selected.is_none(),
        "core clears a retracted row"
    );
    let (dom, mutations) = mount(filters, props.clone());
    dispatch(
        &dom,
        listener(&mutations, "change", 0),
        "change",
        SerializedFormData::new("status:".into(), Vec::new()),
    );
    assert_eq!(
        props.fixture.borrow().view().tasks.filter.status,
        Some(String::new()),
        "empty supplied status is not the all-statuses sentinel"
    );
}

fn feedback(props: PanelFixture) -> Element {
    rsx! { ProjectionFeedback { view: props.view, onaction: move |event| props.actions.borrow_mut().push(event) } }
}

#[test]
fn busy_disconnected_and_suspended_refresh_buttons_block_synthetic_activation() {
    for state in [
        ProjectionLoadState::Idle,
        ProjectionLoadState::Loading,
        ProjectionLoadState::Suspended,
        ProjectionLoadState::Ready,
    ] {
        let mut view = Fixture::new().expect("fixture").view();
        view.load = state.clone();
        if state == ProjectionLoadState::Idle {
            view.context = None;
        }
        let actions = Rc::default();
        let (dom, mutations) = mount(
            feedback,
            PanelFixture {
                view,
                layout: ProjectionLayout::List,
                actions: Rc::clone(&actions),
            },
        );
        click(&dom, &mutations, 0);
        assert_eq!(
            actions.borrow().len(),
            usize::from(state == ProjectionLoadState::Ready),
            "refresh availability follows {state:?}"
        );
    }
}
