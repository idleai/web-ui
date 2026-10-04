use app_core::history::{ContentValue, FieldContent, OperationDetailsState, RecordLookupStatus};
use app_core::module::EffectError;

use super::{lookup, prepare, reference, snapshot, view};

fn pending_states() -> [OperationDetailsState; 3] {
    [
        OperationDetailsState::Loading,
        OperationDetailsState::Failed(EffectError {
            message: "The source is offline".into(),
        }),
        OperationDetailsState::Idle,
    ]
}

#[test]
fn rejected_sources_stay_unavailable_until_a_successful_retry() {
    let value = snapshot();
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
        assert!(
            prepare(&history, Some(&value))
                .unwrap()
                .indicators
                .is_empty(),
            "the rejected source starts without marks"
        );
        for state in pending_states() {
            history.operation_details.first_mut().unwrap().state = state;
            let data = prepare(&history, Some(&value)).unwrap();
            assert!(
                data.indicators.is_empty(),
                "a pending or failed retry cannot restore rejected source marks"
            );
            assert!(
                !data.unavailable_sources.is_empty(),
                "unavailable sources remain inspectable during retries"
            );
        }
        history.operation_details = vec![lookup(reference(2), RecordLookupStatus::Found)];
        let data = prepare(&history, Some(&value)).unwrap();
        assert_eq!(
            data.indicators, value.indicators,
            "a successful lookup of the exact source restores the marks"
        );
        assert!(
            data.unavailable_sources.is_empty(),
            "the accepted source is no longer unavailable"
        );
    }
}

#[test]
fn rejected_snapshots_stay_unavailable_until_matching_bytes_arrive() {
    let value = snapshot();
    for content in [
        ContentValue::Missing,
        ContentValue::Corrupt,
        ContentValue::Unresolvable,
        ContentValue::Available(b"different bytes".to_vec()),
    ] {
        let mut history = view();
        let mut loaded = lookup(reference(1), RecordLookupStatus::Found);
        if let OperationDetailsState::Ready(details) = &mut loaded.state {
            details.fields.push(FieldContent {
                record: reference(1),
                field: "\"FileAfter\"".into(),
                content_id: value.content_id.clone(),
                declared_length: None,
                value: content,
            });
        }
        history.operation_details.push(loaded.clone());
        assert!(
            prepare(&history, Some(&value)).unwrap().text.is_none(),
            "the unavailable or mismatching snapshot starts without range text"
        );
        for state in pending_states() {
            history.operation_details.first_mut().unwrap().state = state;
            let data = prepare(&history, Some(&value)).unwrap();
            assert!(
                data.text.is_none() && data.indicators.is_empty(),
                "a pending or failed retry cannot restore a rejected snapshot"
            );
        }
        if let OperationDetailsState::Ready(details) = &mut loaded.state {
            details.fields.first_mut().unwrap().value =
                ContentValue::Available(value.text.as_ref().unwrap().as_bytes().to_vec());
        }
        history.operation_details = vec![loaded];
        let data = prepare(&history, Some(&value)).unwrap();
        assert_eq!(
            data.text, value.text,
            "matching loaded bytes restore the text"
        );
        assert_eq!(
            data.indicators, value.indicators,
            "matching loaded bytes restore the range marks"
        );
    }
}
