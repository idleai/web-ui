//! Browser-owned history motion and viewport geometry with shared graph routes.
//! No application selection, host effects or Dioxus runtime lives in this crate.

pub use idle_history_graph::live;
pub mod motion;
pub mod viewport;
