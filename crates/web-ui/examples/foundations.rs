//! Interactive foundation gallery with in-memory host adapters only.

use dioxus::prelude::*;
use dioxus_html as dioxus_elements;
use web_ui::controls::{
    Button, ButtonVariant, Checkbox, ControlState, Disclosure, FieldMessage, IconButton, Select,
    SelectOption, TextField, TextFieldKind,
};
use web_ui::host::{
    FileTarget, HostActionButton, HostCapabilities, HostCapability, HostKind, HostRequest,
};
use web_ui::icons::IconName;
use web_ui::status::{EmptyState, ErrorState, LoadingState, StatusBadge, StatusTone};
use web_ui::theme::{Density, Theme, ThemeProvider};

/// Mount the local fixture. No backend or native API is used by this example.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    dioxus_web::launch::launch(Gallery, Vec::new(), Vec::new());
}

/// Render the same fixture for static previews and inspection.
#[must_use]
pub fn render_fixture() -> String {
    let mut dom = VirtualDom::new(Gallery);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

/// Browser and simulated VS Code consumers of the identical components.
#[component]
pub fn Gallery() -> Element {
    let view = app_core::ViewModel {
        initialized: true,
        ..app_core::ViewModel::default()
    };
    rsx! {
        header { class: "gallery-header",
            h1 { "Idle foundations" }
            p { "Shared controls · keyboard accessible · host supplied actions" }
        }
        main { class: "gallery-grid",
            Preview { prefix: "browser", host: HostKind::Browser, initial_theme: "light", view: view.clone() }
            div { class: "vscode-dark fixture-vscode",
                Preview { prefix: "vscode", host: HostKind::VsCode, initial_theme: "vscode", view }
            }
        }
    }
}

#[component]
fn Preview(
    prefix: String,
    host: HostKind,
    initial_theme: String,
    view: app_core::ViewModel,
) -> Element {
    let mut palette = use_hook(|| Signal::new(initial_theme));
    let mut compact = use_hook(|| Signal::new(host == HostKind::VsCode));
    let mut query = use_hook(|| Signal::new(String::new()));
    let mut expanded = use_hook(|| Signal::new(false));
    let mut busy = use_hook(|| Signal::new(false));
    let mut feedback = use_hook(|| Signal::new("No action requested.".to_owned()));
    let theme = match palette.read().as_str() {
        "light" => Theme::Light,
        "dark" => Theme::Dark,
        "vscode" => Theme::VsCode,
        _ => Theme::System,
    };
    let density = if compact() {
        Density::Compact
    } else {
        Density::Comfortable
    };
    let capabilities = match host {
        HostKind::Browser => HostCapabilities::new(host).with(HostCapability::CopyText),
        HostKind::VsCode => HostCapabilities::new(host)
            .with(HostCapability::CopyText)
            .with(HostCapability::OpenFile),
    };
    let file = FileTarget {
        repository: "fixture-repository".into(),
        path: "src/lib.rs".into(),
        revision: Some("recorded-revision".into()),
        position: None,
    };
    let title = match host {
        HostKind::Browser => "Browser",
        HostKind::VsCode => "VS Code",
    };
    rsx! {
        ThemeProvider { theme, density,
            section { class: "gallery-panel idle-stack", aria_label: title,
                h2 { class: "idle-heading", "{title}" }
                Select {
                    id: "{prefix}-palette", label: "Palette", value: palette(),
                    options: ["light", "dark", "system", "vscode"].into_iter().map(|value| SelectOption {
                        value: value.into(), label: value.into(), disabled: false,
                    }).collect(),
                    onchange: move |value| palette.set(value),
                }
                Checkbox { id: "{prefix}-density", label: "Compact controls", checked: compact(), onchange: move |value| compact.set(value) }
                TextField {
                    id: "{prefix}-search", label: "Find in history", kind: TextFieldKind::Search,
                    value: query(), placeholder: "Search recorded work",
                    message: FieldMessage::Hint("Enter to search. Escape to clear.".into()),
                    oninput: move |value| query.set(value),
                    oncommit: move |value| feedback.set(format!("Search requested: {value}")),
                    onescape: move |()| { query.set(String::new()); feedback.set("Search cleared.".into()); },
                }
                div { class: "idle-inline",
                    Button {
                        label: if busy() { "Saving…" } else { "Save changes" },
                        variant: ButtonVariant::Primary,
                        state: if busy() { ControlState::Busy } else { ControlState::Ready },
                        onpress: move |()| { busy.set(true); feedback.set("Save pending.".into()); },
                    }
                    Button { label: "Finish pending action", onpress: move |()| { busy.set(false); feedback.set("Save completed.".into()); } }
                    IconButton { label: "Refresh history", icon: IconName::Refresh, onpress: move |()| feedback.set("Refresh requested.".into()) }
                    Button { label: "Unavailable", state: ControlState::Disabled, onpress: move |()| feedback.set("Unexpected disabled action.".into()) }
                }
                HostActionButton {
                    id: "{prefix}-copy", label: "Copy reference", unavailable_reason: "Clipboard is unavailable.",
                    capabilities: capabilities.clone(), request: HostRequest::CopyText { text: "recorded-revision".into() },
                    onrequest: move |_| feedback.set("Host received copy request.".into()),
                }
                HostActionButton {
                    id: "{prefix}-open", label: "Open recorded file", unavailable_reason: "This preview has no browser editor adapter.",
                    capabilities, request: HostRequest::OpenFile(file),
                    onrequest: move |_| feedback.set("Host received recorded-file request.".into()),
                }
                p { class: "idle-code", role: "status", aria_live: "polite", "{feedback}" }
                Disclosure { id: "{prefix}-details", label: "Evidence details", expanded: expanded(), onchange: move |value| expanded.set(value),
                    TextField { id: "{prefix}-revision", label: "Revision", value: "", message: FieldMessage::Error("Choose a recorded revision.".into()), oninput: move |_| {} }
                }
                div { class: "idle-inline",
                    StatusBadge { label: if view.initialized { "Ready" } else { "Starting" }, tone: if view.initialized { StatusTone::Success } else { StatusTone::Neutral } }
                    StatusBadge { label: "Needs input", tone: StatusTone::Warning }
                    StatusBadge { label: "Failed", tone: StatusTone::Danger }
                    StatusBadge { label: "Unknown" }
                }
                LoadingState { label: "Loading recorded work…" }
                ErrorState { title: "History unavailable", message: "The supplied view could not be loaded.", onretry: move |()| feedback.set("Retry requested.".into()) }
                EmptyState { title: "No sessions yet", message: "Sessions will appear when supplied by the application." }
            }
        }
    }
}
