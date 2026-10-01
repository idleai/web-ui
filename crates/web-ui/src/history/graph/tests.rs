//! Relationship preservation, viewport continuity and Dioxus interaction checks.

use std::cell::RefCell;
use std::rc::Rc;

use app_core::history::{
    ActivityKind, Detail, Endpoint, Event as HistoryEvent, ItemView, ObservationView, Paging,
    RecordRef, Selected, ViewModel,
};
use dioxus::prelude::*;
use dioxus_core::{ElementId, Mutation, Mutations};
use dioxus_html::SerializedHtmlEventConverter;

use super::HistoryGraph;
use super::model::{ConnectionKind, EndpointState, GraphSnapshot};
use super::paths;
use super::state::State;
use super::viewport::Viewport;

fn observation(operation: &str, item: &str, time: u64, parents: &[&str]) -> ObservationView {
    ObservationView {
        record: RecordRef {
            operation: operation.into(),
            hash: format!("hash-{operation}"),
        },
        item: item.into(),
        kind: ActivityKind::Message,
        author: Some("author".into()),
        recorder: Some("recorder".into()),
        session: Some("session".into()),
        turn: None,
        time_ms: Some(time),
        sequence: None,
        parents: parents.iter().map(|parent| (*parent).into()).collect(),
        causes: Vec::new(),
        original: None,
        converter: None,
        legacy: None,
        detail: Detail::Other,
        preview: None,
        problem: None,
    }
}

fn item(key: &str, time: u64, parents: &[&str]) -> ItemView {
    ItemView {
        key: key.into(),
        observations: vec![observation(key, key, time, parents)],
        blocks: Vec::new(),
        expanded: false,
        paging: Paging::default(),
    }
}

fn view(items: Vec<ItemView>) -> ViewModel {
    ViewModel {
        chain: Some("chain-a".into()),
        items,
        ..ViewModel::default()
    }
}

#[test]
fn every_parent_and_logical_relation_retains_its_full_record_reference() {
    let mut child = item("child", 20, &["p1", "p2", "p3", "late"]);
    child.observations.first_mut().unwrap().causes = vec!["p1".into()];
    child
        .observations
        .push(observation("child-update", "child", 21, &["child"]));
    let mut link = item("link", 1, &[]);
    link.observations.first_mut().unwrap().detail = Detail::Link {
        from: Endpoint::Item("p1".into()),
        relation: "related-to".into(),
        to: vec![
            Endpoint::Observation("child-update".into()),
            Endpoint::Git {
                repository: "18446744073709551615".into(),
                oid: "full-git-id".into(),
            },
        ],
    };
    let graph = GraphSnapshot::from_view(&view(vec![
        child,
        item("p1", 1, &[]),
        item("p2", 2, &[]),
        item("p3", 3, &[]),
        link,
    ]));
    assert_eq!(
        graph.connections.len(),
        8,
        "all five physical parents, one cause and two Link destinations survive"
    );
    assert_eq!(
        graph
            .connections
            .iter()
            .filter(|connection| connection.kind == ConnectionKind::Causal)
            .count(),
        5,
        "Links and causes cannot change causal ancestry"
    );
    let missing = graph
        .connections
        .iter()
        .find(|connection| connection.to == Endpoint::Observation("late".into()))
        .unwrap();
    assert_eq!(
        missing.target,
        EndpointState::Unloaded,
        "missing endpoints remain explicit"
    );
    assert_eq!(
        missing.record,
        RecordRef {
            operation: "child".into(),
            hash: "hash-child".into()
        },
        "rendered connections keep the immutable record digest"
    );
    assert!(
        graph
            .connections
            .iter()
            .any(|connection| connection.target == EndpointState::ExternalGit),
        "repository-scoped Git endpoints are retained without invented item aliases"
    );
    assert_eq!(
        graph.items.first().unwrap().observations.len(),
        2,
        "grouping by item never drops observations"
    );
}

