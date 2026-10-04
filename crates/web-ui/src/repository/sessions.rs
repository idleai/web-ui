use app_core::repository::{Event, ViewModel};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{Element, EventHandler, Props, component, dioxus_core, dioxus_elements, rsx};

use super::{RepositoryFeedback, SourceReports};
use crate::controls::Button;

/// Recorded local/imported sessions with exact item and observation selection.
#[component]
pub fn RecordedSessions(
    view: ViewModel,
    onaction: EventHandler<Event>,
    now_ms: Option<u64>,
) -> Element {
    rsx! {
        section { class: "idle-repository idle-stack", aria_label: "Recorded sessions",
            h2 { "Sessions" }
            article { class: "idle-repository-control", aria_label: "Control model",
                h3 { "Control model" }
                p { "No runtime connected" }
                p { class: "idle-muted", "Agent creation and input become available when a runtime is connected. Recorded sessions remain available below." }
            }
            RepositoryFeedback { view: view.clone(), onaction }
            h3 { "Recorded sessions" }
            if let Some(snapshot) = &view.snapshot {
                for session in &snapshot.sessions {
                    article { key: "{session.id}", class: "idle-recorded-session", "data-session-id": session.id.clone(),
                        "data-selected": (view.selected_session.as_ref() == Some(&session.id)).to_string(),
                        Button { label: session.labels.first().cloned().unwrap_or_else(|| "Untitled recorded session".into()), onpress: {
                            let id = session.id.clone(); move |()| onaction.call(Event::SelectSession(Some(id.clone())))
                        } }
                        for label in session.labels.iter().skip(1) { p { "Also recorded as: {label}" } }
                        p { class: "idle-muted", "Recorded events: " {session.actions.join(", ")} }
                        details {
                            summary { "Session sources and records" }
                            code { "{session.id}" }
                            for source in &session.sources { p { "{source}" } }
                            for (index, record) in session.records.iter().enumerate() {
                                Button { label: format!("Inspect session record {}", index.saturating_add(1)), onpress: {
                                    let session = session.id.clone(); let record = record.clone();
                                    move |()| onaction.call(Event::Inspect { session: session.clone(), record: record.clone() })
                                } }
                            }
                        }
                    }
                }
                if snapshot.sessions.is_empty() { p { "No recorded sessions loaded. Capture activity or import a session, then refresh." } }
            }
            if view.selected_session.is_some() {
                Button { label: "Show all recorded history", onpress: move |()| onaction.call(Event::SelectSession(None)) }
            }
            SourceReports { view, prefix: "history.sessions", now_ms }
        }
    }
}
