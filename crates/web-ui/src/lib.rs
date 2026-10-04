//! Browser presentation only; application state belongs to app-core.
//! Host credentials, networking and native actions stay with the hosts.

pub mod assembly;
pub mod configuration;
pub mod controls;
pub mod history;
pub mod host;
pub mod icons;
pub mod navigation;
pub mod projections;
pub mod provenance;
pub mod repository;
pub mod resources;
pub mod sessions;
pub mod status;
pub mod theme;

use app_core::ViewModel;
use dioxus::prelude::*;
use status::{LoadingState, StatusBadge, StatusTone};
use theme::{Density, Theme, ThemeProvider};

/// Shared bootstrap view retained for existing browser and extension consumers.
///
/// The default theme reads VS Code variables when present and otherwise uses
/// the system palette. New hosts should choose their theme and density explicitly.
#[component]
pub fn Scaffold(
    view: ViewModel,
    #[props(default = Theme::VsCode)] theme: Theme,
    #[props(default)] density: Density,
) -> Element {
    rsx! {
        ThemeProvider { theme, density,
            main { class: "idle-shell idle-stack",
                h1 { class: "idle-heading", "Idle" }
                if view.initialized {
                    StatusBadge { tone: StatusTone::Success, label: "Ready for workspace setup." }
                } else {
                    LoadingState { label: "Starting…" }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