#[test]
fn late_endpoints_resolve_in_place_and_conflicting_mappings_never_choose_a_winner() {
    let mut initial = view(vec![item("child", 10, &["parent-record"])]);
    let before = GraphSnapshot::from_view(&initial).connections.remove(0);
    let mut parent = item("parent-item", 20, &[]);
    parent.observations.first_mut().unwrap().record.operation = "parent-record".into();
    initial.items.push(parent.clone());
    let after = GraphSnapshot::from_view(&initial).connections.remove(0);
    assert_eq!(
        before.key, after.key,
        "endpoint arrival keeps the connection's stable identity"
    );
    assert_eq!(
        after.target,
        EndpointState::Loaded("parent-item".into()),
        "physical parents resolve through observation membership"
    );
    parent.key = "conflicting-item".into();
    initial.items.push(parent);
    let conflict = GraphSnapshot::from_view(&initial).connections.remove(0);
    assert_eq!(
        conflict.target,
        EndpointState::Ambiguous,
        "ambiguous membership must not attach a line to an arbitrary item"
    );
}

#[test]
fn typed_endpoint_domains_and_record_variants_have_distinct_connection_keys() {
    let mut relation = item("link", 1, &[]);
    relation.observations.first_mut().unwrap().detail = Detail::Link {
        from: Endpoint::Item("same".into()),
        relation: "r:1".into(),
        to: vec![
            Endpoint::Item("same".into()),
            Endpoint::Observation("same".into()),
            Endpoint::Git {
                repository: "repo".into(),
                oid: "same".into(),
            },
        ],
    };
    let mut variant = relation.observations.first().unwrap().clone();
    variant.record.hash = "other-full-digest".into();
    relation.observations.push(variant);
    let snapshot = GraphSnapshot::from_view(&view(vec![relation]));
    let keys: std::collections::BTreeSet<_> = snapshot
        .connections
        .iter()
        .map(|connection| &connection.key)
        .collect();
    assert_eq!(
        keys.len(),
        6,
        "neither domains nor retained record variants can collide"
    );
}

#[test]
fn links_do_not_reorder_or_reassign_causal_lanes_and_retractions_remove_paths() {
    let mut data = view(vec![item("child", 3, &["parent"]), item("parent", 1, &[])]);
    let mut state = State::default();
    state.reconcile(&data, 0.0);
    let order = state.layout.order.clone();
    let lanes = state.layout.lanes.clone();
    data.items
        .first_mut()
        .unwrap()
        .observations
        .first_mut()
        .unwrap()
        .detail = Detail::Link {
        from: Endpoint::Item("parent".into()),
        relation: "reverse-logical-link".into(),
        to: vec![Endpoint::Item("child".into())],
    };
    state.reconcile(&data, 10.0);
    assert_eq!(
        state.layout.order, order,
        "a reverse Link cannot impose a causal ordering clock"
    );
    assert_eq!(
        state.layout.lanes, lanes,
        "logical Links have their own routing layer"
    );
    data.items
        .first_mut()
        .unwrap()
        .observations
        .first_mut()
        .unwrap()
        .parents
        .clear();
    state.reconcile(&data, 20.0);
    assert!(
        state
            .snapshot
            .connections
            .iter()
            .all(|connection| matches!(connection.kind, ConnectionKind::Link(_))),
        "a retracted parent must disappear while the unrelated Link remains"
    );
}

#[test]
fn grouping_observations_can_create_cycles_without_hiding_any_relationship() {
    let mut a = item("a", 1, &[]);
    a.observations.push(observation("a-again", "a", 3, &["b"]));
    let data = view(vec![a, item("b", 2, &["a"])]);
    let mut state = State::default();
    state.reconcile(&data, 0.0);
    assert_eq!(
        state.layout.order.len(),
        2,
        "both logical items remain renderable"
    );
    assert_eq!(
        state.snapshot.connections.len(),
        2,
        "both directions retain their records"
    );
    assert!(
        state.layout.warning.is_some(),
        "grouped cycles explain their alternate rendering order"
    );
    assert_eq!(
        paths::visible(
            &state.snapshot.connections,
            &state.layout,
            &state.viewport,
            &state.births
        )
        .len(),
        2,
        "upward connections must be drawn too"
    );
}

fn keys(count: usize) -> Vec<String> {
    (0..count).map(|index| format!("item-{index}")).collect()
}

