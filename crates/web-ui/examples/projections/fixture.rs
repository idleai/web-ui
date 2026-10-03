//! Development-only presentation scenarios behind app-core's projection effects.

use app_core::projections::{
    Event, FreshnessStatus, ProjectionAvailability, ProjectionFreshness, ProjectionGap,
    ProjectionInput, ProjectionKind, ProjectionOutput, ProjectionReference, ProjectionRow,
    ProjectionSnapshot, ViewModel,
};
use app_core::{Core, Effect, Event as AppEvent, module::EffectError, subscriptions::Context};

pub(super) struct Fixture {
    core: Core,
    snapshot: ProjectionSnapshot,
    pending: Vec<Effect>,
}

impl Fixture {
    pub(super) fn new() -> Result<Self, String> {
        let mut fixture = Self {
            core: Core::new(),
            snapshot: snapshot(),
            pending: Vec::new(),
        };
        fixture.dispatch(Event::Connect(Context {
            provider: "standalone".into(),
            workspace: "projection-demo".into(),
            contributor: "alice".into(),
            chain: "demo-chain".into(),
        }))?;
        fixture.complete()?;
        Ok(fixture)
    }

    pub(super) fn view(&self) -> ViewModel {
        self.core.view().projections
    }

    pub(super) fn inspection(&self) -> String {
        format!("{:?}", self.core.view().history.selected)
    }

    pub(super) fn dispatch(&mut self, event: Event) -> Result<(), String> {
        let effects = self.core.process_event(AppEvent::Projections(event));
        self.accept(effects)
    }

    fn accept(&mut self, effects: Vec<Effect>) -> Result<(), String> {
        for effect in effects {
            match effect {
                Effect::Projection(request) => self.pending.push(Effect::Projection(request)),
                Effect::History(mut request) => {
                    let effects = self
                        .core
                        .resolve(
                            request.as_mut(),
                            Err(EffectError {
                                message:
                                    "History content is unavailable in this presentation fixture."
                                        .into(),
                            }),
                        )
                        .map_err(|error| error.to_string())?;
                    self.accept(effects)?;
                }
                Effect::Render(_)
                | Effect::HostInfo(_)
                | Effect::Workspace(_)
                | Effect::Subscription(_)
                | Effect::Session(_)
                | Effect::Resource(_)
                | Effect::Configuration(_) => {}
            }
        }
        Ok(())
    }

    pub(super) fn complete(&mut self) -> Result<(), String> {
        self.reply(&Ok(self.snapshot.clone()))
    }

    pub(super) fn fail(&mut self) -> Result<(), String> {
        self.reply(&Err(EffectError {
            message: "Provider is offline".into(),
        }))
    }

    fn reply(&mut self, result: &ProjectionOutput) -> Result<(), String> {
        let pending = std::mem::take(&mut self.pending);
        for effect in pending {
            if let Effect::Projection(mut request) = effect {
                let effects = self
                    .core
                    .resolve(request.as_mut(), result.clone())
                    .map_err(|error| error.to_string())?;
                self.accept(effects)?;
            }
        }
        Ok(())
    }

    pub(super) fn prepend(&mut self) -> Result<(), String> {
        if let Some(input) = self
            .snapshot
            .inputs
            .iter_mut()
            .find(|input| input.kind == ProjectionKind::Task)
        {
            input.rows.retain(|row| row.key != "task/late");
            input.rows.insert(
                0,
                row(
                    "task/late",
                    "A later arrival with a lower record ID",
                    Some("queued"),
                    1,
                ),
            );
            input.total = Some(u64::try_from(input.rows.len()).map_err(|error| error.to_string())?);
        }
        self.dispatch(Event::Refresh)?;
        self.complete()
    }

    pub(super) fn retract(&mut self) -> Result<(), String> {
        if let Some(selected) = self.view().selected {
            for input in &mut self.snapshot.inputs {
                if input.kind == selected.kind {
                    input.rows.retain(|row| row.key != selected.key);
                    if input.availability == ProjectionAvailability::Complete {
                        input.total = Some(
                            u64::try_from(input.rows.len()).map_err(|error| error.to_string())?,
                        );
                    }
                }
            }
        }
        self.dispatch(Event::Refresh)?;
        self.complete()
    }

