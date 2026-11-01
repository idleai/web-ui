//! Paint validated native turns as two tangent-continuous colored halves.

use app_core::history::timeline::{Geometry, Transition};

use super::x;

pub(super) struct Segment {
    pub path: String,
    pub lane: u32,
    pub muted: bool,
    pub curve: bool,
}

pub(super) enum Shape {
    Activity,
    Bundle,
    Detail,
}

pub(super) struct Options {
    pub pitch: f64,
    pub pan: f64,
    pub height: u8,
    pub shape: Shape,
}

pub(super) fn segments(graph: &Geometry, options: &Options) -> Vec<Segment> {
    let height = f64::from(options.height);
    let pitch = options.pitch;
    let pan = options.pan;
    let bundle = matches!(options.shape, Shape::Bundle);
    let node = !matches!(options.shape, Shape::Detail);
    let middle = height / 2.0;
    let turns: Vec<_> = graph
        .transitions
        .iter()
        .filter(|Transition(from, to)| {
            (node && graph.lane == *from || graph.above.contains(from))
                && (node && graph.lane == *to || graph.below.contains(to))
        })
        .collect();
    let mut segments = Vec::new();
    for (lanes, muted, upper) in [
        (&graph.above, &graph.muted_above, true),
        (&graph.below, &graph.muted_below, false),
    ] {
        for lane in lanes {
            if turns.iter().any(|Transition(from, to)| {
                if upper {
                    *from == *lane && (!node || graph.lane != *from)
                } else {
                    *to == *lane && (!node || graph.lane != *to)
                }
            }) {
                continue;
            }
            let center = if bundle && *lane == graph.lane {
                middle + if upper { -7.0 } else { 7.0 }
            } else {
                middle
            };
            let (start, end) = if upper {
                (0.0, center)
            } else {
                (center, height)
            };
            segments.push(Segment {
                path: format!("M {:.2} {start:.2} V {end:.2}", x(*lane, pitch) - pan),
                lane: *lane,
                muted: muted.contains(lane),
                curve: false,
            });
        }
    }
    for turn in turns {
        let paths = bend(graph, turn, options);
        for (path, lane) in paths.into_iter().zip([turn.0, turn.1]) {
            segments.push(Segment {
                path,
                lane,
                muted: graph.muted_transitions.contains(turn),
                curve: true,
            });
        }
    }
    segments
}

fn midpoint(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0.midpoint(b.0), a.1.midpoint(b.1))
}

fn quadratic(start: (f64, f64), control: (f64, f64), end: (f64, f64)) -> String {
    format!(
        "M {:.2} {:.2} Q {:.2} {:.2} {:.2} {:.2}",
        start.0, start.1, control.0, control.1, end.0, end.1
    )
}

fn bend(graph: &Geometry, Transition(from, to): &Transition, options: &Options) -> [String; 2] {
    let pitch = options.pitch;
    let pan = options.pan;
    let height = f64::from(options.height);
    let bundle = matches!(options.shape, Shape::Bundle);
    let node = !matches!(options.shape, Shape::Detail);
    let start_at_node = node && graph.lane == *from;
    let end_at_node = node && graph.lane == *to;
    let middle = height / 2.0;
    let start = (
        x(*from, pitch) - pan,
        if start_at_node {
            middle + if bundle { 7.0 } else { 0.0 }
        } else {
            0.0
        },
    );
    let end = (
        x(*to, pitch) - pan,
        if end_at_node {
            middle - if bundle { 7.0 } else { 0.0 }
        } else {
            height
        },
    );
    let (source, destination, seam) = if start_at_node == end_at_node {
        let mut seam = midpoint(start, end);
        if node && from.min(to) < &graph.lane && &graph.lane < from.max(to) {
            // Preserve clearance around an unrelated dot on a reused column.
            seam.1 = 4.0;
        }
        ((start.0, seam.1), (end.0, seam.1), seam)
    } else {
        let control = if start_at_node {
            (end.0, start.1)
        } else {
            (start.0, end.1)
        };
        let source = midpoint(start, control);
        let destination = midpoint(control, end);
        (source, destination, midpoint(source, destination))
    };
    [
        quadratic(start, source, seam),
        quadratic(seam, destination, end),
    ]
}

#[cfg(test)]
mod tests {
    use super::{Geometry, Options, Shape, Transition, segments};

    #[test]
    fn dot_to_boundary_keeps_neighboring_rails_and_splits_lane_colors() {
        let graph = Geometry {
            lane: 1,
            above: vec![1],
            below: vec![2],
            transitions: vec![Transition(1, 2)],
            ..Geometry::default()
        };
        let segments = segments(
            &graph,
            &Options {
                pitch: 14.76,
                pan: 0.0,
                height: 34,
                shape: Shape::Activity,
            },
        );
        assert_eq!(segments.len(), 3, "one upper rail and two curve halves");
        assert_eq!(
            segments.first().unwrap().path,
            "M 29.52 0.00 V 17.00",
            "incoming rail remains"
        );
        assert_eq!(
            segments.get(1).unwrap().path,
            "M 29.52 17.00 Q 36.90 17.00 40.59 21.25",
            "node tangent is horizontal"
        );
        assert_eq!(
            segments.get(2).unwrap().path,
            "M 40.59 21.25 Q 44.28 25.50 44.28 34.00",
            "boundary tangent is vertical"
        );
        assert_eq!(
            (segments.get(1).unwrap().lane, segments.get(2).unwrap().lane),
            (1, 2),
            "each half uses its lane color"
        );
    }

    #[test]
    fn dangling_turn_does_not_erase_rails_and_capsule_uses_terminals() {
        let graph = Geometry {
            lane: 1,
            above: vec![1, 2],
            below: vec![1, 2],
            transitions: vec![Transition(2, 3)],
            ..Geometry::default()
        };
        let segments = segments(
            &graph,
            &Options {
                pitch: 14.76,
                pan: 0.0,
                height: 34,
                shape: Shape::Bundle,
            },
        );
        assert_eq!(
            segments.len(),
            4,
            "the unsupported turn contributes no path"
        );
        assert_eq!(
            segments.first().unwrap().path,
            "M 29.52 0.00 V 10.00",
            "upper rail ends at the capsule entry"
        );
        assert_eq!(
            segments.get(2).unwrap().path,
            "M 29.52 24.00 V 34.00",
            "lower rail starts at the capsule exit"
        );
    }
}
