//! Interactive projection compositions using the real app-core reducer.

#[path = "projections/fixture.rs"]
mod fixture;

use app_core::projections::{Event as ProjectionEvent, ProjectionAvailability, ProjectionKind};
use dioxus::prelude::*;
use dioxus_html as dioxus_elements;
use dioxus_ssr as _;
use history_geometry as _;
#[cfg(target_arch = "wasm32")]
use web_sys as _;

use fixture::Fixture;
use web_ui::controls::{Button, Select, SelectOption};
use web_ui::projections::{
    ProjectionBoard, ProjectionCard, ProjectionCards, ProjectionLayout, ProjectionList,
    ProjectionPanel, ProjectionTable,
};
use web_ui::theme::{Density, Theme, ThemeProvider};

/// Mount a development-only gallery with no network or runtime dependencies.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    dioxus_web::launch::launch(Gallery, Vec::new(), Vec::new());
}

/// The browser and sidebar share selection, filters and refresh state from Crux.
#[component]
pub fn Gallery() -> Element {
    let mut fixture = use_signal(Fixture::new);
    let mut action = use_signal(|| "No action dispatched".to_owned());
    let mut layout = use_signal(|| ProjectionLayout::Board);
    let dispatch = move |event: ProjectionEvent| {
        action.set(format!("{event:?}"));
        if let Ok(fixture) = &mut *fixture.write()
            && let Err(error) = fixture.dispatch(event)
        {
            action.set(error);
        }
    };
    let control = Callback::new(move |command: Command| {
        if let Ok(fixture) = &mut *fixture.write() {
            let result = match command {
                Command::Complete => fixture.complete(),
                Command::Fail => fixture.fail(),
                Command::Suspend => fixture.dispatch(ProjectionEvent::Suspend),
                Command::Reconnect => fixture.dispatch(ProjectionEvent::Reconnect),
                Command::Disconnect => fixture.dispatch(ProjectionEvent::Disconnect),
                Command::Prepend => fixture.prepend(),
                Command::Retract => fixture.retract(),
                Command::MoveTask => fixture.move_task(),
                Command::PrependRecords => fixture.prepend_records(),
                Command::LongFields => fixture.long_fields(),
                Command::Empty(availability) => fixture.empty(availability),
            };
            if let Err(error) = result {
                action.set(error);
            }
        }
    });
    let result = fixture.read();
    let Ok(current) = &*result else {
        return rsx! { p { role: "alert", "Fixture could not initialize" } };
    };
    let view = current.view();
    let inspection = current.inspection();
    let layouts = [
        ("board", "Board"),
        ("list", "List"),
        ("table", "Table"),
        ("cards", "Cards"),
    ]
    .into_iter()
    .map(|(value, label)| SelectOption {
        value: value.into(),
        label: label.into(),
        disabled: false,
    })
    .collect();
    rsx! {
        main { class: "projection-gallery",
            ThemeProvider { theme: Theme::Light,
                header { class: "fixture-heading",
                    p { "IDLE / PROJECTIONS" }
                    h1 { "Follow the work" }
                    p { "Development preview · shared app-core state across browser and sidebar compositions." }
                }
                div { class: "fixture-toolbar",
                    Select { id: "layout", label: "Browser task layout", value: match layout() { ProjectionLayout::Board => "board", ProjectionLayout::List => "list", ProjectionLayout::Table => "table", ProjectionLayout::Cards => "cards" }, options: layouts,
                        onchange: move |value: String| layout.set(match value.as_str() { "list" => ProjectionLayout::List, "table" => ProjectionLayout::Table, "cards" => ProjectionLayout::Cards, _ => ProjectionLayout::Board }),
                    }
                    div { class: "fixture-actions",
                        Button { label: "Complete refresh", onpress: move |()| control.call(Command::Complete) }
                        Button { label: "Fail refresh", onpress: move |()| control.call(Command::Fail) }
                        Button { label: "Suspend", onpress: move |()| control.call(Command::Suspend) }
                        Button { label: "Reconnect", onpress: move |()| control.call(Command::Reconnect) }
                        Button { label: "Prepend task", onpress: move |()| control.call(Command::Prepend) }
                        Button { label: "Retract selected", onpress: move |()| control.call(Command::Retract) }
                        Button { label: "Move task", onpress: move |()| control.call(Command::MoveTask) }
                        Button { label: "Prepend records", onpress: move |()| control.call(Command::PrependRecords) }
                        Button { label: "Long fields", onpress: move |()| control.call(Command::LongFields) }
                        Button { label: "Empty", onpress: move |()| control.call(Command::Empty(ProjectionAvailability::Complete)) }
                        Button { label: "Partial empty", onpress: move |()| control.call(Command::Empty(ProjectionAvailability::Partial)) }
                        Button { label: "Unavailable", onpress: move |()| control.call(Command::Empty(ProjectionAvailability::Unavailable)) }
                        Button { label: "Disconnect", onpress: move |()| control.call(Command::Disconnect) }
                        Button { label: "Reset", onpress: move |()| { fixture.set(Fixture::new()); action.set("No action dispatched".into()); } }
                    }
                }
                details { class: "fixture-events",
                    summary { "App-core actions and history selection" }
                    pre { id: "last-action", "{action}" }
                    pre { id: "history-selection", "{inspection}" }
                }
                details { id: "standalone", class: "fixture-standalone",
                    summary { "Standalone layouts in a narrow container" }
                    div { id: "standalone-list",
                        ProjectionList { view: view.tasks.clone(), selected: view.selected.clone(), onaction: dispatch }
                    }
                    div { id: "standalone-cards",
                        ProjectionCards { view: view.tasks.clone(), selected: view.selected.clone(), onaction: dispatch }
                    }
                    div { id: "standalone-board",
                        ProjectionBoard { view: view.tasks.clone(), selected: view.selected.clone(), onaction: dispatch }
                    }
                    div { id: "standalone-table",
                        ProjectionTable { view: view.tasks.clone(), selected: view.selected.clone(), onaction: dispatch }
                    }
                    div { id: "standalone-card",
                        if let Some(row) = view.tasks.rows.first() {
                            ProjectionCard { kind: view.tasks.kind, row: row.clone(), selected: view.selected.clone(), onaction: dispatch }
                        }
                    }
                }
            }
            div { class: "fixture-compositions",
                ThemeProvider { theme: Theme::Light,
                    section { class: "fixture-browser", aria_label: "Browser composition",
                        p { class: "fixture-kicker", "BROWSER" }
                        ProjectionPanel { id: "browser-tasks", view: view.clone(), kind: ProjectionKind::Task, layout: layout(), onaction: dispatch }
                        ProjectionPanel { id: "browser-errors", view: view.clone(), kind: ProjectionKind::Error, layout: ProjectionLayout::Table, onaction: dispatch }
                        ProjectionPanel { id: "browser-triage", view: view.clone(), kind: ProjectionKind::Triage, layout: ProjectionLayout::Cards, onaction: dispatch }
                        ProjectionPanel { id: "browser-input", view: view.clone(), kind: ProjectionKind::NeedInput, layout: ProjectionLayout::Cards, onaction: dispatch }
                    }
                }
                ThemeProvider { theme: Theme::Dark, density: Density::Compact,
                    aside { class: "fixture-sidebar", aria_label: "Sidebar composition",
                        p { class: "fixture-kicker", "SIDEBAR" }
                        ProjectionPanel { id: "sidebar-tasks", view: view.clone(), kind: ProjectionKind::Task, onaction: dispatch }
                        ProjectionPanel { id: "sidebar-errors", view: view.clone(), kind: ProjectionKind::Error, onaction: dispatch }
                        ProjectionPanel { id: "sidebar-triage", view: view.clone(), kind: ProjectionKind::Triage, onaction: dispatch }
                        ProjectionPanel { id: "sidebar-input", view, kind: ProjectionKind::NeedInput, onaction: dispatch }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Command {
    Complete,
    Fail,
    Suspend,
    Reconnect,
    Disconnect,
    Prepend,
    Retract,
    MoveTask,
    PrependRecords,
    LongFields,
    Empty(ProjectionAvailability),
}
