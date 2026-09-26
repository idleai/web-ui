//! Scoped theme selection. Hosts bundle the stylesheet; components never inject it.

use dioxus::prelude::*;

/// Bundled stylesheet for hosts that assemble assets without copying a file.
pub const STYLESHEET: &str = include_str!("../assets/theme.css");

/// Palette selection independent of host capabilities.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    /// Follow the browser's preferred color scheme.
    #[default]
    System,
    /// Always use the built-in light palette.
    Light,
    /// Always use the built-in dark palette.
    Dark,
    /// Read injected VS Code variables, falling back to the system palette.
    VsCode,
}

impl Theme {
    const fn attribute(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
            Self::VsCode => "vscode",
        }
    }
}

/// Control sizing selected by the host, independent of the palette.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Density {
    /// Roomier browser controls.
    #[default]
    Comfortable,
    /// Compact sidebar controls; coarse pointers still receive larger targets.
    Compact,
}

impl Density {
    const fn attribute(self) -> &'static str {
        match self {
            Self::Comfortable => "comfortable",
            Self::Compact => "compact",
        }
    }
}

/// Scope theme tokens and density to a subtree without changing the host page.
///
/// Load [`STYLESHEET`] once in the host. Instances can use different themes on
/// the same page; no document access or platform detection is performed here.
#[component]
pub fn ThemeProvider(
    #[props(default)] theme: Theme,
    #[props(default)] density: Density,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: "idle-theme",
            "data-idle-theme": theme.attribute(),
            "data-idle-density": density.attribute(),
            {children}
        }
    }
}
