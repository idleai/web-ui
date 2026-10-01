//! Rendering contracts for complete bytes, stream gaps and exact record variants.

use app_core::history::{
    ActivityKind, BlockState, BlockView, Comparison, ContentText, ContentValue, Detail,
    FieldContent, ItemView, ObservationView, OperationDetails, OperationDetailsState,
    OperationDetailsView, Paging, RawRecord, RecordLookupStatus, RecordRef, RequestState, Selected,
    ViewModel,
};
use dioxus::prelude::*;

use super::content::{BytesView, ExactBytes};
use super::records::OperationPanel;
use super::{HistoryDetails, HistoryRow};
use crate::host::{HostCapabilities, HostKind};

mod actions;

fn id(value: u64) -> String {
    format!("{value:064x}")
}

fn reference(value: u64) -> RecordRef {
    RecordRef {
        operation: id(value),
        hash: id(value.saturating_add(100)),
    }
}

fn observation(value: u64) -> ObservationView {
    ObservationView {
        record: reference(value),
        item: id(10),
        kind: ActivityKind::Message,
        author: Some("human author".into()),
        recorder: Some("import adapter".into()),
        session: Some(id(11)),
        turn: Some(id(12)),
        time_ms: Some(42),
        sequence: Some(3),
        parents: vec![id(7), id(8), id(9)],
        causes: vec![id(13)],
        original: Some(id(20)),
        converter: Some("recorded-converter".into()),
        legacy: Some(id(30)),
        detail: Detail::Message {
            category: "Reasoning".into(),
            stage: "Started".into(),
        },
        preview: None,
        problem: None,
    }
}

fn block(value: u64, bytes: &[u8]) -> BlockView {
    BlockView {
        block: id(value),
        position: Some(u32::try_from(value).unwrap()),
        attempt: None,
        channel: None,
        media_type: Some("text/plain".into()),
        state: BlockState::Content {
            bytes: bytes.to_vec(),
            complete: true,
            finished: false,
            head: id(1),
        },
    }
}

fn item() -> ItemView {
    ItemView {
        key: id(10),
        observations: vec![observation(1)],
        blocks: vec![block(1, b"first block")],
        expanded: false,
        paging: Paging::default(),
    }
}

fn details() -> OperationDetails {
    OperationDetails {
        operation: id(1),
        status: RecordLookupStatus::Found,
        observation: Some(app_core::history::Observation {
            record: reference(1),
            operation_json: b"transport".to_vec(),
        }),
        records: vec![RawRecord {
            record: reference(1),
            bytes: b" {\"z\": 1, \"a\":2}\r\n".to_vec(),
        }],
        fields: Vec::new(),
        comparison: None,
    }
}

fn field(selector: &str, value: ContentValue) -> FieldContent {
    FieldContent {
        record: reference(1),
        field: selector.into(),
        content_id: Some("exact-content-id".into()),
        declared_length: Some(5),
        value,
    }
}

fn render<P: Clone + 'static>(app: fn(P) -> Element, props: P) -> String {
    let mut dom = VirtualDom::new_with_props(app, props);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

fn row_fixture(item: ItemView) -> Element {
    rsx! { HistoryRow { id: "row", item, onaction: |_| {} } }
}

fn operation_fixture(state: OperationDetailsState) -> Element {
    rsx! { OperationPanel { id: "record", operation: id(1), state, original: false, capabilities: HostCapabilities::new(HostKind::Browser), open: RequestState::Idle, onaction: |_| {} } }
}

fn inspector_fixture(view: ViewModel) -> Element {
    rsx! { HistoryDetails { id: "inspector", view, capabilities: HostCapabilities::new(HostKind::Browser), onaction: |_| {} } }
}

#[test]
fn byte_previews_preserve_unicode_and_expansion_exposes_the_tail() {
    fn fixture(expanded: bool) -> Element {
        rsx! { BytesView { bytes: format!("{}末尾", "🙂".repeat(100)).into_bytes(), expanded, label: "Unicode" } }
    }
    let preview = render(fixture, false);
    assert!(
        preview.contains("Preview truncated: showing 320 of 406 bytes"),
        "the byte limit and full size are explicit"
    );
    assert!(
        !preview.contains('�') && !preview.contains("末尾"),
        "truncation never splits a scalar"
    );
    let complete = render(fixture, true);
    assert!(
        complete.contains("末尾") && !complete.contains("Preview truncated"),
        "expansion renders the entire loaded value"
    );
}

