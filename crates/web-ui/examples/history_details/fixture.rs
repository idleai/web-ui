//! Scripted responses that exercise recorded-byte and availability presentation.

use app_core::history::{
    ActivityKind, BlockState, BlockView, Comparison, ContentText, ContentValue, Detail,
    FieldContent, ItemView, Observation, ObservationView, OperationDetails, Paging, RawRecord,
    RecordLookupStatus, RecordRef, RequestState, ViewModel,
};

pub(super) fn id(value: u64) -> String {
    format!("{value:064x}")
}

fn reference(value: u64) -> RecordRef {
    RecordRef {
        operation: id(value),
        hash: id(value.saturating_add(1000)),
    }
}

pub(super) fn complete_scan() -> Paging {
    Paging {
        state: RequestState::Ready,
        exhausted: true,
        ..Paging::default()
    }
}

fn observation(value: u64, kind: ActivityKind) -> ObservationView {
    ObservationView {
        record: reference(value),
        item: id(value.saturating_mul(10)),
        kind,
        author: Some("human:ada".into()),
        recorder: Some("fixture-importer".into()),
        session: Some(id(100)),
        turn: Some(id(101)),
        time_ms: Some(1000_u64.saturating_sub(value)),
        sequence: Some(value),
        parents: Vec::new(),
        causes: Vec::new(),
        original: (value == 1).then(|| id(7)),
        converter: (value == 1).then(|| "fixture-converter".into()),
        legacy: None,
        detail: Detail::Other,
        preview: None,
        problem: None,
    }
}

fn content(value: u64, bytes: &[u8]) -> BlockView {
    BlockView {
        block: id(value),
        position: Some(0),
        attempt: None,
        channel: None,
        media_type: Some("text/plain".into()),
        state: BlockState::Content {
            bytes: bytes.to_vec(),
            complete: true,
            finished: true,
            head: id(1),
        },
    }
}

fn item(observation: ObservationView, blocks: Vec<BlockView>) -> ItemView {
    ItemView {
        key: observation.item.clone(),
        observations: vec![observation],
        blocks,
        expanded: false,
        paging: Paging::default(),
    }
}

pub(super) fn view() -> ViewModel {
    let mut message = observation(1, ActivityKind::Message);
    message.detail = Detail::Message {
        category: "Text".into(),
        stage: "Finished".into(),
    };
    let long = format!(
        "{}\nMESSAGE_TAIL <script>window.recordExecuted = true</script>",
        "Read the full recorded message. 🙂 ".repeat(30)
    );
    let mut second = content(12, b"The second message block remains available.");
    second.position = Some(1);
    let message = item(message, vec![content(11, long.as_bytes()), second]);
    let mut tool = observation(2, ActivityKind::Tool);
    tool.detail = Detail::Tool {
        attempt: id(200),
        channel: "Stdout".into(),
        stage: "Updated".into(),
        parent_call: Some(id(199)),
    };
    let mut first = content(21, b"output without its initial predecessor");
    first.attempt = Some(id(200));
    first.channel = Some("Stdout".into());
    first.state = BlockState::Content {
        bytes: b"output without its initial predecessor".to_vec(),
        complete: false,
        finished: true,
        head: id(2),
    };
    let mut second = content(21, b"retry stderr");
    second.attempt = Some(id(201));
    second.channel = Some("Stderr".into());
    let mut missing = content(22, b"");
    missing.attempt = Some(id(201));
    missing.channel = Some("Stdout".into());
    missing.state = BlockState::Unavailable("output blob has not arrived".into());
    let tool = item(tool, vec![first, second, missing]);
    let mut file = observation(3, ActivityKind::File);
    file.preview = Some(ContentText::new("src/main.rs".into(), true));
    file.detail = Detail::File {
        path: "18446744073709551615".into(),
        revision: Some(id(300)),
        action: "Edit".into(),
        change: Some("Proposed".into()),
        before: Some("before-content".into()),
        after: Some("after-content".into()),
        caused_by: Some(id(2)),
    };
    let file = item(file, Vec::new());
    let mut conflict = observation(4, ActivityKind::Message);
    conflict.problem = Some("Quarantined observation".into());
    let mut block = content(41, b"");
    block.state = BlockState::Conflicted("two retained encodings".into());
    let conflict = item(conflict, vec![block]);
    let mut original = observation(7, ActivityKind::Original);
    original.preview = Some(ContentText::new("Captured source preview".into(), false));
    ViewModel {
        chain: Some("local-details-fixture".into()),
        items: vec![message, tool, file, conflict, item(original, Vec::new())],
        paging: complete_scan(),
        ..ViewModel::default()
    }
}

fn field(operation: u64, selector: &str, value: ContentValue) -> FieldContent {
    FieldContent {
        record: reference(operation),
        field: selector.into(),
        content_id: None,
        declared_length: None,
        value,
    }
}

pub(super) fn details(operation: &str, late: bool) -> OperationDetails {
    let value = (1..=7).find(|value| id(*value) == operation).unwrap_or(0);
    let reference = reference(value);
    let mut details = OperationDetails {
        operation: operation.into(),
        status: RecordLookupStatus::Found,
        observation: Some(Observation {
            record: reference.clone(),
            operation_json: b"fixture transport representation".to_vec(),
        }),
        records: vec![RawRecord {
            record: reference,
            bytes: format!(" {{\"fixture\": {value}, \"retained\":true}}\r\n").into_bytes(),
        }],
        fields: Vec::new(),
        comparison: None,
    };
    if value == 7 {
        details.fields = vec![field(
            value,
            r#"{"Record":"Content"}"#,
            ContentValue::Available(b"{\"z\": 1, \"a\":2}\r\n\0\xff".to_vec()),
        )];
    } else if value == 3 {
        details.comparison = Some(if late {
            Comparison::Changed {
                before_start: 0,
                before_end: 3,
                after_start: 0,
                after_end: 3,
            }
        } else {
            Comparison::Unavailable
        });
        details.fields = vec![
            field(
                value,
                "\"FileBase\"",
                if late {
                    ContentValue::Available(b"old\n".to_vec())
                } else {
                    ContentValue::Missing
                },
            ),
            field(
                value,
                "\"FileAfter\"",
                ContentValue::Available(b"new\n".to_vec()),
            ),
            field(
                value,
                r#"{"Record":"Path"}"#,
                ContentValue::Available(b"src/main.rs".to_vec()),
            ),
        ];
    } else if value == 4 {
        details.status = RecordLookupStatus::Conflicted;
        details.observation = None;
        details.records.push(RawRecord {
            record: RecordRef {
                operation: operation.into(),
                hash: id(9004),
            },
            bytes: b"SECOND_CONFLICT_VARIANT".to_vec(),
        });
    } else if value == 2 {
        details.fields = vec![
            field(
                value,
                r#"{"Record":"Arguments"}"#,
                ContentValue::Available(b"{\"cmd\":\"cargo check\"}".to_vec()),
            ),
            field(value, r#"{"Record":"Output"}"#, ContentValue::Missing),
            field(value, r#"{"Record":"Outcome"}"#, ContentValue::NotRecorded),
        ];
    } else {
        details.fields = vec![field(
            value,
            r#"{"Record":{"MessageBlock":0}}"#,
            ContentValue::Available(b"Captured message block update\r\n".to_vec()),
        )];
    }
    details
}
