//! Byte-preserving text/hex presentation with explicit preview bounds.

use std::fmt::Write as _;

use app_core::history::{BlockState, BlockView, ContentText};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{Element, Props, component, dioxus_core, dioxus_elements, rsx};

const PREVIEW_BYTES: usize = 320;

pub(super) fn hex(bytes: &[u8]) -> String {
    let mut result = String::new();
    for (line, chunk) in bytes.chunks(16).enumerate() {
        if line > 0 {
            result.push('\n');
        }
        for (column, byte) in chunk.iter().enumerate() {
            if column > 0 {
                result.push(' ');
            }
            let _written = write!(result, "{byte:02x}");
        }
    }
    result
}

fn prefix(bytes: &[u8], expanded: bool) -> &[u8] {
    if expanded || bytes.len() <= PREVIEW_BYTES {
        return bytes;
    }
    let mut end = PREVIEW_BYTES;
    if let Ok(text) = std::str::from_utf8(bytes) {
        while !text.is_char_boundary(end) {
            end = end.saturating_sub(1);
        }
    }
    bytes.get(..end).unwrap_or_default()
}

#[component]
pub(super) fn BytesView(bytes: Vec<u8>, expanded: bool, label: String) -> Element {
    let shown = prefix(&bytes, expanded);
    let truncated = shown.len() < bytes.len();
    let count = bytes.len();
    let shown_count = shown.len();
    let text = std::str::from_utf8(&bytes)
        .ok()
        .and_then(|_| std::str::from_utf8(shown).ok());
    let value = text.map_or_else(|| hex(shown), str::to_owned);
    let format = if text.is_some() {
        "UTF-8 text"
    } else {
        "Hex bytes"
    };
    rsx! {
        div { class: "idle-history-bytes", "data-truncated": truncated.to_string(),
            p { class: "idle-history-caption", "{label} · {count} bytes · {format}" }
            if bytes.is_empty() {
                p { class: "idle-history-empty-value", "Recorded empty content (0 bytes)" }
            } else {
                pre {
                    class: "idle-history-source", role: "region", tabindex: "0",
                    "data-history-action": "content", aria_label: "{label}: {format}",
                    onscroll: move |event| event.stop_propagation(),
                    "{value}"
                }
            }
            if truncated {
                p { class: "idle-history-warning", "Preview truncated: showing {shown_count} of {count} bytes. Expand this item for all loaded bytes." }
            }
        }
    }
}

#[component]
pub(super) fn ExactBytes(bytes: Vec<u8>, label: String) -> Element {
    let encoded = hex(&bytes);
    rsx! {
        BytesView { bytes: bytes.clone(), expanded: true, label: label.clone() }
        if !bytes.is_empty() && std::str::from_utf8(&bytes).is_ok() {
            p { class: "idle-history-caption", "Exact bytes in hexadecimal (includes whitespace and control bytes)" }
            pre {
                class: "idle-history-source idle-history-hex", role: "region", tabindex: "0",
                "data-history-action": "content", aria_label: "{label}: exact hexadecimal bytes",
                onscroll: move |event| event.stop_propagation(),
                "{encoded}"
            }
        }
    }
}

#[component]
pub(super) fn PreviewText(preview: ContentText, expanded: bool) -> Element {
    rsx! {
        BytesView { bytes: preview.text.into_bytes(), expanded, label: "Recorded preview" }
        if !preview.complete {
            p { class: "idle-history-warning", "Source preview is incomplete. Inspect the record for full fields and availability." }
        }
    }
}

#[component]
pub(super) fn HistoryBlock(block: BlockView, expanded: bool) -> Element {
    rsx! {
        section { class: "idle-history-block", "data-block": block.block.clone(),
            "data-attempt": block.attempt.clone(), "data-channel": block.channel.clone(),
            header { class: "idle-history-block-heading",
                strong { if block.attempt.is_some() { "Tool output" } else { "Message block" } }
                if let Some(position) = block.position { span { "Position {position}" } }
                if let Some(channel) = &block.channel { span { "Channel: {channel}" } }
            }
            if let Some(attempt) = &block.attempt { p { class: "idle-history-identity", "Attempt: {attempt}" } }
            if expanded {
                p { class: "idle-history-identity", "Block: {block.block}" }
                p { class: "idle-history-caption", "Media type: {block.media_type.as_deref().unwrap_or(\"not recorded\")}" }
            }
            {match &block.state {
                BlockState::Content { bytes, complete, finished, head } => rsx! {
                    p { class: if *complete { "idle-history-caption" } else { "idle-history-warning" },
                        if *complete { "Recorded prefix available" } else { "Partial content — predecessor or prefix missing" }
                        if *finished { " · Terminal lifecycle recorded" } else { " · No terminal lifecycle recorded" }
                    }
                    BytesView { bytes: bytes.clone(), expanded, label: "Reconstructed content" }
                    if expanded { p { class: "idle-history-identity", "Contributing head: {head}" } }
                },
                BlockState::Unavailable(reason) => rsx! { p { class: "idle-history-warning", "Content unavailable: {reason}. Inspect the observations to load or refresh recorded fields." } },
                BlockState::Conflicted(reason) => rsx! { p { class: "idle-history-warning", "Conflicted content: {reason}. Inspect the retained records; no branch is selected as complete." } },
            }}
        }
    }
}
