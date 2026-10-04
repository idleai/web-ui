use std::collections::BTreeSet;

use app_core::history::{Event as HistoryEvent, RequestState, ViewModel};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{
    Element, EventHandler, Key, Props, WritableExt, component, dioxus_core, dioxus_elements, rsx,
    use_signal,
};

use super::model::{Presentation, prepare, segments};
use super::sources::Sources;
use super::{ActivityIndicator, ActivityKind, ActivitySnapshot, ByteRange};
use crate::controls::Button;
use crate::host::HostCapabilities;

/// Attribution heatmap and source drill-down for the selected file observation.
///
/// Both hosts use the same component. Without a matching supplied snapshot, only
/// file-level actions from the shared history view are shown. Unknown ranges are
/// never painted as human, AI, unexposed or untouched. Load the history details
/// stylesheet and dispatch `onaction` through the owning app-core instance.
#[component]
pub fn AuthorActivity(
    id: String,
    view: ViewModel,
    capabilities: HostCapabilities,
    onaction: EventHandler<HistoryEvent>,
    activity: Option<ActivitySnapshot>,
) -> Element {
    let Some(presentation) = prepare(&view, activity.as_ref()) else {
        return rsx! {};
    };
    rsx! {
        ActivityBody {
            key: "{view.chain:?}:{presentation.record.operation}:{presentation.record.hash}",
            id, presentation, view, capabilities, onaction,
        }
    }
}

#[component]
fn ActivityBody(
    id: String,
    presentation: Presentation,
    view: ViewModel,
    capabilities: HostCapabilities,
    onaction: EventHandler<HistoryEvent>,
) -> Element {
    let mut selected = use_signal(|| None::<ByteRange>);
    let parts = presentation
        .text
        .as_ref()
        .map_or_else(Vec::new, |text| segments(text, &presentation.indicators));
    let selected_range = selected().filter(|range| parts.iter().any(|part| part.range == *range));
    let indicators: Vec<_> = presentation
        .indicators
        .iter()
        .filter(|indicator| {
            selected_range.is_none_or(|selected| {
                indicator
                    .range
                    .is_some_and(|range| range.start < selected.end && range.end > selected.start)
            })
        })
        .cloned()
        .collect();
    let mut sources: BTreeSet<_> = indicators
        .iter()
        .flat_map(|indicator| indicator.sources.iter().cloned())
        .collect();
    if selected_range.is_none() {
        sources.extend(presentation.unavailable_sources.iter().cloned());
    }
    rsx! {
        section { id: id.clone(), class: "idle-author-activity", aria_label: "Author and observed activity", aria_busy: (presentation.state == RequestState::Loading).to_string(),
            h3 { "Author and observed activity" }
            p { class: "idle-history-identity", "Revision: {presentation.revision.as_deref().unwrap_or(\"Not recorded\")}" }
            p { class: "idle-history-caption", "Attribution applies to the resulting recorded snapshot. File observations do not assign every byte to an author." }
            match &presentation.state {
                RequestState::Idle => rsx! { p { role: "status", "Additional activity has not been requested." } },
                RequestState::Loading => rsx! { p { role: "status", "Loading activity observations…" } },
                RequestState::Failed(error) => rsx! { p { role: "alert", "Activity unavailable: {error.message}" } },
                RequestState::Ready => rsx! {},
            }
            ActivitySummary { indicators: presentation.indicators.clone() }
            for issue in &presentation.issues { p { class: "idle-history-caption", "{issue}" } }
            if let Some(text) = &presentation.text {
                if text.is_empty() { p { "Recorded empty snapshot (0 bytes). There are no code ranges to color." } }
                else {
                    div { class: "idle-activity-legend", aria_label: "Attribution legend",
                        for (tone, label) in [("human", "Human"), ("ai", "AI"), ("mixed", "Multiple classifications"), ("other", "Other"), ("unknown", "Unknown")] {
                            span { "data-attribution": tone, "{label}" }
                        }
                    }
                    p { id: "{id}-help", class: "idle-history-caption", "Select a colored range to inspect its sources. Labels include exposure and touch observations. Byte ranges are half-open." }
                    pre { class: "idle-activity-heatmap", tabindex: "0", role: "region", aria_label: "Recorded code attribution heatmap", aria_describedby: "{id}-help",
                        code {
                            for part in parts {
                                span {
                                    key: "{part.range.start}:{part.range.end}", role: "button", tabindex: "0", class: "idle-activity-range",
                                    "data-attribution": part.tone(),
                                    "data-exposure": (part.kinds.contains(&ActivityKind::Exposure) || part.kinds.contains(&ActivityKind::Read)).to_string(),
                                    "data-touch": part.kinds.contains(&ActivityKind::Touch).to_string(),
                                    aria_label: part.label(), title: part.label(), aria_pressed: (selected_range == Some(part.range)).to_string(),
                                    onclick: move |_| selected.set(Some(part.range)),
                                    onkeydown: move |event| {
                                        if event.key() == Key::Enter || event.key() == Key::Character(" ".into()) {
                                            event.prevent_default();
                                            selected.set(Some(part.range));
                                        }
                                    },
                                    "{text.get(part.range.start..part.range.end).unwrap_or_default()}"
                                }
                            }
                        }
                    }
                }
            } else {
                p { class: "idle-activity-no-ranges", "Range heatmap unavailable: no matching complete snapshot and mapped ranges were supplied. File-level observations remain available below." }
            }
            if let Some(range) = selected_range {
                p { role: "status", "Sources for bytes {range.start}–{range.end}" }
                Button { label: "Show all activity sources", onpress: move |()| selected.set(None) }
            }
            h4 { "Recorded observations" }
            if indicators.is_empty() { p { "No observations supplied for this range. Attribution, exposure and touch are unknown." } }
            ul { class: "idle-activity-observations",
                for indicator in &indicators {
                    li {
                        strong { "{indicator.kind.label()}" }
                        " · {indicator.label} · "
                        if let Some(range) = indicator.range { "Bytes {range.start}–{range.end}" }
                        else { "File observation only; range coverage unknown" }
                    }
                }
            }
            Sources { id: "{id}-sources", sources: sources.into_iter().collect(), view, capabilities, onaction }
        }
    }
}

#[component]
fn ActivitySummary(indicators: Vec<ActivityIndicator>) -> Element {
    let has = |kind| indicators.iter().any(|indicator| indicator.kind == kind);
    let authors: Vec<_> = [
        ActivityKind::Human,
        ActivityKind::Ai,
        ActivityKind::Other,
        ActivityKind::Unknown,
    ]
    .into_iter()
    .filter(|kind| has(*kind))
    .map(ActivityKind::label)
    .collect();
    let attribution = if authors.is_empty() {
        "Unknown attribution".into()
    } else {
        authors.join(" · ")
    };
    rsx! {
        ul { class: "idle-activity-summary", aria_label: "Observed activity summary",
            li { "{attribution}" }
            li { if has(ActivityKind::Read) { "Read interval observed" } else if has(ActivityKind::Exposure) { "Exposure observed" } else { "Exposure unknown" } }
            li { if has(ActivityKind::Touch) { "Touch observed" } else { "Touch unknown" } }
        }
    }
}