#[test]
fn text_is_escaped_and_binary_and_control_bytes_remain_exact() {
    fn fixture(bytes: Vec<u8>) -> Element {
        rsx! { ExactBytes { bytes, label: "Original" } }
    }
    let binary = render(fixture, vec![0, 255, 254, 13, 10]);
    assert!(
        binary.contains("00 ff fe 0d 0a") && !binary.contains('�'),
        "binary data is never lossily decoded"
    );
    let source = b"<script>run()</script>\r\n\t".to_vec();
    let text = render(fixture, source.clone());
    assert!(
        !text.contains("<script>") && text.contains("script") && text.contains("run()"),
        "recorded markup is text, never executable HTML: {text}"
    );
    assert!(
        text.contains(&super::content::hex(&source)),
        "hex retains CRLF, tabs and every source byte"
    );
}

#[test]
fn row_discloses_every_block_without_conflating_attempts_or_completion() {
    let mut value = item();
    let mut later = block(2, b"second message");
    later.position = Some(0);
    value.blocks.push(later);
    let collapsed = render(row_fixture, value.clone());
    assert!(
        collapsed.contains("1 more blocks") && collapsed.contains("second message"),
        "collapsed rows report hidden blocks and honor recorded positions"
    );
    value.expanded = true;
    let complete = render(row_fixture, value.clone());
    assert!(
        complete.find("second message") < complete.find("first block"),
        "recorded message positions determine display order"
    );
    for (attempt, channel, state) in [
        (
            40,
            "Stdout",
            BlockState::Content {
                bytes: b"partial tail".to_vec(),
                complete: false,
                finished: true,
                head: id(50),
            },
        ),
        (
            41,
            "Stderr",
            BlockState::Unavailable("missing payload".into()),
        ),
        (
            42,
            "Stdout",
            BlockState::Conflicted("divergent heads".into()),
        ),
    ] {
        let mut output = block(5, b"");
        output.attempt = Some(id(attempt));
        output.channel = Some(channel.into());
        output.state = state;
        value.blocks.push(output);
    }
    let html = render(row_fixture, value);
    for text in [
        "Partial content",
        "Terminal lifecycle recorded",
        "No terminal lifecycle recorded",
        "missing payload",
        "divergent heads",
        "Stderr",
        "Stdout",
        "partial tail",
    ] {
        assert!(
            html.contains(text),
            "each independent stream state is visible: {text}"
        );
    }
    for attempt in [40, 41, 42] {
        assert!(
            html.contains(&id(attempt)),
            "attempt identity survives reused block IDs"
        );
    }
}

#[test]
fn incomplete_source_previews_do_not_become_full_content_after_expansion() {
    let mut value = item();
    value.blocks.clear();
    value.expanded = true;
    value.observations.first_mut().unwrap().preview = Some(ContentText {
        text: "short source preview".into(),
        complete: false,
    });
    let html = render(row_fixture, value);
    for text in [
        "Source preview is incomplete",
        "human author",
        "import adapter",
        "Recorded stage",
        "Reasoning",
        "Started",
        "Legacy observation",
        "Item scan incomplete",
    ] {
        assert!(
            html.contains(text),
            "source state and distinct attribution are visible: {text}"
        );
    }
    for parent in [7, 8, 9] {
        assert!(html.contains(&id(parent)), "all parents stay available");
    }
}

#[test]
fn all_field_availability_states_and_exact_empty_values_are_distinct() {
    let mut value = details();
    value.fields = [
        ("empty", ContentValue::Available(Vec::new())),
        ("binary", ContentValue::Available(vec![255, 0, 13, 10])),
        ("absent", ContentValue::NotRecorded),
        ("missing", ContentValue::Missing),
        ("corrupt", ContentValue::Corrupt),
        ("unresolved", ContentValue::Unresolvable),
    ]
    .into_iter()
    .map(|(selector, value)| field(selector, value))
    .collect();
    let html = render(operation_fixture, OperationDetailsState::Ready(value));
    for text in [
        "Recorded empty content (0 bytes)",
        "ff 00 0d 0a",
        "Not recorded or not applicable",
        "Content missing",
        "Content corrupt",
        "Content unresolvable",
        "exact-content-id",
        "Declared byte length",
    ] {
        assert!(
            html.contains(text),
            "field availability remains explicit: {text}"
        );
    }
    assert!(
        !html.contains("transport"),
        "transport JSON cannot substitute for the stored encoding"
    );
}

