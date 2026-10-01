//! Resolve identities without discarding observation-level relationships.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use app_core::history::{ActivityKind, Detail, Endpoint, ItemView, RecordRef, ViewModel};
use history_geometry::live::{GraphNode, LiveGraph, RowGeometry};

/// The relationship's recorded domain, also used for line styling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionKind {
    /// Physical causal parent of an immutable observation.
    Causal,
    /// Explicit logical cause, independent of physical ancestry.
    Cause,
    /// A Link observation's relation string, without inferred meaning.
    Link(String),
}

impl ConnectionKind {
    pub(super) const fn class(&self) -> &'static str {
        match self {
            Self::Causal => "causal",
            Self::Cause => "cause",
            Self::Link(_) => "link",
        }
    }

    pub(super) fn label(&self) -> &str {
        match self {
            Self::Causal => "Causal parent",
            Self::Cause => "Logical cause",
            Self::Link(relation) => relation,
        }
    }
}

/// Endpoint resolution is local to the loaded window; absence is not deletion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndpointState {
    /// Exactly one loaded item contains the requested identity.
    Loaded(String),
    /// No loaded observation or item resolves the recorded endpoint yet.
    Unloaded,
    /// Conflicting loaded observations name different logical items.
    Ambiguous,
    /// An explicitly repository-scoped Git object, outside the item scan.
    ExternalGit,
}

impl EndpointState {
    pub(super) fn item(&self) -> Option<&str> {
        match self {
            Self::Loaded(key) => Some(key),
            Self::Unloaded | Self::Ambiguous | Self::ExternalGit => None,
        }
    }
}

/// A recorded connection retains its exact source even when endpoints are absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    /// Stable key formed from full typed identities, not a row index or title.
    pub key: String,
    /// Kind of relationship; Links never become causal parents.
    pub kind: ConnectionKind,
    /// Exact observation and encoded-record digest carrying this connection.
    pub record: RecordRef,
    /// Item containing that observation, including Link items.
    pub owner: String,
    /// Original source endpoint, retaining its recorded domain.
    pub from: Endpoint,
    /// Original target endpoint, retaining its recorded domain.
    pub to: Endpoint,
    /// Current resolution of the source.
    pub source: EndpointState,
    /// Current resolution of the target.
    pub target: EndpointState,
}

impl Connection {
    pub(super) fn caption(&self) -> String {
        format!(
            "{}: {} → {}",
            self.kind.label(),
            endpoint_label(&self.from),
            endpoint_label(&self.to)
        )
    }

    pub(super) fn unresolved(&self) -> bool {
        self.source.item().is_none() || self.target.item().is_none()
    }
}

/// Inspectable graph input assembled exclusively from the app-core view.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraphSnapshot {
    /// Complete supplied items, retaining every cached observation and problem.
    pub items: Vec<ItemView>,
    /// All physical parents, logical causes and Link destinations.
    pub connections: Vec<Connection>,
}

impl GraphSnapshot {
    /// Resolve the loaded window without choosing a latest observation or alias.
    #[must_use]
    pub fn from_view(view: &ViewModel) -> Self {
        let items = view.items.clone();
        let resolver = Resolver::new(&items);
        let mut connections = BTreeMap::new();
        for item in &items {
            for observation in &item.observations {
                let mut add = |kind: ConnectionKind, from: Endpoint, to: Endpoint| {
                    let key = identity(&[
                        &observation.record.operation,
                        &observation.record.hash,
                        kind.class(),
                        kind.label(),
                        &endpoint_key(&from),
                        &endpoint_key(&to),
                    ]);
                    let connection = Connection {
                        source: resolver.resolve(&from),
                        target: resolver.resolve(&to),
                        key: key.clone(),
                        kind,
                        record: observation.record.clone(),
                        owner: item.key.clone(),
                        from,
                        to,
                    };
                    drop(connections.insert(key, connection));
                };
                for parent in &observation.parents {
                    add(
                        ConnectionKind::Causal,
                        Endpoint::Observation(observation.record.operation.clone()),
                        Endpoint::Observation(parent.clone()),
                    );
                }
                for cause in &observation.causes {
                    add(
                        ConnectionKind::Cause,
                        Endpoint::Observation(observation.record.operation.clone()),
                        Endpoint::Item(cause.clone()),
                    );
                }
                if let Detail::Link { from, relation, to } = &observation.detail {
                    for target in to {
                        add(
                            ConnectionKind::Link(relation.clone()),
                            from.clone(),
                            target.clone(),
                        );
                    }
                }
            }
        }
        Self {
            items,
            connections: connections.into_values().collect(),
        }
    }
}

struct Resolver<'a> {
    items: BTreeSet<&'a str>,
    observations: BTreeMap<&'a str, BTreeSet<&'a str>>,
}

impl<'a> Resolver<'a> {
    fn new(items: &'a [ItemView]) -> Self {
        let mut observations: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for item in items {
            for observation in &item.observations {
                let _: bool = observations
                    .entry(&observation.record.operation)
                    .or_default()
                    .insert(&item.key);
            }
        }
        Self {
            items: items.iter().map(|item| item.key.as_str()).collect(),
            observations,
        }
    }

    fn resolve(&self, endpoint: &Endpoint) -> EndpointState {
        match endpoint {
            Endpoint::Item(item) => {
                if self.items.contains(item.as_str()) {
                    EndpointState::Loaded(item.clone())
                } else {
                    EndpointState::Unloaded
                }
            }
            Endpoint::Observation(id) => match self.observations.get(id.as_str()) {
                Some(items) if items.len() > 1 => EndpointState::Ambiguous,
                Some(items) => items.first().map_or(EndpointState::Unloaded, |item| {
                    EndpointState::Loaded((*item).to_owned())
                }),
                None => EndpointState::Unloaded,
            },
            Endpoint::Git { .. } => EndpointState::ExternalGit,
        }
    }
}

