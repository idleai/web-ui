//! Shared author, exposure and touch presentation over supplied history data.
//!
//! Hosts supply classifications and ranges; this module never guesses authors
//! from recorder names, parses raw records or carries marks between revisions.
//! Styles are included in [`crate::history::details::STYLESHEET`].

mod model;
mod panel;
mod sources;
mod types;

pub use panel::{AuthorActivity, AuthorActivityProps};
pub use types::{ActivityIndicator, ActivityKind, ActivitySnapshot, ActivitySource, ByteRange};

#[cfg(test)]
mod tests;
