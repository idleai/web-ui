use std::{any::Any, rc::Rc};

use app_core::history::{ContentValue, FieldContent, OperationDetailsState, RecordLookupStatus};
use dioxus::prelude::*;
use dioxus_core::{AttributeValue, Mutation, Mutations};
use dioxus_html::SerializedHtmlEventConverter;

use super::{
    ActivityIndicator, ActivityKind, ActivitySource, AuthorActivity, ByteRange, HostCapabilities,
    HostKind, lookup, reference, snapshot, view,
};

pub(super) fn toggle(dom: &mut VirtualDom, mutations: &Mutations, id: &str) {
    let target = mutations
        .edits
        .iter()
        .find_map(|edit| {
            if let Mutation::SetAttribute {
                name: "id",
                value: AttributeValue::Text(value),
                id: target,
                ..
            } = edit
            {
                (value == id).then_some(*target)
            } else {
                None
            }
        })
        .expect("source summary exists");
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let data: Rc<dyn Any> = Rc::new(PlatformEventData::new(Box::new(
        SerializedMouseData::default(),
    )));
    dom.runtime()
        .handle_event("click", Event::new(data, true), target);
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
}

fn many_sources(count: u64) -> Element {
    let mut history = view();
    let mut value = snapshot();
    value.indicators = (0..count)
        .map(|index| ActivityIndicator {
            kind: ActivityKind::Human,
            range: Some(ByteRange { start: 0, end: 1 }),
            label: format!("Recorded author {index}"),
            sources: vec![ActivitySource {
                record: reference(index.saturating_add(1000)),
                item: None,
                original: Some(reference(3).operation),
            }],
        })
        .collect();
    let mut loaded = lookup(reference(3), RecordLookupStatus::Found);
    if let OperationDetailsState::Ready(details) = &mut loaded.state {
        details.fields.push(FieldContent {
            record: reference(3),
            field: "\"ImportRaw\"".into(),
            content_id: None,
            declared_length: Some(65536),
            value: ContentValue::Available(vec![b'x'; 65536]),
        });
    }
    history.operation_details.push(loaded);
    rsx! { AuthorActivity { id: "activity", view: history, activity: Some(value), capabilities: HostCapabilities::new(HostKind::Browser), onaction: |_| {} } }
}

#[test]
fn collapsed_sources_do_not_render_loaded_original_payloads() {
    let mut dom = VirtualDom::new_with_props(many_sources, 64);
    let mutations = dom.rebuild_to_vec();
    let html = dioxus_ssr::render(&dom);
    assert_eq!(
        html.matches("<details").count(),
        64,
        "every supporting record has a disclosure"
    );
    assert_eq!(
        html.matches("data-field=").count(),
        0,
        "a shared Original is not rendered inside any collapsed source"
    );
    assert!(
        !html.contains("78 78 78") && !html.contains(&"x".repeat(65536)),
        "neither full text nor hexadecimal content is mounted while closed"
    );
    for expected in [1, 0, 1] {
        toggle(&mut dom, &mutations, "activity-sources-0-summary");
        let html = dioxus_ssr::render(&dom);
        assert_eq!(
            html.matches("data-field=").count(),
            expected,
            "opening, closing and reopening mount only the selected source payload"
        );
        assert_eq!(
            html.matches(&"x".repeat(65536)).count(),
            expected,
            "the complete Original remains available when its source is open"
        );
    }
}
