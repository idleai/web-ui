//! Typed disclosure, lookup and capability-gated native actions.

use std::cell::RefCell;
use std::rc::Rc;

use app_core::history::{Event as HistoryEvent, OpenTarget};
use app_core::module::EffectError;
use dioxus::prelude::*;
use dioxus_core::{ElementId, Mutation, Mutations};
use dioxus_html::SerializedHtmlEventConverter;

use super::super::actions::OpenButton;
use super::super::row::ItemPaging;
use super::{
    HistoryRow, HostCapabilities, HostKind, ItemView, OperationDetailsState, Paging, RequestState,
};
use super::{id, item, reference};
use crate::host::HostCapability;

fn mount<P: Clone + 'static>(app: fn(P) -> Element, props: P) -> (VirtualDom, Mutations) {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new_with_props(app, props);
    let mutations = dom.rebuild_to_vec();
    (dom, mutations)
}

fn button(mutations: &Mutations, index: usize) -> ElementId {
    mutations
        .edits
        .iter()
        .filter_map(|mutation| {
            if let Mutation::NewEventListener { name, id } = mutation {
                (name == "click").then_some(*id)
            } else {
                None
            }
        })
        .nth(index)
        .expect("fixture contains the requested button")
}

fn click(dom: &VirtualDom, target: ElementId) {
    let data: Rc<dyn std::any::Any> = Rc::new(PlatformEventData::new(Box::new(
        SerializedMouseData::default(),
    )));
    dom.runtime()
        .handle_event("click", Event::new(data, true), target);
}

fn click_buttons(dom: &VirtualDom, mutations: &Mutations) {
    for mutation in &mutations.edits {
        if let Mutation::NewEventListener { name, id } = mutation
            && name == "click"
        {
            click(dom, *id);
        }
    }
}

#[derive(Clone)]
struct RowFixture {
    item: Rc<RefCell<ItemView>>,
    actions: Rc<RefCell<Vec<HistoryEvent>>>,
}

fn row_fixture(props: RowFixture) -> Element {
    rsx! { HistoryRow { id: "row", item: props.item.borrow().clone(), onaction: move |action| props.actions.borrow_mut().push(action) } }
}

#[test]
fn disclosure_and_selection_emit_full_identities_and_wait_for_app_core() {
    let value = Rc::new(RefCell::new(item()));
    let actions = Rc::default();
    let (mut dom, mutations) = mount(
        row_fixture,
        RowFixture {
            item: Rc::clone(&value),
            actions: Rc::clone(&actions),
        },
    );
    click_buttons(&dom, &mutations);
    assert!(
        actions.borrow().iter().any(|action| matches!(action, HistoryEvent::Select(selected) if selected.item == Some(id(10)) && selected.observation.is_none())),
        "selection retains the full item key"
    );
    assert!(
        actions
            .borrow()
            .iter()
            .any(|action| matches!(action, HistoryEvent::ToggleDisclosure(key) if *key == id(10))),
        "expansion is a semantic event"
    );
    assert!(
        dioxus_ssr::render(&dom).contains("aria-expanded=\"false\""),
        "the component does not maintain a second disclosure state"
    );
    value.borrow_mut().expanded = true;
    dom.mark_dirty(ScopeId::APP);
    let mutations = dom.render_immediate_to_vec();
    click_buttons(&dom, &mutations);
    assert!(
        actions.borrow().iter().any(|action| matches!(action, HistoryEvent::Select(selected) if selected.item == Some(id(10)) && selected.observation == Some(id(1)))),
        "record drill-down selects the exact observation"
    );
    assert!(
        dioxus_ssr::render(&dom).contains("aria-expanded=\"true\""),
        "returned state drives the rendered disclosure"
    );
}

#[derive(Clone)]
struct OpenFixture {
    capabilities: Rc<RefCell<HostCapabilities>>,
    target: OpenTarget,
    state: RequestState,
    actions: Rc<RefCell<Vec<HistoryEvent>>>,
}

fn open_fixture(props: OpenFixture) -> Element {
    rsx! { OpenButton { id: "native", record: reference(1), target: props.target, capabilities: props.capabilities.borrow().clone(), state: props.state, onaction: move |action| props.actions.borrow_mut().push(action) } }
}

