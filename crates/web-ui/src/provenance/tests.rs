use app_core::history::{
    ActivityKind as HistoryKind, ContentValue, Detail, FieldContent, ItemView, Observation,
    ObservationView, OperationDetails, OperationDetailsState, OperationDetailsView, Paging,
    RecordLookupStatus, RecordRef, RequestState, Selected, ViewModel,
};
use dioxus::prelude::*;

use super::model::{prepare, segments};
use super::{
    ActivityIndicator, ActivityKind, ActivitySnapshot, ActivitySource, AuthorActivity, ByteRange,
};
use crate::host::{HostCapabilities, HostKind};

mod retries;
mod sources;

fn reference(value: u64) -> RecordRef {
    RecordRef {
        operation: format!("{value:064x}"),
        hash: format!("{:064x}", value.saturating_add(100)),
    }
}

fn observation(value: u64, action: &str) -> ObservationView {
    ObservationView {
        record: reference(value),
        item: format!("{:064x}", value.saturating_add(200)),
        kind: HistoryKind::File,
        author: Some("agent-looking-name".into()),
        recorder: Some("human-looking-recorder".into()),
        session: None,
        turn: None,
        time_ms: None,
        sequence: None,
        parents: Vec::new(),
        causes: Vec::new(),
        original: None,
        converter: None,
        legacy: None,
        preview: None,
        problem: None,
        detail: Detail::File {
            path: "42".into(),
            revision: Some("revision-one".into()),
            action: action.into(),
            change: Some("Applied".into()),
            before: None,
            after: Some("content-one".into()),
            caused_by: None,
        },
    }
}

fn item(observation: ObservationView) -> ItemView {
    ItemView {
        key: observation.item.clone(),
        observations: vec![observation],
        blocks: Vec::new(),
        expanded: false,
        paging: Paging::default(),
    }
}

fn view() -> ViewModel {
    let selected = observation(1, "Snapshot");
    ViewModel {
        chain: Some("chain-one".into()),
        selected: Selected {
            item: Some(selected.item.clone()),
            observation: Some(selected.record.operation.clone()),
        },
        items: vec![item(selected)],
        ..ViewModel::default()
    }
}

fn indicator(kind: ActivityKind, range: Option<ByteRange>) -> ActivityIndicator {
    ActivityIndicator {
        kind,
        range,
        label: "Recorded author <script>run()</script>".into(),
        sources: vec![ActivitySource {
            record: reference(2),
            item: Some("source-item".into()),
            original: Some(reference(3).operation),
        }],
    }
}

fn snapshot() -> ActivitySnapshot {
    ActivitySnapshot {
        chain: "chain-one".into(),
        record: reference(1),
        path: "42".into(),
        revision: Some("revision-one".into()),
        content_id: Some("content-one".into()),
        text: Some("a🙂bc\n".into()),
        indicators: vec![
            indicator(ActivityKind::Human, Some(ByteRange { start: 0, end: 1 })),
            indicator(ActivityKind::Ai, Some(ByteRange { start: 1, end: 5 })),
            indicator(ActivityKind::Exposure, Some(ByteRange { start: 0, end: 5 })),
            indicator(ActivityKind::Touch, Some(ByteRange { start: 1, end: 5 })),
        ],
        issues: vec!["Capture gap with unknown affected ranges".into()],
        state: RequestState::Ready,
    }
}

fn lookup(record: RecordRef, status: RecordLookupStatus) -> OperationDetailsView {
    OperationDetailsView {
        operation: record.operation.clone(),
        state: OperationDetailsState::Ready(OperationDetails {
            operation: record.operation.clone(),
            observation: (status == RecordLookupStatus::Found).then_some(Observation {
                record,
                operation_json: Vec::new(),
            }),
            status,
            records: Vec::new(),
            fields: Vec::new(),
            comparison: None,
        }),
    }
}

#[test]
fn ranges_preserve_unicode_disjoint_activity_and_unknown_gaps() {
    let value = snapshot();
    let data = prepare(&view(), Some(&value)).unwrap();
    let parts = segments(data.text.as_deref().unwrap(), &data.indicators);
    assert_eq!(
        parts.len(),
        3,
        "boundaries follow byte ranges rather than scalar or UTF-16 counts"
    );
    assert_eq!(
        parts
            .iter()
            .map(super::model::Segment::tone)
            .collect::<Vec<_>>(),
        ["human", "ai", "unknown"],
        "unknown bytes remain an explicit band"
    );
    assert!(
        parts.get(1).unwrap().label().contains("Touch observed"),
        "touch is independently recorded"
    );
    assert!(
        parts.get(2).unwrap().label().contains("Exposure unknown")
            && parts.get(2).unwrap().label().contains("Touch unknown"),
        "unmarked bytes never imply known inactivity"
    );
    let bytes: usize = parts
        .iter()
        .map(|part| part.range.end.saturating_sub(part.range.start))
        .sum();
    assert_eq!(
        bytes,
        value.text.unwrap().len(),
        "heatmap covers each byte once, including unknown bytes"
    );
}

