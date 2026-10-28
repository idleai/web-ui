//! Recorded time and readable descriptions for Activity rows.

mod markdown;
#[cfg(test)]
mod tests;

/// Join meaningful lines as plain text, matching the shipped table summary.
pub(super) fn plain(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .split('\n')
        .map(markdown::markdown_plain_line)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}

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
