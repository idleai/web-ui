//! Recorded time and readable descriptions for Activity rows.

mod markdown;
#[cfg(test)]
mod tests;

#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{Element, dioxus_core, dioxus_elements, rsx};
use markdown::{MdInline, MdLineKind, Summary};

pub(super) fn timestamp(milliseconds: u64) -> String {
    #[cfg(target_arch = "wasm32")]
    if milliseconds <= 8_640_000_000_000_000
        && let Ok(value) = milliseconds.to_string().parse::<f64>()
    {
        let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(value));
        let month = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ]
        .get(usize::try_from(date.get_utc_month()).unwrap_or(0))
        .copied()
        .unwrap_or("");
        let hour = date.get_utc_hours();
        return format!(
            "{month} {}, {} {:02}:{:02} {}",
            date.get_utc_date(),
            date.get_utc_full_year(),
            if hour.is_multiple_of(12) {
                12
            } else {
                hour % 12
            },
            date.get_utc_minutes(),
            if hour < 12 { "AM" } else { "PM" }
        );
    }
    format!("{milliseconds} ms since Unix epoch")
}

/// Render the first meaningful line without creating links or inserting HTML.
pub(super) fn preview(text: &str) -> (Element, usize) {
    let summary = Summary::parse(text);
    let prefix = match &summary.line.kind {
        MdLineKind::Task { done } => if *done { "☑ " } else { "☐ " }.into(),
        MdLineKind::Unordered => "• ".into(),
        MdLineKind::Ordered { marker } => format!("{marker} "),
        MdLineKind::Quote { callout } => callout
            .as_ref()
            .map_or_else(|| "│ ".into(), |label| format!("{label}: ")),
        MdLineKind::Fence { language } => format!("{language} "),
        MdLineKind::Empty => Summary::EMPTY_LABEL.into(),
        MdLineKind::Plain | MdLineKind::Heading { .. } => String::new(),
    };
    let heading = matches!(summary.line.kind, MdLineKind::Heading { .. });
    (
        rsx! { span { class: "idle-summary-line", "data-heading": heading.to_string(),
            "{prefix}" {inline(&summary.line.inline)}
        } },
        summary.more,
    )
}

fn inline(tokens: &[MdInline]) -> Element {
    rsx! { for token in tokens { { match token {
        MdInline::Text(text) => rsx! { "{text}" },
        MdInline::Space => rsx! { "\u{a0}" },
        MdInline::Code(text) => rsx! { code { "{text}" } },
        MdInline::Strong(children) => rsx! { strong { {inline(children)} } },
        MdInline::Em(children) => rsx! { em { {inline(children)} } },
        MdInline::Strike(children) => rsx! { s { {inline(children)} } },
        MdInline::Link { label, target } | MdInline::Image { label, target } =>
            rsx! { span { class: "idle-summary-link", title: target.clone(), {inline(label)} } },
    } } } }
}