#[test]
fn overlapping_authors_remain_multiple_and_file_marks_do_not_color_code() {
    let mut value = snapshot();
    value.indicators.push(indicator(
        ActivityKind::Human,
        Some(ByteRange { start: 1, end: 5 }),
    ));
    value.indicators.push(indicator(ActivityKind::Human, None));
    let parts = segments(value.text.as_deref().unwrap(), &value.indicators);
    assert_eq!(
        parts.get(1).unwrap().tone(),
        "mixed",
        "overlap cannot choose an author"
    );
    assert!(
        parts.get(1).unwrap().label().contains("Human attribution")
            && parts.get(1).unwrap().label().contains("AI attribution"),
        "both classifications remain accessible without color"
    );
    assert_eq!(
        parts.get(2).unwrap().tone(),
        "unknown",
        "file observations never claim unchanged ranges"
    );
}

#[test]
fn invalid_or_unsupported_ranges_and_unsourced_marks_are_not_painted() {
    for range in [
        ByteRange { start: 2, end: 5 },
        ByteRange { start: 1, end: 4 },
        ByteRange { start: 5, end: 1 },
        ByteRange { start: 1, end: 1 },
        ByteRange {
            start: 0,
            end: usize::MAX,
        },
    ] {
        let mut value = snapshot();
        value.indicators = vec![indicator(ActivityKind::Ai, Some(range))];
        let data = prepare(&view(), Some(&value)).unwrap();
        assert!(
            data.indicators.is_empty(),
            "out-of-bounds, empty, reversed and split-scalar ranges are rejected"
        );
        assert!(
            !data.unavailable_sources.is_empty(),
            "rejected marks still offer source drill-down"
        );
    }
    let mut value = snapshot();
    value.indicators.first_mut().unwrap().sources.clear();
    let data = prepare(&view(), Some(&value)).unwrap();
    assert!(
        !data
            .indicators
            .iter()
            .any(|mark| mark.kind == ActivityKind::Human),
        "positive attribution needs a source"
    );
}

#[test]
fn every_scope_dimension_rejects_stale_marks_even_with_equal_text() {
    for change in 0..6 {
        let mut value = snapshot();
        match change {
            0 => value.chain = "other-chain".into(),
            1 => value.record.operation = reference(9).operation,
            2 => value.record.hash = reference(9).hash,
            3 => value.path = "other-path".into(),
            4 => value.revision = Some("other-revision".into()),
            _ => value.content_id = Some("other-content".into()),
        }
        let data = prepare(&view(), Some(&value)).unwrap();
        assert!(
            data.text.is_none(),
            "a stale input cannot color the current file"
        );
        assert!(
            !data
                .indicators
                .iter()
                .any(|mark| mark.kind == ActivityKind::Ai),
            "stale author classifications are removed"
        );
        assert!(
            data.issues
                .iter()
                .any(|issue| issue.contains("different chain, record or revision")),
            "the mismatch is visible"
        );
    }
}

#[test]
fn missing_conflicted_and_replaced_source_records_retract_marks_on_refresh() {
    for status in [
        RecordLookupStatus::Missing,
        RecordLookupStatus::Conflicted,
        RecordLookupStatus::Found,
    ] {
        let mut history = view();
        let mut record = reference(2);
        if status == RecordLookupStatus::Found {
            record.hash = reference(9).hash;
        }
        history.operation_details.push(lookup(record, status));
        let data = prepare(&history, Some(&snapshot())).unwrap();
        assert!(
            data.indicators.is_empty(),
            "no supplied classification survives rejected source identity"
        );
        assert!(
            !data.unavailable_sources.is_empty(),
            "the original source remains inspectable"
        );
    }
    let mut history = view();
    history
        .operation_details
        .push(lookup(reference(1), RecordLookupStatus::Conflicted));
    let data = prepare(&history, Some(&snapshot())).unwrap();
    assert!(
        data.text.is_none() && data.indicators.is_empty(),
        "a conflicted selected revision clears the entire heatmap"
    );
}

