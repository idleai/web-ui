//! Compact graph using the shared history geometry, selection and keyboard model.

use app_core::{
    Event,
    history::{self, ItemView},
    workspace::NavigationSection,
};
use dioxus::prelude::*;

use super::chrome::{Notice, navigate};
use crate::history::graph::HistoryGraph;

#[component]
pub(super) fn Activity(
    id: String,
    view: history::ViewModel,
    onaction: EventHandler<Event>,
) -> Element {
    rsx! {
        div { class: "idle-navigation-activity",
            if view.chain.is_some() {
                HistoryGraph { id, view, height: 176, compact: true, label: "Recent workspace activity",
                    render_item: move |item: ItemView| {
                        let title = item.observations.iter().find_map(|record| record.preview.as_ref()).map_or_else(|| item.key.clone(), |preview| preview.text.clone());
                        rsx! { span { class: "idle-navigation-activity-title", title: title.clone(), "{title}" } }
                    },
                    onaction: move |event| {
                        let selecting = matches!(event, history::Event::Select(_));
                        onaction.call(Event::History(event));
                        if selecting { navigate(onaction, NavigationSection::Activity); }
                    },
                }
            } else { Notice { text: "Activity not connected" } }
        }
    }
}
