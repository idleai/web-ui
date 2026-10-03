//! Viewport-clipped SVG paths retain full relationship keys across repairs.

use history_geometry::{motion, viewport::pixel_integer};
use std::collections::BTreeMap;

use super::model::{Connection, ConnectionKind, Layout};
use super::viewport::{Viewport, number};

pub(super) const PITCH: f64 = 18.0;
pub(super) const INSET: f64 = 16.0;
const TURN_HEIGHT: f64 = 32.0;

type Point = (f64, f64);

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Path {
    pub(super) connection: Connection,
    pub(super) d: String,
    pub(super) born: Option<f64>,
    pub(super) delay: f64,
    pub(super) duration: f64,
}

pub(super) fn x(lane: usize) -> f64 {
    INSET + number(lane) * PITCH
}

pub(super) fn width(layout: &Layout) -> f64 {
    x(layout.max_lane) + 54.0
}

pub(super) fn visible(
    connections: &[Connection],
    layout: &Layout,
    viewport: &Viewport,
    births: &BTreeMap<String, f64>,
) -> Vec<Path> {
    let anchor = |key: &str| {
        let row = viewport.row(key)?;
        Some((
            x(*layout.lanes.get(key)?),
            row.top + viewport.row_height / 2.0,
        ))
    };
    let mut paths = Vec::new();
    let mut segments = Vec::new();
    for connection in connections {
        let source = connection.source.item().and_then(anchor);
        let target = connection.target.item().and_then(anchor);
        let Some(fallback) = source.or(target).or_else(|| anchor(&connection.owner)) else {
            continue;
        };
        let start = source.unwrap_or((fallback.0 + 14.0, fallback.1 - 12.0));
        let end = target.unwrap_or((fallback.0 + 14.0, fallback.1 + 12.0));
        let low = start.1.min(end.1) - 16.0;
        let high = start.1.max(end.1) + 16.0;
        if high < viewport.top - 120.0 || low > viewport.top + viewport.height + 120.0 {
            continue;
        }
        let d = if connection.kind == ConnectionKind::Causal {
            let route = connection
                .source
                .item()
                .zip(connection.target.item())
                .and_then(|(from, to)| layout.routes.get(&(from.into(), to.into())));
            route
                .and_then(|points| routed_path(points, viewport))
                .unwrap_or_else(|| causal_path(start, end))
        } else {
            side_path(start, end, width(layout) - 10.0)
        };
        let born = births.get(&connection.key).copied();
        if born.is_some() {
            segments.push((
                paths.len(),
                motion::Segment {
                    start: motion::Point {
                        x: pixel_integer(start.0),
                        y: pixel_integer(start.1),
                    },
                    end: motion::Point {
                        x: pixel_integer(end.0),
                        y: pixel_integer(end.1),
                    },
                },
            ));
        }
        paths.push(Path {
            connection: connection.clone(),
            d,
            born,
            delay: 0.0,
            duration: 240.0,
        });
    }
    let growth = motion::plan(
        &segments
            .iter()
            .map(|(_, segment)| *segment)
            .collect::<Vec<_>>(),
    );
    for ((index, _), timing) in segments.iter().zip(growth.segments) {
        if let Some(path) = paths.get_mut(*index) {
            path.delay = timing.delay;
            path.duration = timing.duration;
        }
    }
    paths
}

fn routed_path(points: &[(String, usize)], viewport: &Viewport) -> Option<String> {
    let points: Option<Vec<_>> = points
        .iter()
        .map(|(key, lane)| {
            viewport
                .row(key)
                .map(|row| (x(*lane), row.top + viewport.row_height / 2.0))
        })
        .collect();
    smooth_route(&points?)
}

fn smooth_route(points: &[Point]) -> Option<String> {
    if (points.first()?.1 - points.last()?.1).abs() < 0.5 {
        return None;
    }
    let mut rounded = points.to_vec();
    for (index, pair) in points.windows(2).enumerate() {
        let &(from_x, from_y) = pair.first()?;
        let &(to_x, to_y) = pair.get(1)?;
        if (from_x - to_x).abs() < 0.5 || (from_y - to_y).abs() >= 0.5 {
            continue;
        }
        // Each turn borrows at most half of either neighboring vertical run.
        // Adjacent turns can meet smoothly without crossing or moving a node.
        let before = index.checked_sub(1).and_then(|at| points.get(at));
        let after = points.get(index.saturating_add(2));
        rounded.get_mut(index)?.1 -= before.map_or(0.0, |point| turn(from_y - point.1));
        rounded.get_mut(index.saturating_add(1))?.1 +=
            after.map_or(0.0, |point| turn(point.1 - to_y));
    }
    let points = rounded;
    let (sx, sy) = *points.first()?;
    let mut path = format!("M {sx} {sy}");
    for pair in points.windows(2) {
        let from = *pair.first()?;
        let to = *pair.get(1)?;
        if (from.0 - to.0).abs() < 0.5 {
            line(&mut path, from, to);
        } else if (from.1 - to.1).abs() < 0.5 {
            return None;
        } else {
            curve(&mut path, from, to);
        }
    }
    Some(path)
}

fn turn(distance: f64) -> f64 {
    distance.signum() * (distance.abs() / 2.0).min(TURN_HEIGHT)
}

fn line(path: &mut String, from: Point, (tx, ty): Point) {
    use std::fmt::Write as _;
    if (from.0 - tx).abs() >= 0.5 || (from.1 - ty).abs() >= 0.5 {
        let _written = write!(path, " L {tx} {ty}");
    }
}

fn curve(path: &mut String, (sx, sy): Point, (tx, ty): Point) {
    use std::fmt::Write as _;
    let middle = sy.midpoint(ty);
    // Vertical handles keep both endpoint tangents parallel to the tracks.
    let _written = write!(path, " C {sx} {middle} {tx} {middle} {tx} {ty}");
}

fn causal_path((sx, sy): Point, (tx, ty): Point) -> String {
    if (sy - ty).abs() < 0.5 {
        return side_path((sx, sy), (tx, ty), sx.max(tx) + 12.0);
    }
    let mut path = format!("M {sx} {sy}");
    if (sx - tx).abs() < 0.5 {
        line(&mut path, (sx, sy), (tx, ty));
    } else {
        let approach = (sx, ty - turn(ty - sy));
        line(&mut path, (sx, sy), approach);
        curve(&mut path, approach, (tx, ty));
    }
    path
}

fn side_path((sx, sy): Point, (tx, ty): Point, side: f64) -> String {
    if (sy - ty).abs() < 0.5 {
        return format!(
            "M {sx} {sy} C {sx} {} {side} {} {side} {} C {side} {} {tx} {} {tx} {ty}",
            sy - 16.0,
            sy - 16.0,
            sy.midpoint(ty),
            ty + 16.0,
            ty + 16.0
        );
    }
    let bend = turn(ty - sy);
    let from = (side, sy + bend);
    let to = (side, ty - bend);
    let mut path = format!("M {sx} {sy}");
    curve(&mut path, (sx, sy), from);
    line(&mut path, from, to);
    curve(&mut path, to, (tx, ty));
    path
}
