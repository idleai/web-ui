//! Derivative and endpoint checks for forks, merges and short neighboring turns.

use super::{Point, causal_path, side_path, smooth_route};

fn point(tokens: &mut std::str::SplitWhitespace<'_>) -> Point {
    (
        tokens.next().unwrap().parse().unwrap(),
        tokens.next().unwrap().parse().unwrap(),
    )
}

fn segments(path: &str) -> Vec<[Point; 4]> {
    let mut tokens = path.split_whitespace();
    assert_eq!(tokens.next(), Some("M"), "a path starts at its source");
    let mut start = point(&mut tokens);
    let mut segments = Vec::new();
    while let Some(command) = tokens.next() {
        let (first, last, end) = if command == "C" {
            (point(&mut tokens), point(&mut tokens), point(&mut tokens))
        } else {
            assert_eq!(command, "L", "only lines and cubic curves are expected");
            let end = point(&mut tokens);
            let middle = (start.0.midpoint(end.0), start.1.midpoint(end.1));
            (middle, middle, end)
        };
        segments.push([start, first, last, end]);
        start = end;
    }
    segments
}

fn difference(to: Point, from: Point) -> Point {
    (to.0 - from.0, to.1 - from.1)
}

fn assert_near(actual: Point, expected: Point) {
    assert!(
        (actual.0 - expected.0).hypot(actual.1 - expected.1) < 0.001,
        "path must meet its node: {actual:?} != {expected:?}"
    );
}

fn assert_smooth(path: &str, from: Point, to: Point, monotone: bool) {
    let segments = segments(path);
    assert_near(*segments.first().unwrap().first().unwrap(), from);
    assert_near(*segments.last().unwrap().last().unwrap(), to);
    let mut previous: Option<Point> = None;
    for [start, first, last, end] in segments {
        let incoming = difference(first, start);
        let outgoing = difference(end, last);
        for tangent in [incoming, outgoing] {
            assert!(
                tangent.0.abs() < 0.001,
                "a join must follow the vertical track"
            );
            assert!(
                tangent.1.abs() > 0.001,
                "a join cannot have a zero derivative"
            );
        }
        if let Some((x, y)) = previous {
            assert!(
                (x * incoming.1 - y * incoming.0).abs() < 0.001,
                "successive segment tangents must be parallel"
            );
            assert!(
                x * incoming.0 + y * incoming.1 > 0.0,
                "successive segment tangents cannot reverse at a cusp"
            );
        }
        if monotone {
            let direction = (to.1 - from.1).signum();
            for (before, after) in [(start, first), (first, last), (last, end)] {
                assert!(
                    direction * (after.1 - before.1) >= 0.0,
                    "a causal turn cannot double back between adjacent rows"
                );
            }
        }
        previous = Some(outgoing);
    }
}

#[test]
fn forks_merges_and_retained_spines_join_with_vertical_tangents() {
    for corners in [
        vec![(34.0, 20.0), (34.0, 100.0), (16.0, 100.0)],
        vec![(16.0, 20.0), (34.0, 20.0), (34.0, 100.0)],
        vec![(16.0, 20.0), (16.0, 60.0), (34.0, 60.0), (34.0, 100.0)],
        vec![(16.0, 20.0), (52.0, 20.0), (52.0, 100.0), (34.0, 100.0)],
        vec![(16.0, 20.0), (34.0, 20.0), (34.0, 60.0), (52.0, 60.0)],
        vec![(16.0, 100.0), (16.0, 60.0), (34.0, 60.0), (34.0, 20.0)],
    ] {
        let path = smooth_route(&corners).unwrap();
        assert_smooth(
            &path,
            *corners.first().unwrap(),
            *corners.last().unwrap(),
            true,
        );
    }
}

#[test]
fn fallback_and_side_routes_remain_smooth_in_either_direction() {
    for (from, to) in [
        ((16.0, 20.0), (34.0, 60.0)),
        ((34.0, 60.0), (16.0, 20.0)),
        ((16.0, 20.0), (16.0, 60.0)),
    ] {
        assert_smooth(&causal_path(from, to), from, to, true);
        assert_smooth(&side_path(from, to, 78.0), from, to, true);
    }
}

#[test]
fn self_loops_close_without_a_cusp() {
    let from = (16.0, 20.0);
    for to in [from, (34.0, 20.0)] {
        let path = causal_path(from, to);
        assert_smooth(&path, from, to, false);
        let segments = segments(&path);
        let [start, first, _, _] = *segments.first().unwrap();
        let [_, _, last, end] = *segments.last().unwrap();
        assert!(
            difference(first, start).1 * difference(end, last).1 > 0.0,
            "the loop must enter and leave its nodes in the same vertical direction"
        );
    }
}
