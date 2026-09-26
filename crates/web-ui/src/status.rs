//! Shared loading, error and empty views. They never start work or retry on mount.

use dioxus::prelude::*;

use crate::controls::{Button, ControlState};
use crate::icons::{Icon, IconName};

/// Semantic presentation of a supplied status, always paired with text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatusTone {
    /// Neutral metadata or unknown status.
    #[default]
    Neutral,
    /// Informational status.
    Info,
    /// Confirmed success.
    Success,
    /// Warning requiring attention.
    Warning,
    /// Error or failure.
    Danger,
}

impl StatusTone {
    const fn attribute(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Info => "info",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Danger => "danger",
        }
    }

    const fn icon(self) -> IconName {
        match self {
            Self::Neutral | Self::Info => IconName::Info,
            Self::Success => IconName::Check,
            Self::Warning | Self::Danger => IconName::Alert,
        }
    }
}

/// Compact static status with a text label; lists do not create many live regions.
#[component]
pub fn StatusBadge(label: String, #[props(default)] tone: StatusTone) -> Element {
    rsx! {
        span { class: "idle-badge", "data-tone": tone.attribute(),
            Icon { name: tone.icon() }
            "{label}"
        }
    }
}

/// Politely announce loading without stealing focus.
///
/// Set `aria-busy` on the separate content region being updated. This live region
/// is deliberately not busy, so assistive technology can announce its label.
#[component]
pub fn LoadingState(label: String) -> Element {
    rsx! {
        div { class: "idle-status", role: "status", aria_live: "polite", aria_atomic: "true",
            span { class: "idle-spinner", aria_hidden: "true" }
            span { "{label}" }
        }
    }
}

/// Announce an error with optional, explicitly requested retry.
///
/// Pass user-safe text, not raw transport diagnostics. Retry availability and
/// pending state come from the caller; rendering this view never initiates work.
#[component]
pub fn ErrorState(
    title: String,
    message: String,
    onretry: Option<EventHandler<()>>,
    #[props(default)] retry_state: ControlState,
) -> Element {
    rsx! {
        div { class: "idle-state idle-stack", "data-tone": "danger",
            div { role: "alert", aria_atomic: "true",
                p { class: "idle-state-title", Icon { name: IconName::Alert } "{title}" }
                p { class: "idle-state-message", "{message}" }
            }
            if let Some(retry) = onretry {
                Button { label: "Retry", icon: IconName::Refresh, state: retry_state, onpress: retry }
            }
        }
    }
}

/// Quiet empty state, distinct from pending or failed data.
#[component]
pub fn EmptyState(title: String, message: String) -> Element {
    rsx! {
        div { class: "idle-state",
            p { class: "idle-state-title", "{title}" }
            p { class: "idle-state-message", "{message}" }
        }
    }
}
