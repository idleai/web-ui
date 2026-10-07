//! A user is always keyed and named by contributor identity.

use app_core::{
    Event,
    workspace::{self, MemberStatus, NavigationSection, PresenceStatus, WorkspaceRequestState},
};
use dioxus::prelude::*;

use super::super::chrome::{NavRow, Notice, RowStatus, Section, navigate};
use crate::icons::IconName;

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
                        NavRow { name: user.member.display_name.clone(), icon: IconName::User,
                            identity: user.member.contributor_id.clone(), status: connection_status(user),
                            detail: work_summary(user), disabled: view.selected_workspace.is_none(),
                            onpress: move |()| navigate(onaction, NavigationSection::Members),
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

fn work_summary(user: &workspace::MemberView) -> String {
    let connections = user
        .connections
        .iter()
        .map(|connection| {
            let mut parts = Vec::new();
            if let Some(summary) = &connection.summary {
                parts.push(summary.clone());
            }
            if let Some(file) = &connection.file {
                parts.push(file.clone());
            }
            if let Some(branch) = &connection.branch {
                parts.push(format!("Branch: {branch}"));
            }
            if let Some(host) = &connection.host_id {
                parts.push(format!("Host: {host}"));
            }
            parts.join(" · ")
        })
        .filter(|summary| !summary.is_empty())
        .collect::<Vec<_>>();
    if connections.is_empty() {
        connection_status(user).label
    } else {
        connections.join("; ")
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
