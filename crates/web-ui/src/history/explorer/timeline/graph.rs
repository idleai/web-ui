//! Paint native lane fragments without reconstructing relationships.

use app_core::history::timeline::{Geometry, Transition};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{Element, Props, component, dioxus_core, dioxus_elements, rsx};

mod paths;

pub(super) fn x(lane: u32, pitch: f64) -> f64 {
    (f64::from(lane) + 1.0) * pitch
}

pub(super) fn maximum(graph: &Geometry) -> u32 {
    graph
        .above
        .iter()
        .chain(&graph.below)
        .copied()
        .chain(
            graph
                .transitions
                .iter()
                .flat_map(|Transition(from, to)| [*from, *to]),
        )
        .chain(std::iter::once(graph.lane))
        .max()
        .unwrap_or(0)
}

#[component]
pub(super) fn GraphRow(
    graph: Geometry,
    width: f64,
    pitch: f64,
    pan: f64,
    highlight: Option<u32>,
    selected: bool,
    #[props(default = 28)] height: u8,
    #[props(default = false)] bundle: bool,
    #[props(default = true)] node: bool,
) -> Element {
    let lanes = || {
        graph
            .above
            .iter()
            .chain(&graph.below)
            .chain(std::iter::once(&graph.lane))
    };
    let extent = |lane: &u32| {
        if node && *lane == graph.lane {
            4.5
        } else {
            0.7
        }
    };
    let left = lanes().any(|lane| x(*lane, pitch) - extent(lane) < pan);
    let right = lanes().any(|lane| x(*lane, pitch) + extent(lane) > pan + width);
    let node_x = x(graph.lane, pitch) - pan;
    let middle = f64::from(height) / 2.0;
    let color = graph.lane % 10;
    let shape = if node {
        if bundle {
            paths::Shape::Bundle
        } else {
            paths::Shape::Activity
        }
    } else {
        paths::Shape::Detail
    };
    let segments = paths::segments(
        &graph,
        &paths::Options {
            pitch,
            pan,
            height,
            shape,
        },
    );
    rsx! {
        div { class: "idle-timeline-graph-cell", role: "gridcell",
            svg { class: "idle-timeline-graph", width: "{width}", height: "{height}", view_box: "0 0 {width} {height}",
                "aria-hidden": "true", "focusable": "false", "data-selected": selected.to_string(),
                for (index, segment) in segments.into_iter().enumerate() {
                    path { key: "segment-{index}", class: if segment.curve { "idle-timeline-bend" } else { "idle-timeline-rail" }, d: segment.path,
                        "data-highlight": (highlight == Some(segment.lane)).to_string(), "data-muted": segment.muted.to_string(),
                        style: "--rail:var(--idle-history-lane-{segment.lane % 10})" }
                }
                if node {
                    if bundle {
                        rect { class: "idle-timeline-capsule", x: "{node_x - 3.5}", y: "{middle - 10.0}", width: "7", height: "20", rx: "3.5",
                            style: "--rail:var(--idle-history-lane-{color})" }
                        for terminal in [middle - 7.0, middle + 7.0] {
                            circle { class: "idle-timeline-node idle-timeline-terminal", cx: "{node_x}", cy: "{terminal}", r: "3",
                                style: "--rail:var(--idle-history-lane-{color})" }
                        }
                    } else {
                        circle { class: "idle-timeline-node", cx: "{node_x}", cy: "{middle}", r: "4",
                            style: "--rail:var(--idle-history-lane-{color})" }
                    }
                }
                if graph.clipped_below {
                    path { class: "idle-timeline-continuation", d: "M {node_x} {height.saturating_sub(6)} v6", stroke_dasharray: "1 3",
                        title { "Routing continues through filtered activities" } }
                }
                if left { path { class: "idle-timeline-continuation", d: "M 7 3 L 3 7 L 7 11", title { "Lanes continue to the left; use graph pan" } } }
                if right { path { class: "idle-timeline-continuation", d: format!("M {} 3 L {} 7 L {} 11", width - 7.0, width - 3.0, width - 7.0), title { "Lanes continue to the right; use graph pan" } } }
            }
        }
    }
}
