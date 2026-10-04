//! A user is always keyed and named by contributor identity.

use app_core::{
    Event,
    workspace::{self, MemberStatus, NavigationSection, PresenceStatus, WorkspaceRequestState},
};
use dioxus::prelude::*;

use super::super::chrome::{Notice, RowStatus, Section, Status};
use crate::icons::{Icon, IconName};

#[component]
pub(in crate::navigation) fn Users(
    view: workspace::ViewModel,
    onaction: EventHandler<Event>,
) -> Element {
    let count = view
        .snapshot
        .as_ref()
        .map(|_| view.members.len().to_string());
    rsx! {
        Section { section: NavigationSection::Members, current: view.section, enabled: view.selected_workspace.is_some(), count, onaction,
            ul { class: "idle-navigation-list",
                for user in &view.members {
                    li { key: "{user.member.contributor_id}", "data-contributor": user.member.contributor_id.clone(),
                        div { class: "idle-navigation-row", title: "{user.member.display_name} · {user.member.contributor_id}",
                            Status { status: connection_status(user) }
                            Icon { name: IconName::User }
                            span { class: "idle-navigation-name", "{user.member.display_name}" }
                            span { class: "idle-navigation-detail", "{connection_status(user).label}" }
                        }
                        if !user.connections.is_empty() {
                            ul { class: "idle-navigation-connections",
                                for connection in &user.connections {
                                    li { key: "{connection.connection_id}",
                                        if let Some(summary) = &connection.summary { span { "{summary}" } }
                                        if let Some(host) = &connection.host_id { span { " · Host: {host}" } }
                                        if let Some(branch) = &connection.branch { span { " · Branch: {branch}" } }
                                        if let Some(file) = &connection.file { span { " · {file}" } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            match view.presence_state {
                WorkspaceRequestState::Idle => rsx! { Notice { text: "Online status not loaded" } },
                WorkspaceRequestState::Loading => rsx! { Notice { text: "Updating online status…" } },
                WorkspaceRequestState::Ready => rsx! {},
                WorkspaceRequestState::Failed(error) => rsx! { Notice { text: error.message, error: true } },
            }
            if view.snapshot.is_some() && view.members.is_empty() { Notice { text: "No users in this workspace" } }
        }
    }
}

fn connection_status(user: &workspace::MemberView) -> RowStatus {
    let (label, tone) = if user.member.status == MemberStatus::Revoked {
        ("Revoked", "danger")
    } else {
        match user.presence {
            PresenceStatus::Unknown => ("Online status unknown", "neutral"),
            PresenceStatus::Online => ("Online", "success"),
            PresenceStatus::Away => ("Away", "warning"),
            PresenceStatus::Offline => ("Offline", "neutral"),
        }
    };
    RowStatus {
        label: label.into(),
        tone,
    }
}
