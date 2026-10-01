//! Development-only scripted history views for both host compositions.

#[path = "history_details/fixture.rs"]
mod fixture;

use app_core::history::{
    Event as HistoryEvent, ItemView, OperationDetailsState, OperationDetailsView, RequestState,
    ViewModel,
};
use dioxus::prelude::*;
use dioxus_html as dioxus_elements;
use dioxus_ssr as _;
use history_geometry as _;
#[cfg(target_arch = "wasm32")]
use web_sys as _;

use web_ui::controls::Button;
use web_ui::history::details::HistoryTimeline;
use web_ui::host::{HostCapabilities, HostCapability, HostKind};
use web_ui::theme::{Theme, ThemeProvider};

/// Mount the local scripted fixture with no production connections.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    dioxus_web::launch::launch(Gallery, Vec::new(), Vec::new());
}

/// Browser and extension use the same typed views and actions.
#[component]
pub fn Gallery() -> Element {
    let mut view = use_signal(fixture::view);
    let mut last_action = use_signal(|| "No history action dispatched".to_owned());
    let mut adapters = use_signal(|| true);
    let mut late_bytes = use_signal(|| false);
    let dispatch = move |action: HistoryEvent| {
        last_action.set(format!("{action:?}"));
        apply(&mut view.write(), action, late_bytes());
    };
    let extension = if adapters() {
        HostCapabilities::new(HostKind::VsCode)
            .with(HostCapability::OpenRecord)
            .with(HostCapability::OpenOriginal)
            .with(HostCapability::OpenFile)
            .with(HostCapability::OpenDiff)
    } else {
        HostCapabilities::new(HostKind::VsCode)
    };
    rsx! {
        ThemeProvider { theme: Theme::Light,
            main { class: "details-gallery",
                h1 { "History content" }
                p { "Scripted app-core views · browser and extension · native actions are logged only" }
                div { class: "fixture-actions",
                    Button { label: "Make late bytes available", onpress: move |()| late_bytes.set(true) }
                    Button { label: "Revoke native adapters", onpress: move |()| adapters.set(false) }
                    Button { label: "Report native failure", onpress: move |()| view.write().open = RequestState::Failed(app_core::module::EffectError { message: "Fixture editor is offline".into() }) }
                }
                p { class: "fixture-action", role: "status", "{last_action}" }
                p { class: "fixture-availability", "Late bytes available to next lookup: {late_bytes}" }
                div { class: "fixture-compositions",
                    section { h2 { "Browser" }
                        HistoryTimeline { id: "browser-history", view: view(), capabilities: HostCapabilities::new(HostKind::Browser), onaction: dispatch, height: 440 }
                    }
                    ThemeProvider { theme: Theme::Dark,
                        section { h2 { "Extension" }
                            HistoryTimeline { id: "extension-history", view: view(), capabilities: extension, onaction: dispatch, height: 440 }
                        }
                    }
                }
            }
        }
    }
}

fn load(view: &mut ViewModel, operation: &str, late: bool) {
    view.operation_details
        .retain(|entry| entry.operation != operation);
    view.operation_details.push(OperationDetailsView {
        operation: operation.into(),
        state: OperationDetailsState::Ready(fixture::details(operation, late)),
    });
}

fn apply(view: &mut ViewModel, event: HistoryEvent, late: bool) {
    if let HistoryEvent::Select(selected) = event {
        if let Some(operation) = &selected.observation {
            load(view, operation, late);
        }
        view.selected = selected;
    } else if let HistoryEvent::ToggleDisclosure(key) = event {
        if let Some(item) = view.items.iter_mut().find(|item| item.key == key) {
            item.expanded = !item.expanded;
        }
    } else if let HistoryEvent::LoadItem(key) = event {
        if let Some(item) = view.items.iter_mut().find(|item| item.key == key) {
            item.paging = fixture::complete_scan();
        }
    } else if let HistoryEvent::LoadOperationDetails { operation, .. } = event {
        load(view, &operation, late);
    } else if matches!(event, HistoryEvent::Open { .. }) {
        view.open = RequestState::Loading;
    }
    view.selected_item = view
        .items
        .iter()
        .find(|item| Some(&item.key) == view.selected.item.as_ref())
        .cloned();
    view.expanded = view
        .items
        .iter()
        .filter(|item| item.expanded)
        .map(|item: &ItemView| item.key.clone())
        .collect();
}
