//! Repository, author and recorded-session views over shared application state.

mod sessions;
#[cfg(test)]
mod tests;
mod users;

use app_core::repository::{Event, ReadReport, ReadState, RepositoryLoadState, ViewModel};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{Element, EventHandler, Props, component, dioxus_core, dioxus_elements, rsx};

use crate::controls::{Button, ControlState};
pub use sessions::{RecordedSessions, RecordedSessionsProps};
pub use users::{RepositoryUsers, RepositoryUsersProps};

/// Shared repository presentation styles.
pub const STYLESHEET: &str = include_str!("../assets/repository.css");

/// Local checkout details and accessible GitHub repository metadata.
#[component]
pub fn RepositoryOverview(
    view: ViewModel,
    onaction: EventHandler<Event>,
    onopen: Option<EventHandler<String>>,
    now_ms: Option<u64>,
) -> Element {
    rsx! {
        section { class: "idle-repository idle-stack", aria_label: "Repository workspace",
            h2 { "Repository" }
            RepositoryFeedback { view: view.clone(), onaction }
            if let Some(snapshot) = &view.snapshot {
                if let Some(checkout) = &snapshot.checkout {
                    dl { class: "idle-repository-facts",
                        dt { "Checkout" } dd { code { "{checkout.root}" } }
                        dt { "Branch" } dd { {checkout.branch.clone().unwrap_or_else(|| "Detached HEAD".into())} }
                        dt { "HEAD" } dd { code { {checkout.head.clone().unwrap_or_else(|| "No commit yet".into())} } }
                        if let Some(remote) = &checkout.remote {
                            dt { "Remote {checkout.remote_name.as_deref().unwrap_or_default()}" } dd { code { "{remote}" } }
                        }
                    }
                    if let Some(status) = &checkout.status {
                        p { "{status.staged} staged · {status.unstaged} unstaged · {status.untracked} untracked · {status.conflicted} conflicted" }
                    }
                    details {
                        summary { "Worktree details" }
                        dl { class: "idle-repository-facts",
                            dt { "Checkout metadata" } dd { code { "{checkout.git_directory}" } }
                            dt { "Shared Git metadata" } dd { code { "{checkout.common_directory}" } }
                        }
                    }
                }
                if let Some(repository) = &snapshot.github {
                    section { aria_label: "GitHub repository",
                        h3 { "{repository.full_name}" }
                        if let Some(description) = &repository.description { p { "{description}" } }
                        p { "{repository.visibility} · default branch {repository.default_branch}" }
                        SourceLink { label: "Open repository on GitHub", url: repository.url.clone(), onopen }
                    }
                }
                p { class: "idle-muted", "GitHub access: " {snapshot.account.clone().unwrap_or_else(|| "public, signed out".into())} }
            }
            Button { label: "Connect GitHub repository access", state: control_state(&view), onpress: move |()| onaction.call(Event::SignIn) }
            SourceReports { view, prefix: String::new(), now_ms }
        }
    }
}

/// Read feedback shared by repository, author and recorded-session screens.
#[component]
pub fn RepositoryFeedback(view: ViewModel, onaction: EventHandler<Event>) -> Element {
    let state = control_state(&view);
    rsx! {
        div { class: "idle-repository-feedback",
            if view.load == RepositoryLoadState::Loading { p { role: "status", "Reading repository sources…" } }
            if view.needs_refresh && view.snapshot.is_some() { p { role: "status", "Showing the previous repository snapshot while a fresh read is pending." } }
            if let RepositoryLoadState::Failed(error) = &view.load { p { role: "alert", "{error.message}" } }
            if view.load == RepositoryLoadState::Suspended { p { role: "status", "Repository reads are paused until the workspace reconnects." } }
            if let Some(error) = &view.action_error { p { role: "alert", "{error.message}" } }
            Button { label: "Refresh repository", state, onpress: move |()| onaction.call(Event::Refresh) }
        }
    }
}

/// Per-source bounds, read failures and check times; an unavailable source is never empty.
#[component]
pub fn SourceReports(view: ViewModel, prefix: String, now_ms: Option<u64>) -> Element {
    let reports = view
        .snapshot
        .as_ref()
        .map(|snapshot| {
            snapshot
                .reports
                .iter()
                .filter(|report| report.topic.starts_with(&prefix))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    rsx! {
        div { class: "idle-repository-reports",
            for report in reports.iter().filter(|report| report.state != ReadState::Complete) {
                p { key: "notice:{report.topic}", role: "status", class: "idle-repository-notice", "{report.message}" }
            }
            if !reports.is_empty() {
                details {
                    summary { "Source read details" }
                    ul {
                        for report in reports {
                            li { key: "{report.topic}",
                                strong { "{source_name(&report.topic)} · {read_label(report.state)}" }
                                p { "{report.message}" }
                                p { class: "idle-muted", "{read_time(report, now_ms)}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Open a supplied source through a host callback; no callback means unavailable.
#[component]
pub fn SourceLink(label: String, url: String, onopen: Option<EventHandler<String>>) -> Element {
    rsx! {
        button { class: "idle-button idle-source-link", r#type: "button", title: url.clone(), disabled: onopen.is_none(),
            onclick: move |_| { if let Some(open) = onopen { open.call(url.clone()); } }, "{label}"
        }
    }
}

fn control_state(view: &ViewModel) -> ControlState {
    if view.context.is_none() || view.load == RepositoryLoadState::Suspended {
        ControlState::Disabled
    } else if view.load == RepositoryLoadState::Loading {
        ControlState::Busy
    } else {
        ControlState::Ready
    }
}

fn read_label(state: ReadState) -> &'static str {
    match state {
        ReadState::Complete => "Read complete",
        ReadState::Partial => "Partial",
        ReadState::Unavailable => "Unavailable",
        ReadState::NotApplicable => "No applicable source",
    }
}

fn source_name(topic: &str) -> &str {
    match topic {
        "git.checkout" => "Git checkout",
        "git.status" => "Git worktree",
        "git.authors" => "Git authors",
        "github.repository" => "GitHub repository",
        "github.contributors" => "GitHub contributors",
        "github.collaborators" => "GitHub collaborators",
        "github.issues" => "GitHub issues",
        "github.pulls" => "GitHub pull requests",
        "github.checks" => "GitHub checks",
        "github.runs" => "GitHub workflow runs",
        "history.sessions" => "Recorded sessions",
        _ => topic,
    }
}

fn read_time(report: &ReadReport, now_ms: Option<u64>) -> String {
    let Some(now) = now_ms else {
        return "Check time supplied by the repository reader.".into();
    };
    let seconds = now.saturating_sub(report.checked_at_ms) / 1000;
    let checked = if seconds < 60 {
        format!("Checked {seconds}s ago")
    } else if seconds < 3600 {
        format!("Checked {}m ago", seconds / 60)
    } else {
        format!("Checked {}h ago", seconds / 3600)
    };
    report.retry_at_ms.filter(|retry| *retry > now).map_or_else(
        || checked.clone(),
        |retry| {
            format!(
                "{checked} · retry in {}s",
                retry.saturating_sub(now).div_ceil(1000)
            )
        },
    )
}
