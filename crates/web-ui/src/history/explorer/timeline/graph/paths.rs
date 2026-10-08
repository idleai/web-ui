//! Smooth turns consume their rail halves and meet row boundaries vertically.

use app_core::history::timeline::{Geometry, Transition};

use super::x;

pub(super) fn color_lane(graph: &Geometry, from: u32, to: u32) -> u32 {
    if graph.lane == from || (graph.above.contains(&from) && graph.below.contains(&from)) {
        to
    } else {
        from
    }
}

pub(super) fn rail(graph: &Geometry, lane: u32, height: u8) -> (u8, u8) {
    let above = graph.above.contains(&lane);
    let below = graph.below.contains(&lane);
    let node = graph.lane == lane;
    let from = graph
        .transitions
        .iter()
        .any(|Transition(from, _)| *from == lane);
    let to = graph
        .transitions
        .iter()
        .any(|Transition(_, to)| *to == lane);
    (
        if above && (below || node || !from) {
            0
        } else {
            height / 2
        },
        if below && (above || node || !to) {
            height
        } else {
            height / 2
        },
    )
}

pub(super) fn bend(
    graph: &Geometry,
    Transition(from, to): &Transition,
    pitch: f64,
    pan: f64,
    height: u8,
) -> String {
    let (from, to) = (*from, *to);
    let height = f64::from(height);
    let start_x = x(from, pitch) - pan;
    let end_x = x(to, pitch) - pan;
    let start_y: f64 = if graph.lane == from {
        height / 2.0
    } else {
        0.0
    };
    let end_y: f64 = if graph.lane == to {
        height / 2.0
    } else {
        height
    };
    if from.min(to) < graph.lane && graph.lane < from.max(to) {
        // A passing turn must clear this row's unrelated node, even when the
        // node lies halfway between the two lane columns.
        return format!("M {start_x} 0 C {start_x} 4 {end_x} 4 {end_x} 8 V {height}");
    }
    let middle = start_y.midpoint(end_y);
    format!("M {start_x} {start_y} C {start_x} {middle} {end_x} {middle} {end_x} {end_y}")
}
