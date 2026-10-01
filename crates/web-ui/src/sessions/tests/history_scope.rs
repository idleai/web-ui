//! Session binding survives full-item scans without accepting another session.

use app_core::history::{BlockState, Selected};

use super::{ConversationFixture, WorkspaceMode, connected, conversation, output, render};
use crate::host::HostKind;

#[test]
fn expanded_output_and_inspector_keep_records_without_repeated_session_fields() {
    for mode in [WorkspaceMode::Standalone, WorkspaceMode::Managed] {
        for kind in [HostKind::Browser, HostKind::VsCode] {
            let view = connected(mode).view().sessions;
            let mut history = output::view(view.selected_history.as_ref().expect("binding"));
            history.items.truncate(1);
            let before = render(
                conversation,
                ConversationFixture {
                    view: view.clone(),
                    kind,
                    history: Some(history.clone()),
                },
            );
            assert!(
                before.contains("Recorded assistant response"),
                "initial row"
            );

            // A full-item response can add valid observations that omit context.
            let item = history.items.first_mut().expect("message");
            let mut appended = item.observations.first().expect("bound record").clone();
            appended.record.operation = format!("{:064x}", 3);
            appended.record.hash = format!("{:064x}", 1003);
            appended.session = None;
            if let BlockState::Content { bytes, head, .. } =
                &mut item.blocks.first_mut().expect("message block").state
            {
                bytes.extend_from_slice(b"\nHYDRATED_OUTPUT");
                head.clone_from(&appended.record.operation);
            }
            item.observations.push(appended);
            item.expanded = true;
            history.selected = Selected {
                item: Some(item.key.clone()),
                observation: None,
            };
            history.selected_item = Some(item.clone());
            let after = render(
                conversation,
                ConversationFixture {
                    view,
                    kind,
                    history: Some(history),
                },
            );
            for text in ["MESSAGE_END", "HYDRATED_OUTPUT", "History record details"] {
                assert!(
                    after.contains(text),
                    "loaded content remains visible: {text}"
                );
            }
        }
    }
}

#[test]
fn missing_or_conflicting_item_bindings_hide_output_and_retained_inspector() {
    let view = connected(WorkspaceMode::Standalone).view().sessions;
    for case in 0..3 {
        let mut history = output::view(view.selected_history.as_ref().expect("binding"));
        history.items.truncate(1);
        let item = history.items.first_mut().expect("message");
        let mut additional = item.observations.first().expect("bound record").clone();
        additional.record.operation = format!("{:064x}", 3);
        match case {
            0 => {
                for record in &mut item.observations {
                    record.session = None;
                }
                additional.session = None;
            }
            1 => additional.session = Some("another-session".into()),
            _ => {
                additional.session = None;
                additional.item = "another-item".into();
            }
        }
        item.observations.push(additional);
        item.expanded = true;
        history.selected.item = Some(item.key.clone());
        history.selected_item = Some(item.clone());
        let html = render(
            conversation,
            ConversationFixture {
                view: view.clone(),
                kind: HostKind::Browser,
                history: Some(history),
            },
        );
        assert!(
            !html.contains("MESSAGE_END") && !html.contains("History record details"),
            "unbound or conflicting case {case} must not expose content"
        );
    }
}
