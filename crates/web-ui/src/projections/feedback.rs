//! Read lifecycle and supplied result metadata.

use app_core::projections::{
    Event as ProjectionEvent, FreshnessStatus, ProjectionAvailability, ProjectionLoadState,
    ProjectionView, ViewModel,
};
use dioxus::prelude::*;

use crate::controls::{Button, ControlState};
use crate::status::{LoadingState, StatusBadge, StatusTone};

/// Read feedback and explicit refresh intent; mounting never requests a read.
/// Suspended connections are resumed by the host's connection lifecycle.
#[component]
pub fn ProjectionFeedback(view: ViewModel, onaction: EventHandler<ProjectionEvent>) -> Element {
    let state = if view.context.is_none() || view.load == ProjectionLoadState::Suspended {
        ControlState::Disabled
    } else if view.load == ProjectionLoadState::Loading {
        ControlState::Busy
    } else {
        ControlState::Ready
    };
    rsx! {
        div { class: "idle-projection-feedback",
            {match &view.load {
                ProjectionLoadState::Idle => rsx! { p { "Connect a workspace to load projections." } },
                ProjectionLoadState::Loading => rsx! { LoadingState { label: "Loading projections…" } },
                ProjectionLoadState::Ready => rsx! {},
                ProjectionLoadState::Suspended => rsx! { p { role: "status", "Connection interrupted. Retained results may be out of date; waiting to reconnect." } },
                ProjectionLoadState::Failed(error) => rsx! { p { role: "alert", "Projection refresh failed: {error.message}. Retained results may be out of date." } },
            }}
            if view.needs_refresh && view.load == ProjectionLoadState::Ready {
                p { role: "status", "Results need refreshing." }
            }
            if let Some(error) = &view.action_error { p { role: "alert", "{error.message}" } }
            Button {
                label: "Refresh projections", state,
                onpress: move |()| onaction.call(ProjectionEvent::Refresh),
            }
        }
    }
}

/// Supplied scope total, loaded/visible counts and independent freshness/coverage.
/// Unknown totals stay unknown; timestamps and checkpoints never imply freshness.
#[component]
pub fn ProjectionSummary(view: ProjectionView) -> Element {
    let (freshness, tone) = match view.freshness.status {
        FreshnessStatus::Current => ("Current", StatusTone::Success),
        FreshnessStatus::Stale => ("Stale", StatusTone::Warning),
        FreshnessStatus::Unknown => ("Freshness unknown", StatusTone::Neutral),
    };
    let coverage = match view.availability {
        ProjectionAvailability::Complete => "Complete",
        ProjectionAvailability::Partial => "Partial results",
        ProjectionAvailability::Unavailable => "Unavailable",
    };
    rsx! {
        div { class: "idle-projection-summary",
            p { class: "idle-projection-counts",
                span { "{view.visible_count} shown" }
                span { "{view.loaded_count} loaded" }
                span { if let Some(total) = view.total { "{total} total" } else { "Total unknown" } }
            }
            div { class: "idle-inline",
                StatusBadge { label: freshness, tone }
                StatusBadge { label: coverage }
            }
            if view.freshness.generated_at_ms.is_some() || view.freshness.checkpoint.is_some() {
                details { class: "idle-projection-result-details",
                    summary { "Result details" }
                    dl { class: "idle-projection-facts",
                        if let Some(time) = view.freshness.generated_at_ms {
                            dt { "Generated time (Unix milliseconds)" } dd { "{time}" }
                        }
                        if let Some(checkpoint) = &view.freshness.checkpoint {
                            dt { "Provider checkpoint" } dd { "{checkpoint}" }
                        }
                    }
                }
            }
        }
    }
}
