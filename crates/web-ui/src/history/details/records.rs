//! Exact stored encodings and field availability, including quarantined variants.

use app_core::history::{
    Comparison, ContentValue, Event as HistoryEvent, FieldContent, OpenTarget, OperationDetails,
    OperationDetailsState, RecordLookupStatus, RequestState,
};
use dioxus::prelude::*;

use super::actions::{LoadButton, OpenButton};
use super::content::ExactBytes;
use super::observation::Fact;
use crate::host::HostCapabilities;

#[component]
pub(crate) fn OperationPanel(
    id: String,
    operation: String,
    state: OperationDetailsState,
    original: bool,
    capabilities: HostCapabilities,
    open: RequestState,
    onaction: EventHandler<HistoryEvent>,
) -> Element {
    let loading = state == OperationDetailsState::Loading;
    let label = match &state {
        OperationDetailsState::Idle => "Load exact records and content",
        OperationDetailsState::Loading => "Loading records…",
        OperationDetailsState::Failed(_) => "Retry records and content",
        OperationDetailsState::Ready(_) => "Refresh records and content",
    };
    rsx! {
        section { id: id.clone(), class: "idle-history-operation", "data-operation": operation.clone(), aria_busy: loading.to_string(),
            h3 { if original { "Original captured content" } else { "Recorded content" } }
            p { class: "idle-history-identity", "Observation: {operation}" }
            LoadButton { operation: operation.clone(), label, busy: loading, refresh: matches!(state, OperationDetailsState::Ready(_)), onaction }
            {match state {
                OperationDetailsState::Idle => rsx! { p { "Exact bytes have not been requested." } },
                OperationDetailsState::Loading => rsx! { p { role: "status", "Loading exact records and fields…" } },
                OperationDetailsState::Failed(error) => rsx! { p { role: "alert", "{error.message}" } },
                OperationDetailsState::Ready(details) => rsx! { RecordContents { id: id.clone(), details, original, capabilities, open, onaction } },
            }}
        }
    }
}

#[component]
fn RecordContents(
    id: String,
    details: OperationDetails,
    original: bool,
    capabilities: HostCapabilities,
    open: RequestState,
    onaction: EventHandler<HistoryEvent>,
) -> Element {
    let found = details.status == RecordLookupStatus::Found;
    rsx! {
        {match details.status {
            RecordLookupStatus::Found => rsx! { p { class: "idle-history-caption", "One accepted record. Each field reports its own availability." } },
            RecordLookupStatus::Missing => rsx! { p { class: "idle-history-warning", "Record missing. No stored representation is available; this is not empty content." } },
            RecordLookupStatus::Conflicted => rsx! { p { class: "idle-history-warning", "Conflicted observation. All retained variants are shown separately; none is selected as fact." } },
        }}
        if found {
            if let Some(accepted) = &details.observation {
                div { class: "idle-history-native-actions",
                    if original {
                        OpenButton { id: "{id}-original", record: accepted.record.clone(), target: OpenTarget::Original, capabilities: capabilities.clone(), state: open.clone(), onaction }
                    }
                    if details.comparison.is_some() {
                        OpenButton { id: "{id}-file", record: accepted.record.clone(), target: OpenTarget::File, capabilities: capabilities.clone(), state: open.clone(), onaction }
                        OpenButton { id: "{id}-diff", record: accepted.record.clone(), target: OpenTarget::Diff, capabilities: capabilities.clone(), state: open.clone(), onaction }
                    }
                }
            }
            if let Some(comparison) = &details.comparison { ComparisonView { comparison: comparison.clone() } }
            h4 { if original { "Captured Original fields" } else { "Recorded fields" } }
            if details.fields.is_empty() { p { "No content fields were recorded." } }
            div { class: "idle-history-fields",
                for field in &details.fields {
                    FieldView { key: "{field.record.operation}:{field.record.hash}:{field.field}", field: field.clone() }
                }
            }
        }
        h4 { "Stored encodings" }
        p { class: "idle-history-caption", "These are the retained bytes. Text is displayed without reformatting; hexadecimal preserves every byte. Scroll within a content panel to read all bytes." }
        for record in &details.records {
            article { key: "{record.record.operation}:{record.record.hash}", class: "idle-history-raw-record",
                "data-record-hash": record.record.hash.clone(),
                dl { class: "idle-history-facts",
                    Fact { label: "Observation", value: record.record.operation.clone() }
                    Fact { label: "Record digest", value: record.record.hash.clone() }
                }
                OpenButton { id: "{id}-record-{record.record.hash}", record: record.record.clone(), target: OpenTarget::Record, capabilities: capabilities.clone(), state: open.clone(), onaction }
                ExactBytes { bytes: record.bytes.clone(), label: "Stored record" }
            }
        }
    }
}

