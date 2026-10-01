//! Development fixture for the graph's typed input and event boundary.

use app_core::history::{
    ActivityKind, Detail, Endpoint, Event as HistoryEvent, ItemView, ObservationView, Paging,
    RecordRef, RequestState, ViewModel,
};
use dioxus::prelude::*;
use dioxus_html as dioxus_elements;
use dioxus_ssr as _;
use history_geometry as _;
#[cfg(target_arch = "wasm32")]
use web_sys as _;

use web_ui::controls::Button;
use web_ui::history::graph::HistoryGraph;
use web_ui::theme::{Theme, ThemeProvider};

/// Mount the local fixture, with no host connection or production data source.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    dioxus_web::launch::launch(Gallery, Vec::new(), Vec::new());
}

/// Two host compositions over the same app-core-shaped fixture.
#[component]
pub fn Gallery() -> Element {
    let mut view = use_signal(fixture);
    let mut narrow = use_signal(|| false);
    let mut updates = use_signal(|| 0_u64);
    let mut last_action =
        use_signal(|| "Choose an item, then use arrows, Enter and left/right.".to_owned());
    let dispatch = move |action: HistoryEvent| {
        last_action.set(format!("{action:?}"));
        if let HistoryEvent::Select(selected) = action {
            view.write().selected = selected;
        } else if let HistoryEvent::ToggleDisclosure(key) = action {
            if let Some(item) = view.write().items.iter_mut().find(|item| item.key == key) {
                item.expanded = !item.expanded;
            }
        } else if matches!(action, HistoryEvent::LoadMore) {
            view.write().paging.exhausted = true;
        }
    };
    let content = Callback::new(move |item: ItemView| {
        rsx! {
            div { class: "fixture-item",
                strong { "Recorded activity" }
                code { "{item.key}" }
                if item.expanded {
                    p { "Expanded context for this logical item. Its complete observation references remain attached while the graph measures this content." }
                    for record in &item.observations { p { "Observation: {record.record.operation}" } }
                }
            }
        }
    });
    rsx! {
        ThemeProvider { theme: Theme::Light,
            main { class: "history-gallery",
                h1 { "Activity graph" }
                p { "Browser and extension compositions · 300 recorded items · fixture data" }
                div { class: "fixture-actions",
                    Button { label: "Insert late record", onpress: move |()| {
                        let next = updates().saturating_add(1); updates.set(next);
                        view.write().items.push(item(10_000_u64.saturating_add(next), 155, &[]));
                    } }
                    Button { label: "Resolve endpoints", onpress: move |()| {
                        let mut view = view.write();
                        if !view.items.iter().any(|item| item.key == id(900)) { view.items.push(item(900, 100, &[])); }
                    } }
                    Button { label: "Retract selected item", onpress: move |()| {
                        let selected = view.read().selected.item.clone();
                        view.write().items.retain(|item| Some(&item.key) != selected.as_ref());
                    } }
                    Button { label: "Toggle narrow layout", onpress: move |()| { let next = !narrow(); narrow.set(next); } }
                }
                p { class: "fixture-action", role: "status", "{last_action}" }
                div { class: "fixture-compositions", "data-narrow": narrow().to_string(),
                    section { h2 { "Browser" }
                        HistoryGraph { id: "browser-history", view: view(), onaction: dispatch, render_item: content, height: 480 }
                    }
                    ThemeProvider { theme: Theme::Dark,
                        section { h2 { "Extension" }
                            HistoryGraph { id: "extension-history", view: view(), onaction: dispatch, height: 480 }
                        }
                    }
                }
            }
        }
    }
}

fn id(value: u64) -> String {
    format!("{value:064x}")
}

fn item(value: u64, time: u64, parents: &[u64]) -> ItemView {
    let key = id(value);
    ItemView {
        key: key.clone(),
        expanded: false,
        paging: Paging::default(),
        blocks: Vec::new(),
        observations: vec![ObservationView {
            record: RecordRef {
                operation: key.clone(),
                hash: id(value.saturating_add(20_000)),
            },
            item: key,
            kind: ActivityKind::Message,
            author: Some(id(1)),
            recorder: Some(id(2)),
            session: Some(id(3)),
            turn: None,
            time_ms: Some(time),
            sequence: Some(value),
            parents: parents.iter().copied().map(id).collect(),
            causes: Vec::new(),
            original: None,
            converter: None,
            legacy: None,
            detail: Detail::Other,
            preview: None,
            problem: None,
        }],
    }
}

fn fixture() -> ViewModel {
    let mut items: Vec<_> = (1..=300_u64)
        .map(|value| {
            item(
                value,
                value,
                &if value > 4 {
                    vec![value.saturating_sub(4)]
                } else {
                    Vec::new()
                },
            )
        })
        .collect();
    if let Some(record) = items
        .last_mut()
        .and_then(|item| item.observations.first_mut())
    {
        record.parents.extend([id(295), id(294), id(900)]);
        record.causes.push(id(290));
        record.detail = Detail::Link {
            from: Endpoint::Item(id(300)),
            relation: "references".into(),
            to: vec![Endpoint::Item(id(284)), Endpoint::Observation(id(900))],
        };
    }
    ViewModel {
        chain: Some("local-gallery".into()),
        items,
        paging: Paging {
            state: RequestState::Ready,
            exhausted: true,
            ..Paging::default()
        },
        ..ViewModel::default()
    }
}
