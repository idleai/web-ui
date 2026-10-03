//! Projection summaries keep supplied totals distinct from loaded rows.

use app_core::{
    Event,
    projections::{self, FreshnessStatus, ProjectionAvailability, ProjectionLoadState},
    workspace::NavigationSection,
};
use dioxus::prelude::*;

use super::super::chrome::{NavRow, Notice, RowStatus, Section, navigate};
use crate::icons::IconName;

#[component]
pub(in crate::navigation) fn Projections(
    view: projections::ViewModel,
    section: NavigationSection,
    enabled: bool,
    onaction: EventHandler<Event>,
) -> Element {
    let destinations: Vec<_> = [&view.tasks, &view.errors, &view.triage, &view.need_input]
        .into_iter()
        .filter(|projection| projection.availability != ProjectionAvailability::Unavailable)
        .collect();
    let count = (view.load == ProjectionLoadState::Ready || !destinations.is_empty())
        .then(|| destinations.len().to_string());
    rsx! {
        Section { section: NavigationSection::Projections, current: section, enabled, count, onaction,
            for projection in destinations {
                NavRow { key: "{projection.kind:?}", name: crate::projections::title(projection.kind), icon: IconName::Table,
                    detail: projection.total.map_or_else(|| "Total unknown".into(), |count| count.to_string()),
                    status: RowStatus { label: projection_status(projection), tone: if projection.freshness.status == FreshnessStatus::Stale { "warning" } else { "neutral" } },
                    disabled: !enabled,
                    onpress: move |()| navigate(onaction, NavigationSection::Projections),
                }
            }
            match view.load {
                ProjectionLoadState::Idle => rsx! { Notice { text: "Projections not connected" } },
                ProjectionLoadState::Loading => rsx! { Notice { text: "Loading projections…" } },
                ProjectionLoadState::Suspended => rsx! { Notice { text: "Projections waiting to reconnect" } },
                ProjectionLoadState::Failed(error) => rsx! { Notice { text: error.message, error: true } },
                ProjectionLoadState::Ready => rsx! {},
            }
        }
    }
}

fn projection_status(view: &projections::ProjectionView) -> String {
    let freshness = match view.freshness.status {
        FreshnessStatus::Unknown => "Freshness unknown",
        FreshnessStatus::Current => "Current",
        FreshnessStatus::Stale => "Stale",
    };
    let completeness = match view.availability {
        ProjectionAvailability::Complete => "Complete",
        ProjectionAvailability::Partial => "Partial results",
        ProjectionAvailability::Unavailable => "Unavailable",
    };
    format!(
        "{freshness} · {completeness} · {} loaded",
        view.loaded_count
    )
}