#[component]
fn FieldView(field: FieldContent) -> Element {
    let label = field_label(&field.field);
    rsx! {
        article { class: "idle-history-field", "data-field": field.field.clone(), "data-record-hash": field.record.hash.clone(),
            h5 { "{label}" }
            dl { class: "idle-history-facts",
                Fact { label: "Field selector", value: field.field.clone() }
                if let Some(content_id) = &field.content_id { Fact { label: "Content identity", value: content_id.clone() } }
                if let Some(length) = field.declared_length { Fact { label: "Declared byte length", value: length.to_string() } }
            }
            {match &field.value {
                ContentValue::Available(bytes) => rsx! { ExactBytes { bytes: bytes.clone(), label } },
                ContentValue::NotRecorded => rsx! { p { class: "idle-history-caption", "Not recorded or not applicable. No empty value is implied." } },
                ContentValue::Missing => rsx! { p { class: "idle-history-warning", "Content missing. Referenced bytes have not arrived. Refresh to check again." } },
                ContentValue::Corrupt => rsx! { p { class: "idle-history-warning", "Content corrupt: recorded address or length verification failed." } },
                ContentValue::Unresolvable => rsx! { p { class: "idle-history-warning", "Content unresolvable by this host." } },
            }}
        }
    }
}

fn field_label(selector: &str) -> &'static str {
    match selector {
        "\"FileBase\"" => "Before snapshot",
        "\"FileAfter\"" => "After snapshot",
        "\"FileEdit\"" | r#"{"Record":"Edit"}"# => "Recorded edit",
        "\"ImportRaw\"" | "\"UnknownRaw\"" | r#"{"Record":"Content"}"# => "Captured content",
        r#"{"Record":"Path"}"# => "Recorded file path",
        r#"{"Record":"RenamedTo"}"# => "Rename destination",
        r#"{"Record":"Arguments"}"# => "Tool arguments",
        "\"ToolName\"" | r#"{"Record":"ToolName"}"# => "Tool name",
        r#"{"Record":"Output"}"# => "Tool output",
        r#"{"Record":"Command"}"# => "Command",
        r#"{"Record":"Cwd"}"# => "Working directory",
        r#"{"Record":"Outcome"}"# => "Recorded outcome",
        _ => "Recorded field",
    }
}

#[component]
fn ComparisonView(comparison: Comparison) -> Element {
    rsx! {
        section { class: "idle-history-comparison", aria_label: "Recorded file comparison",
            h4 { "File comparison" }
            {match comparison {
                Comparison::Unavailable => rsx! { p { class: "idle-history-warning", "Comparison unavailable: at least one complete snapshot is unavailable. Recorded edits alone do not establish a complete file." } },
                Comparison::Identical => rsx! { p { "The complete before and after snapshots are identical." } },
                Comparison::Changed { before_start, before_end, after_start, after_end } => rsx! {
                    p { "Replace before bytes [{before_start}, {before_end}) with after bytes [{after_start}, {after_end}). Offsets are bytes; the end is exclusive." }
                    p { class: "idle-history-caption", "The exact before and after snapshots are shown below." }
                },
            }}
        }
    }
}