#[test]
fn conflicts_keep_every_raw_variant_without_presenting_accepted_fields() {
    let mut value = details();
    value.status = RecordLookupStatus::Conflicted;
    value.observation = None;
    value.records.push(RawRecord {
        record: RecordRef {
            operation: id(1),
            hash: id(999),
        },
        bytes: b"quarantined second variant".to_vec(),
    });
    let html = render(operation_fixture, OperationDetailsState::Ready(value));
    assert!(
        html.contains("Conflicted observation") && html.contains("none is selected as fact"),
        "conflicts never choose a variant"
    );
    assert!(
        html.contains(&id(101))
            && html.contains(&id(999))
            && html.contains("quarantined second variant"),
        "every retained encoding and digest is reachable"
    );
    assert!(
        !html.contains("One accepted record") && !html.contains("Recorded fields"),
        "conflicts cannot appear resolved"
    );
    let missing = OperationDetails {
        operation: id(1),
        status: RecordLookupStatus::Missing,
        observation: None,
        records: Vec::new(),
        fields: Vec::new(),
        comparison: None,
    };
    assert!(
        render(operation_fixture, OperationDetailsState::Ready(missing)).contains("Record missing"),
        "missing records differ from conflict and empty bytes"
    );
}

#[test]
fn file_comparisons_keep_snapshots_and_missing_sides_explicit() {
    for (comparison, expected) in [
        (Comparison::Unavailable, "Comparison unavailable"),
        (Comparison::Identical, "snapshots are identical"),
        (
            Comparison::Changed {
                before_start: 1,
                before_end: 2,
                after_start: 1,
                after_end: 3,
            },
            "Offsets are bytes",
        ),
    ] {
        let mut value = details();
        value.comparison = Some(comparison);
        value.fields = vec![
            field(
                "\"FileBase\"",
                ContentValue::Available(b"old file".to_vec()),
            ),
            field("\"FileAfter\"", ContentValue::Missing),
        ];
        let html = render(operation_fixture, OperationDetailsState::Ready(value));
        for text in [
            expected,
            "Before snapshot",
            "After snapshot",
            "old file",
            "Content missing",
            "Open recorded revision",
            "Open recorded comparison",
        ] {
            assert!(
                html.contains(text),
                "file comparison preserves states and exact sides: {text}"
            );
        }
    }
    let mut value = item();
    value.expanded = true;
    value.observations.first_mut().unwrap().detail = Detail::File {
        path: "18446744073709551615".into(),
        revision: Some(id(80)),
        action: "Edit".into(),
        change: Some("Proposed".into()),
        before: Some("before-id".into()),
        after: None,
        caused_by: Some(id(90)),
    };
    let html = render(row_fixture, value);
    for text in [
        "18446744073709551615",
        "Proposed",
        "before-id",
        "Not recorded",
    ] {
        assert!(
            html.contains(text),
            "file identity is never coerced into a path or working copy"
        );
    }
}

#[test]
fn inspector_retains_selected_items_outside_the_page_and_drills_into_originals() {
    let value = item();
    let view = ViewModel {
        selected: Selected {
            item: Some(value.key.clone()),
            observation: Some(id(1)),
        },
        selected_item: Some(value),
        operation_details: vec![OperationDetailsView {
            operation: id(1),
            state: OperationDetailsState::Ready(details()),
        }],
        ..ViewModel::default()
    };
    let html = render(inspector_fixture, view.clone());
    assert!(
        html.contains("human author")
            && html.contains("Original captured content")
            && html.contains(&id(20)),
        "selection and Original links are independent of mounted graph rows"
    );
    assert!(
        html.contains("Exact bytes have not been requested"),
        "a preview does not imply the Original was loaded"
    );
    let mut original = details();
    original.operation = id(20);
    original.fields = vec![field(
        r#"{"Record":"Content"}"#,
        ContentValue::Available(b"original raw line\r\n".to_vec()),
    )];
    let mut loaded = view;
    loaded.operation_details.push(OperationDetailsView {
        operation: id(20),
        state: OperationDetailsState::Ready(original),
    });
    let html = render(inspector_fixture, loaded);
    assert!(
        html.contains("original raw line") && html.contains("0d 0a"),
        "Original content keeps exact captured bytes"
    );
}
