//! Browser-owned geometry shared with the retiring `EditChain` viewer adapters.
//! No application selection, host effects or Dioxus runtime lives in this crate.

pub mod layout;
pub mod live;
pub mod motion;
pub mod viewport;

pub mod legacy_projection;
pub mod legacy_protocol;

#[cfg(test)]
use blake3 as _;
#[cfg(test)]
use editchain_engine as _;
#[cfg(test)]
use editchain_import as _;
#[cfg(test)]
use editchain_store as _;
#[cfg(test)]
use tempfile as _;
#[cfg(test)]
use tokio as _;