#[test]
fn every_native_target_preserves_the_record_and_obeys_revocation_and_busy_state() {
    let targets = [
        (OpenTarget::Record, HostCapability::OpenRecord),
        (OpenTarget::Original, HostCapability::OpenOriginal),
        (OpenTarget::File, HostCapability::OpenFile),
        (OpenTarget::Diff, HostCapability::OpenDiff),
    ];
    for kind in [HostKind::Browser, HostKind::VsCode] {
        for (target, capability) in targets {
            let capabilities = Rc::new(RefCell::new(HostCapabilities::new(kind).with(capability)));
            for (other, _) in targets {
                assert_eq!(
                    capabilities.borrow().supports_history(other),
                    target == other,
                    "native adapters are negotiated independently"
                );
            }
            let actions = Rc::default();
            let props = OpenFixture {
                capabilities: Rc::clone(&capabilities),
                target,
                state: RequestState::Idle,
                actions: Rc::clone(&actions),
            };
            let (mut dom, mutations) = mount(open_fixture, props.clone());
            let button = button(&mutations, 0);
            click(&dom, button);
            assert!(
                matches!(actions.borrow().first(), Some(HistoryEvent::Open { record, target: actual }) if *record == reference(1) && *actual == target),
                "native actions retain operation, digest and intent"
            );
            *capabilities.borrow_mut() = HostCapabilities::new(kind);
            dom.mark_dirty(ScopeId::APP);
            dom.render_immediate(&mut dioxus_core::NoOpMutations);
            click(&dom, button);
            assert_eq!(
                actions.borrow().len(),
                1,
                "a revoked adapter blocks synthetic activation too"
            );
            assert!(
                dioxus_ssr::render(&dom).contains("aria-describedby=\"native-unavailable\""),
                "the unavailable explanation is associated with the control"
            );
            *capabilities.borrow_mut() = HostCapabilities::new(kind).with(capability);
            let (busy, changes) = mount(
                open_fixture,
                OpenFixture {
                    state: RequestState::Loading,
                    ..props
                },
            );
            click(&busy, self::button(&changes, 0));
            assert_eq!(
                actions.borrow().len(),
                1,
                "pending opens cannot dispatch twice"
            );
        }
    }
}

#[derive(Clone)]
struct PagingFixture {
    paging: Paging,
    actions: Rc<RefCell<Vec<HistoryEvent>>>,
}

fn paging_fixture(props: PagingFixture) -> Element {
    rsx! { ItemPaging { item: id(10), paging: props.paging, onaction: move |action| props.actions.borrow_mut().push(action) } }
}

#[test]
fn item_paging_retries_gaps_and_never_claims_content_completion() {
    for state in [
        RequestState::Idle,
        RequestState::Loading,
        RequestState::Failed(EffectError {
            message: "offline".into(),
        }),
    ] {
        let actions = Rc::default();
        let (dom, changes) = mount(
            paging_fixture,
            PagingFixture {
                paging: Paging {
                    state: state.clone(),
                    ..Paging::default()
                },
                actions: Rc::clone(&actions),
            },
        );
        click_buttons(&dom, &changes);
        assert_eq!(
            actions.borrow().len(),
            usize::from(state != RequestState::Loading),
            "paging requests wait for pending work"
        );
        if let Some(action) = actions.borrow().first() {
            assert!(
                matches!(action, HistoryEvent::LoadItem(key) if *key == id(10)),
                "retry addresses the full item"
            );
        }
    }
}

#[derive(Clone)]
struct LookupFixture {
    state: OperationDetailsState,
    actions: Rc<RefCell<Vec<HistoryEvent>>>,
}

fn lookup_fixture(props: LookupFixture) -> Element {
    rsx! { super::OperationPanel { id: "lookup", operation: id(20), state: props.state, original: true, capabilities: HostCapabilities::new(HostKind::Browser), open: RequestState::Idle, onaction: move |action| props.actions.borrow_mut().push(action) } }
}

#[test]
fn original_lookups_refresh_and_retry_through_typed_actions() {
    for state in [
        OperationDetailsState::Idle,
        OperationDetailsState::Loading,
        OperationDetailsState::Failed(EffectError {
            message: "connection lost".into(),
        }),
        OperationDetailsState::Ready(super::details()),
    ] {
        let actions = Rc::default();
        let (dom, changes) = mount(
            lookup_fixture,
            LookupFixture {
                state: state.clone(),
                actions: Rc::clone(&actions),
            },
        );
        click_buttons(&dom, &changes);
        assert_eq!(
            actions.borrow().len(),
            usize::from(state != OperationDetailsState::Loading),
            "loading cannot issue a duplicate lookup"
        );
        if let Some(action) = actions.borrow().first() {
            assert!(
                matches!(action, HistoryEvent::LoadOperationDetails { operation, refresh } if *operation == id(20) && *refresh == matches!(state, OperationDetailsState::Ready(_))),
                "Original requests preserve the requested identity and refresh intent"
            );
        }
    }
}
