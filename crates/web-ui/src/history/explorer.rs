//! Editor history and sidebar activity share summaries, graph geometry and actions.
//!
//! Load [`STYLESHEET`] with the theme, graph and details stylesheets. The editor
//! fills its containing block; hosts must give that block a definite height.

mod search;
mod summary;
mod timeline;

use app_core::history::{
    Event, ViewModel,
    timeline::{Event as TimelineEvent, Surface},
};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{
    Element, EventHandler, Key, Modifiers, ModifiersInteraction, Props, component, dioxus_core,
    dioxus_elements, rsx, use_effect, use_reactive,
};

use crate::host::HostCapabilities;
use crate::provenance::ActivitySnapshot;

/// Styles for the table editor and compact sidebar, included in `assets::STYLESHEET`.
pub const STYLESHEET: &str = include_str!("../../assets/history-explorer.css");

/// Continuous Activity table with indexed Find and normal native editor actions.
#[component]
pub fn HistoryExplorer(
    id: String,
    view: ViewModel,
    capabilities: HostCapabilities,
    onaction: EventHandler<Event>,
    activity: Option<ActivitySnapshot>,
) -> Element {
    drop(activity);
    let _load = use_effect(use_reactive((&view.chain,), move |(chain,)| {
        if chain.is_some() {
            onaction.call(Event::Timeline(TimelineEvent::Load(Surface::Editor)));
        }
    }));
    let search_id = format!("{id}-search");
    let focus_id = search_id.clone();
    let native = capabilities.supports_history(app_core::history::OpenTarget::OperationJson);
    rsx! {
        section { class: "idle-history-explorer", aria_label: "Activity history editor",
            onkeydown: move |event| {
                if event.modifiers().intersects(Modifiers::CONTROL | Modifiers::META)
                    && matches!(event.key(), Key::Character(value) if value.eq_ignore_ascii_case("f")) {
                    event.prevent_default(); event.stop_propagation(); focus(&focus_id);
                }
            },
            search::HistorySearch { key: "{view.chain:?}", id: search_id, view: view.timeline.editor.search.clone(),
                refreshing: view.timeline.editor.state == app_core::history::RequestState::Loading, onaction }
            if !native { p { class: "idle-timeline-host-note", "Native file and operation editors are unavailable in this host." } }
            timeline::TimelineTable { key: "{view.chain:?}", id, view: view.timeline, onaction }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn focus(id: &str) {
    use wasm_bindgen::JsCast;
    if let Some(element) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id(id))
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let options = web_sys::FocusOptions::new();
        options.set_prevent_scroll(true);
        let _focused = element.focus_with_options(&options);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn focus(_id: &str) {}

/// Short activity rows for a narrow host pane. Hosts open the editor on selection.
/// Uses native timeline rows and routing with an independent bounded viewport.
#[component]
pub fn HistoryMini(
    id: String,
    view: ViewModel,
    onaction: EventHandler<Event>,
    onopen: EventHandler<()>,
    #[props(default = 176)] height: u32,
) -> Element {
    rsx! {
        timeline::MiniTimeline { key: "{view.chain:?}", id, chain: view.chain,
            view: view.timeline, height, onaction, onopen }
    }
}

#[cfg(test)]
mod tests;
