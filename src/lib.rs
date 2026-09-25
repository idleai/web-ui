//! Browser presentation only; application state belongs to app-core.
//! Host credentials, networking and native actions stay with the hosts.

pub mod configuration;
pub mod history;
pub mod host;
pub mod navigation;
pub mod projections;
pub mod provenance;
pub mod resources;
pub mod sessions;

use app_core::ViewModel;
use dioxus::prelude::*;

/// Minimal shared component proving both hosts can render an app-core view.
#[component]
pub fn Scaffold(view: ViewModel) -> Element {
    rsx! {
        main { class: "idle-shell",
            h1 { "Idle" }
            p { role: "status",
                if view.initialized { "Ready for workspace setup." }
                else { "Starting…" }
            }
        }
    }
}
