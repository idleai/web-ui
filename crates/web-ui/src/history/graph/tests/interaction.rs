//! Paging and pointer selection over measured, variable-height rows.

use app_core::history::{Event, Selected};
use dioxus::prelude::Key;

use super::super::state::State;
use super::{item, view};

#[test]
fn resize_retains_visible_focus_without_revealing_an_offscreen_selection() {
    let mut state = State::default();
    state.reconcile(
        &view(
            (1..=40)
                .rev()
                .map(|index| item(&index.to_string(), index, &[]))
                .collect(),
        ),
        0.0,
    );
    state.viewport.focus(39);
    assert!(
        state.viewport.resize(800.0, 200.0),
        "inspector changes the viewport"
    );
    let row = state.viewport.rows.last().unwrap();
    assert!(
        row.top >= state.viewport.top
            && row.top + row.height <= state.viewport.top + state.viewport.height,
        "a visible focused row stays visible when the inspector opens"
    );
    state.viewport.scroll(400.0);
    assert!(
        state.viewport.resize(800.0, 180.0),
        "subsequent host resize is applied"
    );
    assert!(
        (state.viewport.top - 400.0).abs() < 0.01,
        "manual scrolling away from selection is preserved"
    );
}

#[test]
fn page_keys_cross_tall_rows_and_stop_at_the_ends() {
    for height in [480.0, 2000.0] {
        let mut state = State::default();
        state.reconcile(
            &view(vec![
                item("tall", 3, &[]),
                item("after", 2, &[]),
                item("last", 1, &[]),
            ]),
            0.0,
        );
        assert!(
            state.viewport.measure("tall", height),
            "content is measured"
        );
        state.viewport.focus(1);
        for (key, expected) in [
            (Key::PageUp, "tall"),
            (Key::PageUp, "tall"),
            (Key::PageDown, "after"),
            (Key::PageDown, "last"),
            (Key::PageDown, "last"),
        ] {
            assert!(state.key(&key).0, "paging keys are handled by the tree");
            assert_eq!(
                state.viewport.focused.as_deref(),
                Some(expected),
                "{key:?} must advance through a {height}px row until the boundary"
            );
        }
    }
}

#[test]
fn pointer_selection_preserves_scroll_through_delayed_and_repeated_updates() {
    let mut data = view(vec![item("tall", 2, &[]), item("after", 1, &[])]);
    let mut state = State::default();
    state.reconcile(&data, 0.0);
    assert!(
        state.viewport.measure("tall", 2000.0),
        "content is measured"
    );
    let selected = Selected {
        item: Some("tall".into()),
        observation: None,
    };
    for offset in [800.0, 1200.0] {
        state.viewport.scroll(offset);
        assert!(
            matches!(state.click("tall".into()), Event::Select(ref actual) if actual == &selected),
            "pointer selection sends its item to app-core"
        );
        state.reconcile(&data, 10.0);
        assert!(
            (state.viewport.top - offset).abs() < 0.01,
            "waiting for the selection update must not reveal the row start"
        );
        data.selected.clone_from(&selected);
        state.reconcile(&data, 20.0);
        assert!(
            (state.viewport.top - offset).abs() < 0.01,
            "the selection update preserves the pointer's reading position"
        );
    }
    assert!(
        state.key(&Key::ArrowDown).0,
        "keyboard navigation is handled"
    );
    assert_eq!(
        state.viewport.focused.as_deref(),
        Some("after"),
        "navigation still moves focus after a pointer selection"
    );
    assert!(
        state.viewport.top > 1200.0,
        "keyboard focus reveals the next row"
    );
    data.selected.observation = Some("tall".into());
    state.reconcile(&data, 30.0);
    assert!(
        state.viewport.top.abs() < 0.01,
        "an external observation selection still reveals the target"
    );
}

#[test]
fn clicking_cancels_an_older_pending_reveal_without_hiding_future_selections() {
    let mut data = view(vec![item("tall", 2, &[])]);
    data.selected.item = Some("later".into());
    let mut state = State::default();
    state.reconcile(&data, 0.0);
    assert!(
        state.viewport.measure("tall", 2000.0),
        "content is measured"
    );
    state.viewport.scroll(800.0);
    let _action = state.click("tall".into());
    data.items.push(item("later", 1, &[]));
    state.reconcile(&data, 10.0);
    assert_eq!(
        state.viewport.focused.as_deref(),
        Some("tall"),
        "arrival of an older selection cannot undo the click"
    );
    assert!(
        (state.viewport.top - 800.0).abs() < 0.01,
        "reading position stays put"
    );
    data.selected.item = Some("tall".into());
    state.reconcile(&data, 20.0);
    data.selected.item = Some("later".into());
    state.reconcile(&data, 30.0);
    assert_eq!(
        state.viewport.focused.as_deref(),
        Some("later"),
        "a subsequent external selection still changes focus"
    );
    assert!(
        state.viewport.top > 800.0,
        "external selection reveals an offscreen row"
    );
}