#[test]
fn fallback_uses_only_recorded_actions_and_exact_revision_identity() {
    let mut history = view();
    history.items = vec![item(observation(1, "Change"))];
    history.items.push(item(observation(2, "View")));
    history.items.push(item(observation(3, "Read")));
    let data = prepare(&history, None).unwrap();
    let kinds: Vec<_> = data.indicators.iter().map(|mark| mark.kind).collect();
    assert_eq!(
        kinds,
        [
            ActivityKind::Unknown,
            ActivityKind::Touch,
            ActivityKind::Exposure,
            ActivityKind::Read
        ],
        "recorded actions survive without guessing authors from names"
    );
    let empty = ActivitySnapshot {
        indicators: Vec::new(),
        ..snapshot()
    };
    let data = prepare(&history, Some(&empty)).unwrap();
    assert!(
        data.indicators
            .iter()
            .any(|indicator| indicator.kind == ActivityKind::Touch && indicator.range.is_none()),
        "missing supplied ranges do not erase a known file-level action"
    );
    for other in history.items.iter_mut().skip(1) {
        if let Detail::File { revision, .. } = &mut other.observations.first_mut().unwrap().detail {
            *revision = Some("another-occurrence-with-equal-bytes".into());
        }
    }
    let data = prepare(&history, None).unwrap();
    assert_eq!(
        data.indicators.len(),
        2,
        "equal content cannot transfer exposure from another occurrence"
    );
    if let Detail::File { change, .. } = &mut history
        .items
        .first_mut()
        .unwrap()
        .observations
        .first_mut()
        .unwrap()
        .detail
    {
        *change = Some("Proposed".into());
    }
    let data = prepare(&history, None).unwrap();
    assert_eq!(
        data.indicators.len(),
        1,
        "a proposed edit is not a recorded touch"
    );
}

#[test]
fn missing_empty_and_contradictory_snapshots_remain_distinct() {
    let mut value = snapshot();
    value.text = None;
    assert!(
        prepare(&view(), Some(&value))
            .unwrap()
            .indicators
            .is_empty(),
        "ranges cannot be used without a snapshot"
    );
    value.text = Some(String::new());
    value.indicators.clear();
    assert_eq!(
        prepare(&view(), Some(&value)).unwrap().text,
        Some(String::new()),
        "known empty stays distinct from missing"
    );
    for content in [
        ContentValue::Missing,
        ContentValue::Corrupt,
        ContentValue::Available(b"different bytes".to_vec()),
    ] {
        let mut history = view();
        let mut loaded = lookup(reference(1), RecordLookupStatus::Found);
        if let OperationDetailsState::Ready(details) = &mut loaded.state {
            details.fields.push(FieldContent {
                record: reference(1),
                field: "\"FileAfter\"".into(),
                content_id: None,
                declared_length: None,
                value: content,
            });
        }
        history.operation_details.push(loaded);
        assert!(
            prepare(&history, Some(&snapshot())).unwrap().text.is_none(),
            "known snapshot failures cannot be replaced with supplied text"
        );
    }
}

#[test]
fn loading_failure_and_selected_item_retention_do_not_leak_supplied_marks() {
    let mut history = view();
    history.selected_item = history.items.pop();
    assert!(
        prepare(&history, Some(&snapshot())).unwrap().text.is_some(),
        "selected files survive page filtering"
    );
    for state in [
        RequestState::Idle,
        RequestState::Loading,
        RequestState::Failed(app_core::module::EffectError {
            message: "offline".into(),
        }),
    ] {
        let value = ActivitySnapshot {
            state: state.clone(),
            ..snapshot()
        };
        let data = prepare(&history, Some(&value)).unwrap();
        assert_eq!(data.state, state, "supplied request state remains visible");
        assert!(
            data.text.is_none(),
            "stale snapshot marks are not displayed as current"
        );
    }
}

#[test]
fn both_hosts_render_escaped_labels_and_keep_source_and_original_identities() {
    fn fixture(kind: HostKind) -> Element {
        rsx! { AuthorActivity { id: "activity", view: view(), activity: Some(snapshot()), capabilities: HostCapabilities::new(kind), onaction: |_| {} } }
    }
    for host in [HostKind::Browser, HostKind::VsCode] {
        let mut dom = VirtualDom::new_with_props(fixture, host);
        let mutations = dom.rebuild_to_vec();
        sources::toggle(&mut dom, &mutations, "activity-sources-0-summary");
        let html = dioxus_ssr::render(&dom);
        for label in [
            "Human attribution",
            "AI attribution",
            "Exposure observed",
            "Touch observed",
            "Unknown",
            "Capture gap",
            "Original captured content",
            &reference(2).operation,
            &reference(2).hash,
            &reference(3).operation,
        ] {
            assert!(html.contains(label), "shared presentation retains {label}");
        }
        assert!(
            !html.contains("<script>run()"),
            "recorded labels never execute as markup"
        );
        assert!(
            html.contains("unavailable in this host"),
            "native actions require explicit capabilities"
        );
    }
}
