//! Compact navigation controls with native keyboard behavior.

use app_core::{
    Event,
    workspace::{self, NavigationSection},
};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{Element, EventHandler, Props, component, dioxus_core, dioxus_elements, rsx};

use crate::icons::{Icon, IconName};

pub(super) const fn label(section: NavigationSection) -> &'static str {
    match section {
        NavigationSection::Workspace => "Workspace",
        NavigationSection::Members => "Users",
        NavigationSection::Sessions => "Sessions",
        NavigationSection::Projections => "Projections",
        NavigationSection::ComputeHosts => "Compute hosts",
        NavigationSection::ModelProviders => "Model providers",
        NavigationSection::Activity => "Activity",
        NavigationSection::Settings => "Settings",
        NavigationSection::AgentRules => "Agent Rules",
    }
}

pub(super) fn navigate(onaction: EventHandler<Event>, section: NavigationSection) {
    onaction.call(Event::Workspace(workspace::Event::Navigate(section)));
}

#[component]
pub(super) fn Section(
    section: NavigationSection,
    current: NavigationSection,
    enabled: bool,
    onaction: EventHandler<Event>,
    count: Option<String>,
    action: Option<Element>,
    children: Element,
) -> Element {
    let title = label(section);
    rsx! {
        section { class: "idle-navigation-section", "data-section": "{section:?}", aria_label: title,
            header { class: "idle-navigation-heading",
                h2 {
                    button {
                        r#type: "button", class: "idle-navigation-destination", disabled: !enabled,
                        aria_current: (section == current).then_some("page"),
                        onclick: move |_| { if enabled { navigate(onaction, section); } },
                        "{title}"
                    }
                }
                if let Some(count) = count { span { class: "idle-navigation-count", title: "{title} count", "{count}" } }
                {action}
            }
            {children}
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RowStatus {
    pub label: String,
    pub tone: &'static str,
}

#[component]
pub(super) fn Status(status: RowStatus) -> Element {
    rsx! {
        span { class: "idle-navigation-status", "data-tone": status.tone, title: status.label.clone(),
            span { class: "idle-visually-hidden", "{status.label}" }
        }
    }
}

#[component]
pub(super) fn NavRow(
    name: String,
    icon: IconName,
    onpress: EventHandler<()>,
    #[props(default)] selected: bool,
    #[props(default)] disabled: bool,
    identity: Option<String>,
    detail: Option<String>,
    status: Option<RowStatus>,
) -> Element {
    let title = detail
        .as_ref()
        .map_or_else(|| name.clone(), |detail| format!("{name} · {detail}"));
    rsx! {
        button {
            r#type: "button", class: "idle-navigation-row", title,
            "data-identity": identity, aria_pressed: selected.to_string(), disabled,
            onclick: move |_| { if !disabled { onpress.call(()); } },
            if let Some(status) = status { Status { status } }
            Icon { name: icon }
            span { class: "idle-navigation-name", "{name}" }
            if let Some(detail) = detail { span { class: "idle-navigation-detail", "{detail}" } }
        }
    }
}

#[component]
pub(super) fn Notice(text: String, #[props(default)] error: bool) -> Element {
    rsx! { p { class: "idle-navigation-notice", role: if error { "alert" } else { "status" }, "{text}" } }
}
