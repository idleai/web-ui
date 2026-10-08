//! Paint coalesced native lane fragments without reconstructing relationships.

use app_core::history::timeline::{Geometry, Transition};
#[cfg(debug_assertions)]
use dioxus::prelude::dioxus_signals;
use dioxus::prelude::{Element, Props, component, dioxus_core, dioxus_elements, rsx};

mod paths;

pub(super) fn x(lane: u32, pitch: f64) -> f64 {
    14.0 + f64::from(lane) * pitch
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
) -> Element {
    let mut lanes = graph.above.clone();
    lanes.extend(&graph.below);
    lanes.sort_unstable();
    lanes.dedup();
    let extent = |lane: &u32| {
        if *lane == graph.lane {
            if selected { 5.1 } else { 3.6 }
        } else if highlight == Some(*lane) {
            1.35
        } else {
            0.75
        }
    };
    let left = lanes
        .iter()
        .chain(std::iter::once(&graph.lane))
        .any(|lane| x(*lane, pitch) - extent(lane) < pan);
    let right = lanes
        .iter()
        .chain(std::iter::once(&graph.lane))
        .any(|lane| x(*lane, pitch) + extent(lane) > pan + width);
    let node_x = x(graph.lane, pitch) - pan;
    let color = graph.lane % 6;
    rsx! {
        div { class: "idle-timeline-graph-cell", role: "gridcell",
            svg { class: "idle-timeline-graph", width: "{width}", height: "{height}", view_box: "0 0 {width} {height}",
                "aria-hidden": "true", "focusable": "false",
                for lane in lanes {
                    {
                        let rail_x = x(lane, pitch) - pan;
                        let (top, bottom) = paths::rail(&graph, lane, height);
                        let muted = (top == height / 2 || graph.muted_above.contains(&lane))
                            && (bottom == height / 2 || graph.muted_below.contains(&lane));
                        let highlighted = highlight == Some(lane);
                        rsx! { if top != bottom {
                            path { key: "rail-{lane}", class: "idle-timeline-rail", d: "M {rail_x} {top} V {bottom}",
                                "data-highlight": highlighted.to_string(), "data-muted": muted.to_string(),
                                style: "--rail:var(--idle-history-lane-{lane % 6})" }
                        } }
                    }
                }
                for Transition(from, to) in &graph.transitions {
                    path { key: "bend-{from}-{to}", class: "idle-timeline-bend", d: paths::bend(&graph, &Transition(*from, *to), pitch, pan, height),
                        "data-highlight": (highlight == Some(*from) || highlight == Some(*to)).to_string(),
                        "data-muted": graph.muted_transitions.contains(&Transition(*from, *to)).to_string(),
                        style: "--rail:var(--idle-history-lane-{paths::color_lane(&graph, *from, *to) % 6})" }
                }
                circle { class: "idle-timeline-node", cx: "{node_x}", cy: "{height / 2}", r: if selected { "4.5" } else { "3" },
                    style: "--rail:var(--idle-history-lane-{color})", "data-selected": selected.to_string() }
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
