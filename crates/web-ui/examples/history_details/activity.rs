//! Development-only supplied range classifications; no production adapter.

use app_core::history::{ActivityKind as HistoryKind, ContentText, Detail, ItemView, RequestState};
use web_ui::provenance::{
    ActivityIndicator, ActivityKind, ActivitySnapshot, ActivitySource, ByteRange,
};

use super::fixture::{id, item, observation, reference};

const HUMAN: &str = "fn greet() {\n";
const AI: &str = "    let name = \"Ada 🙂\";\n";
const UNKNOWN: &str = "    println!(\"Hello, {name}\");\n}\n";

pub(super) fn text() -> String {
    format!("{HUMAN}{AI}{UNKNOWN}")
}

pub(super) fn file() -> ItemView {
    let mut file = observation(8, HistoryKind::File);
    file.preview = Some(ContentText::new("src/greet.rs".into(), true));
    file.detail = Detail::File {
        path: "88".into(),
        revision: Some(id(800)),
        action: "Change".into(),
        change: Some("Applied".into()),
        before: None,
        after: Some("greet-content".into()),
        caused_by: None,
    };
    item(file, Vec::new())
}

pub(super) fn snapshot() -> ActivitySnapshot {
    let human = ByteRange {
        start: 0,
        end: HUMAN.len(),
    };
    let ai = ByteRange {
        start: HUMAN.len(),
        end: HUMAN.len().saturating_add(AI.len()),
    };
    let marks = [
        (ActivityKind::Human, human, 8, "Human author: Ada"),
        (ActivityKind::Ai, ai, 9, "AI author: recorded agent"),
        (
            ActivityKind::Exposure,
            human,
            10,
            "Visible interval recorded",
        ),
        (ActivityKind::Touch, human, 8, "Applied human edit recorded"),
    ];
    ActivitySnapshot {
        chain: "local-details-fixture".into(),
        record: reference(8),
        path: "88".into(),
        revision: Some(id(800)),
        content_id: Some("greet-content".into()),
        text: Some(text()),
        indicators: marks
            .into_iter()
            .map(|(kind, range, number, label)| ActivityIndicator {
                kind,
                range: Some(range),
                label: label.into(),
                sources: vec![ActivitySource {
                    record: reference(number),
                    item: Some(id(number.saturating_mul(10))),
                    original: Some(id(7)),
                }],
            })
            .collect(),
        issues: vec!["Capture gap: affected ranges are unknown.".into()],
        state: RequestState::Ready,
    }
}
