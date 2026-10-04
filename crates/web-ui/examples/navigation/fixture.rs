//! Development-only navigation scenarios using the real app-core dispatcher.

mod data;

use super::SessionCreation;
use app_core::{
    Core, Effect, Event, ViewModel, history, module::EffectError, projections, resources, sessions,
    subscriptions::Context, workspace,
};

pub(super) struct Fixture {
    core: Core,
    workspace: workspace::WorkspaceSnapshot,
    sessions: sessions::SessionSnapshot,
    resources: resources::ResourceSnapshot,
    pending: Vec<Effect>,
}

impl Fixture {
    pub(super) fn new(mode: workspace::WorkspaceMode) -> Result<Self, String> {
        let sessions = data::sessions(mode)?;
        let resources = data::resources(mode)?;
        let workspace = data::workspace(&sessions, &resources);
        let mut fixture = Self {
            core: Core::new(),
            workspace,
            sessions,
            resources,
            pending: Vec::new(),
        };
        fixture.dispatch(Event::Workspace(workspace::Event::Load))?;
        fixture.dispatch(Event::Workspace(workspace::Event::SelectWorkspace(
            "workspace-one".into(),
        )))?;
        Ok(fixture)
    }

    pub(super) fn view(&self) -> ViewModel {
        let mut view = self.core.view();
        // Graph records are presentation fixtures. Selection remains in app-core.
        if view.history.chain.as_deref() == Some("chain-original") {
            view.history.items = data::history();
        }
        view
    }

    pub(super) fn creation(&self) -> SessionCreation {
        SessionCreation {
            context: self.sessions.context.clone(),
            mutation: sessions::SessionMutationId {
                request_id: "navigation-create-1".into(),
                expires_at_ms: 3000,
            },
            draft: sessions::SessionDraft {
                title: "New investigation".into(),
                host_id: "host-shared".into(),
                parent: None,
            },
        }
    }

    pub(super) fn dispatch(&mut self, event: Event) -> Result<(), String> {
        let connect = matches!(&event, Event::Workspace(workspace::Event::SelectWorkspace(id)) if id == "workspace-one");
        let effects = self.core.process_event(event);
        self.accept(effects)?;
        if connect {
            self.dispatch(Event::Sessions(sessions::Event::Connect(
                self.sessions.context.clone(),
            )))?;
            self.dispatch(Event::Resources(resources::Event::Connect(
                self.resources.context.clone(),
            )))?;
            self.dispatch(Event::Projections(projections::Event::Connect(Context {
                provider: "fixture".into(),
                workspace: "workspace-one".into(),
                contributor: "contributor-alice".into(),
                chain: "chain-original".into(),
            })))?;
        }
        Ok(())
    }

    pub(super) fn pending_creations(&self) -> usize {
        self.pending.iter().filter(|effect| matches!(effect, Effect::Session(request) if matches!(request.operation.action, sessions::SessionAction::Mutate { mutation: sessions::SessionMutation::Create(_), .. }))).count()
    }

    pub(super) fn disconnect(&mut self) -> Result<(), String> {
        self.pending.clear();
        self.dispatch(Event::Workspace(workspace::Event::Disconnect))
    }

    pub(super) fn expire(&mut self) -> Result<(), String> {
        self.dispatch(Event::Resources(resources::Event::AdvanceClock(4000)))?;
        self.dispatch(Event::Workspace(workspace::Event::Tick(4000)))
    }

    pub(super) fn fail_resources(&mut self) -> Result<(), String> {
        let effects = self
            .core
            .process_event(Event::Resources(resources::Event::Refresh));
        for effect in effects {
            if let Effect::Resource(mut request) = effect {
                let effects = self
                    .core
                    .resolve(
                        request.as_mut(),
                        Err(resources::ResourceError {
                            code: resources::ResourceErrorCode::Unavailable,
                            message: "Resource provider is offline".into(),
                            retry: resources::ResourceRetryAdvice::Never,
                        }),
                    )
                    .map_err(|error| error.to_string())?;
                self.accept(effects)?;
            }
        }
        Ok(())
    }

    fn accept(&mut self, effects: Vec<Effect>) -> Result<(), String> {
        for effect in effects {
            let next = match effect {
                Effect::Workspace(mut request) => {
                    let result = match &request.operation {
                        workspace::WorkspaceOperation::List => {
                            workspace::WorkspaceResult::Directory(vec![
                                self.workspace.workspace.clone(),
                                data::other_workspace(),
                            ])
                        }
                        workspace::WorkspaceOperation::Snapshot { workspace_id, .. } => {
                            workspace::WorkspaceResult::Snapshot(
                                if workspace_id == "workspace-one" {
                                    self.workspace.clone()
                                } else {
                                    workspace::WorkspaceSnapshot {
                                        workspace: data::other_workspace(),
                                        members: Vec::new(),
                                        host_ids: Vec::new(),
                                        provider_ids: Vec::new(),
                                    }
                                },
                            )
                        }
                        workspace::WorkspaceOperation::Presence { workspace_id, .. } => {
                            workspace::WorkspaceResult::Presence(data::presence(workspace_id))
                        }
                    };
                    self.core.resolve(request.as_mut(), Ok(result))
                }
                Effect::Session(mut request)
                    if request.operation.action == sessions::SessionAction::Snapshot =>
                {
                    self.core.resolve(
                        request.as_mut(),
                        Ok(sessions::SessionResult::Snapshot(Box::new(
                            self.sessions.clone(),
                        ))),
                    )
                }
                Effect::Resource(mut request)
                    if request.operation.kind == resources::ResourceOperationKind::Snapshot =>
                {
                    self.core.resolve(
                        request.as_mut(),
                        Ok(resources::ResourceResult::Snapshot(Box::new(
                            self.resources.clone(),
                        ))),
                    )
                }
                Effect::Projection(mut request) => {
                    self.core.resolve(request.as_mut(), Ok(data::projections()))
                }
                Effect::History(mut request) => {
                    let result = match request.operation.action {
                        history::QueryAction::History { .. }
                        | history::QueryAction::Item { .. } => Ok(history::QueryResult::History(
                            history::HistoryPage::default(),
                        )),
                        history::QueryAction::Search { .. }
                        | history::QueryAction::OperationDetails { .. }
                        | history::QueryAction::Open { .. }
                        | history::QueryAction::Reconcile(_) => Err(EffectError {
                            message: "Record details are outside this navigation fixture".into(),
                        }),
                    };
                    self.core.resolve(request.as_mut(), result)
                }
                Effect::Render(_) => continue,
                other @ (Effect::HostInfo(_)
                | Effect::Subscription(_)
                | Effect::Session(_)
                | Effect::Resource(_)
                | Effect::Configuration(_)
                | Effect::Repository(_)) => {
                    self.pending.push(other);
                    continue;
                }
            }
            .map_err(|error| error.to_string())?;
            self.accept(next)?;
        }
        Ok(())
    }
}
