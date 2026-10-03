//! Interactive sidebar preview with development-only host responses.

#[path = "navigation/fixture.rs"]
mod fixture;

use app_core::{Event as AppEvent, workspace::WorkspaceMode};
use dioxus::prelude::*;
use dioxus_html as dioxus_elements;
use dioxus_ssr as _;
use history_geometry as _;
#[cfg(target_arch = "wasm32")]
use web_sys as _;

use fixture::Fixture;
use web_ui::controls::Button;
use web_ui::navigation::{SessionCreation, WorkspaceNavigation};
use web_ui::theme::{Density, Theme, ThemeProvider};

/// Launch an isolated local preview without credentials or network adapters.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    dioxus_web::launch::launch(Gallery, Vec::new(), Vec::new());
}

/// Narrow and wide hosts share the same app-core selection.
#[component]
pub fn Gallery() -> Element {
    let mut fixture = use_signal(|| Fixture::new(WorkspaceMode::Managed));
    let mut action = use_signal(|| "No action dispatched".to_owned());
    let dispatch = move |event: AppEvent| {
        action.set(format!("{event:?}"));
        if let Ok(fixture) = &mut *fixture.write()
            && let Err(error) = fixture.dispatch(event)
        {
            action.set(error);
        }
    };
    let mut command = move |command| {
        if let Ok(fixture) = &mut *fixture.write() {
            let result = match command {
                Command::Disconnect => fixture.disconnect(),
                Command::Expire => fixture.expire(),
                Command::Fail => fixture.fail_resources(),
            };
            if let Err(error) = result {
                action.set(error);
            }
        }
    };
    let state = fixture.read();
    let Ok(current) = &*state else {
        return rsx! { p { role: "alert", "{state.as_ref().err():?}" } };
    };
    let view = current.view();
    let creation = current.creation();
    let pending = current.pending_creations();
    rsx! {
        main { class: "navigation-gallery",
            ThemeProvider { theme: Theme::Light,
                header { class: "fixture-header",
                    p { "IDLE / WORKSPACE NAVIGATION" }
                    h1 { "One workspace. Shared context." }
                    p { "Development fixture · app-core selections · independent user and compute identities" }
                    div { class: "fixture-actions",
                        Button { label: "Expire presence and health", onpress: move |()| command(Command::Expire) }
                        Button { label: "Fail resource refresh", onpress: move |()| command(Command::Fail) }
                        Button { label: "Disconnect", onpress: move |()| command(Command::Disconnect) }
                        Button { label: "Reset managed", onpress: move |()| fixture.set(Fixture::new(WorkspaceMode::Managed)) }
                        Button { label: "Reset standalone", onpress: move |()| fixture.set(Fixture::new(WorkspaceMode::Standalone)) }
                    }
                    p { id: "last-action", "{action}" }
                    p { id: "pending-creations", "{pending} pending creations" }
                    p { id: "selection", "Section: {view.workspace.section:?} · Session: {view.sessions.selected:?} · Host: {view.resources.selected_host:?} · Provider: {view.resources.selected_provider:?}" }
                }
            }
            div { class: "fixture-compositions",
                ThemeProvider { theme: Theme::Dark, density: Density::Compact,
                    WorkspaceNavigation { id: "sidebar", view: view.clone(), creation: creation.clone(), now_ms: 1000, onaction: dispatch }
                }
                ThemeProvider { theme: Theme::Light, density: Density::Comfortable,
                    WorkspaceNavigation { id: "browser", view, creation, now_ms: 1000, onaction: dispatch }
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Command {
    Disconnect,
    Expire,
    Fail,
}