    pub(super) fn empty(&mut self, availability: ProjectionAvailability) -> Result<(), String> {
        for input in &mut self.snapshot.inputs {
            input.rows.clear();
            input.availability = availability;
            input.total = if availability == ProjectionAvailability::Complete {
                Some(0)
            } else {
                None
            };
            input.freshness.status = FreshnessStatus::Unknown;
            input.gaps = if availability == ProjectionAvailability::Complete {
                Vec::new()
            } else {
                vec![ProjectionGap {
                    reference: None,
                    message: if availability == ProjectionAvailability::Partial {
                        "Some records are still being read."
                    } else {
                        "The projection provider is not connected."
                    }
                    .into(),
                }]
            };
        }
        self.dispatch(Event::Refresh)?;
        self.complete()
    }
}

fn reference(value: u8) -> ProjectionReference {
    ProjectionReference {
        observation: Some(format!("{value:02x}").repeat(32)),
        item: Some("a0".repeat(32)),
        record_hash: Some("b0".repeat(32)),
    }
}

fn row(key: &str, title: &str, status: Option<&str>, value: u8) -> ProjectionRow {
    ProjectionRow {
        key: key.into(), title: title.into(),
        summary: Some("Supplied details remain linked to their recorded sources.\nSecond line <literal text>.".into()),
        status: status.map(str::to_owned), labels: vec!["workspace".into(), "owner,infra".into()],
        sources: vec![reference(value)],
        related: vec![ProjectionReference { observation: None, item: Some("c0".repeat(32)), record_hash: None },
            ProjectionReference { observation: Some("d0".repeat(32)), item: None, record_hash: None }],
    }
}

fn input(kind: ProjectionKind, rows: Vec<ProjectionRow>) -> ProjectionInput {
    ProjectionInput {
        kind,
        freshness: ProjectionFreshness {
            status: FreshnessStatus::Current,
            generated_at_ms: Some(1_791_072_000_123),
            checkpoint: Some("snapshot/alpha".into()),
        },
        availability: ProjectionAvailability::Complete,
        total: Some(u64::try_from(rows.len()).unwrap_or(u64::MAX)),
        rows,
        gaps: Vec::new(),
    }
}

fn snapshot() -> ProjectionSnapshot {
    let mut queued = row("task/review", "Review retry policy", Some("queued"), 0x20);
    queued.labels = vec!["needs review".into(), String::new()];
    let tasks = input(
        ProjectionKind::Task,
        vec![
            row("task/checks", "Run workspace checks", Some("active"), 0x30),
            queued,
            row("task/no-status", "Confirm the next milestone", None, 0x40),
            row(
                "task/empty-status",
                "Inspect an empty provider status",
                Some(""),
                0x50,
            ),
        ],
    );
    let errors = input(
        ProjectionKind::Error,
        vec![
            row(
                "error/network",
                "Dependency fetch failed",
                Some("open"),
                0x60,
            ),
            row(
                "error/schema",
                "Review a conflicting record",
                Some("investigating"),
                0x70,
            ),
        ],
    );
    let mut triage = input(
        ProjectionKind::Triage,
        vec![row(
            "triage/retry",
            "Review retry policy",
            Some("unreviewed"),
            0x80,
        )],
    );
    triage.availability = ProjectionAvailability::Partial;
    triage.freshness.status = FreshnessStatus::Stale;
    triage.total = Some(17);
    triage.gaps.push(ProjectionGap {
        reference: Some(reference(0x81)),
        message: "This bounded read does not include all triage records.".into(),
    });
    let mut human = input(
        ProjectionKind::NeedInput,
        vec![row(
            "input/provider",
            "Choose a provider",
            Some("waiting"),
            0x90,
        )],
    );
    human.freshness.status = FreshnessStatus::Unknown;
    human.freshness.checkpoint = None;
    ProjectionSnapshot {
        version: 1,
        workspace_id: "projection-demo".into(),
        chain: "demo-chain".into(),
        inputs: vec![
            input(ProjectionKind::Activity, Vec::new()),
            tasks,
            errors,
            triage,
            human,
        ],
    }
}
