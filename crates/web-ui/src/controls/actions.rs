//! Buttons and controlled disclosure sections.

#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{Element, EventHandler, Props, component, dioxus_core, dioxus_elements, rsx};

use super::ControlState;
use crate::icons::{Icon, IconName};

/// Visual emphasis, independent of availability.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Main action.
    Primary,
    /// Supporting action.
    #[default]
    Secondary,
    /// Quiet toolbar action.
    Quiet,
    /// Destructive action; use an explicit label as well as color.
    Danger,
}

impl ButtonVariant {
    const fn attribute(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Secondary => "secondary",
            Self::Quiet => "quiet",
            Self::Danger => "danger",
        }
    }
}

/// Native button activated by click, Enter or Space; never implicitly submits.
///
/// Busy buttons stay focusable and report `aria-busy`/`aria-disabled`. Both busy
/// and disabled states guard the callback, including synthetic activation.
#[component]
pub fn Button(
    label: String,
    onpress: EventHandler<()>,
    #[props(default)] variant: ButtonVariant,
    #[props(default)] state: ControlState,
    icon: Option<IconName>,
    described_by: Option<String>,
) -> Element {
    let activate = move |_| {
        if !state.blocked() {
            onpress.call(());
        }
    };
    rsx! {
        button {
            class: "idle-button",
            r#type: "button",
            "data-variant": variant.attribute(),
            disabled: state == ControlState::Disabled,
            aria_disabled: state.blocked().to_string(),
            aria_busy: (state == ControlState::Busy).to_string(),
            aria_describedby: described_by,
            onclick: activate,
            if state == ControlState::Busy {
                span { class: "idle-spinner", aria_hidden: "true" }
            } else if let Some(name) = icon {
                Icon { name }
            }
            "{label}"
        }
    }
}

/// Compact action with a required accessible name and a visible hover tooltip.
#[component]
pub fn IconButton(
    label: String,
    icon: IconName,
    onpress: EventHandler<()>,
    #[props(default)] state: ControlState,
) -> Element {
    let activate = move |_| {
        if !state.blocked() {
            onpress.call(());
        }
    };
    rsx! {
        button {
            class: "idle-button idle-icon-button",
            r#type: "button",
            "data-variant": "quiet",
            title: label.clone(),
            aria_label: label,
            disabled: state == ControlState::Disabled,
            aria_disabled: state.blocked().to_string(),
            aria_busy: (state == ControlState::Busy).to_string(),
            onclick: activate,
            if state == ControlState::Busy {
                span { class: "idle-spinner", aria_hidden: "true" }
            } else {
                Icon { name: icon }
            }
        }
    }
}

/// Controlled disclosure with a stable, host-supplied ID unique in the document.
///
/// Native button keyboard behavior toggles the requested value. Children remain
/// mounted while collapsed, but `hidden` removes them from layout and tab order.
/// The caller owns expansion state; no global shortcuts or autofocus are added.
#[component]
pub fn Disclosure(
    id: String,
    label: String,
    expanded: bool,
    onchange: EventHandler<bool>,
    children: Element,
) -> Element {
    let toggle = move |_| onchange.call(!expanded);
    rsx! {
        div { class: "idle-disclosure",
            button {
                id: "{id}-trigger",
                class: "idle-button idle-disclosure-trigger",
                r#type: "button",
                "data-variant": "quiet",
                aria_expanded: expanded.to_string(),
                aria_controls: "{id}-panel",
                onclick: toggle,
                Icon { name: if expanded { IconName::ChevronDown } else { IconName::ChevronRight } }
                "{label}"
            }
            div {
                id: "{id}-panel",
                class: "idle-disclosure-panel",
                hidden: !expanded,
                {children}
            }
        }
    }
}