pub(super) fn identity(parts: &[&str]) -> String {
    let mut key = String::new();
    for part in parts {
        let _written = write!(key, "{}:{part}", part.len());
    }
    key
}

fn endpoint_key(endpoint: &Endpoint) -> String {
    match endpoint {
        Endpoint::Observation(id) => identity(&["observation", id]),
        Endpoint::Item(id) => identity(&["item", id]),
        Endpoint::Git { repository, oid } => identity(&["git", repository, oid]),
    }
}

fn endpoint_label(endpoint: &Endpoint) -> String {
    match endpoint {
        Endpoint::Observation(id) => format!("observation {id}"),
        Endpoint::Item(id) => format!("item {id}"),
        Endpoint::Git { repository, oid } => format!("Git {repository}/{oid}"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LaneSource {
    Session(String),
    Recorder(String),
}

fn lane_source(item: &ItemView) -> Option<LaneSource> {
    let sessions: BTreeSet<_> = item
        .observations
        .iter()
        .filter_map(|record| record.session.as_ref())
        .collect();
    if !sessions.is_empty() {
        return (sessions.len() == 1)
            .then(|| {
                sessions
                    .first()
                    .map(|session| LaneSource::Session((*session).clone()))
            })
            .flatten();
    }
    let recorders: BTreeSet<_> = item
        .observations
        .iter()
        .filter_map(|record| record.recorder.as_ref())
        .collect();
    (recorders.len() == 1)
        .then(|| {
            recorders
                .first()
                .map(|recorder| LaneSource::Recorder((*recorder).clone()))
        })
        .flatten()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Node {
    key: String,
    parents: Vec<String>,
    time: u64,
    stream: Option<LaneSource>,
    git: bool,
}

impl GraphNode for Node {
    fn is_git(&self) -> bool {
        self.git
    }
    fn key(&self) -> &String {
        &self.key
    }
    fn node_key(&self) -> &String {
        &self.key
    }
    fn parents(&self) -> &Vec<String> {
        &self.parents
    }
    fn sort_time(&self) -> u64 {
        self.time
    }
    fn set_sort_time(&mut self, time: u64) {
        self.time = time;
    }
    fn muted(&self) -> bool {
        false
    }
    fn task_protected(&self) -> bool {
        true
    }
    fn same_source(&self, other: &Self) -> bool {
        self.stream
            .as_ref()
            .zip(other.stream.as_ref())
            .is_some_and(|(left, right)| left == right)
    }
}

type RoutePoints = Vec<(String, usize)>;

/// Browser-local retained lane state; app-core never receives rendering clocks.
#[derive(Debug, Default)]
pub(super) struct Layout {
    graph: LiveGraph<Node>,
    nodes: BTreeMap<String, Node>,
    pub(super) order: Vec<String>,
    pub(super) lanes: BTreeMap<String, usize>,
    pub(super) max_lane: usize,
    pub(super) routes: BTreeMap<(String, String), RoutePoints>,
    pub(super) warning: Option<String>,
}

impl Layout {
    pub(super) fn reconcile(&mut self, snapshot: &GraphSnapshot) {
        let mut parents: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for connection in &snapshot.connections {
            if connection.kind != ConnectionKind::Causal {
                continue;
            }
            if let (Some(source), Some(target)) =
                (connection.source.item(), connection.target.item())
                && source != target
            {
                let _: bool = parents
                    .entry(source.into())
                    .or_default()
                    .insert(target.into());
            }
        }
        let nodes: BTreeMap<_, _> = snapshot
            .items
            .iter()
            .map(|item| {
                let node = Node {
                    git: !item.observations.is_empty()
                        && item
                            .observations
                            .iter()
                            .all(|record| record.kind == ActivityKind::Commit),
                    key: item.key.clone(),
                    parents: parents
                        .remove(&item.key)
                        .unwrap_or_default()
                        .into_iter()
                        .collect(),
                    time: item
                        .observations
                        .iter()
                        .filter_map(|record| record.time_ms)
                        .max()
                        .unwrap_or(0),
                    stream: lane_source(item),
                };
                (item.key.clone(), node)
            })
            .collect();
        let removed: Vec<_> = self
            .nodes
            .keys()
            .filter(|key| !nodes.contains_key(*key))
            .cloned()
            .collect();
        let changed: Vec<_> = nodes
            .values()
            .filter(|node| self.nodes.get(node.key()) != Some(*node))
            .cloned()
            .collect();
        if removed.is_empty() && changed.is_empty() {
            return;
        }
        self.graph.edit(&removed, &[]);
        let changed = if self.warning.is_some() {
            nodes.values().cloned().collect()
        } else {
            changed
        };
        let scheduled = match self.graph.causal_updates(&changed) {
            Ok(nodes) => {
                self.warning = None;
                nodes
            }
            Err(problem) => {
                self.warning = Some(format!(
                    "Layout ordering: {problem}. All recorded connections remain available."
                ));
                changed
            }
        };
        self.graph.edit(&[], &scheduled);
        self.max_lane = self.graph.max_lane();
        self.routes = nodes
            .values()
            .flat_map(|node| {
                node.parents.iter().filter_map(|parent| {
                    self.graph
                        .route_points(&node.key, parent)
                        .map(|points| ((node.key.clone(), parent.clone()), points))
                })
            })
            .collect();
        self.nodes = nodes;
        self.order = self.graph.ordered_keys().cloned().collect();
        self.lanes = self
            .order
            .iter()
            .map(|key| {
                let mut row = RowGeometry::default();
                self.graph.decorate(key, 0, &mut row);
                (key.clone(), row.lane)
            })
            .collect();
    }
}
