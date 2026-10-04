use app_core::{
    repository::{Event, GithubPerson, ViewModel},
    workspace,
};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{Element, EventHandler, Props, component, dioxus_core, dioxus_elements, rsx};

use super::{RepositoryFeedback, SourceLink, SourceReports};

/// Idle membership and current presence, followed by separate recorded author lists.
#[component]
pub fn RepositoryUsers(
    view: ViewModel,
    workspace: workspace::ViewModel,
    onaction: EventHandler<Event>,
    onopen: Option<EventHandler<String>>,
    now_ms: Option<u64>,
) -> Element {
    rsx! {
        section { class: "idle-repository idle-stack", aria_label: "Workspace users",
            h2 { "Users" }
            section { aria_label: "Idle members and presence",
                h3 { "Idle members and presence" }
                for user in &workspace.members {
                    article { key: "{user.member.contributor_id}", class: "idle-repository-person",
                        h4 { "{user.member.display_name}" }
                        p { "{user.member.role:?} · {user.presence:?}" }
                    }
                }
                if workspace.members.is_empty() { p { "No Idle members are available." } }
            }
            if view.context.is_some() {
                RepositoryFeedback { view: view.clone(), onaction }
                p { class: "idle-muted", "The lists below describe recorded authorship and GitHub access. They do not grant Idle permissions or indicate who is online." }
                if let Some(snapshot) = &view.snapshot {
                    section { aria_label: "Git authors",
                        h3 { "Git authors" }
                        for (index, author) in snapshot.git_authors.iter().enumerate() {
                            article { key: "{index}", class: "idle-repository-person",
                                h4 { "{author.name}" }
                                p { "{author.email} · {author.commits} commits in this read" }
                            }
                        }
                        if snapshot.git_authors.is_empty() { p { "No Git authors loaded. See the source read details for coverage." } }
                    }
                    People { title: "GitHub contributors", people: snapshot.contributors.clone(), onopen }
                    People { title: "Accessible GitHub collaborators", people: snapshot.collaborators.clone(), onopen }
                }
                SourceReports { view: view.clone(), prefix: "git", now_ms }
            }
        }
    }
}

#[component]
fn People(
    title: String,
    people: Vec<GithubPerson>,
    onopen: Option<EventHandler<String>>,
) -> Element {
    rsx! {
        section { aria_label: title.clone(),
            h3 { "{title}" }
            for person in &people {
                article { key: "{person.id}", class: "idle-repository-person",
                    h4 { "{person.login}" }
                    if let Some(count) = person.contributions { p { "{count} GitHub contributions" } }
                    if let Some(role) = &person.role { p { "GitHub role: {role}" } }
                    SourceLink { label: format!("Open {} on GitHub", person.login), url: person.url.clone(), onopen }
                }
            }
            if people.is_empty() { p { "No entries loaded. See the source read details for access and coverage." } }
        }
    }
}
