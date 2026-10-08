//! Compact graph using the shared history geometry, selection and keyboard model.

use app_core::{Event, history, workspace::NavigationSection};
use dioxus::prelude::*;

use super::chrome::{Notice, navigate};
use crate::history::explorer::HistoryMini;

#[component]
pub(super) fn Activity(
    id: String,
    view: history::ViewModel,
    onaction: EventHandler<Event>,
) -> Element {
    rsx! {
        div { class: "idle-navigation-activity",
            if view.chain.is_some() {
                HistoryMini { id, view,
                    onaction: move |event| {
                        let selecting = matches!(event, history::Event::Timeline(history::timeline::Event::Select { .. }));
                        onaction.call(Event::History(event));
                        if selecting { navigate(onaction, NavigationSection::Activity); }
                    },
                    onopen: move |()| navigate(onaction, NavigationSection::Activity),
                }
            } else { Notice { text: "Activity not connected" } }
        }
    }
}
