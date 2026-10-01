//! History rows and exact record drill-down over app-core's typed views.
//!
//! Disclosure, selection, content loading and native opens are app-core events.
//! Hosts execute the resulting effects; these components never resolve files or
//! reconstruct streams. Load [`STYLESHEET`] alongside the graph and theme styles.

mod actions;
mod content;
mod inspector;
mod observation;
mod records;
mod row;

pub use inspector::{HistoryDetails, HistoryDetailsProps, HistoryTimeline, HistoryTimelineProps};
pub use row::{HistoryRow, HistoryRowProps};

/// Row and inspector styles shared by browser and extension compositions.
pub const STYLESHEET: &str = include_str!("../../assets/history-details.css");

#[cfg(test)]
mod tests;
