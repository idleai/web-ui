//! Item-keyed Dioxus history graphs over app-core's recorded relationships.
//! Geometry, animation, viewport anchors and focus are local to each component.

mod browser;
mod component;
mod model;
mod paths;
mod state;
mod viewport;

pub use component::{HistoryGraph, HistoryGraphProps};
pub use model::{Connection, ConnectionKind, EndpointState, GraphSnapshot};

/// Graph styles; bundle once alongside the foundation stylesheet.
pub const STYLESHEET: &str = include_str!("../../assets/history.css");

#[cfg(test)]
mod tests;