#[test]
fn inserts_resizes_and_retractions_preserve_the_visible_item_offset() {
    let mut viewport = Viewport::default();
    let _resized = viewport.resize(600.0, 200.0);
    let mut order = keys(100);
    viewport.update(&order);
    viewport.scroll(817.0);
    order.insert(0, "late-lower-id".into());
    viewport.update(&order);
    assert!(
        (viewport.top - 857.0).abs() < 0.01,
        "the anchor stays 17 pixels into item-20 after an insertion above it"
    );
    assert!(
        viewport.measure("item-1", 140.0),
        "variable-height content is measured"
    );
    assert!(
        (viewport.top - 957.0).abs() < 0.01,
        "expansion above the viewport preserves the same anchor"
    );
    order.retain(|key| key != "item-20");
    viewport.update(&order);
    let row = viewport.row("item-21").unwrap();
    assert!(
        (row.top - viewport.top - 23.0).abs() < 0.01,
        "if the anchor retracts, the next surviving item keeps its screen position"
    );
}

#[test]
fn virtualization_bounds_rows_and_keeps_the_active_descendant_mounted() {
    let mut viewport = Viewport::default();
    viewport.update(&keys(10_000));
    viewport.focus(1);
    viewport.scroll(200_000.0);
    let mounted = viewport.mounted_indices();
    assert!(
        mounted.len() <= 21,
        "DOM rows stay bounded by the viewport and overscan, plus focus"
    );
    assert!(
        mounted.contains(&1),
        "an offscreen active descendant stays in the accessibility tree"
    );
    viewport.scroll(f64::NAN);
    assert!(
        viewport.top.is_finite(),
        "invalid browser coordinates cannot poison the layout"
    );
}

#[test]
fn late_causal_parent_reordering_preserves_scroll_and_focus_by_item() {
    let mut data = view(
        (0..100_u64)
            .map(|time| item(&format!("item-{time}"), time, &[]))
            .collect(),
    );
    data.items
        .iter_mut()
        .find(|item| item.key == "item-50")
        .unwrap()
        .observations
        .first_mut()
        .unwrap()
        .parents
        .push("late-parent".into());
    let mut state = State::default();
    state.reconcile(&data, 0.0);
    state.viewport.scroll(1607.0);
    let before = state
        .viewport
        .rows
        .iter()
        .find(|row| row.top <= state.viewport.top && row.top + row.height > state.viewport.top)
        .unwrap()
        .clone();
    let focused = state.viewport.focused.clone();
    data.items.push(item("late-parent", 200, &[]));
    state.reconcile(&data, 50.0);
    let after = state.viewport.row(&before.key).unwrap();
    assert!(
        (after.top - state.viewport.top - (before.top - 1607.0)).abs() < 0.01,
        "causal repair does not move the user's visible item"
    );
    assert_eq!(
        state.viewport.focused, focused,
        "focus follows identity through causal clock repairs"
    );
    assert!(
        state
            .snapshot
            .connections
            .iter()
            .all(|connection| !connection.unresolved()),
        "the late observation resolves its pending edge"
    );
}

#[test]
fn pending_selection_reveals_when_it_arrives_and_a_chain_switch_clears_geometry() {
    let mut data = view(vec![item("first", 1, &[])]);
    data.selected = Selected {
        item: Some("later".into()),
        observation: Some("later-op".into()),
    };
    let mut state = State::default();
    state.reconcile(&data, 0.0);
    data.items.push(item("later", 2, &[]));
    state.reconcile(&data, 10.0);
    assert_eq!(
        state.viewport.focused.as_deref(),
        Some("later"),
        "selection resolves without replacing the selected observation"
    );
    data.chain = Some("chain-b".into());
    data.items = vec![item("unrelated", 1, &[])];
    data.selected = Selected::default();
    state.reconcile(&data, 20.0);
    assert_eq!(
        state.layout.order,
        vec!["unrelated"],
        "geometry cannot leak between chains"
    );
    assert!(
        state.births.is_empty() && state.node_births.is_empty(),
        "new contexts do not animate as live updates to the old chain"
    );
}

