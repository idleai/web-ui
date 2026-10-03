//! Complete host-composable destination surface.

use app_core::projections::{
    Event as ProjectionEvent, ProjectionAvailability, ProjectionFilter, ProjectionKind,
    ProjectionLoadState, ProjectionView, ViewModel,
};
use dioxus::prelude::*;

use super::{
    ProjectionBoard, ProjectionCards, ProjectionFeedback, ProjectionFilters, ProjectionList,
    ProjectionSummary, ProjectionTable, records::Gaps,
};
use crate::controls::Button;
use crate::status::EmptyState;

/// Pure presentation choice made by the host; semantic state stays in app-core.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProjectionLayout {
    /// Compact single-column list, suitable for a sidebar.
    #[default]
    List,
    /// Semantic table with an independently scrollable narrow viewport.
    Table,
    /// Cards wrapping to the available container width.
    Cards,
    /// Status columns, wrapping/stacking in narrow containers.
    Board,
}

/// A complete destination with counts, freshness, filters, load/error feedback,
/// record links and selection. Supply an ID unique across all mounted panels.
/// Dispatch callbacks as `app_core::Event::Projections(event)` and pass back the
/// resulting view. The component never filters, refreshes or selects on mount.
#[component]
pub fn ProjectionPanel(
    id: String,
    view: ViewModel,
    kind: ProjectionKind,
    #[props(default)] layout: ProjectionLayout,
    onaction: EventHandler<ProjectionEvent>,
) -> Element {
    let projection = super::destination(&view, kind).clone();
    let loading = view.load == ProjectionLoadState::Loading;
    let connected = view.context.is_some();
    let selected = view.selected.clone();
    let hidden_selection = selected.as_ref().is_some_and(|selection| {
        selection.kind == kind && !projection.rows.iter().any(|row| row.key == selection.key)
    });
    let show_results = connected
        && (!projection.rows.is_empty()
            || !matches!(
                view.load,
                ProjectionLoadState::Idle | ProjectionLoadState::Loading
            ));
    rsx! {
        section { id: id.clone(), class: "idle-projection-panel", aria_labelledby: "{id}-heading",
            header { h2 { id: "{id}-heading", "{super::title(kind)}" } }
            ProjectionFeedback { view, onaction }
            if connected {
                ProjectionSummary { view: projection.clone() }
                ProjectionFilters { id: "{id}-filters", view: projection.clone(), onaction }
                if selected.as_ref().is_some_and(|selection| selection.kind == kind) {
                    div { class: "idle-projection-selection",
                        if hidden_selection { p { role: "status", "Selected item is outside the current filters." } }
                        Button { label: "Clear selection", onpress: move |()| onaction.call(ProjectionEvent::Select(None)) }
                    }
                }
            }
            div { class: "idle-projection-results", aria_busy: loading.to_string(),
                if show_results {
                    if projection.rows.is_empty() {
                        ResultEmpty { view: projection.clone() }
                    } else {
                        {match layout {
                            ProjectionLayout::List => rsx! { ProjectionList { view: projection.clone(), selected: selected.clone(), onaction } },
                            ProjectionLayout::Table => rsx! { ProjectionTable { view: projection.clone(), selected: selected.clone(), onaction } },
                            ProjectionLayout::Cards => rsx! { ProjectionCards { view: projection.clone(), selected: selected.clone(), onaction } },
                            ProjectionLayout::Board => rsx! { ProjectionBoard { view: projection.clone(), selected: selected.clone(), onaction } },
                        }}
                    }
                    Gaps { gaps: projection.gaps }
                }
            }
        }
    }
}

#[component]
fn ResultEmpty(view: ProjectionView) -> Element {
    let (title, message) = if view.availability == ProjectionAvailability::Unavailable {
        (
            "Projection unavailable",
            "The provider has not supplied this projection.",
        )
    } else if view.filter != ProjectionFilter::default() && view.loaded_count > 0 {
        (
            "No matching items",
            "Clear or change the filters to see the loaded items.",
        )
    } else if view.availability == ProjectionAvailability::Partial {
        (
            "No items loaded",
            "This result is incomplete. See the supplied limitations below.",
        )
    } else {
        (
            "No items",
            "The provider returned an empty result for this scope.",
        )
    };
    rsx! { EmptyState { title, message } }
}
