//! Presentation data extending the shared session and resource fixtures.

use app_core::{
    history, projections, resources, sessions,
    workspace::{self, WorkspaceMode},
};

pub(super) fn sessions(mode: WorkspaceMode) -> Result<sessions::SessionSnapshot, String> {
    let mut snapshot = sessions::scripted::demo_snapshot(mode, "contributor-alice")
        .map_err(|error| error.message)?;
    if let Some(runner) = snapshot.sessions.first_mut() {
        runner.title = "main".into();
    }
    let runner = snapshot
        .sessions
        .first()
        .cloned()
        .ok_or("Missing runner fixture")?;
    for (index, title) in (30_u64..).zip([
        "feature/search",
        "research",
        "docs",
        "eval-run",
        "memo-base",
    ]) {
        let mut next = runner.clone();
        next.id = format!("session-{index}");
        next.title = title.into();
        next.history.item = format!("{index:064x}");
        snapshot.sessions.push(next);
    }
    for (id, name) in [("contributor-alex", "Alex"), ("contributor-dev", "Dev")] {
        snapshot.members.push(workspace::MemberInfo {
            contributor_id: id.into(),
            display_name: name.into(),
            revision: 1,
            role: workspace::MemberRole::Member,
            status: workspace::MemberStatus::Active,
        });
    }
    Ok(snapshot)
}

pub(super) fn resources(mode: WorkspaceMode) -> Result<resources::ResourceSnapshot, String> {
    let mut snapshot = resources::scripted::demo_snapshot(mode).map_err(|error| error.message)?;
    snapshot.context.contributor_id = "contributor-alice".into();
    if let Some(host) = snapshot.hosts.first_mut() {
        host.name = "MacBook Pro".into();
    }
    let mut host = snapshot
        .hosts
        .first()
        .cloned()
        .ok_or("Missing host fixture")?;
    host.id = "host-cluster".into();
    host.name = "prod-cluster".into();
    snapshot.hosts.push(host);
    if let Some(provider) = snapshot.providers.first_mut() {
        provider.name = "SGLang".into();
    }
    let mut provider = snapshot
        .providers
        .first()
        .cloned()
        .ok_or("Missing provider fixture")?;
    provider.id = "provider-external".into();
    provider.name = "OpenAI".into();
    provider.kind = resources::ModelProviderKind::External;
    provider.health.availability = resources::ResourceAvailability::Unknown;
    snapshot.providers.push(provider);
    Ok(snapshot)
}

pub(super) fn workspace(
    sessions: &sessions::SessionSnapshot,
    resources: &resources::ResourceSnapshot,
) -> workspace::WorkspaceSnapshot {
    let mut repositories = vec![workspace::RepositoryInfo {
        id: "repository-one".into(),
        name: "memos".into(),
        remote: None,
    }];
    if sessions.context.mode == WorkspaceMode::Managed {
        repositories.push(workspace::RepositoryInfo {
            id: "repository-two".into(),
            name: "web-ui".into(),
            remote: None,
        });
    }
    workspace::WorkspaceSnapshot {
        workspace: workspace::WorkspaceInfo {
            id: "workspace-one".into(),
            name: "memos".into(),
            chain: "chain-original".into(),
            revision: 1,
            mode: sessions.context.mode,
            repositories,
        },
        members: sessions.members.clone(),
        host_ids: resources.hosts.iter().map(|host| host.id.clone()).collect(),
        provider_ids: resources
            .providers
            .iter()
            .map(|provider| provider.id.clone())
            .collect(),
    }
}

pub(super) fn other_workspace() -> workspace::WorkspaceInfo {
    workspace::WorkspaceInfo {
        id: "workspace-two".into(),
        name: "Empty workspace".into(),
        chain: "chain-two".into(),
        revision: 1,
        mode: WorkspaceMode::Managed,
        repositories: Vec::new(),
    }
}

pub(super) fn presence(workspace_id: &str) -> workspace::PresenceSnapshot {
    let entries = if workspace_id == "workspace-one" {
        vec![workspace::PresenceEntry {
            connection_id: "alice-laptop".into(),
            contributor_id: "contributor-alice".into(),
            status: workspace::PresenceStatus::Online,
            repository_id: Some("repository-one".into()),
            branch: Some("main".into()),
            file: None,
            host_id: Some("host-shared".into()),
            summary: Some("Editing navigation".into()),
            observed_at_ms: 950,
            valid_until_ms: 2500,
        }]
    } else {
        Vec::new()
    };
    workspace::PresenceSnapshot {
        workspace_id: workspace_id.into(),
        as_of_ms: 1000,
        entries,
    }
}

pub(super) fn projections() -> projections::ProjectionSnapshot {
    projections::ProjectionSnapshot {
        version: 1,
        workspace_id: "workspace-one".into(),
        chain: "chain-original".into(),
        inputs: projections::ProjectionKind::ALL
            .into_iter()
            .map(|kind| projections::ProjectionInput {
                kind,
                freshness: projections::ProjectionFreshness {
                    status: if kind == projections::ProjectionKind::Task {
                        projections::FreshnessStatus::Current
                    } else {
                        projections::FreshnessStatus::Unknown
                    },
                    generated_at_ms: Some(1000),
                    checkpoint: None,
                },
                availability: if kind == projections::ProjectionKind::Task {
                    projections::ProjectionAvailability::Partial
                } else {
                    projections::ProjectionAvailability::Unavailable
                },
                total: (kind == projections::ProjectionKind::Task).then_some(24),
                rows: Vec::new(),
                gaps: vec![projections::ProjectionGap {
                    reference: None,
                    message: if kind == projections::ProjectionKind::Task {
                        "The navigation fixture supplies a task total without loading task rows"
                    } else {
                        "This projection is not supplied by the navigation fixture"
                    }
                    .into(),
                }],
            })
            .collect(),
    }
}

pub(super) fn history() -> Vec<history::ItemView> {
    (1_u64..)
        .zip([
            "memo2",
            "memo1: priors review",
            "ref ambientlight",
            "base ambientlight",
            "quest details",
            "fix typos in distribution…",
            "base memo",
            "few more thoughts",
        ])
        .map(|(index, title)| {
            let key = format!("{index:064x}");
            history::ItemView {
                key: key.clone(),
                blocks: Vec::new(),
                expanded: false,
                paging: history::Paging::default(),
                observations: vec![history::ObservationView {
                    record: history::RecordRef {
                        operation: key.clone(),
                        hash: format!("{:064x}", index.saturating_add(100)),
                    },
                    item: key,
                    kind: history::ActivityKind::Note,
                    author: Some("contributor-alice".into()),
                    recorder: None,
                    session: None,
                    turn: None,
                    time_ms: Some(index),
                    sequence: None,
                    parents: if index > 1 {
                        vec![format!("{:064x}", index.saturating_sub(1))]
                    } else {
                        Vec::new()
                    },
                    causes: Vec::new(),
                    original: None,
                    converter: None,
                    legacy: None,
                    detail: history::Detail::Other,
                    preview: Some(history::ContentText::new(title.into(), true)),
                    problem: None,
                }],
            }
        })
        .collect()
}