#[derive(Clone)]
struct Fixture {
    view: Rc<RefCell<ViewModel>>,
    actions: Rc<RefCell<Vec<HistoryEvent>>>,
}

fn fixture(props: Fixture) -> Element {
    rsx! { HistoryGraph { id: "test-history", view: props.view.borrow().clone(), onaction: move |action| props.actions.borrow_mut().push(action) } }
}

fn listener(mutations: &Mutations, name: &str) -> ElementId {
    mutations
        .edits
        .iter()
        .find_map(|mutation| {
            if let Mutation::NewEventListener { name: actual, id } = mutation {
                (*actual == name).then_some(*id)
            } else {
                None
            }
        })
        .expect("the component mounts the requested event listener")
}

#[test]
fn dioxus_emits_typed_keyboard_actions_and_uses_stable_item_ids() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let data = Rc::new(RefCell::new(view(vec![
        item("parent", 1, &[]),
        item("child", 2, &["parent"]),
    ])));
    let actions = Rc::new(RefCell::new(Vec::new()));
    let mut dom = VirtualDom::new_with_props(
        fixture,
        Fixture {
            view: Rc::clone(&data),
            actions: Rc::clone(&actions),
        },
    );
    let mutations = dom.rebuild_to_vec();
    let target = listener(&mutations, "keydown");
    for (key, code) in [
        (Key::ArrowDown, Code::ArrowDown),
        (Key::Enter, Code::Enter),
        (Key::ArrowRight, Code::ArrowRight),
    ] {
        let data: Rc<dyn std::any::Any> = Rc::new(PlatformEventData::new(Box::new(
            SerializedKeyboardData::new(
                key,
                code,
                Location::Standard,
                false,
                Modifiers::empty(),
                false,
            ),
        )));
        let event = Event::new(data, true);
        dom.runtime().handle_event("keydown", event.clone(), target);
        assert!(
            !event.default_action_enabled(),
            "handled navigation must not scroll the host page"
        );
    }
    assert!(
        matches!(actions.borrow().first(), Some(HistoryEvent::Select(Selected { item: Some(key), observation: None })) if key == "parent"),
        "keyboard selection sends a logical item to app-core"
    );
    assert!(
        matches!(actions.borrow().get(1), Some(HistoryEvent::ToggleDisclosure(key)) if key == "parent"),
        "disclosure stays in app-core"
    );
    let html = dioxus_ssr::render(&dom);
    assert!(
        html.contains("data-item-key=\"parent\"")
            && html.contains("data-record-hash=\"hash-parent\""),
        "DOM identity and exact record identity remain separate"
    );
    data.borrow_mut()
        .items
        .iter_mut()
        .find(|item| item.key == "parent")
        .unwrap()
        .observations
        .push(observation("parent-update", "parent", 3, &[]));
    dom.mark_dirty(ScopeId::APP);
    let update = dom.render_immediate_to_vec();
    assert!(
        !update
            .edits
            .iter()
            .any(|mutation| matches!(mutation, Mutation::Remove { .. })),
        "new observations update existing keyed rows without unmounting them"
    );
}

#[test]
fn static_graph_mounts_only_the_window_and_exposes_partial_records() {
    let mut data = view(
        (0..2_000_u64)
            .map(|time| item(&format!("item-{time}"), time, &[]))
            .collect(),
    );
    data.items
        .last_mut()
        .unwrap()
        .observations
        .first_mut()
        .unwrap()
        .problem = Some("Quarantined record".into());
    let mut dom = VirtualDom::new_with_props(
        fixture,
        Fixture {
            view: Rc::new(RefCell::new(data)),
            actions: Rc::default(),
        },
    );
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    assert!(
        html.matches("role=\"treeitem\"").count() <= 20,
        "initial rendering virtualizes large app-core windows"
    );
    assert!(
        html.contains("Quarantined record") && html.contains("Record needs attention"),
        "record problems remain available to sighted and screen-reader users"
    );
    assert!(
        html.contains("aria-setsize=\"2000\"") && html.contains("aria-activedescendant="),
        "virtualized tree rows expose total count and stable focus"
    );
}
