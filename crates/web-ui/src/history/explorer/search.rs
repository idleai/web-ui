//! Indexed in-place Find keeps the loaded timeline readable.

use app_core::history::{
    Event as HistoryEvent, RequestState,
    timeline::{Event, Search, Surface},
};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{
    Element, EventHandler, Key, Modifiers, ModifiersInteraction, Props, WritableExt, component,
    dioxus_core, dioxus_elements, rsx, use_effect, use_reactive, use_signal,
};

use crate::{
    controls::{ControlState, IconButton},
    icons::IconName,
};

#[component]
pub(super) fn HistorySearch(
    id: String,
    view: Search,
    refreshing: bool,
    onaction: EventHandler<HistoryEvent>,
) -> Element {
    let mut draft = use_signal(|| view.text.clone());
    let _sync = use_effect(use_reactive((&view.text,), move |(text,)| draft.set(text)));
    let busy = view.state == RequestState::Loading;
    let navigation = if view.total == 0 || busy {
        ControlState::Disabled
    } else {
        ControlState::Ready
    };
    let current = view.current.map_or(0, |index| index.saturating_add(1));
    let submitted = view.text.clone();
    let input_id = id.clone();
    let clear = move |()| {
        draft.set(String::new());
        super::focus(&input_id);
        onaction.call(HistoryEvent::Timeline(Event::Find {
            surface: Surface::Editor,
            text: String::new(),
        }));
    };
    let mut clear_key = clear.clone();
    rsx! {
        div { class: "idle-history-search",
            div { class: "idle-history-search-toolbar",
                div { class: "idle-history-find",
                    input { id, r#type: "search", class: "idle-timeline-find-input", value: draft(), maxlength: "4096",
                        aria_label: "Find in activity", placeholder: "Find in activity", spellcheck: "false",
                        oninput: move |event| draft.set(event.value()),
                        onkeydown: move |event| {
                            if event.is_composing() { return; }
                            if event.key() == Key::Enter {
                                event.prevent_default(); event.stop_propagation();
                                if draft() == submitted {
                                    onaction.call(HistoryEvent::Timeline(Event::Match { surface: Surface::Editor, delta: if event.modifiers().contains(Modifiers::SHIFT) { -1 } else { 1 } }));
                                } else {
                                    onaction.call(HistoryEvent::Timeline(Event::Find { surface: Surface::Editor, text: draft() }));
                                }
                            } else if event.key() == Key::Escape {
                                event.prevent_default(); event.stop_propagation(); clear_key(());
                            }
                        },
                    }
                    IconButton { label: "Find in activity", icon: IconName::Search,
                        onpress: move |()| onaction.call(HistoryEvent::Timeline(Event::Find { surface: Surface::Editor, text: draft() })) }
                    if !view.text.is_empty() {
                        span { class: "idle-history-match-count", role: "status", if busy { "Finding…" } else { "{current} of {view.total}" } }
                        span { class: "idle-history-previous",
                            IconButton { label: "Previous match", icon: IconName::ChevronRight, state: navigation,
                                onpress: move |()| onaction.call(HistoryEvent::Timeline(Event::Match { surface: Surface::Editor, delta: -1 })) }
                        }
                        IconButton { label: "Next match", icon: IconName::ChevronRight, state: navigation,
                            onpress: move |()| onaction.call(HistoryEvent::Timeline(Event::Match { surface: Surface::Editor, delta: 1 })) }
                        IconButton { label: "Clear activity Find", icon: IconName::Close, onpress: clear }
                    }
                }
                IconButton { label: "Refresh activity", icon: IconName::Refresh,
                    state: if refreshing { ControlState::Busy } else { ControlState::Ready },
                    onpress: move |()| onaction.call(HistoryEvent::Timeline(Event::Refresh(Surface::Editor))) }
            }
            if let RequestState::Failed(error) = &view.state { p { class: "idle-history-search-feedback", role: "alert", "{error.message}" } }
            if view.unavailable > 0 { span { class: "idle-history-search-feedback", role: "status", "{view.unavailable} records have unavailable text." } }
        }
    }
}
