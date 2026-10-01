//! Recorded output views for the session preview; no production data adapter.

use app_core::history::{
    ActivityKind, BlockState, BlockView, Detail, Filter, ItemView, ObservationView, Paging,
    RecordRef, RequestState, ViewModel,
};
use app_core::sessions::SessionItemBinding;

pub(super) fn view(binding: &SessionItemBinding) -> ViewModel {
    let message = item(
        binding,
        1,
        ActivityKind::Message,
        vec![block(
            10,
            None,
            None,
            BlockState::Content {
                bytes: format!(
                    "{}\nMESSAGE_END <script>window.sessionMarkupExecuted = true</script>",
                    "Recorded assistant response. 🙂 ".repeat(20)
                )
                .into_bytes(),
                complete: true,
                finished: true,
                head: id(1),
            },
        )],
    );
    let tool = item(
        binding,
        2,
        ActivityKind::Tool,
        vec![
            block(
                20,
                Some(100),
                Some("Stdout"),
                BlockState::Content {
                    bytes: b"Running checks\nAll checks passed\n".to_vec(),
                    complete: true,
                    finished: true,
                    head: id(2),
                },
            ),
            block(
                20,
                Some(101),
                Some("Stderr"),
                BlockState::Content {
                    bytes: b"Output prefix has not arrived\n".to_vec(),
                    complete: false,
                    finished: false,
                    head: id(3),
                },
            ),
            block(
                21,
                Some(101),
                Some("Stdout"),
                BlockState::Unavailable("Waiting for recorded bytes".into()),
            ),
        ],
    );
    ViewModel {
        chain: Some(binding.chain.clone()),
        filter: Filter {
            session: Some(binding.item.clone()),
            ..Filter::default()
        },
        items: vec![message, tool],
        paging: Paging {
            state: RequestState::Ready,
            exhausted: true,
            ..Paging::default()
        },
        reconciliation: RequestState::Ready,
        ..ViewModel::default()
    }
}

fn id(value: u64) -> String {
    format!("{value:064x}")
}

fn block(value: u64, attempt: Option<u64>, channel: Option<&str>, state: BlockState) -> BlockView {
    BlockView {
        block: id(value),
        position: Some(0),
        attempt: attempt.map(id),
        channel: channel.map(str::to_owned),
        media_type: Some("text/plain".into()),
        state,
    }
}

fn item(
    binding: &SessionItemBinding,
    value: u64,
    kind: ActivityKind,
    blocks: Vec<BlockView>,
) -> ItemView {
    let key = id(value);
    ItemView {
        key: key.clone(),
        expanded: false,
        blocks,
        paging: Paging {
            state: RequestState::Ready,
            exhausted: true,
            ..Paging::default()
        },
        observations: vec![ObservationView {
            record: RecordRef {
                operation: key.clone(),
                hash: id(value.saturating_add(1000)),
            },
            item: key,
            kind,
            author: Some("agent:runner".into()),
            recorder: Some("runtime-recorder".into()),
            session: Some(binding.item.clone()),
            turn: Some(id(90)),
            time_ms: Some(1000),
            sequence: Some(value),
            parents: Vec::new(),
            causes: Vec::new(),
            original: None,
            converter: None,
            legacy: None,
            preview: None,
            problem: None,
            detail: if kind == ActivityKind::Tool {
                Detail::Tool {
                    attempt: id(100),
                    channel: "Stdout".into(),
                    stage: "Finished".into(),
                    parent_call: None,
                }
            } else {
                Detail::Message {
                    category: "Text".into(),
                    stage: "Finished".into(),
                }
            },
        }],
    }
}
