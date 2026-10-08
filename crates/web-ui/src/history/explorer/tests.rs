//! Composition contracts over native rows and routing windows.

use app_core::history::ViewModel;
use dioxus::prelude::*;

use super::{HistoryExplorer, HistoryMini};
use crate::host::{HostCapabilities, HostKind};

fn gallery(view: ViewModel) -> Element {
    rsx! {
        HistoryExplorer { id: "editor", view: view.clone(), capabilities: HostCapabilities::new(HostKind::VsCode), onaction: |_| {} }
        HistoryMini { id: "mini", view, onaction: |_| {}, onopen: |()| {} }
    }
}

fn render(view: ViewModel) -> String {
    let mut dom = VirtualDom::new_with_props(gallery, view);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn both_compositions_escape_recorded_text_and_keep_unknown_time_explicit() {
    let mut native = native_window();
    let first = native.rows.first_mut().expect("native row");
    first.preview = "<script>captured_markup()</script>".into();
    first.author = "Recorded author".into();
    first.timestamp = None;
    let mut view = ViewModel::default();
    view.timeline.editor.window = Some(native.clone());
    view.timeline.mini.window = Some(native);
    let html = render(view);
    assert!(html.contains("&#60;script&#62;captured_markup()"));
    assert!(!html.contains("<script>captured_markup()"));
    assert!(html.contains("Recorded author"));
    assert!(html.contains("Time not recorded"));
    assert!(html.contains("History continues"));
    assert!(html.contains("Open Activity"));
}

#[test]
fn construction_reports_unknown_counts_and_retains_the_previous_snapshot() {
    let mut view = ViewModel::default();
    view.timeline.editor.state = app_core::history::RequestState::Loading;
    view.timeline.editor.progress = Some(
        serde_json::from_value(serde_json::json!({
            "build": "fixture-build", "stage": "Reading recorded activities",
            "processed": 250, "total": null, "previous_revision": null,
        }))
        .expect("native build progress"),
    );
    let html = render(view.clone());
    assert!(html.contains("Counting activities…"));
    assert!(!html.contains(">0 activities<"));
    let native = native_window();
    let previous_count = native.activities;
    view.timeline.editor.window = Some(native);
    let html = render(view);
    assert!(html.contains(&format!(
        "{previous_count} activities in the previous snapshot"
    )));
    assert!(html.contains("idle-timeline-row"));
}

pub(super) fn native_window() -> app_core::history::timeline::Window {
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../examples/history_explorer/timeline.json"
    ))
    .expect("native fixture JSON");
    serde_json::from_value(value.get("topology").expect("native topology").clone())
        .expect("timeline window")
}

#[test]
fn editor_keeps_the_continuous_table_and_exact_addresses_after_selection() {
    let native = native_window();
    let first = native.rows.first().expect("native row").clone();
    let mut view = ViewModel::default();
    view.timeline.mini.window = Some(native.clone());
    view.timeline.editor.window = Some(native);
    let html = render(view.clone());
    assert!(
        html.contains("idle-timeline-columns"),
        "editor renders aligned native columns"
    );
    assert!(
        html.contains("idle-history-mini-row"),
        "sidebar retains its separate composition"
    );
    assert!(
        html.contains(&first.address.record().expect("record target").record.hash),
        "native address remains exact"
    );
    assert!(
        !html.contains("idle-history-inspector"),
        "all rows belong to one continuous table"
    );
    view.timeline.selected = Some(app_core::history::timeline::Selection {
        occurrence: first.occurrence,
        address: first.address,
    });
    let html = render(view);
    assert!(
        html.contains("aria-selected=\"true\""),
        "selection stays visible in the table"
    );
    assert!(
        !html.contains("idle-history-inspector"),
        "selection never adds an inspector"
    );
}

#[test]
fn native_fixture_contains_all_children_nested_routes_and_folded_entry_exit_records() {
    let native = native_window();
    assert!(
        native.max_lane >= 10,
        "dense graph has readable independent lanes"
    );
    assert!(
        native.rows.iter().any(|row| row.graph.parents.len() == 11),
        "join retains ten children and the parent continuation"
    );
    assert!(
        native.rows.iter().any(|row| row
            .group
            .as_ref()
            .is_some_and(|group| !group.live && !group.expanded && group.entry != group.exit)
            && row.records.len() > 1),
        "folded groups retain constituent record addresses"
    );
    assert!(
        native
            .rows
            .iter()
            .any(|row| row.open == app_core::history::OpenTarget::Diff),
        "file changes have a native diff action"
    );
}
