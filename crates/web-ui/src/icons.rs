//! Small, self-contained SVG icons with no font or network dependency.

use dioxus::prelude::*;

/// Shared control and status glyphs, drawn on a 24-unit grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconName {
    /// Add an item.
    Add,
    /// Close or dismiss.
    Close,
    /// Search supplied content.
    Search,
    /// Expand a collapsed section.
    ChevronRight,
    /// Collapse an expanded section.
    ChevronDown,
    /// Retry an operation.
    Refresh,
    /// Confirmed success.
    Check,
    /// Informational state.
    Info,
    /// Warning or error state; adjacent text supplies the meaning.
    Alert,
    /// Open outside this surface.
    ExternalLink,
    /// Copy text.
    Copy,
    /// Open a file.
    File,
    /// Compare two file revisions.
    Diff,
}

impl IconName {
    const fn path(self) -> &'static str {
        match self {
            Self::Add => "M12 5v14M5 12h14",
            Self::Close => "m6 6 12 12M6 18 18 6",
            Self::Search => "M21 21l-5-5M18 10a8 8 0 1 1-16 0 8 8 0 0 1 16 0",
            Self::ChevronRight => "m9 5 7 7-7 7",
            Self::ChevronDown => "m5 9 7 7 7-7",
            Self::Refresh => "M20 4v6h-6M20 10a8 8 0 1 0 0 6",
            Self::Check => "m5 12 4 4L19 6",
            Self::Info => "M12 11v6M12 7h.01M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0",
            Self::Alert => "m12 3 10 18H2L12 3M12 9v5M12 17h.01",
            Self::ExternalLink => "M14 3h7v7M21 3 10 14M10 3H3v18h18v-7",
            Self::Copy => "M8 8h13v13H8zM16 8V3H3v13h5",
            Self::File => "M14 2H4v20h16V8l-6-6v6h6M8 13h8M8 17h8",
            Self::Diff => "M9 3H3v18h6M15 3h6v18h-6M7 12h4M15 10v4M13 12h4",
        }
    }
}

/// An icon is decorative unless a standalone accessible label is supplied.
///
/// Buttons provide their own names and leave this label unset to avoid duplicate
/// announcements. SVGs are never keyboard focus targets.
#[component]
pub fn Icon(name: IconName, label: Option<String>) -> Element {
    rsx! {
        svg {
            class: "idle-icon",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.7",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "focusable": "false",
            role: label.as_ref().map(|_| "img"),
            "aria-label": label.clone(),
            "aria-hidden": label.is_none().then_some("true"),
            path { d: name.path() }
        }
    }
}
