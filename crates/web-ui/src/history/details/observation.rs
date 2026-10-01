//! Recorded attribution, lifecycle and file identities without inferred state.

use app_core::history::{
    ActivityKind, Detail, Endpoint, Event as HistoryEvent, ObservationView, Selected,
};
use dioxus::prelude::*;

use crate::controls::Button;

pub(super) const fn kind_label(kind: ActivityKind) -> &'static str {
    match kind {
        ActivityKind::Session => "Session",
        ActivityKind::Turn => "Turn",
        ActivityKind::Message => "Message",
        ActivityKind::Tool => "Tool",
        ActivityKind::File => "File",
        ActivityKind::Commit => "Commit",
        ActivityKind::Note => "Note",
        ActivityKind::Author => "Author",
        ActivityKind::Link => "Link",
        ActivityKind::Original => "Original",
        ActivityKind::Initialization => "Chain initialization",
        ActivityKind::Unknown => "Unknown record",
    }
}

#[component]
pub(super) fn Fact(label: String, value: String) -> Element {
    rsx! { dt { "{label}" } dd { "{value}" } }
}

#[component]
pub(super) fn ObservationCard(
    observation: ObservationView,
    onaction: EventHandler<HistoryEvent>,
) -> Element {
    let selected = Selected {
        item: Some(observation.item.clone()),
        observation: Some(observation.record.operation.clone()),
    };
    rsx! {
        article { class: "idle-history-observation", "data-operation": observation.record.operation.clone(), "data-record-hash": observation.record.hash.clone(),
            header { class: "idle-history-block-heading",
                strong { "{kind_label(observation.kind)} observation" }
                Button { label: "Inspect record", onpress: move |()| onaction.call(HistoryEvent::Select(selected.clone())) }
            }
            if let Some(problem) = &observation.problem { p { class: "idle-history-warning", "{problem}" } }
            dl { class: "idle-history-facts",
                Fact { label: "Observation", value: observation.record.operation.clone() }
                Fact { label: "Record digest", value: observation.record.hash.clone() }
                Fact { label: "Author", value: observation.author.clone().unwrap_or_else(|| "Not recorded".into()) }
                Fact { label: "Recorder", value: observation.recorder.clone().unwrap_or_else(|| "Not recorded".into()) }
                if let Some(session) = &observation.session { Fact { label: "Session", value: session.clone() } }
                if let Some(turn) = &observation.turn { Fact { label: "Turn", value: turn.clone() } }
                if let Some(time) = observation.time_ms { Fact { label: "Recorded time (Unix milliseconds)", value: time.to_string() } }
                if let Some(sequence) = observation.sequence { Fact { label: "Recorder sequence", value: sequence.to_string() } }
                if let Some(original) = &observation.original { Fact { label: "Original observation", value: original.clone() } }
                if let Some(converter) = &observation.converter { Fact { label: "Converter", value: converter.clone() } }
                if let Some(legacy) = &observation.legacy { Fact { label: "Legacy observation", value: legacy.clone() } }
                for parent in &observation.parents { Fact { label: "Causal parent", value: parent.clone() } }
                for cause in &observation.causes { Fact { label: "Logical cause", value: cause.clone() } }
                DetailFacts { detail: observation.detail.clone() }
            }
        }
    }
}

#[component]
pub(super) fn DetailFacts(detail: Detail) -> Element {
    match detail {
        Detail::Other => rsx! {},
        Detail::Message { category, stage } => rsx! {
            Fact { label: "Category", value: category }
            Fact { label: "Recorded stage", value: stage }
        },
        Detail::Tool {
            attempt,
            channel,
            stage,
            parent_call,
        } => rsx! {
            Fact { label: "Attempt", value: attempt }
            Fact { label: "Channel", value: channel }
            Fact { label: "Recorded stage", value: stage }
            if let Some(parent_call) = parent_call { Fact { label: "Parent call", value: parent_call } }
        },
        Detail::File {
            path,
            revision,
            action,
            change,
            before,
            after,
            caused_by,
        } => rsx! {
            Fact { label: "Path identity", value: path }
            Fact { label: "Revision", value: revision.unwrap_or_else(|| "Not recorded".into()) }
            Fact { label: "Recorded action", value: action }
            Fact { label: "Change state", value: change.unwrap_or_else(|| "Not recorded".into()) }
            Fact { label: "Before content identity", value: before.unwrap_or_else(|| "Not recorded".into()) }
            Fact { label: "After content identity", value: after.unwrap_or_else(|| "Not recorded".into()) }
            if let Some(caused_by) = caused_by { Fact { label: "Causing call", value: caused_by } }
        },
        Detail::Link { from, relation, to } => rsx! {
            Fact { label: "From", value: endpoint_label(&from) }
            Fact { label: "Relation", value: relation }
            for endpoint in to { Fact { label: "To", value: endpoint_label(&endpoint) } }
        },
    }
}

fn endpoint_label(endpoint: &Endpoint) -> String {
    match endpoint {
        Endpoint::Observation(id) => format!("Observation {id}"),
        Endpoint::Item(id) => format!("Item {id}"),
        Endpoint::Git { repository, oid } => format!("Git {oid} in repository {repository}"),
    }
}
