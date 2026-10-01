//! Retained lanes follow recorded session or recorder identity.

use super::super::state::State;
use super::{item, observation, view};

#[test]
fn linear_appends_without_sessions_keep_the_same_lane_as_a_reload() {
    let mut data = view(Vec::new());
    let mut incremental = State::default();
    for number in 0..5_u64 {
        let key = format!("item-{number}");
        let parent = format!("item-{}", number.saturating_sub(1));
        let mut next = item(
            &key,
            number,
            &if number == 0 {
                Vec::new()
            } else {
                vec![parent.as_str()]
            },
        );
        next.observations.first_mut().unwrap().session = None;
        data.items.push(next);
        incremental.reconcile(&data, 1000.0);
        let mut reloaded = State::default();
        reloaded.reconcile(&data, 1000.0);
        assert_eq!(
            incremental.layout.lanes, reloaded.layout.lanes,
            "live append {number} must keep the same lanes as loading the same records together"
        );
        assert!(
            incremental.layout.lanes.values().all(|lane| *lane == 0),
            "one unbranched recorder chain does not need lane changes"
        );
    }
}

#[test]
fn sessions_take_priority_and_session_and_recorder_ids_stay_distinct() {
    for (parent_session, child_session, parent_recorder, child_recorder, continues) in [
        (None, None, Some("r"), Some("r"), true),
        (Some("s"), Some("s"), Some("r1"), Some("r2"), true),
        (Some("s1"), Some("s2"), Some("r"), Some("r"), false),
        (Some("shared"), None, Some("shared"), Some("shared"), false),
        (None, None, Some("r1"), Some("r2"), false),
        (None, None, None, None, false),
    ] {
        let mut parent = item("parent", 1, &[]);
        let record = parent.observations.first_mut().unwrap();
        record.session = parent_session.map(str::to_owned);
        record.recorder = parent_recorder.map(str::to_owned);
        let mut child = item("child", 2, &["parent"]);
        let record = child.observations.first_mut().unwrap();
        record.session = child_session.map(str::to_owned);
        record.recorder = child_recorder.map(str::to_owned);
        let mut data = view(vec![parent]);
        let mut state = State::default();
        state.reconcile(&data, 0.0);
        data.items.push(child);
        state.reconcile(&data, 10.0);
        assert_eq!(
            state.layout.lanes.get("parent") == state.layout.lanes.get("child"),
            continues,
            "continuity must use the recorded source, not the shared author or an untyped ID"
        );
    }
}

#[test]
fn mixed_sources_do_not_choose_an_arbitrary_continuation() {
    for with_sessions in [true, false] {
        let mut parent = item("parent", 1, &[]);
        let mut child = item("child", 2, &["parent"]);
        let mut update = observation("child-update", "child", 3, &["child"]);
        if with_sessions {
            update.session = Some("another-session".into());
        } else {
            parent.observations.first_mut().unwrap().session = None;
            child.observations.first_mut().unwrap().session = None;
            update.session = None;
            update.recorder = Some("another-recorder".into());
        }
        child.observations.push(update);
        let mut data = view(vec![parent]);
        let mut state = State::default();
        state.reconcile(&data, 0.0);
        data.items.push(child);
        state.reconcile(&data, 10.0);
        assert_ne!(
            state.layout.lanes.get("parent"),
            state.layout.lanes.get("child"),
            "a grouped item with conflicting sources cannot pick one for lane continuation"
        );
    }
}
