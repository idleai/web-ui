//! Labelled form fields with native semantics and associated feedback.

#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{
    Element, EventHandler, Key, KeyboardEvent, ModifiersInteraction, Props, component, dioxus_core,
    dioxus_elements, rsx,
};

use super::ControlState;

/// Feedback associated with a field through `aria-describedby`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FieldMessage {
    /// No additional description.
    #[default]
    None,
    /// Guidance shown beneath the field.
    Hint(String),
    /// Validation failure, also exposed as `aria-invalid`.
    Error(String),
}

impl FieldMessage {
    fn text(&self) -> Option<&str> {
        match self {
            Self::None => None,
            Self::Hint(text) | Self::Error(text) => Some(text),
        }
    }

    const fn invalid(&self) -> bool {
        matches!(self, Self::Error(_))
    }
}

/// Editing availability for text fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FieldState {
    /// Accept edits.
    #[default]
    Editable,
    /// Focusable and selectable, but not editable.
    ReadOnly,
    /// Unavailable, using native disabled behavior.
    Disabled,
}

/// Native input semantics; passwords and credential collection belong to hosts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextFieldKind {
    /// Ordinary single-line text.
    #[default]
    Text,
    /// Search input with the browser's search semantics.
    Search,
}

impl TextFieldKind {
    const fn attribute(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Search => "search",
        }
    }
}

/// Controlled input with a visible label and stable, document-unique ID.
///
/// Plain Enter/Escape are handled only when the corresponding callback exists.
/// Composition and modified keys pass through. An Escape handler decides whether
/// to clear or cancel; without one, native input behavior is preserved. Callers
/// retain semantic search, validation and cancellation state.
#[component]
pub fn TextField(
    id: String,
    label: String,
    value: String,
    oninput: EventHandler<String>,
    #[props(default)] kind: TextFieldKind,
    #[props(default)] state: FieldState,
    #[props(default)] message: FieldMessage,
    #[props(default)] placeholder: String,
    oncommit: Option<EventHandler<String>>,
    onescape: Option<EventHandler<()>>,
) -> Element {
    let described_by = message.text().map(|_| format!("{id}-message"));
    let current_value = value.clone();
    rsx! {
        div { class: "idle-field",
            label { r#for: id.clone(), class: "idle-label", "{label}" }
            input {
                id: id.clone(),
                class: "idle-input",
                r#type: kind.attribute(),
                value,
                placeholder,
                disabled: state == FieldState::Disabled,
                readonly: state == FieldState::ReadOnly,
                aria_invalid: message.invalid().to_string(),
                aria_describedby: described_by,
                oninput: move |event| {
                    if state == FieldState::Editable { oninput.call(event.value()); }
                },
                onkeydown: move |event: KeyboardEvent| {
                    if state != FieldState::Editable || event.is_composing() || !event.modifiers().is_empty() {
                        return;
                    }
                    if event.key() == Key::Enter {
                        if let Some(commit) = oncommit {
                            event.prevent_default();
                            event.stop_propagation();
                            commit.call(current_value.clone());
                        }
                    } else if event.key() == Key::Escape && let Some(cancel) = onescape {
                        event.prevent_default();
                        event.stop_propagation();
                        cancel.call(());
                    }
                },
            }
            if let Some(text) = message.text() {
                p {
                    id: "{id}-message",
                    class: "idle-field-message",
                    "data-invalid": message.invalid().to_string(),
                    "{text}"
                }
            }
        }
    }
}

/// One native select option; its stable value is separate from its display label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectOption {
    /// Stable value returned to the caller.
    pub value: String,
    /// User-facing text.
    pub label: String,
    /// Whether this individual option is unavailable.
    pub disabled: bool,
}

/// Controlled native select; the browser supplies arrow-key and typeahead behavior.
#[component]
pub fn Select(
    id: String,
    label: String,
    value: String,
    options: Vec<SelectOption>,
    onchange: EventHandler<String>,
    #[props(default)] state: ControlState,
    #[props(default)] message: FieldMessage,
) -> Element {
    let described_by = message.text().map(|_| format!("{id}-message"));
    rsx! {
        div { class: "idle-field",
            label { r#for: id.clone(), class: "idle-label", "{label}" }
            select {
                id: id.clone(),
                class: "idle-input",
                value: value.clone(),
                disabled: state.blocked(),
                aria_busy: (state == ControlState::Busy).to_string(),
                aria_invalid: message.invalid().to_string(),
                aria_describedby: described_by,
                onchange: move |event| {
                    if !state.blocked() { onchange.call(event.value()); }
                },
                for option in options {
                    option {
                        key: "{option.value}",
                        selected: option.value == value,
                        value: option.value,
                        disabled: option.disabled,
                        "{option.label}"
                    }
                }
            }
            if let Some(text) = message.text() {
                p { id: "{id}-message", class: "idle-field-message",
                    "data-invalid": message.invalid().to_string(),
                    "{text}"
                }
            }
        }
    }
}

/// Controlled native checkbox; Space and label clicks use browser behavior.
#[component]
pub fn Checkbox(
    id: String,
    label: String,
    checked: bool,
    onchange: EventHandler<bool>,
    #[props(default)] state: ControlState,
) -> Element {
    rsx! {
        label { class: "idle-checkbox", r#for: id.clone(),
            input {
                id,
                r#type: "checkbox",
                checked,
                disabled: state.blocked(),
                aria_busy: (state == ControlState::Busy).to_string(),
                onchange: move |event| {
                    if !state.blocked() { onchange.call(event.checked()); }
                },
            }
            span { "{label}" }
        }
    }
}
