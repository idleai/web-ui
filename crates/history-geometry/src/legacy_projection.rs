//! Geometry for shared history projections and lazily laid out activity views.

use crate::layout::{GraphLayout, GraphRow, LayoutContext, compute_lane_assignment};
use editchain_project::activity_view::ActivityView as CoreActivityView;
use editchain_project::taxonomy::ChainState;
use editchain_project::{HistoryNode, HistoryProjection, NodeKey, ResolvedGraph};
use std::{collections::HashMap, ops::Deref, sync::OnceLock};

/// Build visual geometry from a resolved semantic graph.
pub trait GraphGeometry {
    /// Retain the complete edge table for windowed geometry queries.
    #[must_use]
    fn layout_context(&self) -> LayoutContext;
    /// Assign lanes without constructing edges.
    #[must_use]
    fn lane_assignment(&self) -> Vec<GraphRow>;
    /// Build all lane and edge geometry.
    #[must_use]
    fn layout(&self) -> GraphLayout;
}

impl GraphGeometry for ResolvedGraph {
    /// Build geometry from the finalized edge table.
    fn layout_context(&self) -> LayoutContext {
        let input = LayoutInput::new(self);
        LayoutContext::new_with_chain_state(
            &input.keys,
            &|key| input.parents(key),
            &|key| input.is_git(key),
            &|key| {
                input
                    .rows
                    .get(key)
                    .map_or(ChainState::Active, |row| row.state)
            },
        )
    }

    /// Assign lanes without constructing full edge geometry.
    fn lane_assignment(&self) -> Vec<GraphRow> {
        let input = LayoutInput::new(self);
        compute_lane_assignment(&input.keys, &|key| input.parents(key), &|key| {
            input.is_git(key)
        })
    }

    /// Build the complete layout for this graph.
    fn layout(&self) -> GraphLayout {
        let context = self.layout_context();
        let edges = context.edges_for_window(0, self.keys().len());
        GraphLayout {
            rows: context.lanes,
            edges,
        }
    }
}

/// Geometry operations over the shared history projection's stable row order.
pub trait ProjectionGeometry {
    /// Lay out the complete history in canonical display order.
    #[must_use]
    fn graph_layout(&self) -> GraphLayout;
    /// Lay out a caller-supplied ordered row selection.
    #[must_use]
    fn graph_layout_filtered(&self, sorted: &[HistoryNode]) -> GraphLayout;
    /// Assign lanes to a caller-supplied ordered row selection.
    #[must_use]
    fn lane_assignment_filtered(&self, sorted: &[HistoryNode]) -> Vec<GraphRow>;
    /// Retain windowed geometry for a caller-supplied ordered row selection.
    #[must_use]
    fn layout_context(&self, sorted: &[HistoryNode]) -> LayoutContext;
}

impl ProjectionGeometry for HistoryProjection {
    /// Compute the graph layout for rendering unified history.
    ///
    /// The layout is computed over the same canonical topologically-sorted node
    /// list as [`HistoryProjection::nodes`]/[`HistoryProjection::window`], so layout row indices always
    /// correspond to window row positions. Every edge's parent appears below its
    /// child (newest-first).
    fn graph_layout(&self) -> GraphLayout {
        self.graph_layout_filtered(&self.nodes())
    }

    /// Compute the graph layout over a pre-sorted node list.
    ///
    /// `sorted` must be in the same canonical newest-first order as the rows the
    /// webview renders (i.e. from [`HistoryProjection::nodes`], possibly filtered), so
    /// layout row indices correspond to window row positions. Every edge's parent
    /// appears below its child.
    fn graph_layout_filtered(&self, sorted: &[HistoryNode]) -> GraphLayout {
        self.resolved_graph(sorted).layout()
    }

    /// Assign lanes for this exact ordered graph, without edge geometry.
    fn lane_assignment_filtered(&self, sorted: &[HistoryNode]) -> Vec<GraphRow> {
        self.resolved_graph(sorted).lane_assignment()
    }

    /// Build reusable geometry from the same finalized typed graph as rows.
    fn layout_context(&self, sorted: &[HistoryNode]) -> LayoutContext {
        self.resolved_graph(sorted).layout_context()
    }
}

/// Shared activity data with browser-owned, lazily cached graph geometry.
#[derive(Debug)]
pub struct ActivityView<R> {
    view: CoreActivityView<R>,
    layout: OnceLock<ActivityLayout>,
}

#[derive(Debug)]
struct ActivityLayout {
    context: LayoutContext,
    max_lane: usize,
}

impl<R> From<CoreActivityView<R>> for ActivityView<R> {
    fn from(view: CoreActivityView<R>) -> Self {
        Self {
            view,
            layout: OnceLock::new(),
        }
    }
}

impl<R> Deref for ActivityView<R> {
    type Target = CoreActivityView<R>;
    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl<R> ActivityView<R> {
    /// Existing geometry, absent until requested after first paint.
    #[must_use]
    pub fn layout(&self) -> Option<&LayoutContext> {
        self.layout.get().map(|layout| &layout.context)
    }

    /// Build graph geometry once, without changing rows or coordinates.
    #[must_use]
    pub fn ensure_layout(&self) -> &LayoutContext {
        &self
            .layout
            .get_or_init(|| {
                let context = self.graph().layout_context();
                let max_lane = context.lanes.iter().map(|row| row.lane).max().unwrap_or(0);
                ActivityLayout { context, max_lane }
            })
            .context
    }

    /// Highest assigned lane, or zero before geometry is requested.
    #[must_use]
    pub fn max_lane(&self) -> usize {
        self.layout.get().map_or(0, |layout| layout.max_lane)
    }
}

/// The geometry engine treats formatted labels as opaque keys. It receives
/// one converted table and never parses them back into domain identities.
struct LayoutInput {
    keys: Vec<String>,
    rows: HashMap<String, LayoutRow>,
}

struct LayoutRow {
    parents: Vec<String>,
    state: ChainState,
    is_git: bool,
}

impl LayoutInput {
    fn new(graph: &ResolvedGraph) -> Self {
        let keys = graph.keys().iter().map(ToString::to_string).collect();
        let rows = graph
            .keys()
            .iter()
            .map(|key| {
                (
                    key.to_string(),
                    LayoutRow {
                        parents: graph
                            .parents(*key)
                            .iter()
                            .map(ToString::to_string)
                            .collect(),
                        state: graph.chain_state(*key),
                        is_git: matches!(key, NodeKey::Git(_)),
                    },
                )
            })
            .collect();
        Self { keys, rows }
    }

    fn parents(&self, key: &str) -> Vec<String> {
        self.rows
            .get(key)
            .map_or_else(Vec::new, |row| row.parents.clone())
    }

    fn is_git(&self, key: &str) -> bool {
        self.rows.get(key).is_some_and(|row| row.is_git)
    }
}
