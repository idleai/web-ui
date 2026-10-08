//! Development fixture for the actual editor and native Activity pane compositions.

#[path = "history_explorer/fixture.rs"]
mod fixture;

use app_core::{Event as AppEvent, workspace};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{Element, WritableExt, component, dioxus_core, rsx, use_hook, use_signal};
use dioxus_html as dioxus_elements;
use dioxus_ssr as _;
use history_geometry as _;
use std::{cell::RefCell, rc::Rc};
#[cfg(target_arch = "wasm32")]
use {js_sys as _, web_sys as _};

use web_ui::assembly::{Surface, WorkspaceSurface};
use web_ui::controls::Button;
use web_ui::host::{HostCapabilities, HostCapability, HostKind};
use web_ui::theme::{Theme, ThemeProvider};

/// Start the isolated fixture without opening host files or connecting to a service.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    dioxus_web::launch::launch(Gallery, Vec::new(), Vec::new());
}

/// Both layouts consume the same typed state; selection is reported below.
#[component]
pub fn Gallery() -> Element {
    let loaded = use_hook(|| Rc::new(RefCell::new(fixture::Fixture::load())));
    let mut view = use_signal(|| {
        loaded
            .borrow()
            .as_ref()
            .map(|fixture| fixture.view.clone())
            .unwrap_or_default()
    });
    let mut theme = use_signal(|| Theme::Dark);
    let mut last_action =
        use_signal(|| "Native windows · ten child agents · three nested levels".to_owned());
    let mut opens = use_signal(|| 0u64);
    let mut case = use_signal(|| "dense".to_owned());
    let dispatch_state = Rc::clone(&loaded);
    let dispatch = move |event: AppEvent| {
        let event = match event {
            AppEvent::History(event) => event,
            AppEvent::Workspace(workspace::Event::Navigate(
                workspace::NavigationSection::Activity,
            )) => {
                let selected = dispatch_state
                    .borrow()
                    .as_ref()
                    .ok()
                    .and_then(|fixture| fixture.view.history.timeline.selected.clone());
                let Some(selection) = selected else {
                    return;
                };
                app_core::history::Event::Timeline(app_core::history::timeline::Event::Reveal {
                    surface: app_core::history::timeline::Surface::Editor,
                    selection,
                })
            }
            AppEvent::Start
            | AppEvent::Bootstrap(_)
            | AppEvent::Workspace(_)
            | AppEvent::Subscriptions(_)
            | AppEvent::Sessions(_)
            | AppEvent::Projections(_)
            | AppEvent::Resources(_)
            | AppEvent::Configuration(_)
            | AppEvent::Repository(_) => return,
        };
        if matches!(
            event,
            app_core::history::Event::Timeline(app_core::history::timeline::Event::Visible { .. })
        ) {
            if let Ok(fixture) = dispatch_state.borrow_mut().as_mut() {
                fixture.dispatch(event);
            }
            return;
        }
        last_action.set(format!("{event:?}"));
        if let Ok(fixture) = dispatch_state.borrow_mut().as_mut() {
            fixture.dispatch(event);
            opens.set(fixture.opens);
            view.set(fixture.view.clone());
        }
    };
    let fail_state = Rc::clone(&loaded);
    let reset_state = Rc::clone(&loaded);
    let case_state = Rc::clone(&loaded);
    let capabilities = HostCapabilities::new(HostKind::VsCode)
        .with(HostCapability::OpenOperationJson)
        .with(HostCapability::OpenFile)
        .with(HostCapability::OpenDiff)
        .with(HostCapability::OpenRecord)
        .with(HostCapability::OpenOriginal);
    rsx! {
        div { class: "explorer-fixture",
            ThemeProvider { theme: theme(),
                header { class: "fixture-toolbar",
                    span { "Development fixture" }
                    select { aria_label: "Graph case", value: case(), onchange: move |event| {
                        let selected = event.value();
                        *case_state.borrow_mut() = fixture::Fixture::load_case(&selected);
                        if let Ok(fixture) = case_state.borrow().as_ref() {
                            view.set(fixture.view.clone()); opens.set(0); last_action.set(format!("Graph case: {selected}"));
                        }
                        case.set(selected);
                    },
                        option { value: "dense", "Dense recording" }
                        option { value: "linear", "Single chain" }
                        option { value: "fork", "Parent and child" }
                        option { value: "join", "Fork and join" }
                        option { value: "siblings", "Two children" }
                        option { value: "nested", "Nested child" }
                        option { value: "independent", "Independent streams" }
                        option { value: "passing", "Passing connection" }
                    }
                    Button { label: "Switch theme", onpress: move |()| {
                        let next = if theme() == Theme::Dark { Theme::Light } else { Theme::Dark }; theme.set(next);
                    } }
                    Button { label: "Fail history page", onpress: move |()| {
                        if let Ok(fixture) = fail_state.borrow_mut().as_mut() { fixture.fail(); view.set(fixture.view.clone()); }
                    } }
                    Button { label: "Reset", onpress: move |()| {
                        *reset_state.borrow_mut() = fixture::Fixture::load_case(&case());
                        if let Ok(fixture) = reset_state.borrow().as_ref() { view.set(fixture.view.clone()); opens.set(0); }
                    } }
                    span { id: "native-opens", "{opens}" }
                    span { id: "last-action", "{last_action}" }
                }
            }
            div { class: "fixture-surfaces",
                aside { class: "fixture-sidebar",
                    WorkspaceSurface { view: view(), theme: theme(), capabilities: capabilities.clone(), surface: Surface::SidebarPane(workspace::NavigationSection::Activity), onaction: dispatch.clone() }
                }
                div { class: "fixture-editor",
                    WorkspaceSurface { view: view(), theme: theme(), capabilities, surface: Surface::Detail, onaction: dispatch }
                }
            }
        }
    }
}
