//! Shared projection presentations over app-core rows, counts and typed actions.
//!
//! [`ProjectionPanel`] includes filters, result details and load feedback. Hosts
//! can also compose the list, table, cards and board independently. Board columns
//! group exact supplied statuses in first-seen order, without assigning workflow
//! meanings or supporting local task transitions. Load [`STYLESHEET`] with the
//! theme stylesheet and choose density through [`crate::theme::ThemeProvider`].

#[cfg(target_arch = "wasm32")]
mod board_state;
mod feedback;
mod filters;
mod panel;
mod records;
mod rows;

pub use feedback::{
    ProjectionFeedback, ProjectionFeedbackProps, ProjectionSummary, ProjectionSummaryProps,
};
pub use filters::{ProjectionFilters, ProjectionFiltersProps};
pub use panel::{ProjectionLayout, ProjectionPanel, ProjectionPanelProps};
pub use rows::{
    ProjectionBoard, ProjectionBoardProps, ProjectionCard, ProjectionCardProps, ProjectionCards,
    ProjectionCardsProps, ProjectionList, ProjectionListProps, ProjectionTable,
    ProjectionTableProps,
};

use app_core::projections::{ProjectionKind, ProjectionSelection, ProjectionView, ViewModel};

/// Projection styles, scoped to the shared theme and suitable for either host.
pub const STYLESHEET: &str = include_str!("../assets/projections.css");

fn destination(view: &ViewModel, kind: ProjectionKind) -> &ProjectionView {
    match kind {
        ProjectionKind::Activity => &view.activity,
        ProjectionKind::Task => &view.tasks,
        ProjectionKind::Error => &view.errors,
        ProjectionKind::Triage => &view.triage,
        ProjectionKind::NeedInput => &view.need_input,
    }
}

fn title(kind: ProjectionKind) -> &'static str {
    match kind {
        ProjectionKind::Activity => "Activity",
        ProjectionKind::Task => "Tasks",
        ProjectionKind::Error => "Errors",
        ProjectionKind::Triage => "Triage",
        ProjectionKind::NeedInput => "Human input",
    }
}

fn selection(kind: ProjectionKind, key: &str) -> ProjectionSelection {
    ProjectionSelection {
        kind,
        key: key.into(),
    }
}

fn status_label(status: Option<&str>) -> &str {
    match status {
        None => "No status",
        Some("") => "Empty status",
        Some(status) => status,
    }
}

#[cfg(test)]
mod tests;
